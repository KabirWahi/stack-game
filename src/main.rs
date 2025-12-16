use std::error::Error;
use std::io::{stdout, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use std::env;
use std::process::Command;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Terminal;
use std::time::Instant;
use rand::seq::SliceRandom;
use rand::thread_rng;

type Term = Terminal<CrosstermBackend<Stdout>>;

// Core dimensions and rendering geometry.
const BOARD_W: usize = 10;
const BOARD_H: usize = 20;
const CELL_W: usize = 2; // render each block as two characters wide (letter + filler)
const PLAY_W: usize = BOARD_W * CELL_W + 2; // inner width plus side walls
const PLAY_H: usize = BOARD_H + 2; // inner height plus ceiling/floor
const MIN_PANE_WIDTH: u16 = 36;

#[derive(Clone, Copy)]
enum Cell {
    Empty,
    Filled(char),
}

struct Board {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Board {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::Empty; width * height],
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn get(&self, x: usize, y: usize) -> Cell {
        self.cells[self.idx(x, y)]
    }

    fn set(&mut self, x: usize, y: usize, value: Cell) {
        let idx = self.idx(x, y);
        self.cells[idx] = value;
    }
}

#[derive(Clone, Copy)]
enum Shape {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

#[derive(Clone)]
struct Piece {
    shape: Shape,
    rotation: u8,
    x: i32,
    y: i32,
    payload: Vec<char>,
}

impl Piece {
    fn with_payload(shape: Shape, payload: Vec<char>) -> Self {
        Self {
            shape,
            rotation: 0,
            x: 3, // spawn near center top
            y: 0,
            payload,
        }
    }

    fn cells(&self) -> Vec<(i32, i32, char)> {
        let offsets = shape_offsets(self.shape, self.rotation);
        offsets
            .iter()
            .enumerate()
            .map(|(i, (dx, dy))| {
                let ch = self.payload.get(i).copied().unwrap_or_else(|| {
                    *self.payload.last().unwrap_or(&'#')
                });
                (self.x + dx, self.y + dy, ch)
            })
            .collect()
    }

    fn rotated(&self) -> Self {
        let mut next = self.clone();
        next.rotation = (next.rotation + 1) % 4;
        next
    }

    fn shifted(&self, dx: i32, dy: i32) -> Self {
        let mut next = self.clone();
        next.x += dx;
        next.y += dy;
        next
    }
}

fn shape_offsets(shape: Shape, rotation: u8) -> &'static [(i32, i32)] {
    // Offsets assume a 4x4 grid origin at top-left.
    const I: [[(i32, i32); 4]; 4] = [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ];
    const O: [[(i32, i32); 4]; 4] = [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ];
    const T: [[(i32, i32); 4]; 4] = [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ];
    const S: [[(i32, i32); 4]; 4] = [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ];
    const Z: [[(i32, i32); 4]; 4] = [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ];
    const J: [[(i32, i32); 4]; 4] = [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ];
    const L: [[(i32, i32); 4]; 4] = [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ];

    let r = (rotation % 4) as usize;
    match shape {
        Shape::I => &I[r],
        Shape::O => &O[r],
        Shape::T => &T[r],
        Shape::S => &S[r],
        Shape::Z => &Z[r],
        Shape::J => &J[r],
        Shape::L => &L[r],
    }
}

struct Game {
    board: Board,
    current: Piece,
    game_over: bool,
    score: u64,
    lines_cleared: u64,
    pending_clear: Vec<usize>,
    clear_flash_frames: u8,
    lock_flash_cells: Vec<(usize, usize)>,
    lock_flash_frames: u8,
}

impl Game {
    fn new() -> Self {
        let board = Board::new(BOARD_W, BOARD_H);
        let mut game = Self {
            board,
            current: Piece::with_payload(Shape::I, vec!['I'; 4]),
            game_over: false,
            score: 0,
            lines_cleared: 0,
            pending_clear: Vec::new(),
            clear_flash_frames: 0,
            lock_flash_cells: Vec::new(),
            lock_flash_frames: 0,
        };
        game.spawn_random();
        game
    }

    fn can_place(&self, piece: &Piece) -> bool {
        for (x, y, _) in piece.cells() {
            if x < 0 || y < 0 {
                return false;
            }
            let (xu, yu) = (x as usize, y as usize);
            if xu >= self.board.width || yu >= self.board.height {
                return false;
            }
            if let Cell::Filled(_) = self.board.get(xu, yu) {
                return false;
            }
        }
        true
    }

    fn lock_piece(&mut self) {
        self.lock_flash_cells.clear();
        for (x, y, ch) in self.current.cells() {
            if x >= 0 && y >= 0 {
                let (xu, yu) = (x as usize, y as usize);
                if xu < self.board.width && yu < self.board.height {
                    self.board.set(xu, yu, Cell::Filled(ch));
                    self.lock_flash_cells.push((xu, yu));
                }
            }
        }
        self.lock_flash_frames = 1;
        let full_rows: Vec<usize> = (0..self.board.height)
            .filter(|y| (0..self.board.width).all(|x| matches!(self.board.get(x, *y), Cell::Filled(_))))
            .collect();
        if !full_rows.is_empty() {
            self.pending_clear = full_rows;
            self.clear_flash_frames = 2;
        }
    }

    fn spawn_piece(&mut self, shape: Shape, payload: Vec<char>) -> bool {
        let mut piece = Piece::with_payload(shape, payload);
        // Try default spawn position; if blocked, game over in future steps.
        if self.can_place(&piece) {
            self.current = piece;
            true
        } else {
            false
        }
    }

    fn move_current(&mut self, dx: i32, dy: i32) -> bool {
        if self.game_over {
            return false;
        }
        let next = self.current.shifted(dx, dy);
        if self.can_place(&next) {
            self.current = next;
            true
        } else {
            false
        }
    }

    fn rotate_current(&mut self) -> bool {
        if self.game_over {
            return false;
        }
        let next = self.current.rotated();
        if self.can_place(&next) {
            self.current = next;
            true
        } else {
            false
        }
    }

    fn tick_gravity(&mut self) {
        if self.game_over {
            return;
        }
        if !self.move_current(0, 1) {
            self.lock_piece();
            self.spawn_random();
        }
    }

    fn hard_drop(&mut self) {
        if self.game_over {
            return;
        }
        while self.move_current(0, 1) {}
        self.lock_piece();
        self.spawn_random();
    }

    fn process_effects(&mut self) {
        if self.lock_flash_frames > 0 {
            self.lock_flash_frames -= 1;
        }
        if self.clear_flash_frames > 0 {
            self.clear_flash_frames -= 1;
            if self.clear_flash_frames == 0 && !self.pending_clear.is_empty() {
                self.perform_pending_clear();
            }
        }
    }

    fn spawn_random(&mut self) {
        let shapes = [
            Shape::I,
            Shape::O,
            Shape::T,
            Shape::S,
            Shape::Z,
            Shape::J,
            Shape::L,
        ];
        let mut rng = thread_rng();
        let shape = *shapes.choose(&mut rng).unwrap_or(&Shape::I);
        let payload = vec![shape_char(shape); 4];
        let piece = Piece::with_payload(shape, payload);
        if self.can_place(&piece) {
            self.current = piece;
        } else {
            self.game_over = true;
        }
    }

    fn ghost_piece(&self) -> Piece {
        let mut ghost = self.current.clone();
        while {
            let next = ghost.shifted(0, 1);
            self.can_place(&next)
        } {
            ghost.y += 1;
        }
        ghost
    }

    fn clear_full_lines(&mut self) -> u64 {
        // Deprecated in favor of perform_pending_clear; kept for reference.
        0
    }

    fn add_score(&mut self, cleared: u64) {
        let add = match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };
        self.score += add;
    }

    fn perform_pending_clear(&mut self) {
        let cleared = self.pending_clear.len() as u64;
        if cleared == 0 {
            return;
        }
        let mut new_cells = Vec::with_capacity(self.board.cells.len());
        for y in 0..self.board.height {
            if self.pending_clear.contains(&y) {
                continue;
            }
            for x in 0..self.board.width {
                new_cells.push(self.board.get(x, y));
            }
        }
        for _ in 0..cleared {
            for _ in 0..self.board.width {
                new_cells.insert(0, Cell::Empty);
            }
        }
        self.board.cells = new_cells;
        self.lines_cleared += cleared;
        self.add_score(cleared);
        self.pending_clear.clear();
    }
}

