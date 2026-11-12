# Structural parameters: Modelica → rumoca → Base Modelica → BaseModelica.jl → MTK / Dyad

Scope: read-only investigation. No source changed, nothing pushed. Probes in `probes/`, emitted Base Modelica in `bmo/` (array-preserving `base-modelica` target) and `bmo-scalar/` (`base-modelica-scalar`), flattened Modelica in `flat/`, Julia scripts and raw outputs in `julia/`. Regenerate with `./emit.sh <rumoca>` (rumoca built from this branch, nightly-2026-02-27, default features minus gamepad, as in `outputs/REPORT.md`).
Versions: rumoca 0.10.4 (this branch), BaseModelica.jl 1.10.1 (`asinghvi17/BaseModelica.jl@bm-events-and-parser-fixes`), ModelingToolkit 11.45.5, SciCompDSL (for `@mtkmodel`), Dyad CLI source map at `~/tools/dyad-lang/apps/cli`.

## TL;DR

1. **Modelica**: "structural" is not a language category. It is the set of parameters whose value the *translator* must know: array dimensions, conditional-component conditions, for-ranges and if/when conditions that change equation structure, plus parameters with `Evaluate=true` and anything feeding them. Modelica says nothing about *replaceable-class selection by parameter*; classes are chosen by `redeclare`, which is resolved at elaboration time.
2. **Base Modelica (MCP-0031)** removes most of the problem instead of annotating it: no conditional components, no non-constant array sizes outside functions, if-equations must be balanced, `when` under `if`/`for` must have translation-time-evaluable conditions. It has **no** `Evaluate` annotation (a draft `evaluatedParameter = true` exists only on an old side branch with the body "TODO"). So after lowering, every remaining `parameter` is by construction runtime-tunable, *except* that the spec never states what an importer must do with a parameter the exporter *did* evaluate.
3. **rumoca** evaluates structural parameters at flatten time and bakes the result in (array sizes, for ranges, selected if-branches, conditional components), but **still emits the now-dead `parameter` with its old value and no marker**. It does warn (`WD001`) on stderr for arrays, for-ranges and unbalanced if-equations, but not for conditional components or `Evaluate=true`. Internally the DAE already carries `is_tunable=false, evaluable=true` for these parameters; the Base Modelica emitter drops that.
4. **BaseModelica.jl** maps *every* `parameter` and `constant` to a plain tunable MTK `@parameters` (no metadata) and rejects array variables entirely, so only the `base-modelica-scalar` form loads. Changing a dead structural parameter with `remake` **silently does nothing** (probes below); changing a baked-in `Evaluate` parameter silently gives a *stale dependent* value. MTK has the right notion (`tunable=false`, and structural/constructor arguments in `@mtkmodel`); Dyad has first-class `structural parameter`, lowered to plain Julia values at component construction.
5. **Recommendation**: (a) rumoca: stop emitting dead structural parameters as free parameters, or mark them; (b) Base Modelica: standardise one marker; (c) BaseModelica.jl: map marked parameters to `tunable=false`, and warn when a parameter is referenced nowhere. Details and sizing at the end.

## 1. Modelica specification (MLS, `chapters/*.tex` of modelica/ModelicaSpecification, MCP/0031 tree; section numbers as rumoca cites them for MLS 3.7)

