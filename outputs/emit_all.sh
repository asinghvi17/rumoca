#!/usr/bin/env bash
# Regenerate outputs/bmo/ from outputs/models/ and MSL 4.1.0.
#
#   outputs/emit_all.sh [rumoca-binary] [msl-dir]
#
# msl-dir holds `Complex.mo`, `Modelica 4.1.0/` and `ModelicaServices 4.1.0/`
# (the ModelicaStandardLibrary v4.1.0 release zip, unpacked).
set -euo pipefail
cd "$(dirname "$0")/.."
RUMOCA=${1:-target/release/rumoca}
MSL=${2:-.factory-local/msl}
M=outputs/models
rm -rf outputs/bmo
for target in base-modelica base-modelica-scalar; do
  out=outputs/bmo
  [ "$target" = base-modelica-scalar ] && out=outputs/bmo/scalar
  mkdir -p "$out"
  "$RUMOCA" compile $M/SpringMassDamper.mo --target $target -o "$out" >/dev/null
  "$RUMOCA" compile $M/RCCircuit.mo -m RCCircuit.Circuit --target $target -o "$out" >/dev/null
  "$RUMOCA" compile $M/BouncingBall.mo --target $target -o "$out" >/dev/null
  "$RUMOCA" compile $M/UserFunction.mo -m UserFunction.Decay --target $target -o "$out" >/dev/null
  "$RUMOCA" compile $M/RecordFunction.mo -m RecordFunction.Decay --target $target -o "$out" >/dev/null
  "$RUMOCA" compile $M/ArrayModel.mo --target $target -o "$out" >/dev/null 2>&1
  for model in Modelica.Mechanics.Translational.Examples.Oscillator \
               Modelica.Electrical.Analog.Examples.ChuaCircuit \
               Modelica.Mechanics.Rotational.Examples.First \
               Modelica.Electrical.Analog.Examples.CauerLowPassAnalog; do
    "$RUMOCA" compile "$MSL/Modelica 4.1.0/package.mo" -m $model \
      --source-root "$MSL/Complex.mo" \
      --source-root "$MSL/Modelica 4.1.0" \
      --source-root "$MSL/ModelicaServices 4.1.0" \
      --target $target -o "$out" >/dev/null 2>&1
  done
done
ls outputs/bmo outputs/bmo/scalar
