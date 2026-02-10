use std::fs::{File, OpenOptions};
use std::io;

use super::ast::{Redirect, RedirectKind};

/// Saved file descriptors for restoring after a command.
pub struct SavedFds {
    saves: Vec<SavedFd>,
}

struct SavedFd {
    original_fd: i32,
    saved_file: Option<File>,
    was_stdout: bool,
    was_stderr: bool,
}

/// State of stdout/stderr capture.
pub struct RedirectState {
    pub original_stdout: Option<File>,
    pub original_stderr: Option<File>,
    pub stdout_file: Option<String>,
    pub stderr_file: Option<String>,
    pub stdin_content: Option<String>,
}

impl RedirectState {
    pub fn new() -> Self {
        RedirectState {
            original_stdout: None,
            original_stderr: None,
            stdout_file: None,
            stderr_file: None,
            stdin_content: None,
        }
    }
}

/// Apply redirections for a command. Returns paths to temp files for pipe-like behavior.
/// In WASM, we can't actually redirect file descriptors, so we use temp files.
pub fn apply_redirections(
    redirections: &[Redirect],
    expanded_targets: &[String],
) -> io::Result<RedirectState> {
    let mut state = RedirectState::new();

    for (i, redir) in redirections.iter().enumerate() {
        let target = expanded_targets.get(i).map(|s| s.as_str()).unwrap_or("");
        let fd = redir.fd.unwrap_or(match &redir.kind {
            RedirectKind::Input | RedirectKind::HereDoc(_) | RedirectKind::HereString | RedirectKind::DupInput => 0,
            _ => 1,
        });

        match &redir.kind {
            RedirectKind::Output => {
                if fd == 1 {
                    state.stdout_file = Some(target.to_string());
                } else if fd == 2 {
                    state.stderr_file = Some(target.to_string());
                }
                // Create/truncate the file
                File::create(target)?;
            }
            RedirectKind::Append => {
                if fd == 1 {
                    state.stdout_file = Some(target.to_string());
                } else if fd == 2 {
                    state.stderr_file = Some(target.to_string());
                }
                // Touch the file (append mode will be handled by the writer)
                OpenOptions::new().create(true).append(true).open(target)?;
            }
            RedirectKind::Input => {
                let content = std::fs::read_to_string(target)?;
                state.stdin_content = Some(content);
            }
            RedirectKind::HereDoc(content) => {
                state.stdin_content = Some(content.clone());
            }
            RedirectKind::HereString => {
                state.stdin_content = Some(format!("{}\n", target));
            }
            RedirectKind::DupOutput => {
                // 2>&1 — redirect stderr to stdout's target
                if fd == 2 && target == "1" {
                    state.stderr_file = state.stdout_file.clone();
                } else if fd == 1 && target == "2" {
                    state.stdout_file = state.stderr_file.clone();
                }
            }
            RedirectKind::DupInput => {
                // Not commonly used, ignore
            }
        }
    }

    Ok(state)
}
