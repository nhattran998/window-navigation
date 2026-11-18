# Git hooks

Enforce lint/format gates locally so nothing broken reaches GitHub.

- `pre-commit` — rustfmt + taplo + `clippy -D warnings`; stages formatted files.
- `pre-push` — full CI parity: `cargo test`, `cargo deny check`, Bazel test.

## Install (once per clone)

```sh
git config core.hooksPath .githooks
```

Skip a one-off commit with `git commit --no-verify` (use sparingly — CI still
enforces everything via `.github/workflows/ci.yml`).