| Kind | Spec text | Effect |
|---|---|---|
| Variability | `constant` < `parameter` < `discrete` < continuous; "Parameter Expressions", `operatorsandexpressions.tex` §Variability of Expressions: constants, `parameter` variables, function inputs, and functions of those | A *parameter* only has to be constant during simulation; it is not required to be known at translation. |
| Array dimensions | `classes.tex` Component Declaration Static Semantics : "Array dimensions shall be scalar non-negative parameter expressions of type `Integer`, an enum/Boolean type, or `:`"; `arrays.tex` Array Declarations (§10.1) | The *size* must be fixed during translation, so any parameter an array dimension reads is structurally evaluated. The spec requires a parameter *expression*, not a constant, leaving parameter-sized arrays legal but implicitly structural. |
| Conditional component | `classes.tex` Conditional Component Declaration: the condition "must be a `Boolean` scalar expression, and must be a parameter expression … shall be evaluated at compile time" | Parameter is structural; the component's equations and variables exist or not. |
| if-equation | `equations.tex` If-Equations (§8.3.4): branches must have equal equation counts *unless* all conditions are parameter expressions | With parameter conditions and differing branches the parameter must be evaluated; with equal counts a tool may or may not. |
| for-equation range | `equations.tex` For-Equations (§8.3.3): range evaluated once; `when` inside `for`/`if` requires parameter-expression ranges/conditions | Range sizes change equation count ⇒ structural. |
| `Evaluate` | `annotations.tex` Annotations for Code Generation, `Evaluate = true`: "the model developer proposes to utilize the value for the symbolic processing. In that case, it is not possible to change the parameter value after symbolic pre-processing." Only meaningful on `parameter`; may sit on declaration, type, or base class | Explicit opt-in; also lets binding-cycle checks pass (`classes.tex` Declaration Equations: cycles allowed if they disappear when `Evaluate=true` parameters are evaluated). |
| `final` | `final` forbids further modification, nothing about evaluation | Not structural by itself; a tool may fold it. |
| `constant` | constant expressions are evaluated at translation | Always structural-capable; value is not simulation-tunable. |
| `fixed=false` parameters | `introduction.tex`: a tool may reject a model if parameters needed for structural analysis cannot be evaluated at translation (external functions, initial equations) | The reverse failure mode: "must evaluate but can't". |
| assertion level | revisions.tex: "Make assertion level a structural parameter" | Spec itself uses the word "structural" only here and in the unrelated structural-analysis sense. |
| Replaceable classes | no parameter-driven selection exists; `redeclare` is a modification resolved during instantiation. Parameter-driven *behaviour* switching is expressed with if-equations / conditional components, so it falls under the rows above. | |

## 2. Base Modelica (MCP-0031, branch `MCP/0031` @13a0e8a, `RationaleMCP/0031/`)

Cited lines are from `differences.md` unless noted.

- **Conditional components removed** (§"Conditional components", l.91-97): "Base Modelica does not have conditional components … All checks that apply to inactivated components … will need to be checked while generating Base Modelica." So the exporter evaluates the condition.
- **Array sizes** (§"Array size", l.528-562, §"Component declaration with non-constant array size", l.614-620): array size is part of the type, either *constant* (an `Integer` of constant variability) or *flexible* (only in functions). "Component declarations outside functions may only specify constant array sizes". The example `Real[2] a = fill(1.0, p)` with `parameter Integer p` is an **error: expression has flexible size** (l.552-554). The rationale: "In Modelica, array sizes with parameter variability outside of functions are somehow allowed … but the semantics are not defined." So a Base Modelica array dimension must be a literal/constant after lowering; a `parameter Integer n` can still exist but cannot size a component. `constsize(...)` (l.566-604, `constsize.md`) asserts sizes in functions.
- **if-equations** (§"Unbalanced if-equations", l.70-89): all branches must have equal equation size, because "Base Modelica is designed to avoid such implicit evaluation of parameters". Hence a parameter-conditioned if-equation survives as a runtime-tunable conditional.
- **when / for** (l.153-171): `when` in `if`/`for` is not allowed; the exporter assumes "these conditions and ranges should be possible to evaluate during translation, allowing an if-equation to be reduced to one of its branches, or a for-equation to be unrolled".
- **Constant vs parameter expressions** (l.472-505): constant expressions may only call `pure constant` functions "ensur[ing] that a constant expression can always be evaluated to a value at translation time. A function call that is not a constant expression must not be evaluated before simulation starts."
- **Parameters stay parameters** (`Base-Modelica-requirements.md`, "All variabilities, but constant evaluation of parameters is not allowed. (For example, this guarantees that all parameters will remain parameters if a Base Modelica model is exported to an FMU for Model Exchange.)"). The same list asks for "List of all `parameter` variables that were treated as `constant` due to use in parameter expressions, the `Evaluate=true` annotation, or subject to constant evaluation during lowering", and a reviewer comment proposes `Evaluate=true`. This is a requirement, not adopted text.
- **final** (§"Final modification", l.771-776): final parameter equations become *initial equations*; parameters keep `parameter` and "cannot be modified after translation" is not expressed.
- **Initialization** (l.778-): a parameter with a declaration equation is `fixed=true` and its right-hand side "can be overridden during initialization (that is, after translation)". That is the tunability contract for ordinary parameters.
- **Annotations** (`annotations.md`): the standard annotations kept are code-generation ones (`Inline`, `smoothOrder`, …) and `HideResult`, `choices`; **`Evaluate` is not in the list**. Branch `MCP/0031+evaluated-parameter` (commit be05f17, 2022) adds `evaluatedParameter = true – TODO` and nothing else. MoM 2025-01-13 ("Henrik: just look for evaluate=true") and 2025-03-10 ("Lowering of parameters being evaluated treated as constant, marked by an annotation") show the group discussing it; no later MoM (through 2026-03-05) records a decision.
- **Replaceable / class selection**: none in Base Modelica (flat, no `replaceable`/`redeclare`); resolved by the exporter.

**What can stay runtime-tunable vs must be fixed at flattening.** Fixed at flattening: component existence, array dimensions of non-function components, for-ranges and `when`-guarding conditions, class redeclarations, constants. Tunable after translation: every remaining `parameter` (including those only read by balanced if-equation conditions or by ordinary expressions). **Gap**: nothing says which exported parameters were *already* consumed (a dead `n`), so an importer cannot tell a tunable `k` from an inert `n`.

## 3. rumoca (this branch)

Pipeline note: the flatten/DAE phase classifies parameters. `crates/rumoca/tests/suite_core/structural_parameters.rs` documents it: "An array dimension and a for-equation range are fixed at translation, so an ordinary parameter either reads is structural: DAE construction records it evaluable, with every parameter its binding reads, and warns (WD001) at the use". `crates/rumoca-ir-dae/src/model.rs:365-368`: `is_tunable`, and `evaluable` for "a `final` or `Evaluate=true` parameter or constant whose declaration binding is evaluable". Dumping `--emit dae-json` confirms: `EvaluateAnn.p` → `is_tunable=false, evaluable=true` (`q`, `r` tunable); `ArraySize.n` → non-tunable, evaluable; `ConstFinal.k` (final) evaluable, `tt` tunable. **The Base Modelica template (`crates/rumoca-phase-codegen/src/templates/base-modelica`, `codegen/base_modelica.rs`) does not read either flag** (no match for "structural"/"evaluat").

Per kind (probe → emitted output below):

| Kind | Probe | Evaluated & baked in? | `parameter` still emitted? | Dependency on parameter lost? | Warning |
|---|---|---|---|---|---|
| Array size `Real x[n]` | `ArraySize`, `ArrayModified` (`Sub s(n=4)`) | Yes: `x[3]`, loop `1:3` | Yes, `parameter Integer 'n' = 3`, unused | Yes | WD001 (§10.1) and WD001 on the for range |
| for range (param) | `ForParamRange` | Yes: `1:2`, `3:3` | Yes, `m` unused | Yes | WD001 (§8.3.3) |
| for range (constant) | `ForRange` | Yes | `constant n` kept, `m` unused (never structural) | n/a | none |
| if-equation, equal branches | `IfEqSameSize` | **No**: kept as `0 = if fast then (der(x)-2) else (der(x)-1)` | Yes, used | **Kept**, tunable | none (correct) |
| if-equation, parameter-selected | `ParamSelect` (`kind==1`) | No, same shape | Yes, used | Kept | none (correct) |
| if-equation, unbalanced | `IfEqUnbalanced` | **Yes**: branch chosen (`withY=true` ⇒ `der(y)=x`), the other dropped | Yes, unused | Lost | WD001 (§8.3.4) |
| Conditional component | `CondComp` / `CondCompOff` | Yes: `dmp.*` present iff `useDamper` | Yes, `useDamper` kept, unused | Lost | **none** |
| `Evaluate=true` | `EvaluateAnn` | Not folded; annotation ignored, `p`,`q`,`r=p*q` emitted as ordinary parameters | Yes | Kept as a binding | none |
| `final` / `constant` | `ConstFinal` | `constant c` stays `constant`; `final parameter k` emitted as plain `parameter` | Yes | Kept | none |
| Replaceable class | `Replaceable`, `ReplaceableB` (`redeclare model L = LawB`) | Yes (`law.f = 2*law.u`) | n/a | n/a | none |
| Ordinary tunable | `TunableOnly` | No | Yes | Kept | none |

Emitted Base Modelica (`bmo/`), abridged to the model body:

