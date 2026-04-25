//! `forge session` — switch to or create a tmux session.

use std::process::Command;

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::index::{self as index_mod};
use crate::tmux::switch_or_create;

pub fn run(name: Option<String>, setup: bool, open: bool) -> Result<()> {
    let config = ForgeConfig::load()?;

    let name = if let Some(ref n) = name {
        n.clone()
    } else {
        anyhow::bail!("session requires a project name");
    };

    let index = index_mod::load_index()?;
    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    let session_name = format!("forge-{}", name);

    if setup {
        run_project_setup(&project.path.to_string_lossy())?;
    }

    switch_or_create(&session_name, &project.path.to_string_lossy(), &config.tmux_binary)?;

    if open {
        open_project(&project.path, &config)?;
    }

    update_last_opened(&name)?;

    Ok(())
}

fn run_project_setup(path: &str) -> Result<()> {
    std::env::set_current_dir(path)?;
    Command::new("direnv")
        .arg("allow")
        .current_dir(path)
        .status()
        .ok();
    Ok(())
}

fn open_project(path: &std::path::Path, config: &ForgeConfig) -> Result<()> {
    std::env::set_current_dir(path)?;
    let editor = &config.editor;
    let cmd = if editor.contains("nvim") {
        format!("{} -c Oil", editor)
    } else {
        editor.clone()
    };
    Command::new("sh")
        .args(["-c", &cmd])
        .status()?;
    Ok(())
}

fn update_last_opened(name: &str) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::index::{self as index_mod};

    let mut index = index_mod::load_index()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for p in &mut index.projects {
        if p.name == name {
            p.last_opened = Some(format!("{}", now));
            p.open_count += 1;
            break;
        }
    }

    index_mod::save_index(&index)?;
    Ok(())
}
