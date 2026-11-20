# Contributing
- Branches: `feat/…`, `fix/…`, `chore/…`. Conventional Commits. Squash merges.
- Every PR must build, pass fmt/clippy/tests, update docs, and reference an issue.
- No `unwrap()`/`expect()` in contract code. Never commit secrets.
- Escrow, settlement, disputes and bond changes need two maintainer reviews.

## Common Tasks (`justfile`)

A `justfile` is provided for common developer workflows. Run `just --list` to view available targets:

| Command | Action | Wrapped Command |
|---|---|---|
| `just lint` | Code style & clippy checks | `cargo fmt --all -- --check` && `cargo clippy --workspace --all-targets -- -D warnings` |
| `just build` | Build contracts & workspace | `cargo build --workspace` && `scripts/build.sh` |
| `just test` | Run all workspace tests | `cargo test --workspace` |
| `just bindings` | Regenerate TypeScript bindings | `scripts/bindings.sh` |
| `just artifacts` | Export errors & events JSON | `python3 scripts/export_artifacts.py` |
| `just cov` | Run test line coverage | `cargo llvm-cov --workspace --summary-only` |
| `just check` | Full local CI check suite | `just lint test bindings artifacts` |

## Adding a new contract function

See **[docs/adding-a-function.md](docs/adding-a-function.md)** for the full step-by-step checklist: implementation → tests → WASM build → TypeScript bindings → artifact export → docs → downstream notifications.

## Release Process

Release artifacts and checksums are automatically compiled and published by `.github/workflows/release.yml`:

1. Ensure all CI checks pass on `main` and the workspace version in `Cargo.toml` is up to date.
2. Push a version tag matching `v*` (e.g. `v0.1.0`):
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. The release workflow builds all contract WASMs via `scripts/build.sh`, exports `errors.json` and `events.json`, generates a comprehensive `SHA256SUMS` manifest, and creates a tagged GitHub Release with automated changelog notes.