`ArraySize`
```modelica
model 'ArraySize'
  parameter Integer 'n' = 3;
  Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));
equation
  for i in 1:3 loop
    der('x'[i]) = ((-(i * 'x'[i])) + 1);
  end for;
end 'ArraySize';
```

`ArrayModified`
```modelica
model 'ArrayModified'
  parameter Integer 's.n' = 4;
  Real 's.x'[4](start = fill(1, 4), fixed = fill(true, 4));
equation
  der('s.x') = (-'s.x');
end 'ArrayModified';
```

`ForParamRange`
```modelica
model 'ForParamRange'
  parameter Integer 'm' = 2;
  Real 'x'[3](start = fill(0, 3), fixed = fill(true, 3));
equation
  for i in 1:2 loop
    der('x'[i]) = i;
  end for;
  for i in 3:3 loop
    der('x'[i]) = 0;
  end for;
end 'ForParamRange';
```

`IfEqSameSize`
```modelica
model 'IfEqSameSize'
  parameter Boolean 'fast' = true;
  Real 'x'(start = 0, fixed = true);
equation
  0 = (if 'fast' then (der('x') - 2) else (der('x') - 1));
end 'IfEqSameSize';
```

`IfEqUnbalanced`
```modelica
model 'IfEqUnbalanced'
  parameter Boolean 'withY' = true;
  Real 'x'(start = 0, fixed = true);
  Real 'y'(start = 0, fixed = true);
equation
  der('x') = 1;
  der('y') = 'x';
end 'IfEqUnbalanced';
```

`ParamSelect`
```modelica
model 'ParamSelect'
  parameter Integer 'kind' = 1 "1 = linear, 2 = quadratic";
  Real 'x'(start = 1, fixed = true);
equation
  0 = (if ('kind' == 1) then (der('x') - (-'x')) else (der('x') - (-('x' * 'x'))));
end 'ParamSelect';
```

`CondComp`
```modelica
model 'CondComp'
  parameter Boolean 'useDamper' = true;
  parameter Real 'dmp.d' = 1;
  Real 'dmp.v'(start = 1, fixed = true);
  Real 'y'(start = 0, fixed = true);
equation
  der('dmp.v') = (-('dmp.d' * 'dmp.v'));
  der('y') = 1;
end 'CondComp';
```

`CondCompOff`
```modelica
model 'CondCompOff'
  parameter Boolean 'useDamper' = false;
  Real 'y'(start = 0, fixed = true);
equation
  der('y') = 1;
end 'CondCompOff';
```

`EvaluateAnn`
```modelica
model 'EvaluateAnn'
  parameter Real 'p' = 2;
  parameter Real 'q' = 3;
  parameter Real 'r' = ('p' * 'q');
  Real 'x'(start = 0, fixed = true);
equation
  der('x') = 'r';
end 'EvaluateAnn';
```

`ConstFinal`
```modelica
model 'ConstFinal'
  constant Real 'c' = 2;
  parameter Real 'k' = 3;
  parameter Real 'tt' = 1;
  Real 'x'(start = 0, fixed = true);
equation
  der('x') = (('c' * 'k') * 'tt');
end 'ConstFinal';
```

`ReplaceableB`
```modelica
model 'ReplaceableB'
  Real 'law.f'(start = 0.0);
  Real 'law.u' = 'x';
  Real 'x'(start = 1, fixed = true);
equation
  'law.f' = (2 * 'law.u');
  der('x') = (-'law.f');
end 'ReplaceableB';
```


Scalar target for the array case (`bmo-scalar/ArraySize.bmo`) is the same with `x[1..3]` expanded and `n` still present.

Findings:
- **F1 (correctness-of-interchange)**: dead structural parameters are exported as free parameters. A downstream tool that exposes parameters (FMU, MTK `remake`, a GUI) offers `n`, `m`, `withY`, `useDamper` as tunable and changing them does nothing.
- **F2**: for a conditional component with `useDamper=true`, the output has `dmp.*` unconditionally and `useDamper` unreferenced; a reader cannot reconstruct the condition. MCP-0031 requires exactly this lowering, but with no marker it is indistinguishable from a tunable switch.
- **F3**: `Evaluate=true` is dropped at emission, even though the DAE has it (`evaluable=true`). It is not folded either, so there is no loss of tunability, but the intent is lost, and `final` is dropped the same way.
- **F4 (WD001 coverage)**: the warning exists but is on stderr only and misses conditional components and `Evaluate`.

