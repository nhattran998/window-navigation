#!/usr/bin/env bash
# Run the CI pipeline locally: exactly what .github/workflows/ci.yml runs.
# Usage: scripts/ci/ci-local.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

step() { printf '\n=== %s ===\n' "$1"; }

step "rustfmt"
cargo fmt --check

step "taplo fmt --check"
taplo fmt --check

step "clippy -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo-deny"
cargo deny check

step "cargo test"
cargo test --workspace

if command -v bazel >/dev/null 2>&1; then
    step "bazel test"
    bazel test //crates/wnav-core:wnav_core_test
    step "bazel build"
    bazel build //crates/wnav //crates/wnav-app:wnav_app //crates/wnav-macos:wnav_macos
else
    echo "bazel not on PATH — skipping Bazel steps (CI still runs them)"
fi

echo
echo "ci-local: all green"
