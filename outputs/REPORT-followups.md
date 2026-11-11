# Base Modelica backend: follow-ups to the end-to-end run

These fixes sit on top of `4c85084b`, the prototype `outputs/REPORT.md`
describes. They were added as new commits and fast-forward from it; no
existing commit was rewritten.

**Code head: `f8300beaef147a7c66606067016e79a65902930c`.** The commit that
adds this report and the regenerated `outputs/` sits directly on top of it.

## Changelog

| commit | change |
|---|---|
| `485b413e` fix(codegen): write a bound base-modelica parameter with its binding only | gap 2 |
| `d4e213c6` feat(codegen): carry the experiment annotation into base-modelica output | gap 1 |
| `f8300bea` feat(codegen): declare records used by functions as base-modelica classes | gap 3 |
| (next) docs(outputs): base-modelica follow-ups: re-emitted models, validation, report | outputs only |

### 1. `experiment` annotation

MCP-0031 `annotations.md` keeps `experiment` as "the class annotation of the
`model`". The grammar puts it at the end of the model's composition,
`(annotation-comment ;)?` before `end 'M';`.

**Plumbing.** The strict compile already extracts StartTime, StopTime,
Tolerance and Interval from the root class into
`rumoca_compile::CompilationResult`. The CLI's `CompilationResult` dropped
them. Changes:
- The CLI result now keeps them in a new `experiment: TemplateExperiment`
  field.
- A new `render_flat_template_with_experiment` exposes them to Flat
  templates as `experiment`: a map of only the finite entries that are set,
  or `none` when absent.
- `render_flat_template_with_name` is unchanged: it passes none. That keeps
  the wasm and worker callers as they were.

**Template.** It writes, for example:

    equation
      ...
    annotation(experiment(StartTime = -1.0, StopTime = 5.0, Tolerance = 0.00000001, Interval = 0.01));
    end 'M';

Only the entries the source sets are written, in that fixed order, for both
`base-modelica` and `base-modelica-scalar`.

**BaseModelica.jl 1.10.1 parses it and uses it.**
- **Parsing:** the ANTLR parser accepts every emitted file with an
  annotation, as does the `:julia` parser wherever it accepts the file at
  all; see the table below.
- **Use:** `create_odeproblem` reads `composition.annotation` of the model:
  - StartTime/StopTime become `tspan`;
  - Tolerance becomes `reltol`;
  - Interval becomes `saveat`.
- **Evidence:** in `outputs/simulation.tsv` the time span now comes from the
  file, with no stop time given on the command line:
  - SMD: `(0.0, 5.0)`, and 501 points = Interval 0.01;
  - Oscillator: `(0.0, 1.0)`, 1001 points = Interval 0.001;
  - CauerLowPassAnalog: `(0.0, 60.0)`.
  `simulate_bmo.jl` now takes `FILE[:STOP]` and no longer forces `reltol`.
- **Caveats:**
  - BaseModelica.jl only accepts a plain number literal. A negative
    StartTime is written `-1.0`, which is a unary-minus expression, and
    BaseModelica.jl silently ignores it and falls back to 0.0. The rumoca
    output is valid; this is a consumer limitation.
  - With no annotation, BaseModelica.jl still defaults to `(0, 1)` and
    `reltol = 1e-4`.

Test: `the_experiment_annotation_closes_the_model`. It checks both targets,
a partial annotation, and that a model without one emits none.

### 2. Spurious `start = 0.0` on a modifier-bound parameter

**Root cause.**
- The parser gives every `Real` component the default `start = 0.0`; it is
  not marked explicit.
- Instantiation promotes a *declaration* binding to `start`, but per
  MLS §7.2.4 it does not rewrite start from an *outer modification*
  binding. So `Resistor R1(R = 7.0)` reached Flat as `start = 0.0`,
  `binding = 7.0`.
- The template wrote every non-empty start.

**Fix.** A parameter or constant that is not `fixed = false` takes its
binding as its value, and its start is never used. So the template now
writes only the binding:
- `'r1.R'(start = 0.0) = 7.0` becomes `'r1.R' = 7.0`.
- This also removes the duplicated `'m'(start = 1.0) = 1.0` for
  declaration-bound parameters.
- A `fixed = false` parameter keeps its start, since that is the
  initialization guess.
- An unbound parameter still takes its start as the binding, as before.

**Scope.** The change is in the template only; Flat is unchanged. Flat has
no "start was explicit" bit, and it does not need one here.

**Not changed:** non-parameter variables of user-written `Real` types keep
the parser's `start = 0.0`, e.g. `'R1.p.v'(start = 0.0)`. It is redundant,
since it equals the default, but harmless. Telling it apart from an
explicit `start = 0.0` would need a new Flat field.

Test: `a_bound_parameter_carries_only_its_binding`. It covers both targets,
a modifier-bound parameter, a parameter with its own start, a
`fixed = false` parameter, and the RC circuit. Three existing assertions
that encoded the duplicated start were updated.

### 3. Records

