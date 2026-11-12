# `base-modelica`

## Use case

Use this target to hand a flattened Rumoca model to a tool that consumes Base
Modelica ([MCP-0031](https://github.com/modelica/ModelicaSpecification/tree/MCP/0031/RationaleMCP/0031)),
for example BaseModelica.jl and from there ModelingToolkit. It is an
interchange artifact, not a simulation runtime.

## Contract

- Input: checked Flat IR.
- Output: one `.bmo` document: the `//! base 0.1.0` version header, then
  `package 'M'` holding the enumeration types (`type 'P.E' = enumeration(...)`)
  and user functions the model uses, then `model 'M' ... end 'M';` and
  `end 'M';`.
- Every declared component, function and enumeration type is a quoted
  identifier (`'R1.p.v'`, `'Pkg.f'(...)`, `'P.E'.literal`); `time` and loop
  indices stay bare.
- Declarations keep `parameter`/`constant`/`discrete`, top-level
  `input`/`output`, `quantity`/`unit`/`displayUnit`/`min`/`max`/`start`/
  `fixed`/`nominal`/`stateSelect`, bindings and descriptions. MCP-0031 has no
  `each`, `final`, `protected` or `public`: a scalar attribute of an array is
  written `fill(value, dims)`, and function locals are plain declarations.
- A parameter or constant that is not `fixed = false` is written with its
  binding only: its `start` is never used (MLS §8.6), so no `start`
  attribute stands beside the value.
- A parameter translation evaluated keeps full Modelica's
  `annotation(Evaluate = true)` (MCP-0031 keeps annotations as annotations):
  one a structural use fixed (an array dimension, a for-equation range, the
  guard of an unbalanced if-equation, a conditional component's condition),
  one such a parameter's binding reads, an `Evaluate = true` parameter, and a
  `final` one with an evaluable binding. Its value was used to derive the
  emitted structure, so changing it after translation has no effect. Every
  other parameter is unmarked and tunable; a `constant` stays `constant`.
- The source's `annotation(experiment(StartTime, StopTime, Tolerance,
  Interval))` becomes the model's class annotation (MCP-0031
  `annotations.md`), with only the entries the source sets.
- A record type a function uses (as an input, output or local type, or
  through its constructor) is a package-level `record 'R'`. MCP-0031 has no
  implicit record constructors, so `R(...)` calls a generated
  `function '-R'` returning `_result('f' = 'f', ...)`, and a member of a
  record-typed function component reads `'p'.'x'`. Record components of the
  model itself stay flattened to scalar fields (`'q.x'`), which MCP-0031
  permits; a record-valued equation on such a component is written field by
  field (MLS §8.3.1).
- MCP-0031 has no member access on a call (`f(x).re`): a member is reached
  only through a record instance. Each member read off a record-valued call
  is a call of a generated accessor `function '-f().re'`, which takes `f`'s
  inputs, stores the call in a record local `'-record'` and returns
  `'-record'.'re'`.
- Equations, initial equations, assertions, `when`/`elsewhen` chains (with
  `reinit`, discrete assignments, `if` branches, `assert`, `terminate`),
  algorithm and initial algorithm sections are all kept. User function
  bodies are algorithm sections with `:=`; external functions keep their
  `external` clause. Functions every call of which renders as a built-in
  (`Modelica.Math.sin`) are not declared.
- Arrays stay arrays (`Real 'x'[3]`, unmaterialized `for` families as `for`
  loops). The `base-modelica-scalar` target renders the same document with
  every array scalarized, for consumers without array support.
- Negative literals are parenthesized (the grammar has no signed primary).

## Unsupported

The output is not original source and does not preserve class hierarchy or
annotations other than `experiment` and the `Evaluate = true` marker above. Model record components are not
emitted as record-typed declarations, and clocked partitions are not emitted
yet. A Flat construct that cannot be represented fails rendering instead of
being omitted.

## Verification

- `suite_core::base_modelica_target` checks the package layout, quoting,
  `when`/`reinit`, functions, arrays, enumerations, scalarization, bound
  parameters, the `experiment` annotation, records used by functions, and
  that the output recompiles through Rumoca.
- `suite_core::base_modelica_structural_parameters` renders one probe per
  kind of structural parameter and checks which parameters are marked
  `Evaluate = true` and which WD001 warnings the compile reports.
- `cli_target_acceptance::base_modelica_renders_when_and_algorithms` drives
  the CLI target path.
- `template_target_ci::modelica_interchange_targets_round_trip_through_the_compiler`
  reparses and recompiles the generated source; the Modelica interchange
  runtime lane also checks OMC acceptance.

## Example

```sh
rumoca compile Plant.mo --model Plant --target base-modelica --output generated
```
