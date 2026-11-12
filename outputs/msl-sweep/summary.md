# MSL 4.1.0 sweep: rumoca → Base Modelica → BaseModelica.jl → MTK → solve → compare

**46 of 534 MSL example models (8.6%) translate, simulate and match the MAP-LIB
reference results.** 80% of the examples never leave rumoca, and one rumoca bug
(function array-parameter dimensions) alone blocks 212 of them. Of the 107 models
rumoca does emit, 46 (43%) pass end to end.

All 46 passing models match the references to 1e-3 of signal range; the median
error is 7.5e-6. Four more models finish with `Success` but give wrong answers;
these are silent failures (see "Surprises").

## What was run

| piece | version |
|---|---|
| rumoca | this branch at `865daa79` (= `asinghvi17/rumoca` `base-modelica-mcp0031`), release build, nightly-2026-02-27, default features minus `input-gamepad` |
| MSL | 4.1.0 release zip (`Complex.mo`, `Modelica 4.1.0`, `ModelicaServices 4.1.0` as `--source-root`) |
| BaseModelica.jl | `asinghvi17/BaseModelica.jl` `bm-events-and-parser-fixes` @ `af60bf21` (1.10.1); ANTLR parser, `discrete_backend = :mtk` |
| MTK / DyadInterface | ModelingToolkit 11.45.5, ModelingToolkitBase 1.77.5, DyadInterface 7.3.0, OrdinaryDiffEq 7.8.1, SciMLBase 3.58.0; Julia 1.13.3 |
| references | MAP-LIB_ReferenceResults `v4.1.0`: `Modelica/**/*.csv` + `comparisonSignals.txt` |

**Model set.** These are the non-partial classes under `Modelica.*.Examples` whose own
annotation has an `experiment`. `enumerate_models.py` finds **534**. That is exactly
the 534 `Modelica.*` models MAP-LIB has references for: the two lists are identical.
ModelicaTest was not run. It is not in the release zip, and its 608 reference
models would need the MSL git sources (see "Not done").

**Stages, per model.** These are the exact calls; see `worker.jl`.
1. **emit:** `rumoca compile … --target base-modelica` and
   `--target base-modelica-scalar`. The Julia stages use the scalar file when it
   exists (98 models) and the array-preserving file otherwise (9 models).
2. **parse:** `BaseModelica.parse_package(file, :antlr)`.
3. **convert:** `baseModelica_to_ModelingToolkit(pkg; discrete_backend = :mtk, simplify = false)`.
   Together, stages 2–3 are `parse_basemodelica(file; simplify = false)`.
4. **simplify:** `DyadInterface.simplify_model(sys)`.
5. **init / solve:**
   - Problem: `ODEProblem(ssys, [], tspan; missing_guess_value = Constant(0))`, as
     `create_odeproblem` does.
   - Settings:
     - `tspan` from `experiment_settings`;
     - `reltol = min(Tolerance, 1e-6)`, `abstol = 1e-8`;
     - `dtmax = Interval` (output step), because reference tools also step on their output grid;
     - `tstops` = the time-event thresholds BaseModelica.jl collected (its own
       `create_odeproblem` computes these but never passes them on).
   - Solver: `Rodas5P`, with an `FBDF` retry if the retcode is not a success.
   - Models with no differential states also get `tstops` at the reference times;
     see "Surprises".
6. **compare:**
   - Every signal in `comparisonSignals.txt` is compared over the time overlap with
     the reference. ChuaCircuit's reference stops at 3000 s of 50000.
   - Error is `max |sim − ref| / range(ref)`, skipping duplicated event
     timestamps. The sample error is the smallest at t, t − δ and t + δ, with
     δ = 1e-3 of the span; this forgives event-time jitter on discontinuous signals.
   - **Pass if ≤ 1e-3.** The strict version (at t only) is in `max_rel_err_strict`.
     For every passing model they agree to two digits.

**Limits.**
- Per-stage timeout 5 min and per-model cap 10 min, enforced by `run_sweep.py`,
  which kills and restarts the worker.