**What MCP-0031 requires** (`differences.md`, "Lexical scoping and record
definitions", "Records"):
- A record type must be a package-level `record` class wherever a type name
  is needed, i.e. in function component declarations.
- There are no implicit record constructors; a tool generates a function.
- A member is reached through its instance (`'p'.'x'`).

The spec does **not** require a model's record components to stay
record-typed. Flattening them to scalar fields (`'q.x'`, `'q.y'`) is a valid
Base Modelica model, so that is kept.

**Implemented, kept small:**
- **Package-level records.** Every record type an emitted function uses gets
  a `record 'R' ... end 'R';`, with fields taken from its constructor's
  inputs. "Uses" means as an input, output or local type, or through its
  constructor. Rendering fails explicitly if a record type has no
  constructor to take the fields from.
- **Constructors.** A constructor is emitted as the `differences.md` form:

      function '-R' "Automatically generated constructor for 'R'"
        input Real 'x';
        input Real 'y';
        output 'R' _result('x' = 'x', 'y' = 'y');
      end '-R';

  Constructor calls render as `'-R'(...)`. Before, it was an empty
  `function 'R'` with no output, a duplicate name beside the record.
- **Member access.** In function bodies `p.x` renders as `'p'.'x'`; it used
  to be one undeclared quoted name, `'p.x'`. This goes through
  `rumoca_core::split_path_with_indices`, not raw dot splitting, as the
  architecture gate requires.
- **Refusal.** A record-valued equation on a flattened model record, e.g.
  `w = make(time)`, has no Base Modelica spelling while `w` exists only as
  fields. It is now refused with `unsupported-feature:base-modelica-record-value`.
  Before, it was emitted as an undeclared bare `w`.

**Not implemented:**
- record-typed model components;
- per-record-member value modifications in the record class (the
  constructor inputs carry the defaults);
- operator records beyond what the constructor path gives.

Tests:
- `records_used_by_functions_are_package_classes` covers the record class,
  the constructor, its call, the record-typed output and local, member
  access, and the flattened model record.
- `a_record_valued_model_equation_is_refused`.

The record output does not recompile through rumoca itself. Rumoca applies
the full-Modelica rule that a public function component needs
`input`/`output` (MLS §12.2), while MCP-0031 has no `protected`. Plain
function locals fail the same way, which is why the user-function model
was already absent from `output_recompiles_through_the_compiler`.

## Re-emitted test models

**Inputs:**
- `outputs/models/*.mo` (SMD and RC now have an `experiment` annotation);
- a new `outputs/models/RecordFunction.mo` (records through functions);
- **4 MSL 4.1.0 examples:**
  - `Translational.Examples.Oscillator`;
  - `Analog.Examples.ChuaCircuit`;
  - `Rotational.Examples.First` (new);
  - `Analog.Examples.CauerLowPassAnalog` (new).

The MSL 4.1.0 release zip was compiled with `Complex.mo`, `Modelica 4.1.0`
and `ModelicaServices 4.1.0` as `--source-root`. `outputs/emit_all.sh`
lists all of them.

**Binary and emission.**
- The release binary was built with every default feature except
  `input-gamepad`.
- The output was re-emitted from the final committed code, and the result
  is byte-identical to what is committed.

**ANTLR parser (BaseModelica.jl 1.10.1): all 20 files pass with zero syntax
errors**, i.e. 10 models × {array-preserving, scalar}. Full rows are in
`outputs/validation.tsv`.

| model | ANTLR | `:julia` | MTK |
|---|---|---|---|
| SMD, RC, BouncingBall, Oscillator, Chua, First, CauerLowPass | PASS | PASS | PASS |
| ArrayModel (array form) | PASS | FAIL: `{...}` (BM.jl bug 2, known) | — |
| ArrayModel (scalar) | PASS | PASS | PASS |
| UserFunction | PASS | FAIL: indented algorithm (BM.jl bug 1, known) | FAIL: no user functions (known) |
| RecordFunction | PASS | **hangs** (new BM.jl bug, below) | FAIL: no user functions (`KeyError :norm2`) |

Simulation (`outputs/simulation.tsv`, Rodas5P, `tspan` from the
annotation unless a stop time is given) succeeds for:
- SMD: `x(5) = -0.287313`;
- RC: `C1.v(5) = 9.93262`;
- Oscillator;
- Chua (to t=100);
- First;
- CauerLowPassAnalog;
- scalar ArrayModel.

BouncingBall still fails in BaseModelica.jl: discrete variables assigned in
`when` are lost, as in `REPORT.md` gap 4.

**New BaseModelica.jl finding.** The `:julia` (ParserCombinator) parser does
not terminate on a function that has a local whose type is a quoted class
name (`'P' 'p';`) together with an algorithm section. It used more than 1e9
allocations and did not return within 300 s. A record-typed *input* parses
fine. ANTLR parses all of these. Minimal reproducer:

    //! base 0.1.0
    package 'G1'
      record 'P'
        Real 'x';
        Real 'y';
      end 'P';
      function 'g'
        input Real 'a';
        output Real 'n';
        'P' 'p';
      algorithm
        'n' := 'a';
      end 'g';
      model 'G1'
        Real 'x'(start = 1.0, fixed = true);
      equation
        der('x') = (-'x');
      end 'G1';
    end 'G1';

**MSL examples outside the four.** `Blocks.Examples.PID_Controller` and
`Rotational.Examples.CoupledClutches` still fail to render on Expression
variants the Base Modelica renderer does not handle: `StringConversion` and
`Colon`. That predates these fixes.

## Verification

| check | result |
|---|---|
| `suite_core` | 1788/1788 pass (15 in `base_modelica_target`, 4 of them new) |
| `suite_gates` | 19/19 pass |
| `architecture_hardening_test` | 252/253. The one failure is `commit_messages::test_reachable_commit_messages_have_no_ai_attribution`, the known gate tripped by the factory's AI co-author trailers on these and the two earlier branch commits. |
| `cargo clippy -p rumoca-phase-codegen -p rumoca-compile -p rumoca --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |

**Build settings:**
- toolchain `nightly-2026-02-27`;
- features: every `rumoca` default except `input-gamepad`;
- Julia: `JULIA_PACKAGE_SERVER=internal.juliahub.com`,
  `JULIA_PKG_USE_CLI_GIT=true`, using the pinned `outputs/julia` manifest.

**Not run:** `suite_template_runtime`, and the OMC-gated interchange checks
(OMC is not installed here).
