//! `forge open` — open project directory in $EDITOR.

use std::process::Command;

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::index::{self as index_mod};

pub fn run(name: String) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    Command::new("sh")
        .args(["-c", &format!("{} {}", config.editor, project.path.to_string_lossy())])
        .status()?;

    Ok(())
}
