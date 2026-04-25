//! `forge setup` — run setup scripts for a project.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::ForgeConfig;
use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(name: String, dry_run: bool) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    let wl_path = project.path.join(".wl");
    let wl = parse_wl(&wl_path)
        .with_context(|| format!("failed to read {}", wl_path.display()))?;

    let lang_name = wl.lang
        .as_ref()
        .with_context(|| "project has no lang set")?;

    // Run language setup if exists
    run_lang_setup(lang_name, &project.path, &config, dry_run)?;

    // Run each include setup
    for inc_name in &wl.includes {
        run_include_setup(inc_name, &project.path, &config, dry_run)?;
    }

    if !dry_run {
        println!("setup complete for {}", name);
    }

    Ok(())
}

fn run_lang_setup(lang_name: &str, project_path: &PathBuf, config: &ForgeConfig, dry_run: bool) -> Result<()> {
    let setup_sh = config.base.join("languages").join(lang_name).join("setup.sh");
    if !setup_sh.exists() {
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if dry_run {
        println!("[dry-run] bash {}", setup_sh.display());
        return Ok(());
    }

    let status = Command::new("bash")
        .arg(&setup_sh)
        .env("FORGE_PROJECT_NAME", project_name)
        .env("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or(""))
        .env("FORGE_LANG", lang_name)
        .env("FORGE_LANG_TEMPLATE_DIR", config.base.join("languages").join(lang_name).to_str().unwrap_or(""))
        .env("FORGE_BASE", config.base.to_str().unwrap_or(""))
        .env("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or(""))
        .env("FORGE_GITHUB_USER", &config.github_user)
        .env("FORGE_EDITOR", &config.editor)
        .env("FORGE_DRY_RUN", "0")
        .current_dir(project_path)
        .status()
        .with_context(|| format!("language setup '{}' failed", lang_name))?;

    if !status.success() {
        anyhow::bail!("language setup '{}' exited with non-zero", lang_name);
    }

    Ok(())
}

fn run_include_setup(inc_name: &str, project_path: &PathBuf, config: &ForgeConfig, dry_run: bool) -> Result<()> {
    let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
    if !setup_sh.exists() {
        eprintln!("warning: include '{}' not found, skipping", inc_name);
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if dry_run {
        println!("[dry-run] bash {}", setup_sh.display());
        return Ok(());
    }

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
        anyhow::bail!("include setup '{}' exited with non-zero", inc_name);
    }

    Ok(())
}
