# rumoca fix: structural parameters marked in Base Modelica

This implements the rumoca part of `REPORT.md` §5, items (1), (3) and (4), plus the
"snapshot test per probe" item. Item (2) is not done: consumed parameters are still
emitted, now marked, rather than dropped. Commits on this branch, on top of
`1e6a146d`:

| commit | what |
|---|---|
| `ad36019a` feat(dae) | Conditional-component conditions become a structural use (`StructuralParameterUse::ComponentCondition`), so their parameter is evaluable and non-tunable and WD001 is reported. An `Evaluate = true` parameter reports WD001 (`EvaluateAnnotation`). Flat variables carry `is_final`. |
| `f17a0dbb` feat(codegen) | `FlatTemplateContext { experiment, evaluated_parameters }` reaches Flat templates. `base_modelica.mo.jinja` writes `annotation(Evaluate = true)` on every evaluated parameter, for both targets. |
| `34d2a65f` test(codegen) | `suite_core::base_modelica_structural_parameters`: one test per probe (full model text plus exact WD001 messages), a scalar-target test and a recompile test. |

Regenerate: `./emit_marked.sh <rumoca>` writes `bmo-marked/` (base-modelica) and
`bmo-marked/scalar/` (base-modelica-scalar), with each compile's stderr in `<model>.log`.
`julia/probe_marked.jl` parses each file and solves the scalar form; its output is in
`julia/probe_marked.out`.

## Rule

A parameter gets `annotation(Evaluate = true)` when the DAE has it non-tunable and it is
not an initialization-computed parameter (`CalculatedParameter`: `fixed = false`, or an
initial-algorithm target). A `final` parameter whose binding is not evaluable is also
excluded, because it follows parameters that stay tunable. In practice that means:

- a parameter that sizes an array, bounds a for range, selects the branch of an unbalanced
  if-equation, or is read by a conditional component's condition;
- any parameter whose binding such a parameter reads (DAE closure);
- a parameter declared `Evaluate = true`;
- a `final` parameter with an evaluable binding.

`constant` stays `constant` with no annotation. All other parameters stay unmarked and
tunable. The annotation goes on the declaration after the description string, which is
where MCP-0031's grammar puts a component annotation:
`parameter Integer 'n' = 3 annotation(Evaluate = true);`. It is the full-Modelica
annotation carried over, following MCP-0031's rule that annotations stay annotations.

## Before / after per probe

"before" is `bmo/` (the scout's output, which the pre-change binary reproduces byte for
byte). "after" is `bmo-marked/`. Tunability comes from `--emit dae-json`.

| Probe | before: emitted parameter | after: emitted parameter | DAE tunable before → after | WD001 before | WD001 after |
|---|---|---|---|---|---|
| `ArraySize` | `parameter Integer 'n' = 3;` | `… 'n' = 3 annotation(Evaluate = true);` | false → false | §10.1 dimension, §8.3.3 range | unchanged |
| `ArrayModified` | `parameter Integer 's.n' = 4;` | `… 's.n' = 4 annotation(Evaluate = true);` | false → false | §10.1 | unchanged |
| `ForParamRange` | `parameter Integer 'm' = 2;` | `… 'm' = 2 annotation(Evaluate = true);` | false → false | §8.3.3 ×2 (two loops) | unchanged |
| `ForRange` | `constant Integer 'n' = 3;` `parameter Integer 'm' = 2;` | unchanged (constant stays constant, `m` is read by nothing structural) | `m` true → true | none | none |
| `IfEqSameSize` | `parameter Boolean 'fast' = true;` | unchanged (balanced, kept as a runtime `if`) | true → true | none | none |
| `IfEqUnbalanced` | `parameter Boolean 'withY' = true;` | `… 'withY' = true annotation(Evaluate = true);` | false → false | §8.3.4 | unchanged |
| `ParamSelect` | `parameter Integer 'kind' = 1 "…";` | unchanged | true → true | none | none |
| `CondComp` | `parameter Boolean 'useDamper' = true;` | `… = true annotation(Evaluate = true);` | **true → false** | **none** | **§4.4.5 "this component condition is decided at translation, so parameter useDamper cannot be set …"** |
| `CondCompOff` | `parameter Boolean 'useDamper' = false;` | `… = false annotation(Evaluate = true);` | **true → false** | **none** | **§4.4.5 (same)** |
| `EvaluateAnn` | `'p' = 2; 'q' = 3; 'r' = ('p' * 'q');` | `'p' = 2 annotation(Evaluate = true);` with `q` and `r` unmarked | `p` false; `q`, `r` true (unchanged) | **none** | **§18.3 "this declaration has annotation(Evaluate = true), so parameter p is evaluated at translation and cannot be set"** |
| `ConstFinal` | `constant Real 'c' = 2;` `parameter Real 'k' = 3;` | `c` unchanged; `'k' = 3 annotation(Evaluate = true);`; `tt` unmarked | `k` false, `tt` true (unchanged) | none | none (a `final` parameter cannot be modified anyway) |
| `Replaceable`, `ReplaceableB` | no parameters | unchanged | n/a | none | none |
| `TunableOnly` | `parameter Real 'tau' = 1;` | unchanged | true → true | none | none |

