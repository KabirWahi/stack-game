use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let pane_w = env::var("STACK_PANE_W")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(24);

    if !tmux_available() {
        eprintln!("tmux not found on PATH. Please install tmux to use the launcher.");
        return ExitCode::from(1);
    }

    let game_cmd = match game_binary_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("cannot find stack-game binary: {e}");
            return ExitCode::from(1);
        }
    };

    let result = if env::var("TMUX").is_ok() {
        run_inside_tmux(pane_w, &game_cmd)
    } else {
        run_new_tmux_session(pane_w, &game_cmd)
    };

    if let Err(err) = result {
        eprintln!("stack launcher error: {err}");
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn tmux_available() -> bool {
    Command::new("tmux").arg("-V").output().map(|o| o.status.success()).unwrap_or(false)
}

fn run_inside_tmux(pane_w: u16, game_cmd: &str) -> Result<(), String> {
    // Split current window: right pane runs stack-game, focus left.
    let pct = percent_for_width(pane_w);
    let status = Command::new("tmux")
        .args(&[
            "split-window",
            "-h",
            "-p",
            &pct,
            "env",
            "STACK_MANAGED=1",
            "STACK_KILL_SESSION=0",
            game_cmd,
        ])
        .status()
        .map_err(|e| format!("tmux split failed: {e}"))?;
    if !status.success() {
        return Err(format!("tmux split failed with status {}", status));
    }
    if let Ok(session) = current_session_name() {
        apply_session_options(&session);
    }
    let _ = Command::new("tmux").args(&["select-pane", "-L"]).status();
    Ok(())
}

fn run_new_tmux_session(pane_w: u16, game_cmd: &str) -> Result<(), String> {
    // Start new session, split, run stack-game on right, focus left, attach.
    let pct = percent_for_width(pane_w);
    let shell = env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let session = format!("waitris-{}", std::process::id());
    let status = Command::new("tmux")
        .args(&["-f", "/dev/null", "new-session", "-d", "-s", &session, &shell])
        .status()
        .map_err(|e| format!("tmux new-session failed: {e}"))?;
    if !status.success() {
        return Err(format!("tmux new-session failed with status {}", status));
    }
    apply_session_options(&session);
    let status = Command::new("tmux")
        .args(&[
            "split-window",
            "-h",
            "-p",
            &pct,
            "env",
            "STACK_MANAGED=1",
            "STACK_KILL_SESSION=1",
            game_cmd,
        ])
        .status()
        .map_err(|e| format!("tmux split failed: {e}"))?;
    if !status.success() {
        return Err(format!("tmux split failed with status {}", status));
    }
    let _ = Command::new("tmux").args(&["select-pane", "-L"]).status();
    let _ = Command::new("tmux")
        .args(&["attach-session", "-t", &session])
        .status();
    Ok(())
}

fn apply_session_options(session: &str) {
    let _ = Command::new("tmux")
        .args(&["set", "-t", session, "status", "off"])
        .status();
    let _ = Command::new("tmux")
        .args(&["set", "-t", session, "pane-border-status", "off"])
        .status();
    let _ = Command::new("tmux")
        .args(&["set", "-t", session, "display-panes-time", "1"])
        .status();
    let _ = Command::new("tmux")
        .args(&["set-hook", "-t", session, "pane-exited", "kill-session"])
        .status();
    let _ = Command::new("tmux")
        .args(&[
            "bind-key",
            "-n",
            "C-Space",
            "select-pane",
            "-t",
            ":.+",
        ])
        .status();
}

fn current_session_name() -> Result<String, String> {
    let out = Command::new("tmux")
        .args(&["display-message", "-p", "#S"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("tmux display-message failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn percent_for_width(target_width: u16) -> String {
    // tmux split-window -p expects percentage for the new pane.
    // Rough heuristic: if terminal is 120 cols, 36 cols is 30%.
    let pct = ((target_width as f32 / 120.0) * 100.0).clamp(10.0, 90.0);
    format!("{:.0}", pct)
}

fn game_binary_path() -> Result<String, String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut path = exe
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "unable to resolve current exe dir".to_string())?;
    path.push("stack-game");
    if path.exists() {
        return path
            .to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "non-utf8 path to stack-game".to_string());
    }
    // Fallback to relying on PATH.
    Ok("stack-game".to_string())
}
