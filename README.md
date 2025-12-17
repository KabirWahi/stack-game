# WAITRIS

Terminal Tetris driven by your shell commands.

## Install

1) Build the binaries:

```sh
cargo build
```

2) Ensure tmux is installed (required for the split launcher).

3) Run the launcher:

```sh
cargo run --bin waitris
```

This opens a tmux session with:
- left pane: your shell
- right pane: the game

## Shell Hook (optional, to stream commands)

Source the hook in your shell so commands emit START/END events:

```sh
source /path/to/stack-game/scripts/stack-hook.sh
```

If you want it always on, add it to your shell profile (zsh example):

```sh
echo 'source /path/to/stack-game/scripts/stack-hook.sh' >> ~/.zshrc
```

To remove, delete that line from your shell profile.

## Notes

- The game listens on: `/tmp/stack-game.sock`
- You can run just the game (no tmux) with:

```sh
cargo run --bin stack-game
```
