use std::error::Error;
use std::io::{stdout, Stdout};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::io::spawn_socket_listener;
use crate::ui::draw_game;
use crate::Game;

type Term = Terminal<CrosstermBackend<Stdout>>;

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut tui = TuiGuard::new()?;
    let result = run_loop(tui.terminal_mut());
    cleanup_tmux_on_exit();
    result
}

fn run_loop(terminal: &mut Term) -> Result<(), Box<dyn Error>> {
    let mut game = Game::new();
    let (tx, rx) = mpsc::channel();
    spawn_socket_listener(tx);
    let mut last_tick = Instant::now();

    loop {
        for ev in rx.try_iter() {
            game.handle_command_event(ev);
        }

        terminal.draw(|frame| draw_game(frame, &game))?;

        game.process_effects();

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if matches!(key.code, KeyCode::Char('q')) {
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
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
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

fn cleanup_tmux_on_exit() {
    let managed = std::env::var("STACK_MANAGED").unwrap_or_default() == "1";
    let kill_session = std::env::var("STACK_KILL_SESSION").unwrap_or_default() == "1";
    if !managed || std::env::var("TMUX").is_err() {
        return;
    }

    if kill_session {
        if let Ok(session) = tmux_current_session() {
            let _ = Command::new("tmux")
                .args(&["kill-session", "-t", &session])
                .status();
        }
    } else {
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
