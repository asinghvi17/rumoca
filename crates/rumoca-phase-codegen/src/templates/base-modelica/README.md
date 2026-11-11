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
- The source's `annotation(experiment(StartTime, StopTime, Tolerance,
  Interval))` becomes the model's class annotation (MCP-0031
  `annotations.md`), with only the entries the source sets.
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
annotations other than `experiment`. Records as package-level types and
clocked partitions are not emitted yet. A Flat construct that cannot be represented fails rendering
instead of being omitted.

## Verification

- `suite_core::base_modelica_target` checks the package layout, quoting,
  `when`/`reinit`, functions, arrays, enumerations, scalarization, bound
  parameters, the `experiment` annotation, and that the output recompiles
  through Rumoca.
- `cli_target_acceptance::base_modelica_renders_when_and_algorithms` drives
  the CLI target path.
- `template_target_ci::modelica_interchange_targets_round_trip_through_the_compiler`
  reparses and recompiles the generated source; the Modelica interchange
  runtime lane also checks OMC acceptance.

## Example

```sh
rumoca compile Plant.mo --model Plant --target base-modelica --output generated
```
