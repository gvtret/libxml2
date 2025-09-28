#!/usr/bin/env bash
# Run the upstream libxml2 regression suite while preloading the Rust port.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_DIR="${REPO_ROOT}/build-rs-legacy"

cmake -S "${REPO_ROOT}" -B "${BUILD_DIR}"
cmake --build "${BUILD_DIR}"

cargo build --manifest-path "${REPO_ROOT}/rs/Cargo.toml"

export LD_PRELOAD="${REPO_ROOT}/rs/target/debug/liblibxml2_rs.so"
if [[ -n "${LD_LIBRARY_PATH:-}" ]]; then
  export LD_LIBRARY_PATH="${BUILD_DIR}:${LD_LIBRARY_PATH}"
else
  export LD_LIBRARY_PATH="${BUILD_DIR}"
fi

ctest --test-dir "${BUILD_DIR}" --output-on-failure