## 4. BaseModelica.jl and MTK

Scripts: `julia/probe_bmo.jl` (output `julia/probe_bmo.out`), `julia/mtk_structural.jl` (`julia/mtk_structural.out`). Environment: `julia/Project.toml` with `BaseModelica` from `asinghvi17/BaseModelica.jl#bm-events-and-parser-fixes` (see `julia/setup.jl`).

**What BaseModelica.jl does** (`src/evaluator.jl`): `parameter` and `constant` both become `@parameters name::T` (l.264-267, 355-361); there is no `tunable=false`, no structural handling, and no use of annotations. Any array component errors at load (`UnsupportedFeatureError: array variable 'x' is not supported`, l.349-350), so the `bmo/` array probes fail outright and only `bmo-scalar/` loads. `parameter equation` and bindings become MTK bindings. Every probe parameter reports `istunable == true`, including `constant c` and the dependent `r`.

**`remake` experiments** (scalar form, from `julia/probe_bmo.out`; final state at t=1):

| Probe | remake | Before → after | Verdict |
|---|---|---|---|
| ArraySize | `n=5` | 3 unknowns, `[0.31674,0.43233,0.63212]` → identical, still 3 unknowns | **silently ignored**; array not resized |
| ForParamRange | `m=3` | `[2.0,1.0]` → identical | silently ignored |
| IfEqUnbalanced | `withY=false` | `[0.5,1.0]` → identical (still `der(y)=x`) | silently ignored; Modelica semantics would give `y=0` |
| CondComp | `useDamper=false` | `[1.0,0.36788]` → identical (damper still present) | silently ignored |
| ParamSelect | `kind=2` | `0.36788` → `0.5` | **works** (if-expression kept) |
| TunableOnly | `tau=2` | `0.36788` → `0.60653` | works |
| EvaluateAnn | `p=5` | `6.0` → `6.0`; `p=5` with `r=15` gives `15.0` | **stale**: `r=p*q` was baked at problem build, so changing `p` alone is silently wrong in MTK; for an `Evaluate=true` parameter Modelica says "not possible to change", so this is at least a documented hazard that nothing flags |

So yes: with the current chain a structural parameter is left as a tunable MTK parameter that silently gives wrong (unchanged) results when changed.

**MTK notions** (`julia/mtk_structural.out`):
1. `@parameters k = 1.0 [tunable = false]`: the parameter lives in a separate partition of `MTKParameters` (`tunable_parameters` lists only `tau`), is still settable via `remake` (k=2 → `0.13534`), but is excluded from tunable vectors used by optimisation/sensitivity. It is a *portion/non-differentiable* flag, **not** a structural flag: it does not resize anything.
2. A symbolic parameter cannot size an array: `@variables y(t)[1:n]` with symbolic `n` errors (`non-boolean (Num) used in boolean context`).
3. `@mtkmodel … @structural_parameters begin n = 3 end` (package SciCompDSL, `lib/SciCompDSL` in MTK; it is not loaded with `using ModelingToolkit` in MTK 11): `n` is an ordinary Julia value evaluated when the component is constructed. `Chain(n=3)` gives 3 unknowns, `Chain(n=5)` gives 5; `if useDamper` around `@components` removes the subcomponent (unknowns `[y, dmp₊v]` vs `[y]`). This is exactly the Modelica structural semantics, but it is a *model-construction-time* feature: the structural parameter does not exist in the compiled system, so it cannot be a `parameter` of the problem.
4. Not exercised: `mtkcompile(...; split=false)` concerns parameter storage layout only.

**Dyad** (source map of the CLI, `pkgs/parser/src/parser/dyad_parser.ts`, `pkgs/kernel/src/language/mtkir/component.ts`, `pkgs/kernel/src/codegen/mtk2/component.ts`, `pkgs/kernel/src/language/cardinality.ts`): grammar `[final] [structural] parameter NAME …` and the clock lattice constant → structural → parameter → discrete → continuous. Structural parameters are classified as "plain Julia values at construction time (structural parameters and in-scope type-level names) vs MTK symbolics (everything else)"; they become keyword arguments of the generated component function, `final` structural parameters become `name = default` locals, function-typed ones `nameˍ₋value`, and array/connector dimensions are "resolved against the structural parameters in scope". So Dyad's `structural parameter` ≙ MTK `@structural_parameters` ≙ Modelica "evaluated at translation", and non-structural Dyad parameters ≙ `@parameters` (tunable). I did not run the Dyad compiler; this is read from its source.

