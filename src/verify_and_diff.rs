//! Verify .wl syntax and diff applied includes + project state.
//!
//! Called after editor closes on create/edit/pick-ctrl-e/sync.
//! Also updates the index entry if any fields changed.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::applied_includes::{diff_applied, load as load_applied, save as save_applied};
use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry};
use crate::project_state::{ProjectState};
use crate::wl_parser::parse_wl;

/// Verify .wl is parseable, diff includes, diff project state vs index,
/// run new include setups, update index entry if needed.
/// Prints a short summary of what changed.
pub fn verify_and_diff(project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    let wl_path = project_path.join(".wl");

    // Get .wl mtime for state tracking
    let mtime = fs::metadata(&wl_path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        })
        .unwrap_or(0);

    // Parse .wl — abort if syntax is bad
    let wl = parse_wl(&wl_path)
        .with_context(|| format!("failed to parse {} — fix syntax errors first", wl_path.display()))?;

    let current_state = ProjectState::from_wl(&wl, mtime);

    // Load previous state (if any)
    let prev_state = ProjectState::load(project_path).ok().unwrap_or_default();

    // Load current index entry for this project (if any)
    let mut index = index_mod::load_index().unwrap_or_else(|_| {
        crate::index::ProjectIndex::new(config.sync_base.clone())
    });

    // Find index entry by path (path is stable identifier, not name)
    let entry_idx = index.projects.iter().position(|e| e.path == *project_path);

    // Diff includes vs applied-includes and run new setups
    let applied = load_applied(project_path)?;
    let new_includes = diff_applied(&wl.includes, &applied);

    for inc_name in &new_includes {
        run_include_setup(inc_name, project_path, config)?;
    }

    // Update applied-includes
    if !new_includes.is_empty() {
        let mut all_applied = applied;
        all_applied.extend(new_includes.clone());
        save_applied(project_path, &all_applied)?;
    }

    // Diff project state vs previous state
    let changed_fields = prev_state.diff(&current_state);

    // Check if name/lang changed (structural index fields)
    let name_changed = prev_state.name != current_state.name && !prev_state.name.is_empty();
    let lang_changed = prev_state.lang != current_state.lang && !prev_state.lang.is_empty();

    // Update index entry if needed
    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    if let Some(idx) = entry_idx {
        let entry = &mut index.projects[idx];

        if name_changed {
            println!("{}: name: \"{}\" → \"{}\"", project_name, prev_state.name, current_state.name);
            entry.name = current_state.name.clone();
        }
        if lang_changed {
            println!("{}: lang: \"{}\" → \"{}\"", project_name, prev_state.lang, current_state.lang);
            entry.lang = current_state.lang.clone();
        }
    } else {
        // New project — add to index
        println!("{}: added to index", project_name);
        let added_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        index.projects.push(ProjectEntry {
            name: current_state.name.clone(),
            lang: current_state.lang.clone(),
            path: project_path.clone(),
            added_at,
            last_opened: None,
            open_count: 0,
        });
    }

    // Print include changes
    if !new_includes.is_empty() {
        println!("{}: applied includes: {}", project_name, new_includes.join(", "));
    } else {
        println!("{}: no include changes", project_name);
    }

    // Print field changes summary
    if !changed_fields.is_empty() {
        println!("{}: changed: {}", project_name, changed_fields.join(", "));
    }

    // Save updated index
    index_mod::save_index(&index)?;

    // Save current state to .forge/state
    current_state.save(project_path)?;

    Ok(())
}

fn run_include_setup(inc_name: &str, project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    let setup_sh = config.include_dir.join(inc_name).join("setup.sh");
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