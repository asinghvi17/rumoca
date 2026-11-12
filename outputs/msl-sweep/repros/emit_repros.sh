#!/usr/bin/env bash
# Emit every rumoca repro with both base-modelica targets and print the first
# diagnostic line, or "ok".
#   outputs/msl-sweep/repros/emit_repros.sh RUMOCA MSL_DIR OUTDIR
set -uo pipefail
RUMOCA=$1 MSL=$2 OUT=$3
mkdir -p "$OUT"
for f in "$(dirname "$0")"/rumoca/*.mo; do
  m=$(sed -n "s#^// model: ##p" "$f"); [ -z "$m" ] && m=$(basename "$f" .mo)
  for t in base-modelica base-modelica-scalar; do
    log=$("$RUMOCA" compile "$f" -m "$m" --source-root "$MSL/Complex.mo" --source-root "$MSL/Modelica 4.1.0" \
      --source-root "$MSL/ModelicaServices 4.1.0" --target "$t" -o "$OUT/$t" 2>&1)
    rc=$?
    msg=$(sed 's/\x1b\[[0-9;]*m//g' <<<"$log" | grep -v warning | grep -A2 -E 'rumoca::|\[E[A-Z]*[0-9]+\]' | tr -s ' \n│' ' ' | cut -c1-200)
    printf '%-22s %-21s %s\n' "$m" "$t" "$([ $rc -eq 0 ] && echo ok || echo "FAIL $msg")"
  done
done
