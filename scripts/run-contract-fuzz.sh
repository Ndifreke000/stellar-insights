#!/usr/bin/env bash
#
# Runs the same contract test suite as .github/workflows/contract-fuzzing.yml locally.
#
# Usage: ./scripts/run-contract-fuzz.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}/contracts"

echo "==> Ensuring wasm32v1-none target is installed"
rustup target add wasm32v1-none

echo "==> payraider workspace tests"
cargo test --workspace -- --nocapture

echo "==> All active contract tests passed"
