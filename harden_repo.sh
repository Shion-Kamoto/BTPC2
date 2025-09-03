#!/usr/bin/env bash
# harden_repo.sh
# Purpose: install CI, clippy config, and a pre-commit hook for BTPC2.
# Run this from the repository root (where Cargo.toml lives).

set -euo pipefail

ROOT="$(pwd)"
echo "[*] Repo root: $ROOT"

# --- GitHub Actions workflow ---
WF_DIR=".github/workflows"
WF_FILE="$WF_DIR/rust.yml"
mkdir -p "$WF_DIR"
cat > "$WF_FILE" <<'YAML'
name: Rust CI

on:
  push:
  pull_request:

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      - name: fmt
        run: cargo fmt --all -- --check
      - name: clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: test
        run: cargo test --all --quiet

YAML
echo "[*] Wrote $WF_FILE"

# --- Clippy config ---
CLIPPY_FILE="clippy.toml"
if [[ -f "$CLIPPY_FILE" ]]; then
  echo "[*] Backing up existing $CLIPPY_FILE -> $CLIPPY_FILE.bak"
  cp "$CLIPPY_FILE" "$CLIPPY_FILE.bak"
fi
cat > "$CLIPPY_FILE" <<'TOML'
# Clippy configuration for BTPC2
# We keep pedantic at warn initially to avoid blocking merges while tightening later.
warn = [
  "clippy::all",
  "clippy::pedantic",
  "clippy::cargo",
]

deny = [
  "warnings",
  "clippy::unwrap_used",
  "clippy::expect_used",
  "clippy::panic",
  "clippy::todo",
  "clippy::unimplemented",
]

# Reduce noise from common pedantic lints (adjust as the codebase stabilizes)
allow = [
  "clippy::module_name_repetitions",
  "clippy::missing_panics_doc",
  "clippy::missing_errors_doc",
  "clippy::must_use_candidate",
  "clippy::implicit_hasher",
]

TOML
echo "[*] Wrote $CLIPPY_FILE"

# --- Pre-commit hook ---
HOOK_DIR=".git/hooks"
HOOK_FILE="$HOOK_DIR/pre-commit"
mkdir -p "$HOOK_DIR"
cat > "$HOOK_FILE" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail

echo "[pre-commit] rustfmt check..."
cargo fmt --all -- --check

echo "[pre-commit] clippy (deny warnings)..."
cargo clippy --all-targets -- -D warnings

echo "[pre-commit] tests..."
cargo test --all --quiet

HOOK
chmod +x "$HOOK_FILE"
echo "[*] Installed pre-commit hook at $HOOK_FILE"

echo "[*] Done. Next steps:"
echo "  - git add .github/workflows/rust.yml clippy.toml"
echo "  - git commit -m 'chore: add CI, clippy config, and pre-commit hook'"
echo "  - (hooks will run on future commits)"