- Workers ran 5 at a time (24 cores, 30 GB RAM, ~20 GB free). Each is a long-lived
  Julia process that warms up once and is recycled every 20 models or past 7 GB.
- Emit ran 12 jobs at a time.
- The whole sweep is fast:
  - emit: median 3.2 s per model and target;
  - Julia stages: every median under 1 s;
  - all 107 Julia runs: 3.5 min wall.

## Headline: furthest stage reached

The funnel counts models that completed each stage:

| stage | completed | of 534 |
|---|---|---|
| emit (either target) | 107 | 20.0% |
| parse | 103 | 19.3% |
| convert to MTK | 80 | 15.0% |
| simplify_model | 61 | 11.4% |
| init + solve | 55 | 10.3% |
| **compare (pass)** | **46** | **8.6%** |

- Init failures surface inside `solve`, so they are counted at solve.
- **Emit by target:**
  - array-preserving: 107 ok / 427 fail;
  - scalar: 98 ok / 435 fail / 1 timeout (`Lines.SmoothStep`, >300 s);
  - no model emits scalar but not array.
- **Pass rate by entry point:**
  - of models reaching Julia: 46/107 = 43%;
  - of models that parse: 46/103 = 45%;
  - of models that simulate: 46/55 = 84%.

## Per sub-library

The stage columns count models that got through that stage. "solve" means the
solve succeeded.

| sub-library | models | emit | parse | convert | simplify | solve | pass | top failure (count) |
|---|---|---|---|---|---|---|---|---|
| Electrical | 206 | 61 | 57 | 50 | 32 | 27 | 24 | rumoca: function array-parameter dimensions (72) |
| Clocked | 77 | 3 | 3 | 1 | 1 | 1 | 1 | rumoca: `sample()` not rendered (29) |
| Mechanics | 75 | 18 | 18 | 17 | 17 | 17 | 13 | rumoca: function array-parameter dimensions (54) |
| Magnetic | 69 | 5 | 5 | 5 | 4 | 3 | 3 | rumoca: function array-parameter dimensions (30) |
| Blocks | 31 | 8 | 8 | 5 | 5 | 5 | 4 | rumoca: function array-parameter dimensions (15) |
| Fluid | 23 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: `String()` conversion (10) |
| Media | 21 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: ED019 function arrays/records/loops (8) |
| Thermal | 16 | 12 | 12 | 2 | 2 | 2 | 1 | BaseModelica.jl: `assert` in initial algorithm (10) |
| StateGraph | 7 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: function array-parameter dimensions (7) |
| Math | 4 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: unevaluable array dimensions (2) |
| Utilities | 3 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: `String()` conversion (1) |
| ComplexBlocks | 2 | 0 | 0 | 0 | 0 | 0 | 0 | rumoca: ED019 (1) |

What passes:
- Analog circuits: CauerLowPass, Chua, resonance, rectifiers, NandGate,
  DifferenceAmplifier, OpAmps Adder/Subtracter/LCOscillator;
- DC machines and controlled DC drives;
- Spice3 CoupledInductors;
- FluxTubes basic examples;
- Rotational/Translational basics;
- a few Blocks examples;
- HeatTransfer TwoMasses;
- the continuous `Clocked.SimpleControlledDrive`.

Fluid, Media, StateGraph, Math, Utilities and ComplexBlocks get **nothing past
rumoca**.

## All failure categories

Every failure is in `results.tsv` with its normalised signature, and summed in
`categories.tsv`. Categories come from ordered regex rules in `categorize.py`.
The 11 compare/solve failures marked † were diagnosed by hand; that table is
`DIAGNOSED` in `categorize.py`.

