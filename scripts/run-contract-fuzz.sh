#!/usr/bin/env bash
#
# Runs the same fuzz suite as .github/workflows/contract-fuzzing.yml locally.
#
# Use this to re-confirm fuzz coverage after a toolchain or wasm target change
# (e.g. the wasm32-unknown-unknown -> wasm32v1-none migration), since a target
# change can alter numeric/overflow behaviour that only the fuzz targets catch.
#
# Usage: ./scripts/run-contract-fuzz.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}/contracts"

export BOLERO_ITERATIONS="${BOLERO_ITERATIONS:-1000}"

echo "==> Ensuring wasm32v1-none target is installed"
rustup target add wasm32v1-none

echo "==> analytics fuzz targets"
cargo test --package analytics fuzz_ -- --nocapture

echo "==> governance fuzz targets"
cargo test --package governance fuzz_ -- --nocapture

echo "==> full contract workspace test suite"
cargo test --workspace -- --nocapture

echo "==> All contract fuzz targets passed on wasm32v1-none"
