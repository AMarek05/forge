//! `forge sync` — re-scan FORGE_SYNC_BASE and rebuild the index.
//! Also diffs each project's includes against .forge/applied-includes
//! and runs any newly-added include setups.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::applied_includes::{diff_applied, load as load_applied, save as save_applied};
use crate::verify_and_diff::verify_and_diff;
use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry, ProjectIndex};
use crate::wl_parser::parse_wl;

pub fn run() -> Result<()> {
    let config = ForgeConfig::load()?;
    let sync_base = &config.sync_base;

    let mut index = index_mod::load_index()
        .unwrap_or_else(|_| ProjectIndex::new(sync_base.clone()));

    // Build map of existing entries by name (preserve last_opened, open_count)
    let existing: std::collections::HashMap<String, (Option<String>, u32)> = index.projects.iter()
        .map(|p| (p.name.clone(), (p.last_opened.clone(), p.open_count)))
        .collect();

    // Walk sync_base recursively for all .wl files
    let mut new_projects = vec![];

    walk_dir(sync_base, &mut new_projects)?;

    // Preserve last_opened and open_count for known projects
    let mut updated = vec![];
    for (name, lang, path, includes) in new_projects {
        let (last_opened, open_count) = existing.get(&name)
            .cloned()
            .unwrap_or((None, 0));

        // Diff includes against applied-includes and run missing setups
        let project_path = PathBuf::from(&path);
        if let Err(e) = verify_and_diff(&project_path, &config) {
            eprintln!("warning: {}: {} — skipping include sync", project_path.display(), e);
        }

        updated.push(ProjectEntry {
            name,
            lang,
            path: project_path,
            added_at: now_iso(),
            last_opened,
            open_count,
        });
    }

    index.projects = updated;
    index_mod::save_index(&index)?;

    println!("synced {} projects", index.projects.len());

    Ok(())
}

/// Compare the project's current includes against .forge/applied-includes,
/// run setup.sh for any newly added ones, then update applied-includes.
fn diff_and_run_includes(project_path: &PathBuf, current_includes: &[String], config: &ForgeConfig) -> Result<()> {
    let applied = load_applied(project_path)?;
    let new_includes = diff_applied(current_includes, &applied);

    for inc_name in &new_includes {
        run_include_setup(inc_name, project_path, config)?;
    }

    if !new_includes.is_empty() {
        // Merge applied + new and save
        let mut all_applied = applied;
        all_applied.extend(new_includes.clone());
        save_applied(project_path, &all_applied)?;
    }

    Ok(())
}

fn walk_dir(dir: &std::path::Path, results: &mut Vec<(String, String, String, Vec<String>)>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let wl_path = path.join(".wl");
            if wl_path.is_file() {
                if let Ok(wl) = parse_wl(&wl_path) {
                    let name = wl.name.unwrap_or_else(|| {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string()
                    });
                    let lang = wl.lang.unwrap_or_else(|| "txt".to_string());

                    results.push((
                        name,
                        lang,
                        path.to_string_lossy().to_string(),
                        wl.includes,
                    ));
                }
            } else {
                walk_dir(&path, results)?;
            }
        }
    }
    Ok(())
}

fn run_include_setup(inc_name: &str, project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
    if !setup_sh.exists() {
        eprintln!("warning: include '{}' not found, skipping", inc_name);
        return Ok(());
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

    println!("applied include: {} for {}", inc_name, project_name);

    Ok(())
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}