| stage | category | owner | models |
|---|---|---|---|
| emit | function array-parameter dimensions (`u[:]`, `b[size(a,1)]`) not rendered | rumoca | **212** |
| emit | `sample(start, interval)` not rendered | rumoca | 51 |
| emit | Clock / synchronous operators (`Clock`, `superSample`, `hold`, …) | rumoca (+ BM.jl) | 35 |
| emit | `String(x)` in assert/terminate messages | rumoca | 27 |
| emit | ED019 shape check fires in a function branch the specialization never takes | rumoca | 20 |
| emit | record-valued equations / record paths (`Complex`, operator records) | rumoca | 19 |
| emit | ED013 model `algorithm` with loops/`break`/multiple outputs (Digital) | rumoca | 16 |
| simplify | parameter bindings intermittently lose referenced parameters (nondeterministic) | BaseModelica.jl | 15 |
| emit | ED019, other: function arrays/records/loops (Media, Filter, …) | rumoca | 14 |
| convert | `assert` in `initial algorithm` rejected | BaseModelica.jl | 13 |
| emit | other (ET001 unknown member ×3, ED008 ×2, EF004 `print`/`Streams.close` in `when`, EF025, ED020) | rumoca | 9 |
| emit | ED019 impure call statement (`error(...)`) in a function | rumoca | 7 |
| convert | array variables (array-preserving file used) | BaseModelica.jl | 5 |
| emit | ED019 `der()` of an expression | rumoca | 5 |
| emit | ET004 unevaluable array dimensions | rumoca | 4 |
| compare/solve† | relation-defined `Boolean` takes its `start` at init (`off = s < 0`) | BaseModelica.jl | 4 |
| convert | `initial()` / `terminal()` / `edge()` unsupported | BaseModelica.jl | 3 |
| compare† | init converges to a different root (`fixed = false` params, op-amp loop) | BaseModelica.jl/MTK | 2 |
| compare† | borderline: 1.04e-3 against the 1e-3 threshold (friction stick events) | — | 2 |
| simplify | `delay()` (needs a DDE) | BaseModelica.jl/MTK | 2 |
| parse | ANTLR (Python) recursion limit: parenthesis depth 101–201 | BaseModelica.jl (or rumoca) | 2 |
| parse | subscripted array literal `[[…]]['i', 2]` is not Base Modelica | rumoca | 2 |
| simplify | MTK "equations have a cycle" / "graph contains a loop" | MTK | 2 |
| solve | `DtNaN` at t = 0 (DCSE machines) | BaseModelica.jl/MTK | 2 |
| solve | `Unstable` (OpAmps ControlCircuit, Multivibrator) | solver/model | 2 |
| emit | EF020 expandable connector augmentation / ER002 / ER130 | rumoca | 2 / 2 / 2 |
| emit | EF036 external C function with library state (`ImpureGenerator`) | rumoca | 1 |
| convert | `return` in a function; array sized by a parameter | BaseModelica.jl | 1 / 1 |
| solve | MTK `get_initial_values` internal `MethodError` | MTK | 1 |
| compare† | early transient differs (DCPM_Drive, undiagnosed) / PreLoad 1.5% stick-slip | ? | 1 / 1 |
| emit | base-modelica-scalar cannot scalarize (only target tried had this) | rumoca | 1 |

**Owner totals** (488 failures):
- rumoca: 429;
- BaseModelica.jl: 44;
- BaseModelica.jl/MTK: 10, mostly diagnosed init or comparison issues;
- MTK: 3;
- solver: 2.

rumoca reports only its first error, so every emit count is a lower bound for the
work behind it. The prototype below measures what sits behind the largest category.

## Top categories: repro, fix, size, owner

The repros are in `repros/`:
- `repros/rumoca/*.mo` are Modelica sources. `repros/emit_repros.sh` emits them
  with both targets.
- `repros/*.bmo` are Base Modelica files. `repros/run_repros.jl` runs them through
  the Julia stages.

Each repro below fails in the same way as the MSL models.

### 1. Function array-parameter dimensions: 212 models (rumoca, **S**)

**Repros:** `repros/rumoca/FunctionArrayInput.mo` (`input Real u[:]`) and
`FunctionSizedInput.mo` (`input Real b[size(a, 1)]`).

**Errors:**
- `EC002 template render error: unhandled Expression variant: {"Colon": …}`
- `Index missing 'base' field`

Both come from `base_modelica.mo.jinja:156`.

**Root cause:** `function_param` renders `param.shape_expr` with `render_expr`, but
`shape_expr` is a `Vec<ast::Subscript>`, not expressions:
- `Subscript::Index` (a literal) looks like the Expression `Index` and fails on the
  missing `base`;
