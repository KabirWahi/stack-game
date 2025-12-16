use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::thread;

use crate::{CommandEvent, SOCKET_PATH};

pub fn spawn_socket_listener(tx: mpsc::Sender<CommandEvent>) {
    let _ = fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH).ok();
    thread::spawn(move || {
        if let Some(listener) = listener {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    handle_stream(stream, &tx);
                }
            }
        }
    });
}

fn handle_stream(stream: UnixStream, tx: &mpsc::Sender<CommandEvent>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Some(ev) = parse_command_line(&line) {
                let _ = tx.send(ev);
            }
        }
    }
}

fn parse_command_line(line: &str) -> Option<CommandEvent> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("START ") {
        let mut parts = rest.splitn(2, ' ');
        let id_str = parts.next()?;
        let cmd = parts.next().unwrap_or("").trim();
        let id = id_str.parse().ok()?;
        return Some(CommandEvent::Start {
            id,
            command: cmd.to_string(),
        });
    }
    if let Some(rest) = line.strip_prefix("END ") {
        let mut parts = rest.split_whitespace();
        let id_str = parts.next()?;
        let code_str = parts.next().unwrap_or("0");
        let id = id_str.parse().ok()?;
        let exit_code = code_str.parse().unwrap_or(0);
        return Some(CommandEvent::End { id, _exit_code: exit_code });
    }
    None
}
