# STACK  
*A terminal Tetris game driven by your shell commands*

STACK is a Rust TUI game designed to run alongside a real shell.  
Every command you execute spawns Tetris pieces derived from the command text.  
Long-running commands keep the game busy; failed commands corrupt the board.

This project prioritizes:
- terminal-native UX
- ASCII / old-arcade aesthetics (Soviet-era vibes)
- indirect control (you generate the game by working)
- zero interference with the real shell

---

## High-level Architecture

- Two panes (recommended via `tmux`)
  - **Left pane**: real shell (zsh/bash)
  - **Right pane**: `stack-game` (Rust TUI)
- Shell sends command lifecycle events to the game
- Game renders and runs independently
- The game never blocks or alters shell execution

---

## Core Concepts

- **Command-driven piece stream**: commands generate pieces
- **Run mode**: while a command executes, pieces continue spawning
- **ASCII arcade style**: monochrome, chunky glyphs, no modern UI
- **Punitive by design**: long or complex commands create more pieces

---

# PHASE 0 — Repo + Tooling

### Goals
- Create a clean Rust workspace
- Establish terminal UI + input loop

### Steps
1. Create Rust binary crate:
   ```sh
   cargo new stack-game
   ```
2. Add dependencies:
   - `ratatui`
   - `crossterm`
   - `rand`
3. Set up:
   - raw terminal mode
   - alternate screen
   - panic-safe terminal restore

---

# PHASE 1 — Base Tetris + Juiced UI (no command integration)

## 1.1 Game Board

- Dimensions:
  - Width: 10
  - Height: 20
- Representation:
  ```rust
  enum Cell {
      Empty,
      Filled(char),
  }
  ```
- Board stored as `Vec<Vec<Cell>>`

---

## 1.2 Pieces

### Supported Shapes
- Tetrominoes: `I O T S Z J L`
- No hold piece
- Rotations: 4 states per piece

### Piece Struct
```rust
struct Piece {
    shape: Shape,
    rotation: u8,
    x: i32,
    y: i32,
    payload: Vec<char>, // length 1–4 (future: command chunks)
}
```

---

## 1.3 Controls

| Key       | Action        |
|----------|---------------|
| ← / →    | Move left/right |
| ↓        | Soft drop     |
| ↑        | Rotate        |
| Space    | Hard drop     |
| q / Esc  | Quit          |

---

## 1.4 Gravity + Locking

- Gravity tick (start ~450ms)
- If collision on gravity:
  - lock piece
  - write payload chars to board
  - clear lines
  - spawn next piece

---

## 1.5 Line Clears + Scoring (classic only)

| Lines | Score |
|------|-------|
| 1    | 100   |
| 2    | 300   |
| 3    | 500   |
| 4    | 800   |

---

## 1.6 Visual Style (ASCII / Soviet Arcade)

### Playfield rendering contract
- Logical board size: 10×20
- Rendered as a **boxed well** with visible walls:
  - ceiling, left wall, right wall, heavy floor
- Render each filled cell as **2 characters wide** using:
  - payload block: `A░` (letter + shading)
  - empty cell: `  ` (two spaces)
  - ghost cell: `░░` (no letters)
  - infection: `?░`
  - garbage: `#░` (or `X░`)
  - flash: `██`

> Note: even before command-integration, you can still render blocks as `█░` or `#░`.
> When payload letters arrive (Phase 2/3), the same renderer will show them.

### Borders
- Prefer box drawing characters for the well and cabinet:
  - Well: `┌ ┐ └ ┘ │ ─`
  - Floor: `═` (heavier)
- Keep colors minimal (monochrome or very limited).

---

## 1.7 Juiced UI Features (must be done before Phase 2)

### 1.7.1 Cabinet layout (do NOT use full terminal as playfield)
- Draw an outer “cabinet” frame around everything.
- Inside the cabinet:
  - left: **playfield area** (contains the 10×20 well and the falling-piece tower)
  - right: **sidebar** (score/lines/mode + controls legend)
- The well should be fixed-size and centered within its pane.

### 1.7.2 Full well walls (ceiling + sides + heavy floor)
- Draw the well border around the board region:
  - ceiling line
  - left and right walls
  - heavy floor
- The board cells render inside the border only.

### 1.7.3 Ghost piece (“line tracker” / landing indicator)
- Compute ghost by dropping a copy of the active piece until the next step would collide.
- Render ghost using `░░` so it never conflicts with payload letters.

### 1.7.4 Danger line
- Draw a subtle dotted line near the top of the board (e.g., row 2 or 3).
- Use `┈┈` or `··` across the inside width of the well.

### 1.7.5 Sidebar panels
Right pane contains boxed panels:
- **INFO**
  - SCORE
  - LINES
  - MODE: IDLE / RUN
- **CONTROLS**
  - ←/→ move
  - ↑ rotate
  - ↓ soft
  - space slam
  - q/esc quit

### 1.7.6 Game over overlay (only inside the well)
- Overlay message centered inside the well area.
- Do not blank the sidebar.
- Suggested text:
  - `GAME OVER`
  - `q/esc`

### 1.7.7 Line clear flash (1–2 frames)
- When rows clear:
  - flash those rows using `██` for 120ms (or 2 frames)
  - then remove them and shift above rows down

### 1.7.8 Lock thud (1 frame)
- When a piece locks:
  - render those cells with a heavier look for 1 frame (e.g., `▓▓`)
  - then revert to normal

