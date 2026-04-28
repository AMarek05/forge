//! `forge edit` — edit project's .wl in $EDITOR, then diff includes and run any new setups.

use std::process::Command;

use anyhow::{Context, Result};

use crate::applied_includes::{diff_applied, load as load_applied, save as save_applied};
use crate::config::ForgeConfig;
use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(name: String) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    let wl_path = project.path.join(".wl");

    Command::new("sh")
        .args(["-c", &format!("{} {}", config.editor, wl_path.to_string_lossy())])
        .status()?;

    // After editor closes, diff includes and run any newly added setups
    let wl = parse_wl(&wl_path).ok();
    if let Some(ref w) = wl {
        let current = w.includes.clone();
        let applied = load_applied(&project.path)?;
        let new_includes = diff_applied(&current, &applied);

        for inc_name in &new_includes {
            run_include_setup(inc_name, &project.path, &config)?;
        }

        if !new_includes.is_empty() {
            let mut all_applied = applied;
            all_applied.extend(new_includes);
            save_applied(&project.path, &all_applied)?;
        }
    }

    Ok(())
}

fn run_include_setup(inc_name: &str, project_path: &std::path::PathBuf, config: &ForgeConfig) -> Result<()> {
    use std::process::Command;

    let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
    if !setup_sh.exists() {
        eprintln!("warning: include '{}' not found, skipping", inc_name);
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let status = Command::new("bash")
        .arg(&setup_sh)
        .env("FORGE_PROJECT_NAME", project_name)
        .env("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or(""))
        .env("FORGE_BASE", config.base.to_str().unwrap_or(""))
        .env("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or(""))
        .env("FORGE_GITHUB_USER", &config.github_user)
        .env("FORGE_EDITOR", &config.editor)
        .env("FORGE_DRY_RUN", "0")
        .current_dir(project_path)
        .status()
        .with_context(|| format!("include setup '{}' failed", inc_name))?;

    if !status.success() {
        anyhow::bail!("include '{}' setup.sh exited with non-zero status", inc_name);
    }

    println!("applied include: {} for {}", inc_name, project_name);

    Ok(())
}