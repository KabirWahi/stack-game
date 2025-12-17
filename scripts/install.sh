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

ensure_line() {
  local file="$1"
  local line="$2"
  if [ ! -f "$file" ]; then
    touch "$file"
  fi
  if ! grep -Fqx "$line" "$file"; then
    printf '\n%s\n' "$line" >> "$file"
  fi
}

echo "Installing waitris + stack-game via cargo (from ${REPO})"
cargo install --git "${REPO}" --bin waitris --bin stack-game --force

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
if [ -x "${CARGO_BIN}/waitris" ]; then
  export PATH="${CARGO_BIN}:${PATH}"
  ensure_line "${HOME}/.zshrc" "export PATH=\"${CARGO_BIN}:\$PATH\""
  ensure_line "${HOME}/.bashrc" "export PATH=\"${CARGO_BIN}:\$PATH\""
fi

echo "Running: waitris install-hook"
if command -v waitris >/dev/null 2>&1; then
  waitris install-hook
else
  echo "waitris not found on PATH. Try adding ${CARGO_BIN} to PATH and re-run:" >&2
  echo "  export PATH=\"${CARGO_BIN}:\$PATH\"" >&2
  exit 1
fi

echo "Done. Run 'waitris' to launch."
