# Modelica → rumoca → Base Modelica (MCP-0031) → BaseModelica.jl → ModelingToolkit

> Follow-ups (experiment annotation, bound-parameter start, records, a
> re-emitted and revalidated model set) are in `REPORT-followups.md`.

Prototype report. Base: rumoca `main` @ `1b6cf9af` (v0.10.4). On 2026-11-11
`git ls-remote` and the GitHub API both still show `refs/heads/main` at
`1b6cf9af`, so there was nothing to rebase onto. The repository's
`pushed_at` of 20:23:47Z comes from a push to another branch
(`merge-train-620-622-623` and others), not `main`.

## Result at a glance

All 14 emitted files (7 models × {array-preserving, scalarized}) parse with
zero syntax errors in BaseModelica.jl's **ANTLR** parser, with stderr empty.
The `:julia` parser accepts 11 of 14. The 3 rejections come from two
BaseModelica.jl parser bugs, described under "Remaining gaps".

| model | target | ANTLR | `:julia` | → MTK system | simulated (Rodas5P) |
|---|---|---|---|---|---|
| SpringMassDamper | base-modelica | PASS | PASS | PASS | Success: `x(5) = -0.2873` (decay envelope e^(-1.25) ≈ 0.287) |
| RC circuit (connectors + `extends`) | base-modelica | PASS | PASS | PASS | Success: `C1.v(5) = 9.93262` = 10(1-e⁻⁵) exactly |
| BouncingBall (`when`/`reinit`/discrete) | base-modelica | PASS | PASS | PASS | **fails in BaseModelica.jl** (see gaps) |
| UserFunction (2 user functions) | base-modelica | PASS | FAIL (BM.jl bug 1) | FAIL: BM.jl has no user functions (`KeyError :cubicDrag`) | n/a |
| MSL `Translational.Examples.Oscillator` | base-modelica | PASS | PASS | PASS | Success to t=1 (792 points) |
| MSL `Analog.Examples.ChuaCircuit` | base-modelica | PASS | PASS | PASS | Success to t=100 |
| ArrayModel (arrays + `for`) | base-modelica | PASS | FAIL (BM.jl bug 2: `{...}`) | FAIL: BM.jl has no arrays | use scalar ↓ |
| ArrayModel | base-modelica-scalar | PASS | PASS | PASS | Success: `x[1](10) = 0.999955` = 1-e⁻¹⁰ |
| every other model | base-modelica-scalar | same as array-preserving form (they have no arrays) |||||

Full rows: `outputs/validation.tsv` (parse + MTK) and
`outputs/simulation.tsv` (simulation).

## What changed in rumoca

The changes extend the existing `base-modelica` target (its template, manifest
and README) rather than adding a parallel one. The one new target,
`base-modelica-scalar`, is a 5-line template that sets `scalarize = true` and
`{% include %}`s the same `base_modelica.mo.jinja` (published as
`base-modelica.bmo.jinja`). There is no copy of the template. Rumoca has no
per-target option mechanism, and a separate target name was the smallest way
to select scalarization from the CLI.

**Template** (`crates/rumoca-phase-codegen/src/templates/base-modelica/`):
- Starts with the `//! base 0.1.0` header, then `package 'M'`. The package
  holds the enumeration types (`type 'Pkg.E' = enumeration(...)`) and user
  functions, then `model 'M' "desc" … end 'M';` and `end 'M';`.
- Every declared component, function and enumeration type is quoted
  (`'R1.p.v'`, `'cubicDrag'(...)`, `'Types.Mode'.On`). `time` and loop
  indices stay bare.
- Declarations keep `parameter`, `constant` and `discrete`, plus top-level
  `input`/`output`. Their attributes are `quantity`, `unit`, `displayUnit`,
  `min`, `max`, `start`, `fixed`, `nominal` and `stateSelect`, followed by
  any binding and description.
- MCP-0031 strikes `each`, `final`, `public` and `protected` (checked against
  `grammar.md`/`differences.md` on the MCP/0031 branch and BaseModelica.jl's
  `BaseModelica.g4`). So scalar attributes on arrays are written
  `fill(v, dims)`, and function locals are plain declarations.
- Kept in the output:
  - equations, initial equations and assertions;
  - `when`/`elsewhen` chains with `reinit`, discrete assignments, element
    assignments, nested `if`, `assert`, `terminate` and multi-output calls;
  - `algorithm` and `initial algorithm` sections.
- User functions:
  - Inputs carry defaults, there are outputs and locals, and the body is an
    algorithm using `:=` with closed `if`/`for`/`while`/`when` blocks.
  - External functions keep their `external` clause; this path is untested.
  - `impure` is emitted.
  - Functions whose every call renders as a builtin (`Modelica.Math.sin` →
    `sin`) are not declared.
- Negative literals are parenthesized, `(-0.5)`, because the grammar has no
  signed primary. `noEvent`, `smooth` and `homotopy` are kept, not stripped.