- `Subscript::Colon` is unknown to `render_expression`.

Every MSL function with an unsized or `size()`-sized array input hits this:
Polynomials, Vectors, Tables, Matrices, Frames, Machines utilities.

**Fix:** render function-parameter dimensions with the existing
`render_subscript`: Colon → `:`, Expr → the expression, Index → its 1-based
literal. One option is a `render_subscript` template function. Another is three
lines in `render_expression` that dispatch on a Subscript with no `base`.

**Measured with that prototype.** The 7-line patch is
`prototype-function-param-dims.patch`. It was built to a scratch target dir and
reverted; it is not applied on this branch.
- 60 of the 212 emit in array form. The other 152 hit the next rumoca blocker:
  records 77, `String()` 45, `sample()` 20, scalarize 8, Clock 1.
- Of the 60, 59 then fail in the scalar target, because they call functions with
  array arguments, which the scalar target refuses by design.
- In Julia, 40 of the 60 then stop at BaseModelica.jl's lack of arrays
  (category 10).
- The other 20 stop at a **new rumoca bug: description strings are not
  re-escaped.** `"Choose \"Y\" for star"` is written as `"Choose "Y" for star"`,
  which does not parse. Repro: `repros/rumoca/EscapedQuote.mo`.
  - Fix: escape `\` and `"` when rendering descriptions and String literals.
  - Size: S.

So this category unlocks models only together with the escaping fix and
BaseModelica.jl array support.

### 2. `sample(start, interval)`: 51 models (rumoca, **S**)

**Repro:** `repros/rumoca/EventSample.mo`, which contains `when sample(0, 0.1) then …`.

**Error:** `BuiltinCall Sample must be lowered into clock/event metadata and
ordinary equations before template rendering`. It comes from the `when`
condition (Blocks.Discrete, `SampleTrigger`, Digital clocks).

**Fix:**
- Render `BuiltinCall Sample` with two Real arguments as `sample(start, interval)`.
  It is a valid MCP-0031 builtin.
- BaseModelica.jl already lowers it to a periodic `SymbolicDiscreteCallback`
  (`events.jl:293`), but only for `start = 0` under `:mtk`. `start ≠ 0` needs
  `:synchtoolkit`, so a small BaseModelica.jl follow-up is likely: offset the
  periodic callback.
- The one-argument clocked `sample(u, clock)` belongs to category 3.

### 3. Clock / synchronous language: 35 models, plus 29 Clocked models in category 2 (rumoca + BaseModelica.jl, **L**)

**Repro:** `repros/rumoca/ClockedSample.mo`, which contains `xd = sample(x, Clock(0.1))`.

**Errors:**
- `[ED009] unresolved clock schedule`;
- in MSL, `BuiltinCall Clock|SuperSample must be lowered with its clock schedule`.

**What's needed:**
- rumoca emits no clocked partitions; that gap is already listed in `REPORT.md`.
- Base Modelica needs the clock constructs written out.
- BaseModelica.jl needs its `:synchtoolkit` backend (experimental) to consume them.

This is a feature, not a bug. It covers all of `Modelica.Clocked`: 76 of its 77
examples fail.

### 4. `String()` conversion: 27 models (rumoca, **S**)

**Repro:** `repros/rumoca/StringInAssert.mo`, which contains
`assert(k > 0, "k = " + String(k) + …)`.

**Error:** `unhandled Expression variant: {"StringConversion": …}` while rendering
an `assert` (Limiter, PID, Media property checks).

**Fix:** render `StringConversion` as `String(x)`, keeping its format options
(`significantDigits`, `minimumLength`, `leftJustified`). Or, because Base
Modelica consumers ignore assert messages, reduce dynamic messages to their
literal parts.

This is also the second-largest blocker behind category 1 (45 models).

### 5. ED019 shape check in unreachable function branches: 20 models (rumoca frontend, **S/M**)

**Repro:** `repros/rumoca/DeadBranchShape.mo`.

