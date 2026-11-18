#!/usr/bin/env bash
# Regenerate licenses/inventory.json and THIRD-PARTY-NOTICES from Cargo.lock.
# Uses cargo-about when available; falls back to a lockfile-driven summary.
# Usage: scripts/licenses/generate-notices.sh
set -euo pipefail
cd "$(dirname "$0")/../.."

if command -v cargo-about >/dev/null 2>&1; then
    echo "notices: cargo-about"
    cargo about generate scripts/licenses/notices.hbs -o THIRD-PARTY-NOTICES
else
    echo "notices: cargo-about missing — writing a summary from Cargo.lock"
    echo "# Third-party notices" > THIRD-PARTY-NOTICES
    echo >> THIRD-PARTY-NOTICES
    echo "Generated from Cargo.lock; install cargo-about for full license texts." >> THIRD-PARTY-NOTICES
    echo >> THIRD-PARTY-NOTICES
    python3 - <<'PY'
import re

lock = open("Cargo.lock").read()
pkgs = re.findall(r'\[\[package\]\]\nname = "([^"]+)"\nversion = "([^"]+)"(?:[^[]*?source = "([^"]+)")?', lock)
with open("THIRD-PARTY-NOTICES", "a") as out:
    for name, ver, source in pkgs:
        if not source:
            continue  # workspace members carry our own MIT license
        out.write(f"- {name} {ver}\n")
PY
fi
echo "notices: wrote THIRD-PARTY-NOTICES"
