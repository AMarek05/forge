//! `forge session` — switch to or create a tmux session.

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::index::{self as index_mod};
use crate::tmux::switch_or_create;

pub fn run(name: Option<String>, setup: bool) -> Result<()> {
    let config = ForgeConfig::load()?;

    let name = if let Some(ref n) = name {
        n.clone()
    } else {
        // Interactive pick — but session just takes a name
        anyhow::bail!("session requires a project name");
    };

    let index = index_mod::load_index()?;
    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    let session_name = format!("forge-{}", name);

    if setup {
        // Run setup scripts first
        run_project_setup(&project.path.to_string_lossy(), &config)?;
    }

    switch_or_create(&session_name, &project.path.to_string_lossy(), &config.tmux_binary)?;

    // Update last_opened
    update_last_opened(&name)?;

    Ok(())
}

fn run_project_setup(path: &str, _config: &ForgeConfig) -> Result<()> {
    std::process::Command::new("direnv")
        .arg("allow")
        .current_dir(path)
        .status()
        .ok();
    Ok(())
}

fn update_last_opened(name: &str) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::index::{self as index_mod, ProjectIndex};

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