fn shape_char(shape: Shape) -> char {
    match shape {
        Shape::I => 'I',
        Shape::O => 'O',
        Shape::T => 'T',
        Shape::S => 'S',
        Shape::Z => 'Z',
        Shape::J => 'J',
        Shape::L => 'L',
    }
}

struct TuiGuard {
    terminal: Term,
}

impl TuiGuard {
    fn new() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        Ok(Self { terminal })
    }

    fn terminal_mut(&mut self) -> &mut Term {
        &mut self.terminal
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        // Attempt to restore the terminal; ignore errors during drop.
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut tui = TuiGuard::new()?;
    let result = run_app(tui.terminal_mut());
    cleanup_tmux_on_exit();
    result
}

fn run_app(terminal: &mut Term) -> Result<(), Box<dyn Error>> {
    let mut game = Game::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| draw_game(frame, &game))?;

        game.process_effects();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
                handle_input(key.code, &mut game);
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(450) {
            game.tick_gravity();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn cleanup_tmux_on_exit() {
    let managed = env::var("STACK_MANAGED").unwrap_or_default() == "1";
    let kill_session = env::var("STACK_KILL_SESSION").unwrap_or_default() == "1";
    if !managed || env::var("TMUX").is_err() {
        return;
    }

    if kill_session {
        if let Ok(session) = tmux_current_session() {
            let _ = Command::new("tmux")
                .args(&["kill-session", "-t", &session])
                .status();
        }
    } else {
        // Fallback: kill the current pane (game pane) if possible.
        let _ = Command::new("tmux").args(&["kill-pane"]).status();
    }
}

fn tmux_current_session() -> Result<String, Box<dyn Error>> {
    let out = Command::new("tmux")
        .args(&["display-message", "-p", "#S"])
        .output()?;
    if out.status.success() {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(name)
    } else {
        Err("tmux display-message failed".into())
    }
}

fn handle_input(code: KeyCode, game: &mut Game) {
    match code {
        KeyCode::Left => {
            let _ = game.move_current(-1, 0);
        }
        KeyCode::Right => {
            let _ = game.move_current(1, 0);
        }
        KeyCode::Down => {
            let _ = game.move_current(0, 1);
        }
        KeyCode::Up => {
            let _ = game.rotate_current();
        }
        KeyCode::Char(' ') => {
            game.hard_drop();
        }
        _ => {}
    }
}

fn draw_game(frame: &mut Frame, game: &Game) {
    let area = frame.size();

    if area.width < MIN_PANE_WIDTH {
        let msg = Paragraph::new(format!("RESIZE PANE (min width: {})", MIN_PANE_WIDTH))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("STACK"));
        frame.render_widget(msg, area);
        return;
    }

    // Outer "cabinet" frame.
    let cabinet = Block::default()
        .title("STACK")
        .border_type(BorderType::Thick)
        .borders(Borders::ALL)
        .title_alignment(Alignment::Left);
    let cabinet_inner = cabinet.inner(area);
    frame.render_widget(cabinet, area);

    // Split into play area (left) and sidebar (right).
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min((PLAY_W as u16 + 6).max(30)), // padding left of playfield
            Constraint::Length(24),
        ])
        .split(cabinet_inner);

    // Center the fixed-size playfield within the left column.
    let v_center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(PLAY_H as u16),
            Constraint::Min(1),
        ])
        .split(cols[0]);
    let h_center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(PLAY_W as u16),
            Constraint::Min(1),
        ])
        .split(v_center[1]);
    let play_rect = h_center[1];

    draw_playfield(frame, game, play_rect);
    draw_sidebar(frame, game, cols[1]);
}

