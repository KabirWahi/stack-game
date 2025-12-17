use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("quit") => quit_session(),
        _ => {
            eprintln!("usage: waitris quit");
            ExitCode::from(2)
        }
    }
}

fn quit_session() -> ExitCode {
    if env::var("TMUX").is_err() {
        eprintln!("waitris quit must be run inside tmux");
        return ExitCode::from(1);
    }
    let session = match current_session_name() {
        Ok(s) if !s.is_empty() => s,
        _ => return ExitCode::from(1),
    };
    let status = Command::new("tmux")
        .args(&["kill-session", "-t", &session])
        .status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(_) => ExitCode::from(1),
    }
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