- The output file is now `<Model>.bmo`; it used to be `<Model>_base.mo`.
- The events/algorithms capability refusals were removed from `target.toml`.

**Rust plumbing** (`crates/rumoca-phase-codegen/src/codegen/`):
- `expr_config.rs` gains four flags: `base_modelica`, `quote_functions`,
  `quote_types` and `scalarize`.
- `symbol_alloc.rs`:
  - `quote_identifier` builds Q-IDENTs, escaping `\` and `'`.
  - `names_contain` accepts a map or a template-built list.
  - Base Modelica name quoting, including enumeration literal paths, which go
    through the sanctioned `split_last_top_level` helper.
- `render_expr.rs` adds quoted function calls, parenthesized negative
  literals, kept `noEvent`/`smooth`/`homotopy`, and scalar element names
  (`'x[2]'`). In scalar mode it refuses whole-array references.
- `base_modelica.rs` (new) holds the Base Modelica statement renderer. It is
  separate because the shared Modelica-style statement path emits `=` instead
  of `:=` and drops `end if;` when there is no `else`; no template used that
  path, so it was left alone. The file also has the template helpers
  `quote_ident`, `is_builtin_alias`, `scalar_indices` and `array_element`.
- `base_modelica_scalarize.rs` (new) provides `unroll_family` and
  `scalarize_equation`:
  - Unmaterialized `for` families are unrolled from their template body:
    binders are substituted and constant subscripts folded. Without this the
    scalar output printed Flatten's placeholder rows (`der('x[3]') = 0.0`).
  - Whole-array elementwise equations (`der(z) = -(a .* z)`) are split per
    element. `.*` becomes `*` between scalars.
  - Matrix products, reductions over arrays, slices and calls taking arrays
    are refused with `cannot scalarize: …`, never guessed.
- `flat_family_loops.rs` loop rows now also carry their binder ranges.

**`--emit flat-mo`** (`crates/rumoca/src/cli.rs`): this was item 6, and it
was a small change. The command now prints a warning on stderr when the model
has `when` chains or algorithm sections:

```
[rumoca] warning: `--emit flat-mo` omits 1 when-equation chain(s) (including any reinit) and 0 algorithm section(s); …
```

It still prints the model, since it is a debug dump. The `flat-modelica`
*target* already refused such models; that refusal is unchanged.

**Docs**:
- target READMEs (`base-modelica`, new `base-modelica-scalar`);
- the targets table in `docs/user-guide/src/codegen/targets.md`.

## Tests (rumoca style)

- New `crates/rumoca/tests/suite_core/base_modelica_target.rs` (11 tests):
  - package layout and quoting, no `each`/`final`/`protected`;
  - connectors and `extends`;
  - `when`/`reinit`/`pre`;
  - user functions declared before the model;
  - algorithm sections;
  - arrays with `fill`;
  - the scalar target's declarations and equations;
  - unrolling of unmaterialized families;
  - refusal of matrix products;
  - enumeration package types;
  - recompiling the output through rumoca as `'M'.'M'`, for both targets.
- New `codegen/base_modelica_tests.rs` (3 unit tests): quoting/escaping,
  `scalar_indices`, and expression quoting.
- Updated existing tests:
  - `cli_target_acceptance`: `flat-modelica` must still refuse
    when/algorithm models; a new test checks that `base-modelica` renders them.
  - `cli_emit`: a new test for the `flat-mo` omission warning.
  - `acceptance_surface` and `template_target_ci`: rows for the new target
    and for the shared template name.
  - The interchange round-trip tests and OMC workshop tests now load the
    quoted path `'M'.'M'` for `base-modelica`.

Results (`--no-default-features` with every default feature except
`input-gamepad`, which needs libudev):

| suite | result |
|---|---|
| `suite_core` | 1784/1784 pass |
| `architecture_hardening_test` | 253/253 on the working tree; 252/253 once committed (the commit-message gate, see below) |
| `suite_gates` | 19/19 pass |
| `rumoca-phase-codegen` (lib + tests) | all pass |
| `suite_template_runtime` | 119/133 pass |
| clippy (`-p rumoca-phase-codegen -p rumoca --all-targets`) | clean |
| `cargo fmt` | clean |

The 14 `suite_template_runtime` failures are all environmental:
- 13 are jax/casadi tests: `ModuleNotFoundError: No module named 'jax'`;
- 1 is `fluid_onetank`: "requires pinned MSL 4.1.0" (needs the MSL cache).

The modelica-interchange and template-CI subsets pass 13/13. OMC is not
installed on this machine, so the OMC-gated checks returned early and the
OMC acceptance of the quoted `'M'.'M'` path is **unverified**.

## Remaining gaps

**BaseModelica.jl v1.10.1.** Rumoca's output is spec-conformant here; these
are worth reporting upstream after review.

