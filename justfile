# Wnav dev commands. `just` (no args) lists everything.
# Bazel targets need bazelisk on PATH; Rust needs rustup with 1.100 stable.

set fallback := false

default:
    @just --list

# ---- Build ------------------------------------------------------------

# Build the whole workspace (debug).
build:
    cargo build --workspace

# Build the release binaries.
build-release:
    cargo build --workspace --release --locked

# Build all targets under Bazel (same as CI).
bazel-build:
    bazel build //crates/wnav //crates/wnav-app:wnav_app //crates/wnav-macos:wnav_macos

# Regenerate Cargo.Bazel.lock after changing any Cargo.toml.
bazel-repin:
    CARGO_BAZEL_REPIN=true bazel run @crates//:crates_vendor

# ---- Test -------------------------------------------------------------

# Unit tests (workspace).
test:
    cargo test --workspace

# Unit tests under Bazel.
bazel-test:
    bazel test //crates/wnav-core:wnav_core_test

# ---- Lint -------------------------------------------------------------

# Clippy with the workspace's strictest ruleset, warnings as errors.
lint-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# cargo-deny: licenses, advisories (RUSTSEC), duplicate bans, sources.
lint-deny:
    cargo deny check

# Check formatting for Rust and TOML without writing.
lint-fmt:
    cargo fmt --check
    taplo fmt --check

# Everything CI runs, locally, in one shot.
lint: lint-clippy lint-deny lint-fmt

# ---- Format -----------------------------------------------------------

# Write-format Rust and TOML.
fmt:
    cargo fmt
    taplo fmt

# ---- Run --------------------------------------------------------------

# Run the CLI (debug build).
run *ARGS:
    cargo run -p wnav -- {{ARGS}}

# Run the menu-bar app (debug build).
app:
    cargo run -p wnav-app

# ---- Release ----------------------------------------------------------

# Regenerate CHANGELOG.md from conventional commits (git-cliff).
changelog:
    git cliff -o CHANGELOG.md

# Full pre-release gate: tests, lints, changelog. Review the diff, then tag.
pre-release: test lint changelog
    @echo "review CHANGELOG.md, then: git tag vX.Y.Z && git push --tags"

# ---- Vendoring --------------------------------------------------------

# Refresh vendored gpui-* from crates.io (scripts/vendor-sync.sh).
vendor-sync:
    bash scripts/vendor/vendor-sync.sh

# Regenerate licenses/inventory.json + THIRD-PARTY-NOTICES.
notices:
    bash scripts/licenses/generate-notices.sh

# ---- Tooling ----------------------------------------------------------

# Refresh the codegraph index after structural changes.
codegraph:
    codegraph init

# Export brand SVGs to platform raster formats (needs rsvg-convert or magick).
brand-export:
    bash scripts/release/export-brand.sh