**Error:** `` `f` assigns a value of shape [2, 2] to a target of shape [3, 3];
MLS §11.2.1 requires the two to be compatible in every branch, including the
ones this specialization proves unreachable ``.

In MSL every hit is `Electrical.Polyphase.Functions.symmetricOrientationMatrix`
(`if mBase == 2 then oM[1:mBase,1:mBase] := …`). That covers QuasiStatic
polyphase, machines and FundamentalWave.

**Fix:** skip, or defer to run time, the shape check in branches whose condition
is statically false for this specialization, as other tools do.

### 6. Record-valued equations: 19 models, 77 more behind category 1 (rumoca, **M**)

**Repro:** `repros/rumoca/RecordEquation.mo`, which contains
`Complex z; z = Complex(cos(time), sin(time));`.

**Error:** `unsupported-feature:base-modelica-record-value`. The follow-up commit
`f8300bea` added this refusal on purpose: model records are flattened to fields.

**Fix:** expand record-valued equations field by field (`z.re = …; z.im = …`),
including the `Complex` operator-record operators. These are pervasive in
QuasiStatic and ComplexBlocks. The other option is to keep record-typed components.

This is the largest category hidden behind category 1.

### 7. ED013 model `algorithm` with loops / `break` / multiple outputs: 16 models (rumoca, **M**)

The models are the Digital gates, registers and converters, e.g. `DFFREG`:
`for … loop … if clock_flag == 0 then break;` and table-driven multi-output
assignments. There is no small repro beyond `Electrical.Digital.Examples.DFFREG`.

**Fix:** let the canonical DAE carry a general algorithm section as one block
(Base Modelica keeps `algorithm` as is), instead of requiring each statement to
lower to an equation.

Digital also has category 1 and the subscripted array literal (2 models). The
literal repro is `repros/rumoca/ArrayLiteralIndex.mo`: rumoca writes
`'y' = [[1, 2]; [3, 4]]['i', 2];`, which does not parse. Fix it by hoisting the
table to a package constant (S).

### 8. BaseModelica.jl parameter bindings: 15 models (BaseModelica.jl, **M**)

The affected models are the OpAmps examples with a Trapezoid input, the NOR
gates, SimpleTriacCircuit and DCPM_QuasiStatic.

**Error:** `ArgumentError: Bindings for parameters can only be functions of other
parameters. For parameter vIn.signalSource.amplitude, encountered binding 2.0Vin
which contains non-parameter symbolics`. In other runs the same models fail in
`solve` instead, with `UndefVarError: vIn.signalSource.T_width not defined in
ModelingToolkitBase`.

**It is nondeterministic.** `OpAmps.LowPass` passed in one full run and failed in
the next, with no change to its input. VoltageFollower and InvertingSchmittTrigger
moved between the two error forms. That points at iteration order over a Dict or
Set keyed by Symbol, whose hashes differ per process.

**Repros:**
- `repros/TrapezoidSource.mo`, 3 components, reproduces it 3 out of 3 times.
- `repros/TrapezoidBinding.bmo` is its rumoca output, kept unreduced.
  Delta-debugging (`ddmin`) reduces it to ~25 lines, the trapezoid source's
  parameters plus its `when`. The reduced file is not kept.

It is not a plain forward reference: `repros/ForwardParam.bmo` passes. It is also
not a `when` clause plus forward reference.

**Fix:** in BaseModelica.jl, declare every parameter symbol before evaluating any
binding, and resolve bindings in dependency order rather than in container order.
The emitted Modelica is valid.

### 9. `assert` in `initial algorithm`: 13 models (BaseModelica.jl, **S**)

**Repro:** `repros/InitialAssert.bmo`.

**Error:** `UnsupportedFeatureError: initial algorithm sections may only contain
assignments`. In MSL this is FluidHeatFlow's `SimpleFriction`, which is in every
FluidHeatFlow example, DemoPowerSupply and DCPM_Cooling.

**Fix:** accept `assert` in initial algorithms: evaluate it after
initialization, or drop it with a warning.

10 of Thermal's 12 emitted models stop here.

### Also worth fixing early

