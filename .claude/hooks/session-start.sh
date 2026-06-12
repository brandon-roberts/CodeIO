#!/bin/bash
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

echo "CodeIO session start: setting up environment..."

# ── Rust toolchain ───────────────────────────────────────────────────────────
if command -v cargo &>/dev/null; then
  echo "  Rust: $(cargo --version)"
  # Pre-warm the dependency cache (doesn't compile, just resolves)
  if [ -f "$CLAUDE_PROJECT_DIR/services/Cargo.toml" ]; then
    cd "$CLAUDE_PROJECT_DIR/services"
    cargo fetch 2>/dev/null && echo "  Rust deps: fetched" || echo "  Rust deps: fetch skipped (offline?)"
    cd "$CLAUDE_PROJECT_DIR"
  fi
else
  echo "  Rust: not installed (install via rustup.rs for services/ layer)"
fi

# ── Python AI layer ──────────────────────────────────────────────────────────
if command -v pip &>/dev/null && [ -f "$CLAUDE_PROJECT_DIR/ai/pyproject.toml" ]; then
  echo "  Python: installing AI layer deps..."
  cd "$CLAUDE_PROJECT_DIR/ai"
  pip install -e ".[dev]" -q 2>&1 | tail -1
  cd "$CLAUDE_PROJECT_DIR"
else
  echo "  Python: pip not found or ai/pyproject.toml missing"
fi

# ── Runtime temp dir ─────────────────────────────────────────────────────────
mkdir -p /tmp/codeio

echo "CodeIO session start: complete."
