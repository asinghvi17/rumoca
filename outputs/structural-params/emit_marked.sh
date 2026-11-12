#!/usr/bin/env bash
# usage: emit_marked.sh <rumoca-binary>   -- emits every probe to bmo-marked/ (base-modelica)
# and bmo-marked/scalar/ (base-modelica-scalar), with each compile's stderr (WD001 warnings)
# in <model>.log. Same per-probe scratch-dir layout as emit.sh.
set -u
cd "$(dirname "$0")"
R=$(realpath "${1:?rumoca binary}")
rm -rf bmo-marked; mkdir -p bmo-marked/scalar
for f in probes/*.mo; do
  n=$(basename $f .mo)
  d=$(mktemp -d); cp $f $d/
  for m in $n ${n}B; do
    grep -q "^model $m\b" $f || continue
    (cd $d && "$R" compile $n.mo -m $m --target base-modelica -o $d/out) 2>&1 | sed "s|$d|<tmp>|g" > bmo-marked/$m.log
    [ -f $d/out/$m.bmo ] || echo "FAIL base-modelica $m"
    cp $d/out/$m.bmo bmo-marked/ 2>/dev/null
    (cd $d && "$R" compile $n.mo -m $m --target base-modelica-scalar -o $d/outs) 2>&1 | sed "s|$d|<tmp>|g" > bmo-marked/scalar/$m.log
    [ -f $d/outs/$m.bmo ] || echo "FAIL scalar $m"
    cp $d/outs/$m.bmo bmo-marked/scalar/ 2>/dev/null
  done
  rm -rf $d
done