- **BaseModelica.jl array support (L).** Only 5 models today, but **40** of the 60
  models that category 1's fix unlocks stop here. Without it, category 1 mostly
  moves the failure from rumoca to BaseModelica.jl.
- **Relation-defined Booleans take their `start` value at initialization (S/M):**
  BaseModelica.jl. This is the one that produces **wrong answers silently**; see
  below.
- **`initial()` / `terminal()` / `edge()`** in BaseModelica.jl (3 models, S).
- **Parenthesis depth (S).** Either rumoca drops the parentheses on left-associative
  chains of the same operator, or BaseModelica.jl raises the Python recursion
  limit or uses an iterative visitor.

## Surprises

1. **Silent wrong results: relation-defined Booleans are frozen at their start
   value.** Repros: `repros/RelationBoolean.bmo` vs `RelationBooleanStart.bmo`.
   - Under `:mtk`, `Boolean b(start = false); b = (x > 0);` becomes a discrete.
     At t = 0 it takes `start`, not the relation's value.
   - With a consistent start, the later crossing updates it correctly. With an
     inconsistent one it never recovers: b stays `false` while x > 0.
   - The solver returns `Success` with wrong trajectories:
     - CharacteristicIdealDiodes: the `off` diode state;
     - ElastoGap: `contact`, so no contact force and nothing moves;
     - ControlledTemperature: OnOffController hysteresis;
     - BusUsage: a `BooleanStep` on a bus.
   - Anything that only checks `retcode` will count these as passes.
   - IdealDiodes also chatters past the 5-min cap when `dtmax` is set; without it,
     it finishes with the wrong mode.
2. **The BaseModelica.jl binding failure is nondeterministic** (category 8). A
   single-run pass/fail count is ±1–2 models from this alone.
3. **Purely algebraic models compare badly through interpolation.**
   - When a system has no differential states (OpAmps Adder/Subtracter, FluxTubes
     airgap models), Rodas5P takes huge steps: 4 steps over the whole run in one
     case.
   - Its dense output for algebraic variables is not error-controlled. OrdinaryDiffEq
     warns `rosenbrock_no_differential_states`.
   - `sol(t)` therefore returns values off by up to 2× the signal range between
     steps, even with `dtmax`.
   - The harness steps onto the reference times for these 9 models. That is not a
     pipeline bug, but anyone calling `sol(t)` on such a system gets wrong numbers.
4. **BaseModelica.jl's `create_odeproblem` collects `tstops` for time events but
   never passes them** to `ODEProblem`; `model_tstops` is unused in
   `BaseModelica.jl:266`. The harness passes them itself.
5. **The Julia stages are cheap.**
   - Every stage's median is under 1 s.
   - Raw systems have up to 410 unknowns (median 64 among passing models), and
     `simplify_model` reduces them to a median of 4 states.
   - All the cost is in emit (~3 s per model) and in models that chatter.
6. **Huge literals.** rumoca writes tiny constants in full decimal: `min =
   Modelica.Constants.small` becomes `0.000…0001` with 60+ digits, and the
   longest literal is 326 characters. It is valid, but bloats files. Print
   floats in shortest round-trip form.
7. **The scalar target is nearly free.** It loses only 9 models relative to the
   array form (8 refused array-valued calls, 1 timeout), and BaseModelica.jl
   cannot read arrays anyway. For now the scalar target is the one to use.
8. **Two init problems with multiple roots:**
   - **SignalGenerator:** the op-amp relaxation oscillator initializes at the
     unstable equilibrium 0, while the reference starts at −15 V.
   - **DemonstrateLightning:** the DoubleExp source solves `fixed = false`
     parameters `T`, `tau1`, `tau2` from nonlinear initial equations with
     expression-valued `start` guesses. It lands on a different root; the Heidler
     branch of the same model matches.

   Both point at start values not being used as initial guesses, or at
   `homotopy`.

## Not done / caveats

- **ModelicaTest was not run.**
  - Its sources are not in the MSL release zip. It needs a sparse checkout of the
    ModelicaStandardLibrary git repository at `v4.1.0`, plus `ModelicaTest/**` from
    MAP-LIB (608 models).
  - The scripts take any model list, and emit plus Julia for 534 models took under
    15 min, so it is cheap to add. I left it out to spend the time on diagnosis.
