#!/usr/bin/env bash
# Refresh vendor/gpui-* from crates.io at the versions pinned below.
# Usage: scripts/vendor/vendor-sync.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

GPUI_KIT_VERSION="${GPUI_KIT_VERSION:-0.10.0}"
GPUI_PRE_VERSION="${GPUI_PRE_VERSION:-0.3.12}"

fetch_crate() {
    local crate="$1" version="$2" dest="vendor/$1"
    echo "vendor: $crate $version"
    rm -rf "$dest"
    mkdir -p "$dest"
    curl -fsSL "https://crates.io/api/v1/crates/$crate/$version/download" \
        -o "/tmp/$crate.crate" -H "User-Agent: wnav-vendor"
    tar xzf "/tmp/$crate.crate" -C "$dest" --strip-components=1
}

fetch_git() {
    local repo="$1" ref="$2" dest="vendor/$3"
    echo "vendor: $3 @ $ref"
    rm -rf "$dest"
    git clone --depth 1 --branch "$ref" "$repo" "$dest"
}

fetch_git "https://github.com/longbridge/gpui-kit.git" "v$GPUI_KIT_VERSION" "gpui-kit"
for c in gpui-pre gpui-pre-platform gpui-pre-macos gpui-pre-apple; do
    fetch_crate "$c" "$GPUI_PRE_VERSION"
done

echo "vendor: done. cargo build to re-resolve against the patched paths."
