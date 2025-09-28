#!/usr/bin/env bash
# Run the upstream libxml2 regression suite while preloading the Rust port.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_DIR="${REPO_ROOT}/build-rs-legacy"

PRELOAD_MODE="${LIBXML2_RS_PRELOAD:-0}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --preload)
      PRELOAD_MODE=1
      shift
      ;;
    --no-preload)
      PRELOAD_MODE=0
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

cmake -S "${REPO_ROOT}" -B "${BUILD_DIR}"
cmake --build "${BUILD_DIR}"

if [[ "${PRELOAD_MODE}" == "1" ]]; then
  cargo build --manifest-path "${REPO_ROOT}/rs/Cargo.toml"

  export LD_PRELOAD="${REPO_ROOT}/rs/target/debug/liblibxml2_rs.so"
  if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
    export LD_LIBRARY_PATH="${BUILD_DIR}:${LD_LIBRARY_PATH}"
  else
    export LD_LIBRARY_PATH="${BUILD_DIR}"
  fi

  echo "Running legacy suite with Rust shim preloaded"
else
  unset LD_PRELOAD
  export LD_LIBRARY_PATH="${BUILD_DIR}:${LD_LIBRARY_PATH:-}"
  echo "Running legacy suite against the in-tree C library"
fi

ctest --test-dir "${BUILD_DIR}" --output-on-failure
