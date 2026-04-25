//! `forge sync` — re-scan FORGE_SYNC_BASE and rebuild the index.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

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
    for (name, lang, path, desc, tags, includes, build) in new_projects {
        let (last_opened, open_count) = existing.get(&name)
            .cloned()
            .unwrap_or((None, 0));

        updated.push(ProjectEntry {
            name,
            lang,
            path: PathBuf::from(path),
            desc,
            tags,
            includes,
            build,
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

fn walk_dir(dir: &std::path::Path, results: &mut Vec<(String, String, String, Option<String>, Vec<String>, Vec<String>, Option<String>)>) -> Result<()> {
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
                        wl.desc,
                        wl.tags,
                        wl.includes,
                        wl.build,
                    ));
                }
            } else {
                walk_dir(&path, results)?;
            }
        }
    }
    Ok(())
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}
