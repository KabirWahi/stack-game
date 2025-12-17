#!/usr/bin/env bash
# Waitris uninstaller
# Removes binaries, hook, and config.

set -euo pipefail

remove_file() {
  local path="$1"
  if [ -e "$path" ]; then
    rm -f "$path"
    echo "Removed $path"
  fi
}

remove_line() {
  local file="$1"
  local needle="$2"
  if [ ! -f "$file" ]; then
    return
  fi
  # Use perl to safely filter lines without temp files hassles.
  perl -ne "print unless index(\$_, q{$needle}) != -1" "$file" > "${file}.waitris.tmp" && mv "${file}.waitris.tmp" "$file"
}

HOOK_PATH="${HOME}/.config/waitris/stack-hook.sh"
RC_ZSH="${HOME}/.zshrc"
RC_BASH="${HOME}/.bashrc"

echo "Removing binaries (if present)..."
remove_file "${HOME}/.local/bin/waitris"
remove_file "${HOME}/.cargo/bin/waitris"
remove_file "${HOME}/.local/bin/stack-game"
remove_file "${HOME}/.cargo/bin/stack-game"

echo "Removing hook lines from rc files..."
remove_line "${RC_ZSH}" "${HOOK_PATH}"
remove_line "${RC_BASH}" "${HOOK_PATH}"

echo "Removing hook file and config dir..."
rm -rf "${HOME}/.config/waitris"

echo "Done."
