#!/usr/bin/env bash
# Simple developer helper for the Rust port. Runs formatting and lint checks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "Running cargo fmt --all --check"
cargo fmt --all --check

echo "Running cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

if ! command -v cbindgen >/dev/null 2>&1; then
    echo "cbindgen is required to verify the generated headers. Install it via 'cargo install --locked cbindgen'." >&2
    exit 1
fi

echo "Running cbindgen to verify libxml.h"
tmp_header="$(mktemp)"
trap 'rm -f "${tmp_header}"' EXIT
cbindgen --config cbindgen.toml --crate libxml2 --output "${tmp_header}"

if ! diff -u libxml.h "${tmp_header}"; then
    echo "libxml.h is out of date; regenerate it with cbindgen." >&2
    exit 1
fi
