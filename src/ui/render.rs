use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::{Game, BOARD_H, BOARD_W, CELL_W, MIN_PANE_WIDTH, PLAY_H, PLAY_W};
use crate::game::Cell;

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
        .title("WAITRIS")
        .border_type(BorderType::Thick)
        .borders(Borders::ALL)
        .title_alignment(Alignment::Left);
    let cabinet_inner = cabinet.inner(area);
    frame.render_widget(cabinet, area);

    let well_w = PLAY_W as u16;
    let well_h = PLAY_H as u16;

    let col_rect = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(well_w),
            Constraint::Min(0),
        ])
        .split(cabinet_inner)[1];

    let info_h = 5u16;
    let controls_h = 5u16;
    let stack = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(info_h),
            Constraint::Length(well_h),
            Constraint::Length(controls_h),
            Constraint::Min(0),
        ])
        .split(col_rect);

    let mut info_rect = stack[1];
    let well_rect = stack[2];
    let mut controls_rect = stack[3];
    // Widen info/controls boxes slightly while keeping them centered in the cabinet.
    let expand = 8u16;
    let max_right = cabinet_inner.x + cabinet_inner.width;
    let new_x = info_rect.x.saturating_sub(expand);
    let mut new_w = info_rect.width.saturating_add(expand * 2);
    if new_x + new_w > max_right {
        new_w = max_right.saturating_sub(new_x);
    }
    info_rect.x = new_x;
    info_rect.width = new_w;
    controls_rect.x = new_x;
    controls_rect.width = new_w;

    draw_info(frame, game, info_rect);
    draw_playfield(frame, game, well_rect);
    draw_controls(frame, controls_rect);
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
            if let Cell::Filled(left_ch, right_ch) = game.board.get(x, y) {
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

fn draw_info(frame: &mut Frame, game: &Game, area: Rect) {
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

    let block = Block::default().title("INFO").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    let left = Paragraph::new(vec![
        Line::raw(format!("{:<7} {}", "SCORE:", game.score)),
        Line::raw(format!("{:<7} {}", "LINES:", game.lines_cleared)),
        Line::raw(format!("{:<7} {}", "STATUS:", status)),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(left, cols[0]);

    let right = Paragraph::new(vec![
        Line::raw(format!("{:<6} {}", "BOMBS:", game.bombs)),
        Line::raw(format!("{:<6} {}", "VARIETY:", game.variety_meter)),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(right, cols[1]);
}

fn draw_controls(frame: &mut Frame, area: Rect) {
    let block = Block::default().title("CONTROLS").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    let left = Paragraph::new(vec![
        Line::raw("←/→ move"),
        Line::raw("↓ soft"),
        Line::raw("q quit"),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(left, cols[0]);

    let right = Paragraph::new(vec![
        Line::raw("↑ rotate"),
        Line::raw("space slam"),
        Line::raw("Ctrl+Space swap"),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(right, cols[1]);
}
