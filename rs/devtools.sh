#!/usr/bin/env bash
# Simple developer helper for the Rust port. Runs formatting and lint checks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "Running cargo fmt --all --check"
cargo fmt --all --check

echo "Running cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings
