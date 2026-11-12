#!/usr/bin/env bash
# Stage 1 of the MSL sweep: emit Base Modelica for every model with rumoca.
#
#   outputs/msl-sweep/emit.sh RUMOCA MSL_DIR WORK [JOBS] < models.txt
#
# MSL_DIR holds Complex.mo, "Modelica 4.1.0/" and "ModelicaServices 4.1.0/".
# For each model and each target (base-modelica, base-modelica-scalar) this
# writes WORK/bmo/<target>/<Model>.bmo, the rumoca log in WORK/emitlog/, and
# one row in WORK/emit/<Model>.<target>.tsv:
#   model  target  status(ok|fail|timeout)  seconds  signature
# Models that already have a row are skipped, so the script is resumable.
set -uo pipefail
RUMOCA=$1 MSL=$2 WORK=$3 JOBS=${4:-8}
mkdir -p "$WORK/bmo/base-modelica" "$WORK/bmo/base-modelica-scalar" "$WORK/emit" "$WORK/emitlog" "$WORK/rcache"
export RUMOCA MSL WORK NO_COLOR=1

emit_one() {
  local model=$1 target=$2
  local row="$WORK/emit/$model.$target.tsv"
  [ -s "$row" ] && return 0
  local out="$WORK/bmo/$target" log="$WORK/emitlog/$model.$target.log"
  local t0=$(date +%s.%N)
  timeout 300 "$RUMOCA" --cache-dir "$WORK/rcache" compile "$MSL/Modelica 4.1.0/package.mo" -m "$model" \
    --source-root "$MSL/Complex.mo" --source-root "$MSL/Modelica 4.1.0" \
    --source-root "$MSL/ModelicaServices 4.1.0" --target "$target" -o "$out" >"$log" 2>&1
  local rc=$? t1=$(date +%s.%N)
  local secs=$(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.1f", b-a}')
  local status=ok sig=""
  if [ $rc -eq 124 ]; then
    status=timeout sig="timeout 300s"
  elif [ $rc -ne 0 ] || [ ! -s "$out/${model//./_}.bmo" ]; then
    status=fail
    # Raw signature: the diagnostic code and the first line of its message.
    # categorize.py normalises it further from the full log.
    local clean code msg
    clean=$(sed 's/\x1b\[[0-9;]*m//g' "$log")
    code=$(grep -m1 -oE 'rumoca::[A-Za-z_:]*[A-Z]+[0-9]+' <<<"$clean")
    msg=$(grep -m1 -A3 -E 'rumoca::[A-Za-z_:]*[A-Z]+[0-9]+' <<<"$clean" | tail -n +2 | sed -E 's/^[ │×]*//' | grep -v '^\s*$' | head -1)
    [ -z "$msg" ] && msg=$(grep -v '^\s*$' <<<"$clean" | grep -v warning | tail -1)
    sig=$(printf '%s %s' "${code:-exit $rc}" "$msg" | tr '\t' ' ' | cut -c1-300)
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$model" "$target" "$status" "$secs" "$sig" >"$row"
}
export -f emit_one

while read -r m; do
  [ -n "$m" ] && printf '%s base-modelica\n%s base-modelica-scalar\n' "$m" "$m"
done | xargs -P "$JOBS" -L 1 bash -c 'emit_one "$0" "$1"'
