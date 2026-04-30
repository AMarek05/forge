//! `forge sync` — re-scan sync base and rebuild the index.
//! Also refresh langs.json and includes.json from filesystem scan.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::verify_and_diff::verify_and_diff;
use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry, ProjectIndex};
use crate::wl_parser::parse_wl;

/// Entry point — called from main.rs with parsed sync flags.
pub fn run(flags: &SyncFlags) -> Result<()> {
    let config = ForgeConfig::load()?;

    if flags.langs {
        sync_langs(&config)?;
    }
    if flags.includes {
        sync_includes(&config)?;
    }
    if !flags.langs && !flags.includes {
        // Default: sync projects only (original behavior)
        sync_projects(&config)?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct SyncFlags {
    pub langs: bool,
    pub includes: bool,
}

// ─── sync --langs ────────────────────────────────────────────────────────────

fn sync_langs(config: &ForgeConfig) -> Result<()> {
    let langs_dir = &config.lang_dir;
    let output_path = config.config_dir.join("langs.json");

    let mut entries = vec![];

    // Scan default/ and custom/ subdirs
    for subdir in ["default", "custom"] {
        let base = langs_dir.join(subdir);
        if !base.exists() {
            continue;
        }
        for entry in fs::read_dir(&base)? {
            let entry = entry?;
            let lang_path = entry.path();
            if !lang_path.is_dir() {
                continue;
            }
            let wl_path = lang_path.join("lang.wl");
            if !wl_path.exists() {
                continue;
            }
            match parse_wl(&wl_path) {
                Ok(wl) => {
                    let name = wl.name.unwrap_or_else(|| {
                        lang_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    });
                    entries.push(serde_json::json!({
                        "name": name,
                        "description": wl.desc.unwrap_or_default(),
                        "lang_wl": {
                            "name": wl.name,
                            "desc": wl.desc,
                            "path": wl.path,
                            "direnv": wl.direnv,
                            "build": wl.build.unwrap_or_default(),
                            "run": wl.run.unwrap_or_default(),
                            "test": wl.test.unwrap_or_default(),
                            "check": wl.check.unwrap_or_default(),
                        }
                    }));
                }
                Err(e) => {
                    eprintln!("warning: {}: {} — skipping", wl_path.display(), e);
                }
            }
        }
    }

    let json = serde_json::to_string_pretty(&entries)
        .context("failed to serialize langs.json")?;
    fs::write(&output_path, json)
        .context(format!("failed to write {}", output_path.display()))?;

    println!("langs.json: {} entries written", entries.len());
    Ok(())
}

// ─── sync --includes ────────────────────────────────────────────────────────

fn sync_includes(config: &ForgeConfig) -> Result<()> {
    let includes_dir = &config.include_dir;
    let output_path = config.config_dir.join("includes.json");

    let mut entries = vec![];

    for subdir in ["default", "custom"] {
        let base = includes_dir.join(subdir);
        if !base.exists() {
            continue;
        }
        for entry in fs::read_dir(&base)? {
            let entry = entry?;
            let inc_path = entry.path();
            if !inc_path.is_dir() {
                continue;
            }
            let include_wl_path = inc_path.join("include.wl");
            let setup_sh_path = inc_path.join("setup.sh");

            // Parse include.wl for description and provides
            let (description, provides) = if include_wl_path.exists() {
                let content = fs::read_to_string(&include_wl_path)?;
                let desc = extract_field(&content, "description");
                // provides is a JSON array in the file, parse it
                let provides_raw = extract_field(&content, "provides").unwrap_or_default();
                let provides: Vec<String> = parse_json_array(&provides_raw);
                (desc.unwrap_or_default(), provides)
            } else {
                (String::new(), Vec::new())
            };

            let setup_sh = if setup_sh_path.exists() {
                fs::read_to_string(&setup_sh_path)?
            } else {
                String::new()
            };

            let name = inc_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            entries.push(serde_json::json!({
                "name": name,
                "description": description,
                "provides": provides,
                "setup_sh": setup_sh,
            }));
        }
    }

    let json = serde_json::to_string_pretty(&entries)
        .context("failed to serialize includes.json")?;
    fs::write(&output_path, json)
        .context(format!("failed to write {}", output_path.display()))?;

    println!("includes.json: {} entries written", entries.len());
    Ok(())
}

// ─── sync (default — project index sync) ────────────────────────────────────

fn sync_projects(config: &ForgeConfig) -> Result<()> {
    let sync_base = &config.sync_base;

    let mut index = index_mod::load_index()
        .unwrap_or_else(|_| ProjectIndex::new(sync_base.clone()));

    // Detect stale entries (indexed but .wl no longer exists)
    let mut stale_count = 0;
    for entry in &index.projects {
        if !entry.path.join(".wl").exists() {
            println!("removed: {} (directory gone)", entry.name);
            stale_count += 1;
        }
    }

    // Build map of existing entries by name (preserve last_opened, open_count)
    let existing: std::collections::HashMap<String, (Option<String>, u32)> = index.projects.iter()
        .map(|p| (p.name.clone(), (p.last_opened.clone(), p.open_count)))
        .collect();

    // Walk sync_base recursively for all .wl files
    let mut new_projects = vec![];
    walk_dir(sync_base, &mut new_projects)?;

    // Preserve last_opened and open_count for known projects
    let mut updated = vec![];
    for (name, lang, path, _includes) in new_projects {
        let (last_opened, open_count) = existing.get(&name)
            .cloned()
            .unwrap_or((None, 0));

        // Diff includes against applied-includes and run missing setups
        let project_path = PathBuf::from(&path);
        if let Err(e) = verify_and_diff(&project_path, config.clone()) {
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

    println!("synced {} projects (removed {})", index.projects.len(), stale_count);

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn walk_dir(dir: &Path, results: &mut Vec<(String, String, String, Vec<String>)>) -> Result<()> {
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

fn extract_field(content: &str, field: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            if key == field {
                let val = line[eq + 1..].trim();
                // Strip quotes
                let val = if (val.starts_with('"') && val.ends_with('"'))
                    || (val.starts_with('\'') && val.ends_with('\''))
                {
                    &val[1..val.len() - 1]
                } else {
                    val
                };
                return Some(val.to_string());
            }
        }
    }
    None
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}