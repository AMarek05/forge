//! Verify .wl syntax and diff applied includes.
//!
//! Called after editor closes on create/edit/pick-ctrl-e/sync.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::applied_includes::{diff_applied, load as load_applied, save as save_applied};
use crate::config::ForgeConfig;
use crate::wl_parser::parse_wl;

/// Verify .wl is parseable, diff includes vs applied-includes, run new setups.
/// Prints a short summary of what changed.
/// Returns the parsed WlFile on success.
pub fn verify_and_diff(project_path: &PathBuf, config: &ForgeConfig) -> Result<crate::wl_parser::WlFile> {
    let wl_path = project_path.join(".wl");
    let wl = parse_wl(&wl_path)
        .with_context(|| format!("failed to parse {} — fix syntax errors first", wl_path.display()))?;

    let current = &wl.includes;
    let applied = load_applied(project_path)?;
    let new_includes = diff_applied(current, &applied);

    if new_includes.is_empty() {
        println!("{}: no include changes", project_path.file_name().and_then(|n| n.to_str()).unwrap_or("project"));
        return Ok(wl);
    }

    // Run new include setups
    for inc_name in &new_includes {
        run_include_setup(inc_name, project_path, config)?;
    }

    // Update applied-includes
    let mut all_applied = applied;
    all_applied.extend(new_includes.clone());
    save_applied(project_path, &all_applied)?;

    // Print what changed
    let added: Vec<_> = new_includes.iter().map(|s| s.as_str()).collect();
    println!("{}: applied includes: {}", project_path.file_name().and_then(|n| n.to_str()).unwrap_or("project"), added.join(", "));

    Ok(wl)
}

fn run_include_setup(inc_name: &str, project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
    if !setup_sh.exists() {
        anyhow::bail!("include '{}' not found in includes/ — check your .wl", inc_name);
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let status = std::process::Command::new("bash")
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

    Ok(())
}