- **Only rumoca's first error per model is seen.** The prototype under category 1
  shows how much sits behind the largest category; the others were not measured.
- **The † diagnoses are manual.** I read the trajectories with `firstdiv.jl`-style
  scripts. The compare stage itself only says "numbers differ". DCPM_Drive is
  still undiagnosed: its armature current is 0.56 A against 5e-5 A at t = 0.3 ms,
  probably the inverter's controller initialization.
- **Harness changes across runs.** Three full Julia runs were made:
  1. no `dtmax`: 44 passes;
  2. `dtmax` plus reference `tstops` for all models;
  3. reference `tstops` only for models without differential states: 46 passes.

  From the first run to the last:
  - 4 models moved to pass: OpAmps Adder/Subtracter, ToroidalCoreAirgap and
    ToroidalCoreQuadraticCrossSection. They had been interpolation artifacts.
  - 2 moved to fail: CompareBrakingTorque and CompareBrakingForce went from 8.0e-4
    to 1.04e-3, just over the threshold.

  Both are reported as they came out.

## Files

| file | what |
|---|---|
| `results.tsv` | one row per model; columns listed below |
| `categories.tsv` | failure counts by (stage, category), with example models |
| `tables.md` | raw output of `categorize.py`: funnel, categories, sub-library table, emit-by-target counts |
| `models.txt` | the 534 models |
| `setup.sh` | idempotent setup under `target/msl-sweep/` (gitignored): rustup + rumoca build, MSL zip, MAP-LIB sparse checkout, Julia env |
| `enumerate_models.py` | builds `models.txt` from the MSL sources |
| `emit.sh` | stage 1: rumoca, both targets, parallel and resumable (one TSV per model and target) |
| `worker.jl` | Julia stages 2–6 for one model at a time (long-lived worker) |
| `run_sweep.py` | worker pool with the 5-min stage / 10-min model timeouts and crash handling; resumable (one TSV per model) |
| `categorize.py` | merges rows, normalises signatures, assigns categories, prints the tables |
| `julia/` | pinned environment; `CondaPkg.toml` declares `antlr4-python3-runtime==4.13.1` |
| `repros/` | minimal repros (`*.bmo`, `rumoca/*.mo`) and their runners |
| `prototype-function-param-dims.patch` | the uncommitted rumoca patch behind the category 1 measurement |

`results.tsv` columns, one row per model:
- `model`, `sublib`;
- `stage_reached`, `failed_stage`, `status`;
- `category`, `owner`, normalised `signature`;
- `max_rel_err` (tube) and `max_rel_err_strict`, `worst_signal`;
- `n_signals`, `n_found`, `missing_signals`;
- emit status and error per target, `bmo` (which file was used);
- `n_unknowns_raw` / `n_unknowns`, `solver`, `retcode`, `tspan`, `compare_note`;
- per-stage timings `t_emit` … `t_compare`;
- raw `error`.

**Rerun:**

```sh
outputs/msl-sweep/setup.sh   # first time ~20-30 min (Rust build, MTK precompile, 1.9 GB refs)
B=$PWD/target/msl-sweep
outputs/msl-sweep/emit.sh $PWD/target/release/rumoca $B/msl $B/work 12 < outputs/msl-sweep/models.txt
python3 -I outputs/msl-sweep/run_sweep.py --work $B/work --refs $B/refs \
    --models outputs/msl-sweep/models.txt --jobs 5     # re-run until "queued 0"
python3 -I outputs/msl-sweep/categorize.py $B/work outputs/msl-sweep/models.txt outputs/msl-sweep > outputs/msl-sweep/tables.md
```

Each batch stops dispatching at 18 min and kills stragglers at 29 min without
writing a row, so a re-run picks them up. Delete `work/rows/<model>.tsv` to redo
one model. Environment: `JULIA_PKG_USE_CLI_GIT=true`, `GIT_TERMINAL_PROMPT=0`, and
the DyadRegistry added by git URL.