1. **The `:julia` parser rejects indented algorithm statements.** Its
   `statement`/`if_statement`/`composition` rules have no whitespace
   skipping. Only `algorithm'y':='u';` (no whitespace before or between
   statements) parses; `algorithm\n    'y' := 'u';` does not. This is why
   UserFunction fails `:julia`. I did not distort the output to suit it.
2. **The `:julia` parser rejects any `{...}` array constructor**, e.g.
   `parameter Real 'tau'[3] = {1.0, 2.0, 3.0};`. `fill(...)` parses. This is
   why ArrayModel's array form fails `:julia`; the scalar target avoids it.
3. **`reinit` is silently ignored in MTK conversion.** `eval_when_equation`
   handles only `lhs = rhs`. A reinit-only bouncing ball simulates
   "Success" but falls through the floor: `h(3) = -43.1`, `v = -g·t`.
4. **`discrete` variables assigned in `when` are lost.** `bounces` and
   `tLast` give `Symbol bounces(t)/tLast(t) is not present in the system`
   at `ODEProblem` construction. So BouncingBall converts to an MTK system
   but cannot be simulated.
5. **No user functions** in the MTK conversion (`KeyError :cubicDrag`), and
   **no arrays** (its README says so). The scalar target works around arrays.

**Rumoca side, not done in this prototype:**
- Records:
  - Record types are not emitted as package-level `record` classes. A
    function with a record-typed parameter would reference an undeclared
    quoted type.
  - Inside function bodies, a record field reference renders as one quoted
    path (`'r.re'`), not as field access.
  - None of the test models hit either case.
- Clocked partitions (`partition`/`subpartition`) are not emitted.
- `annotation(experiment(...))` is not emitted, because Flat does not carry
  annotations. BaseModelica.jl's `create_odeproblem` therefore defaults to
  `tspan = (0, 1)`, and `simulate_bmo.jl` takes the stop time as an argument.
- Parameters show `start = <binding value>` next to the binding. Flatten
  stores it that way; it is valid but noisy.
- Model and package names use the flattened name (`'RCCircuit_Circuit'`,
  `'Modelica_Electrical_Analog_Examples_ChuaCircuit'`), as the old target's
  file names did.
- Re-reading Base Modelica that contains functions through rumoca's
  full-Modelica frontend fails. Its rule "public function components must be
  input/output" (MLS §12.2) rejects MCP-0031's prefix-less locals. Models
  without functions round-trip; this is covered by a test.
- A heterogeneous `fixed` on a multi-dimensional array is written as a flat
  `{...}` list (correct for 1-D only).
- The scalar target refuses non-elementwise array equations (matrix
  products, reductions, slices) rather than expanding them.

## Commit-message gate (action needed before upstreaming)

The factory's commit hook appends `Co-Authored-By: Claude …`, `Harness:`,
`Model:` and `Agent-*` trailers to every commit on this branch. Rumoca's
SPEC_0025 §6 forbids AI attribution in commit messages, and
`architecture_hardening_test::commit_messages::test_reachable_commit_messages_have_no_ai_attribution`
enforces it, so that one test fails on this branch: 252/253 pass. Before
opening the upstream PR, reword or squash these commits without those
trailers. No code change is needed.

## Branch note

The brief asked for branch `base-modelica-mcp0031`. The factory harness only
returns commits made on the task branch
`factory/5bbd3b95-6c02-4289-97c5-0b0f326c173e`, so the commits are there. To
get the requested name:

```sh
git branch base-modelica-mcp0031 factory/5bbd3b95-6c02-4289-97c5-0b0f326c173e
```

Nothing was pushed, and no PR, issue or comment was opened upstream.

## Reproduce

```sh
# rumoca (rustup nightly-2026-02-27; default features need libudev for gamepad)
cargo build --release -p rumoca --bin rumoca --no-default-features --features \
  scheduled-sim,scenario-config,input-keyboard,transport-udp,transport-zenoh,viewer-web,process-control,native-allocator,fmu-packaging,component-build
outputs/emit_all.sh target/release/rumoca <dir with Complex.mo, "Modelica 4.1.0", "ModelicaServices 4.1.0">

# BaseModelica.jl 1.10.1 (project env in outputs/julia; first run bootstraps conda ANTLR)
julia --project=outputs/julia -e 'using Pkg; Pkg.instantiate()'
julia --project=outputs/julia outputs/julia/validate_bmo.jl outputs/bmo/*.bmo outputs/bmo/scalar/*.bmo
julia --project=outputs/julia outputs/julia/simulate_bmo.jl outputs/bmo/SpringMassDamper.bmo:5 ...
```

Layout:
- `outputs/models/`: the five hand-written test models. The two MSL models
  come from the MSL v4.1.0 release zip.
- `outputs/bmo/`: `base-modelica` output.
- `outputs/bmo/scalar/`: `base-modelica-scalar` output.
- `outputs/julia/`: the validation/simulation scripts and their pinned Julia
  environment.
