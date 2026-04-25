//! `forge open` — cd into project directory and open $EDITOR.

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

    std::env::set_current_dir(&project.path)?;
    Command::new("sh")
        .args(["-c", &config.editor])
        .status()?;

    Ok(())
}
