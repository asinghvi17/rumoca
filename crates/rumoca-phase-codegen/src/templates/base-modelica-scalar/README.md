# `base-modelica-scalar`

## Use case

Use this target to hand a flattened model to a Base Modelica consumer that
does not handle arrays yet (for example BaseModelica.jl, whose ModelingToolkit
conversion reads only scalar component references).

## Contract

- Input: checked Flat IR.
- Output: the `base-modelica` document (`//! base 0.1.0`, `package 'M'` /
  `model 'M'`), with every array variable declared element by element as a
  scalar quoted identifier (`Real 'x[2]'`) and the materialized scalar
  equation rows printed instead of `for` loops.
- Element attributes and bindings are taken from array literals, from
  references to array variables (subscripted per element), or broadcast from a
  scalar value.

## Unsupported

An equation or attribute whose array value cannot be split element by element
at render time (a whole-array reference inside an equation, an array-valued
call) fails rendering with `cannot scalarize ...`; use `base-modelica` for
such models.

## Verification

- `suite_core::base_modelica_target` covers scalarized declarations, unrolled
  `for` families, split elementwise equations and the refusal of a matrix
  product; `codegen::base_modelica_tests` covers the element enumeration.

## Example

```sh
rumoca compile Plant.mo --model Plant --target base-modelica-scalar --output generated
```