---

# PHASE 2 — tmux Split Launcher + Command-driven Pieces (no shell capture yet)

## 2.0 tmux split launcher (do this before tokenization work)

### Goal
Running the launcher should give:
- left pane: real shell
- right narrow pane: `stack-game` TUI

### Implementation plan
- Create 2 binaries:
  - `stack-game` — the game itself
  - `stack` — tmux launcher
- Recommended run command:
  ```sh
  cargo run --bin stack
  ```
- Optional `Cargo.toml`:
  - set `default-run = "stack"` so `cargo run` works.

### tmux behavior
- If not inside tmux:
  - start a session, split window, run `stack-game` on the right, focus left, attach.
- If inside tmux:
  - split current window, run `stack-game` on the right, focus left.

### Pane sizing
- Right pane is a fixed width, configurable:
  - `STACK_PANE_W` env var
  - default: `36` columns

### Too-small pane handling
- `stack-game` checks terminal size each draw.
- If pane is too narrow:
  - render a centered message: `RESIZE PANE (min width: 36)`

---

## 2.1 Tokenization v1 (simple)

- Split command string by whitespace:
  ```text
  git commit -m fix
  → ["git", "commit", "-m", "fix"]
  ```
- Keep punctuation/operators (e.g., `|`, `&`, `*`, `>`, `<`, `=`) as part of tokens if present.
- Ignore empty tokens

---

## 2.2 Chunking Rules

For **each token independently**:
- Greedily chunk characters into:
  - size 4
  - remainder 1–3

Example:
```
"commit" → ["comm", "it"]
```

---

## 2.3 Shape Mapping by Chunk Size

| Chunk size | Shape type |
|-----------|------------|
| 4         | Tetromino  |
| 3         | Triomino   |
| 2         | Domino     |
| 1         | Monomino   |

Each chunk becomes **one piece**.

---

## 2.4 Payload Assignment

- Each cell of a piece renders one char from the chunk
- Order is deterministic (top-left → bottom-right)
- If shape has fewer cells than chars, truncate
- If more cells, repeat last char

---

## 2.5 CommandRun Model

```rust
struct CommandRun {
    id: u64,
    tokens: Vec<String>,
    current_token: usize,
    current_chunk: usize,
    cycle: usize,
    active: bool,
}
```

---

## 2.6 Repeating Cycles (Long-running commands)

- First cycle:
  - consume all chunks derived from the command
- If command still running:
  - reset chunk index
  - increment `cycle`
  - repeat chunks again
  - shapes are randomized per cycle

This continues **until the command finishes**.

---

# PHASE 3 — Shell Integration

## 3.1 Transport

- Game listens on a Unix socket:
  ```
  /tmp/stack-game.sock
  ```

---

## 3.2 Events

### COMMAND_START
```
START <id> <command string>
```

### COMMAND_END
```
END <id> <exit_code>
```

---

## 3.3 Shell Hook (zsh/bash)

Responsibilities:
- On Enter:
  - send START event
- After command completes:
  - send END event with `$?`

Game does **not** execute commands.

---

# PHASE 4 — Run Mode & Timing

## 4.1 Modes

- **IDLE**
  - normal gravity
  - no new command cycles
- **RUN**
  - gravity increased
  - command chunks stream continues

Minimum RUN duration:
- Even instant commands produce at least one piece

---

# PHASE 5 — Success / Failure Effects

## 5.1 Success (exit code 0)

Grant **one Success Bomb** (stackable, capped).

### Success Bomb Behavior
- Special piece appears in queue
- On lock:
  - clears a 3×3 area centered on drop
- Uses normal gravity and controls

---

## 5.2 Failure (exit code ≠ 0)

Apply both:
1. **Garbage row**
   - added at bottom
   - single random hole
2. **Infection**
   - select N random filled cells
   - replace their character with `?`
   - cosmetic only

---

# PHASE 6 — Variety Scoring

## 6.1 Command Identity

- Identity = first token
  ```
  git commit → "git"
  cargo test → "cargo"
  ```

---

## 6.2 Variety Bonus

- Track last command identity
- If different:
  - `+VARIETY_SCORE` (e.g. +25)
- If same:
  - no bonus

Added directly to score (not a multiplier).

---

# PHASE 7 — Quote-aware Tokenization (optional later)

## Goal
Support:
```
gt create -all "first commit"
→ gt, create, -all, first, commit
```

## Rules
- Quotes removed
- Quoted content split on whitespace
- No escape handling initially
- Unmatched quote consumes rest of string

This replaces `split_whitespace()`.

---

# Design Philosophy

- You generate the chaos you must survive
- Long commands = more pressure
- The game never blocks your work
- ASCII over polish
- Punishment is allowed (and intentional)

---

# Non-goals

- Full shell parsing
- Multiplayer
- Mouse support
- Fancy colors
- Undo / rewind

---

# MVP Completion Criteria

- Tetris playable with juiced UI (walls, ghost, danger line, cabinet, flashes)
- tmux launcher opens split: shell left + game right
- Commands spawn letter-based pieces (Phase 2+)
- Long-running commands repeat cycles
- Success bomb + failure corruption
- Variety scoring active
- Stable for long sessions

---

# Possible Names (CLI-style)

- `stack`
- `heap`
- `overflow`
- `fall`
- `blocks`

---

END
