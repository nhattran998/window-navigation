# scripts/

Developer and release automation. `justfile` is the entry point — each recipe
here is invoked by a `just` task, so agents and humans share one interface.

```
scripts/
  vendor/      vendored-dependency maintenance
    vendor-sync.sh       refresh vendor/gpui-* from crates.io (pinned versions)
  licenses/    third-party notice generation
    generate-notices.sh  licenses/inventory.json + THIRD-PARTY-NOTICES
  release/     cutting releases
    release.sh           pre-release gate: test + lint + changelog + tag hint
  ci/          CI parity helpers (run locally what CI runs)
    ci-local.sh          the whole pipeline: fmt + clippy + deny + test + bazel
```

Adding a script: put it under the matching subfolder, wire a `just` recipe,
and document it here. Scripts are POSIX bash, `set -euo pipefail`, runnable
from any cwd.
