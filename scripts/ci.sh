#!/usr/bin/env bash
# Single source of truth for CI. Run locally with: bash scripts/ci.sh
set -euo pipefail

echo "==> rustfmt"
cargo fmt --all --check

echo "==> clippy (all features)"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> build: feature matrix"
cargo build --all-features
cargo build --no-default-features
cargo build --no-default-features --features half

echo "==> build: bare-metal (thumbv7em-none-eabi, no alloc)"
cargo build --no-default-features --target thumbv7em-none-eabi
cargo build --no-default-features --features half --target thumbv7em-none-eabi

echo "==> test (all features)"
cargo test --all-features

echo "==> coverage (llvm-cov -> lcov)"
cargo llvm-cov --all-features --lcov --output-path lcov.info --fail-under-lines 90

echo "==> CRAP metric"
cargo crap --lcov lcov.info --fail-above

echo "==> CI OK"
