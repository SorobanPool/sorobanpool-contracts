# Contributing
- Branches: `feat/…`, `fix/…`, `chore/…`. Conventional Commits. Squash merges.
- Every PR must build, pass fmt/clippy/tests, update docs, and reference an issue.
- No `unwrap()`/`expect()` in contract code. Never commit secrets.
- Escrow, settlement, disputes and bond changes need two maintainer reviews.

## Pre-commit and CI checks

Run the local check script before pushing to mirror CI checks (fmt, clippy `-D warnings`, and tests using the pinned toolchain from `rust-toolchain.toml`):

```bash
./scripts/check.sh
```

To automatically run these checks on every `git commit`, install the pre-commit hook:

```bash
./scripts/check.sh --install-hook
```

## Adding a new contract function

See **[docs/adding-a-function.md](docs/adding-a-function.md)** for the full step-by-step checklist: implementation → tests → WASM build → TypeScript bindings → artifact export → docs → downstream notifications.
