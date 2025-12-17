use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::{Game, BOARD_H, BOARD_W, CELL_W, MIN_PANE_WIDTH, PLAY_H, PLAY_W};

pub fn draw_game(frame: &mut Frame, game: &Game) {
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
    let plot_block = |grid: &mut [Vec<char>], bx: usize, by: usize, left: char, right: char| {
        let gx = 1 + bx * CELL_W;
        let gy = 1 + by;
        if gy < PLAY_H && gx + 1 < PLAY_W {
            grid[gy][gx] = left;
            grid[gy][gx + 1] = right;
        }
    };

    // Locked cells (with optional lock flash override).
    for y in 0..game.board.height {
        for x in 0..game.board.width {
            if let crate::Cell::Filled(left_ch, right_ch) = game.board.get(x, y) {
                let flashing = game.lock_flash_frames > 0
                    && game.lock_flash_cells.contains(&(x, y));
                let left = if flashing { '▓' } else { left_ch };
                let right = if flashing { '▓' } else { right_ch };
                plot_block(&mut grid, x, y, left, right);
            }
        }
    }

    if game.active_piece {
        if game.current_is_bomb {
            // Bomb drop banner along the top inside the well.
            let banner = " BOMB INBOUND ";
            let start = ((PLAY_W as i32 - banner.len() as i32) / 2).max(1) as usize;
            let gy = 0;
            for (i, ch) in banner.chars().enumerate() {
                if start + i < PLAY_W - 1 {
                    grid[gy][start + i] = ch;
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
        for (x, y, (left, right)) in game.current.cells_with_pairs() {
            if x >= 0 && y >= 0 {
                let (xu, yu) = (x as usize, y as usize);
                if xu < game.board.width && yu < game.board.height {
                    plot_block(&mut grid, xu, yu, left, right);
                }
            }
        }
    }

    // Line clear flash overlay overrides everything in the row.
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
        let overlay = Paragraph::new("GAME OVER\nPress q")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(overlay, popup);
    }
}

fn draw_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(16), Constraint::Min(5), Constraint::Length(9)].as_ref())
        .split(area);

    let running = game.is_running();
    let status = if game.game_over {
        "OVER"
    } else if running {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        if (millis / 300) % 2 == 0 {
            "ACTIVE"
        } else {
            "      "
        }
    } else {
        "IDLE"
    };

    let info = Paragraph::new(format!(
        "SCORE\n{}\n\nLINES\n{}\n\nSTATUS\n{}\n\nVARIETY\n{}\n\nBOMBS\n{}",
        game.score,
        game.lines_cleared,
        status,
        game.variety_meter,
        game.bombs
    ))
    .block(Block::default().title("INFO").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    frame.render_widget(info, chunks[0]);

    let controls = Paragraph::new(
        "←/→ move\n↑ rotate\n↓ soft\nspace slam\nq quit",
    )
    .block(Block::default().title("CONTROLS").borders(Borders::ALL))
    .wrap(Wrap { trim: true });
    frame.render_widget(controls, chunks[2]);
}
