#!/usr/bin/env bash
# usage: emit.sh <rumoca-binary>   -- emits every probe to bmo/ (base-modelica), bmo-scalar/ (base-modelica-scalar) and flat/ (flat-mo)
# Each probe is copied to its own scratch dir: rumoca loads sibling .mo files as sources.
set -u
cd "$(dirname "$0")"
R=$(realpath "${1:?rumoca binary}")
rm -rf bmo bmo-scalar flat; mkdir -p bmo bmo-scalar flat
for f in probes/*.mo; do
  n=$(basename $f .mo)
  d=$(mktemp -d); cp $f $d/
  for m in $n ${n}B; do
    grep -q "^model $m\b" $f || continue
    (cd $d && "$R" compile $n.mo -m $m --target base-modelica -o $d/out) > bmo/$m.log 2>&1 || echo "FAIL base-modelica $m"
    cp $d/out/$m.bmo bmo/ 2>/dev/null
    (cd $d && "$R" compile $n.mo -m $m --target base-modelica-scalar -o $d/outs) > bmo-scalar/$m.log 2>&1 || echo "FAIL scalar $m"
    cp $d/outs/$m.bmo bmo-scalar/ 2>/dev/null
    (cd $d && "$R" compile $n.mo -m $m --emit flat-mo) > flat/$m.mo 2> flat/$m.log || echo "FAIL flat $m"
  done
  rm -rf $d
done
