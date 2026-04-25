//! tmux session switch-or-create logic.

use std::process::Command;
use anyhow::Result;

/// Check if a tmux session exists.
pub fn session_exists(session_name: &str, tmux_bin: &str) -> bool {
    Command::new(tmux_bin)
        .args(["has-session", "-t", session_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Switch to an existing tmux session.
pub fn switch_session(session_name: &str, tmux_bin: &str) -> Result<()> {
    Command::new(tmux_bin)
        .args(["switch-client", "-t", session_name])
        .status()?;
    Ok(())
}

/// Create a new detached tmux session and switch to it.
pub fn create_and_switch_session(session_name: &str, path: &str, tmux_bin: &str) -> Result<()> {
    Command::new(tmux_bin)
        .args(["new-session", "-d", "-s", session_name, "-c", path])
        .status()?;
    switch_session(session_name, tmux_bin)
}

/// Switch-or-create: switch if exists, otherwise create then switch.
pub fn switch_or_create(session_name: &str, path: &str, tmux_bin: &str) -> Result<()> {
    if session_exists(session_name, tmux_bin) {
        switch_session(session_name, tmux_bin)
    } else {
        create_and_switch_session(session_name, path, tmux_bin)
    }
}
