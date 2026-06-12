#!/bin/bash
set -euo pipefail

# Only run in remote Claude Code on the web environments
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

# ── Dependency installation ──────────────────────────────────────────────────
# Add install commands here as the project grows. Examples:
#
#   npm install                          # Node.js
#   pip install -e ".[dev]"              # Python (pyproject.toml)
#   pip install -r requirements.txt      # Python (requirements.txt)
#   bundle install                       # Ruby
#   go mod download                      # Go
#   cargo fetch                          # Rust
# ─────────────────────────────────────────────────────────────────────────────

echo "Session start: no dependencies to install (empty project)."
