#!/usr/bin/env bash
set -euo pipefail

# scripts/check.sh: Local pre-commit script mirroring CI checks.
# Runs fmt check, clippy with -D warnings, and workspace tests using pinned toolchain.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

if [[ "${1:-}" == "--install-hook" ]]; then
  HOOK_DEST="$ROOT_DIR/.git/hooks/pre-commit"
  mkdir -p "$ROOT_DIR/.git/hooks"
  cat <<'EOF' > "$HOOK_DEST"
#!/usr/bin/env bash
exec ./scripts/check.sh
EOF
  chmod +x "$HOOK_DEST"
  echo "✓ Pre-commit hook installed at .git/hooks/pre-commit"
  exit 0
fi

# Detect pinned toolchain channel from rust-toolchain.toml
TOOLCHAIN_ARG=""
if [[ -f "rust-toolchain.toml" ]]; then
  CHANNEL=$(grep -E '^\s*channel\s*=' rust-toolchain.toml | sed -E 's/.*"([^"]+)".*/\1/')
  if [[ -n "$CHANNEL" ]] && rustup toolchain list 2>/dev/null | grep -q "$CHANNEL"; then
    TOOLCHAIN_ARG="+$CHANNEL"
  fi
fi

echo "==> Running cargo fmt check..."
cargo ${TOOLCHAIN_ARG} fmt --all -- --check

echo "==> Running cargo clippy..."
cargo ${TOOLCHAIN_ARG} clippy --workspace --all-targets -- -D warnings

echo "==> Running cargo test..."
cargo ${TOOLCHAIN_ARG} test --workspace

echo "✓ All checks passed!"