**How BaseModelica.jl should map.** Base Modelica parameters have three fates, and with MCP-0031 as written only two are distinguishable:
- tunable → `@parameters` (today's behaviour);
- baked/structural → *not a parameter of the problem*. The importer cannot infer it from the text alone, so it needs the marker from §5. When marked: array dimension / for-range / conditional-structure uses should be evaluated at build time (the importer already folds `for` ranges over constants, `functions.jl`), and the parameter either dropped or kept with `tunable=false` and a guard that rejects `remake` of it;
- `constant` → today a tunable parameter; should be a constant (inlined, or `tunable=false`).
Without a marker the importer can still compute a *conservative* heuristic: a parameter referenced by no equation, binding or start value is inert and should warn on `remake`.

## 5. Recommendation (per stage)

| Stage | Change | Size |
|---|---|---|
| Modelica | none; document in downstream notes that parameter-sized arrays, conditional-component conditions and for ranges are structural by the spec's "evaluated at compile time" wording, and that `Evaluate=true` freezes the value | S (docs) |
| Base Modelica (MCP-0031) | Finish the `evaluatedParameter` idea: adopt one annotation (e.g. standard `Evaluate = true` carried over from full Modelica, since the annotations guideline says "information kept in annotations in Modelica shall remain annotations") on every parameter the exporter consumed during lowering (array sizes, ranges, conditional components, selected branches, folded `Evaluate`). State the semantics: a marked parameter's value was used to derive structure and may not be changed after translation; an unmarked parameter must be tunable. Alternative: require exporters to drop consumed parameters, converting them to `constant`. Also add the exporter's "list of parameters treated as constant" from `Base-Modelica-requirements.md`. | M (spec text + review), S for the wording once agreed |
| rumoca | (1) In `base_modelica.rs`/template, read `Variable.evaluable`/`is_tunable` and emit `annotation(Evaluate = true)` on non-tunable parameters, or `constant` when the value is not intended to be exposed; (2) don't emit parameters that were consumed only structurally *and* have no remaining reader unless marked; (3) emit WD001-equivalent for conditional components and `Evaluate=true`; (4) keep `final` as `annotation(Evaluate=true)` or `final` decoration if the grammar allows. Add a snapshot test per probe in this directory. | M (template + flag plumbing + tests); (3) S |
| BaseModelica.jl | (1) Map `constant` to a real constant / `tunable=false`; (2) map marked structural parameters to `tunable=false` and, ideally, error on `remake` of them (a thin wrapper or a metadata + `remake` check); (3) warn at load for parameters read nowhere (catches today's rumoca output without any marker); (4) fix dependent-binding staleness (`r = p*q` after `remake(p)`), or document; (5) array variables: needed for the array-preserving target, currently unsupported, so size-parameter handling is moot there. | S for (1)-(3), M for (4), L for (5) |
| MTK | Nothing missing for scalar models. For array models the `@structural_parameters` mechanism is the correct vehicle, but it needs the importer to *generate model-construction code* (an `@mtkmodel`-like builder with kwargs), not a single `System`. Consider a documented `remake` guard for `tunable=false` that is meant as frozen. | S (guard); L (builder-style import) |
| Dyad | If Base Modelica models are to enter Dyad, map marked parameters to `structural parameter` declarations (final if they have bindings) and tunable ones to plain `parameter`. | M |

Suggested order: rumoca (1)+(3) and the Base Modelica marker decision first (they unblock everything), then BaseModelica.jl (1)-(3), which already removes the silent-wrong-answer failure with the unreferenced-parameter warning.

## Caveats

- MLS section numbers marked §x.y are those cited by rumoca for MLS 3.7; other citations are by section title and file because I did not build the spec PDF.
- Spec text read from `MCP/0031` @13a0e8a (generated 2026-08-18) fetched with `git clone --depth 1`; the `MCP/0031+evaluated-parameter` branch is from 2022 and Flat-Modelica-era.
- `remake` results use the scalar target because the array target cannot load; I did not test a hand-edited array `.bmo`.
- The Dyad compiler was not executed; only its source map was read.
- Julia `.CondaPkg` environment (ANTLR) was generated locally and is not committed.
