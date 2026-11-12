#!/usr/bin/env bash
# Fetch and build everything the MSL sweep needs, idempotently, under BASE
# (default: target/msl-sweep in the repo, which is gitignored).
#
#   outputs/msl-sweep/setup.sh [BASE] [rust|msl|refs|julia ...]
#
# rust:  rustup + nightly-2026-02-27 under BASE, release build of rumoca with
#        every default feature except input-gamepad (needs libudev).
# msl:   ModelicaStandardLibrary v4.1.0 release zip -> BASE/msl.
# refs:  MAP-LIB_ReferenceResults v4.1.0, sparse: Modelica/**/*.csv and
#        comparisonSignals.txt -> BASE/refs (about 1.9 GB).
# julia: instantiate outputs/msl-sweep/julia (BaseModelica.jl fork branch
#        bm-events-and-parser-fixes, DyadInterface from DyadRegistry) and bootstrap
#        the conda ANTLR runtime declared in julia/CondaPkg.toml.
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../.." && pwd)
BASE=${1:-$REPO/target/msl-sweep}
shift || true
STEPS=${*:-rust msl refs julia}
mkdir -p "$BASE"
export GIT_TERMINAL_PROMPT=0 JULIA_PKG_USE_CLI_GIT=true

for step in $STEPS; do
  case $step in
  rust)
    export RUSTUP_HOME=$BASE/rustup CARGO_HOME=$BASE/cargo RUSTUP_TOOLCHAIN=nightly-2026-02-27
    if [ ! -x "$CARGO_HOME/bin/cargo" ]; then
      curl -sSf https://sh.rustup.rs -o "$BASE/rustup-init.sh"
      sh "$BASE/rustup-init.sh" -y --no-modify-path --profile minimal --default-toolchain nightly-2026-02-27
    fi
    (cd "$REPO" && "$CARGO_HOME/bin/cargo" build --release -p rumoca --bin rumoca --no-default-features \
      --features scheduled-sim,scenario-config,input-keyboard,transport-udp,transport-zenoh,viewer-web,process-control,native-allocator,fmu-packaging,component-build)
    ;;
  msl)
    if [ ! -d "$BASE/msl/Modelica 4.1.0" ]; then
      mkdir -p "$BASE/dl"
      curl -sSLf -o "$BASE/dl/msl.zip" \
        https://github.com/modelica/ModelicaStandardLibrary/releases/download/v4.1.0/ModelicaStandardLibrary_v4.1.0.zip
      unzip -q -o "$BASE/dl/msl.zip" -d "$BASE/msl"
    fi
    ;;
  refs)
    if [ ! -d "$BASE/refs/.git" ]; then
      git clone --filter=blob:none --no-checkout --depth 1 -b v4.1.0 \
        https://github.com/modelica/MAP-LIB_ReferenceResults.git "$BASE/refs"
    fi
    git -C "$BASE/refs" sparse-checkout set --no-cone '/Modelica/**/*.csv' '/Modelica/**/comparisonSignals.txt'
    git -C "$BASE/refs" checkout -q v4.1.0
    ;;
  julia)
    julia --startup-file=no --project="$REPO/outputs/msl-sweep/julia" -e '
      using Pkg; Pkg.instantiate(); Pkg.precompile()
      using CondaPkg; CondaPkg.resolve()
      using BaseModelica, DyadInterface; println("julia env ok")'
    ;;
  esac
done