Equations, variables and the numeric results are unchanged for every probe. The only
textual change in each `.bmo` is the annotation on the lines listed above.
`bmo-marked/scalar/` follows the same pattern: in `ArraySize` the scalar parameter `n` is
marked once, next to `'x[1]'..'x[3]'`.

## BaseModelica.jl (`asinghvi17/BaseModelica.jl@bm-events-and-parser-fixes`, 1.10.1; MTK 11.45.5)

From `julia/probe_marked.out`:

- **The ANTLR parser accepts all 28 files** (14 array-form, 14 scalar). So does the Julia
  parser. Both were tested with `parse_package` (AST only).
- Building the system (`parse_basemodelica`) succeeds for every scalar file. For the array
  form it still fails on `ArraySize`, `ArrayModified`, `ForParamRange` and `ForRange`
  with `UnsupportedFeatureError: array variable … is not supported`. That is the same
  pre-existing limitation as `REPORT.md` §4 and is unrelated to the marker.
- Every scalar file solves to the same final state as in `julia/probe_bmo.out`, e.g.
  ArraySize `[0.31674, 0.43233, 0.63212]` and CondComp `[1.0, 0.36788]`.
- The importer parses the annotation and then ignores it: `n`, `useDamper` and the
  others are still plain tunable `@parameters`. Mapping a marked parameter to
  `tunable = false` or a `remake` guard is the BaseModelica.jl follow-up in `REPORT.md`
  §5; the marker it needs is now in the file.

## Verification

Run with Rust nightly-2026-02-27, all default features except `input-gamepad`:

| check | result |
|---|---|
| `suite_core` | 1804/1804 pass (16 new) |
| `suite_gates` | 19/19 pass |
| `architecture_hardening_test` | 252/253. The one failure is the known commit-message gate (`test_reachable_commit_messages_have_no_ai_attribution`) on the AI trailers. |
| `rumoca` lib + bins | 130/130 |
| lib tests of rumoca-phase-instantiate, -flatten, -dae, -codegen, rumoca-compile, rumoca-ir-flat, rumoca-ir-ast | all pass |
| clippy (`--all-targets` on the crates above and `rumoca`) | clean |
| `cargo fmt --check` | clean |

## Behaviour changes outside Base Modelica output

- **A parameter read by a conditional component's condition is now non-tunable and
  evaluable in the DAE** (`useDamper`: tunable true → false). That follows MLS §4.4.5 and
  is needed for the marker to be right, but every target is affected: FMI export and
  `rumoca-sim` read `is_tunable`, so such a parameter is now exported as non-tunable and
  simulation overrides handle it as a structural parameter instead of a tunable one
  (`rumoca-sim` `solve_lowering/overrides.rs`; this path was not exercised beyond
  `suite_core`). MSL models with `useHeatPort`,
  `useSupport` and similar conditions are affected when those parameters are ordinary
  (non-final). A `final` or constant condition is unaffected.
- **More WD001 warnings on stderr**: one per conditional component with a
  parameter-dependent condition, and one per non-final `Evaluate = true` parameter. MSL
  uses both widely, so compiling MSL models will print noticeably more warnings. No test
  in `suite_core` asserted on their absence.
- `render_flat_template_with_experiment` is renamed `render_flat_template_with_context`
  and takes a `FlatTemplateContext`. It is a public API of rumoca-phase-codegen and
  rumoca-compile; the CLI is its only in-tree caller.

## Not done / open

- `REPORT.md` §5 rumoca item (2), not emitting consumed parameters that nothing else reads,
  is not implemented. They are emitted and marked. Dropping them or turning them into
  `constant` needs the Base Modelica marker decision first.
- Whether `annotation(Evaluate = true)` is the marker MCP-0031 adopts is still open
  (`REPORT.md` §2). If it settles on `evaluatedParameter = true` instead, only the
  template's `evaluate_annotation` macro changes.
- `fixed = false` and other initialization-computed parameters are never marked, even when
  `Evaluate = true` is written on them, because their value is computed at
  initialization.
