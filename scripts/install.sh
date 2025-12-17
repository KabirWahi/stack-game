#!/usr/bin/env bash
# Waitris installer (source-based)
# - installs waitris and stack-game via cargo from the repo
# - installs the shell hook

set -euo pipefail

REPO="https://github.com/KabirWahi/waitris.git"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    return 1
  fi
  return 0
}

missing=0
need_cmd cargo || missing=1
need_cmd tmux || missing=1
need_cmd socat || missing=1

if [ "$missing" -ne 0 ]; then
  echo "Install dependencies (cargo, tmux, socat) and re-run the installer." >&2
  exit 1
fi

echo "Installing waitris + stack-game via cargo (from ${REPO})"
cargo install --git "${REPO}" --bin waitris --bin stack-game --force

echo "Running: waitris install-hook"
waitris install-hook

echo "Done. Run 'waitris' to launch."
