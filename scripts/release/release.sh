#!/usr/bin/env bash
# Full pre-release gate: version bump check, tests, lints, changelog.
# Usage: scripts/release/release.sh [X.Y.Z]
set -euo pipefail
cd "$(dirname "$0")/../.."

if [ $# -ne 1 ]; then
    echo "usage: $0 <X.Y.Z>" >&2
    exit 2
fi
version="$1"

current=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
if [ "$version" = "$current" ]; then
    echo "release: workspace already at $version"
else
    echo "release: bump $current -> $version"
    sed -i '' "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
fi

echo "release: running tests + lints"
just test
just lint

echo "release: changelog"
just changelog

last=$(git describe --tags --abbrev=0 --match 'v*' 2>/dev/null || echo "(no tags yet)")
echo
echo "release: changes since $last"
git log --oneline "$last"..HEAD | head -20
echo
echo "next: review CHANGELOG.md, commit, then:"
echo "  git tag v$version && git push origin main v$version"
echo "release.yml publishes the GitHub release from the tag."
