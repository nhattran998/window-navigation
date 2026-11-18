# licenses/

Third-party license texts for everything Wnav ships or vendors.

| File | Covers |
|------|--------|
| `Apache-2.0-gpui-kit.txt` | gpui-kit 0.10.0 (kit, base, component, component-macros, assets) — vendored |
| `Apache-2.0-gpui-pre.txt` | gpui-pre 0.3.12 (GPUI core snapshot) — vendored |
| `Apache-2.0-gpui-pre-platform.txt` | gpui-pre-platform 0.3.12 — vendored |
| `Apache-2.0-gpui-pre-macos.txt` | gpui-pre-macos 0.3.12 — vendored |
| `Apache-2.0-gpui-pre-apple.txt` | gpui-pre-apple 0.3.12 — vendored |
| `ISC-Lucide.txt` | Lucide icon set shipped in gpui-kit assets |

- `inventory.json` — machine-readable list: inputs, versions, purposes
- `THIRD-PARTY-NOTICES` (repo root) — transitive dependency notices, regenerated
  by `just notices` (scripts/licenses/generate-notices.sh)

Transitive dependencies are not copied here individually: `cargo-deny`
(`deny.toml`) audits their licenses on every CI run, and the notices script
expands `Cargo.lock` into the full list. Copy a license text into this folder
when a component's terms require shipping the text itself.