fn draw_playfield(frame: &mut Frame, game: &Game, play_rect: Rect) {
    let mut grid = vec![vec![' '; PLAY_W]; PLAY_H];

    // Border: top/ceiling, sides, heavy floor.
    grid[0][0] = '┌';
    grid[0][PLAY_W - 1] = '┐';
    for x in 1..PLAY_W - 1 {
        grid[0][x] = '─';
    }
    for y in 1..PLAY_H - 1 {
        grid[y][0] = '│';
        grid[y][PLAY_W - 1] = '│';
    }
    grid[PLAY_H - 1][0] = '└';
    grid[PLAY_H - 1][PLAY_W - 1] = '┘';
    for x in 1..PLAY_W - 1 {
        grid[PLAY_H - 1][x] = '═';
    }

    // Helper to plot a filled block in the inner area. Draw as `letter + light filler`.
    let mut plot_block = |grid: &mut [Vec<char>], bx: usize, by: usize, ch: char| {
        let gx = 1 + bx * CELL_W;
        let gy = 1 + by;
        if gy < PLAY_H && gx + 1 < PLAY_W {
            grid[gy][gx] = ch;
            grid[gy][gx + 1] = '░';
        }
    };

    // Locked cells (with optional lock flash override).
    for y in 0..game.board.height {
        for x in 0..game.board.width {
            if let Cell::Filled(ch) = game.board.get(x, y) {
                let flashing = game.lock_flash_frames > 0
                    && game.lock_flash_cells.contains(&(x, y));
                let glyph = if flashing { '▓' } else { ch };
                plot_block(&mut grid, x, y, glyph);
            }
        }
    }

    // Line clear flash override.
    if game.clear_flash_frames > 0 && !game.pending_clear.is_empty() {
        for &row in &game.pending_clear {
            if row < BOARD_H {
                let gy = 1 + row;
                for x in 0..BOARD_W {
                    let gx = 1 + x * CELL_W;
                    if gy < PLAY_H && gx + 1 < PLAY_W {
                        grid[gy][gx] = '█';
                        grid[gy][gx + 1] = '█';
                    }
                }
            }
        }
    }

    // Ghost piece: draw with faint glyphs.
    let ghost = game.ghost_piece();
    for (x, y, _) in ghost.cells() {
        if x >= 0 && y >= 0 {
            let (xu, yu) = (x as usize, y as usize);
            if xu < game.board.width && yu < game.board.height {
                let gx = 1 + xu * CELL_W;
                let gy = 1 + yu;
                if gy < PLAY_H && gx + 1 < PLAY_W {
                    grid[gy][gx] = '·';
                    grid[gy][gx + 1] = '·';
                }
            }
        }
    }

    // Active piece.
    for (x, y, ch) in game.current.cells() {
        if x >= 0 && y >= 0 {
            let (xu, yu) = (x as usize, y as usize);
            if xu < game.board.width && yu < game.board.height {
                plot_block(&mut grid, xu, yu, ch);
            }
        }
    }

    let lines: Vec<Line> = grid
        .iter()
        .map(|row| Line::raw(row.iter().collect::<String>()))
        .collect();

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, play_rect);

    if game.game_over {
        let overlay_w = (PLAY_W as u16).saturating_sub(4).max(8);
        let overlay_h = 5u16;
        let popup = Rect {
            x: play_rect.x + (play_rect.width.saturating_sub(overlay_w)) / 2,
            y: play_rect.y + (play_rect.height.saturating_sub(overlay_h)) / 2,
            width: overlay_w,
            height: overlay_h,
        };
        let overlay = Paragraph::new("GAME OVER\nPress q/Esc")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(overlay, popup);
    }
}

fn draw_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(5), Constraint::Length(9)].as_ref())
        .split(area);

    let info = Paragraph::new(format!(
        "SCORE\n{}\n\nLINES\n{}\n\nMODE\n{}",
        game.score,
        game.lines_cleared,
        if game.game_over { "IDLE" } else { "RUN" }
    ))
    .block(Block::default().title("INFO").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    frame.render_widget(info, chunks[0]);

    let controls = Paragraph::new(
        "←/→ move\n↑ rotate\n↓ soft\nspace slam\nq/esc quit",
    )
    .block(Block::default().title("CONTROLS").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    frame.render_widget(controls, chunks[2]);
}
