//! `forge sync` — re-scan sync base and rebuild the index.
//! Also refresh langs.json and includes.json from filesystem scan.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use walkdir::WalkDir;

use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry, ProjectIndex};
use crate::verify_and_diff::verify_and_diff;
use crate::wl_parser::parse_wl;

/// Entry point — called from main.rs with parsed sync flags.
pub fn run(flags: &SyncFlags) -> Result<()> {
    let config = ForgeConfig::load()?;

    // These can technically run in parallel if you want to spawn threads, 
    // but running them sequentially is usually fine unless all flags are true.
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
    let output_path = config.config_dir().join("langs.json");
    let mut entries = vec![];

    let bases = [config.lang_default_dir(), config.lang_custom_dir()];

    // Flatten all language paths to process them in parallel
    let lang_paths: Vec<PathBuf> = bases
        .iter()
        .filter(|b| b.exists())
        .flat_map(|base| fs::read_dir(base).ok())
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    // Process parsed languages concurrently
    entries = lang_paths
        .par_iter()
        .filter_map(|lang_path| {
            let wl_path = lang_path.join("lang.wl");
            if !wl_path.exists() {
                return None;
            }

            match crate::wl_parser::parse_lang_wl(&wl_path) {
                Ok(lang_meta) => {
                    let name = if !lang_meta.name.is_empty() {
                        lang_meta.name.clone()
                    } else {
                        lang_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    };

                    Some(serde_json::json!({
                        "name": name,
                        "description": lang_meta.desc,
                        "flake": lang_path.join("flake.nix").to_string_lossy().to_string(),
                        "lang_wl": {
                            "name": lang_meta.name,
                            "desc": lang_meta.desc,
                            "path": lang_meta.path,
                            "direnv": lang_meta.direnv,
                            "build": lang_meta.build.unwrap_or_default(),
                            "run": lang_meta.run.unwrap_or_default(),
                            "test": lang_meta.test.unwrap_or_default(),
                            "check": lang_meta.check.unwrap_or_default(),
                        }
                    }))
                }
                Err(e) => {
                    eprintln!("warning: {}: {} — skipping", wl_path.display(), e);
                    None
                }
            }
        })
        .collect();

    write_json_stream(&output_path, &entries)?;
    println!("langs.json: {} entries written", entries.len());
    Ok(())
}

// ─── sync --includes ────────────────────────────────────────────────────────

fn sync_includes(config: &ForgeConfig) -> Result<()> {
    let output_path = config.config_dir().join("includes.json");
    let mut entries = vec![];

    // OPTIMIZATION: Compile Regex ONCE outside the loop
    let key_re = Regex::new(r#"^([a-z_]+)\s*=\s*(.+)$"#).unwrap();

    for base in [config.include_default_dir(), config.include_custom_dir()] {
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

            let (description, provides) = if include_wl_path.exists() {
                let content = fs::read_to_string(&include_wl_path)?;
                let mut desc = String::new();
                let mut provides_raw = String::new();
                
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(caps) = key_re.captures(line) {
                        let key = caps.get(1).unwrap().as_str();
                        let raw_val = caps.get(2).unwrap().as_str();
                        match key {
                            "description" => desc = crate::wl_parser::strip_quotes(raw_val),
                            "provides" => provides_raw = raw_val.to_string(),
                            _ => {}
                        }
                    }
                }
                let provides: Vec<String> = crate::wl_parser::parse_json_array(&provides_raw);
                (desc, provides)
            } else {
                (String::new(), Vec::new())
            };

            let setup_sh = if setup_sh_path.exists() {
                fs::read_to_string(&setup_sh_path)?
            } else {
                String::new()
            };

            let name = inc_path
                .file_name()
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

    write_json_stream(&output_path, &entries)?;
    println!("includes.json: {} entries written", entries.len());
    Ok(())
}

// ─── sync (default — project index sync) ────────────────────────────────────

fn sync_projects(config: &ForgeConfig) -> Result<()> {
    let sync_base = &config.sync_base;

    let mut index = index_mod::load_index()
        .unwrap_or_else(|_| ProjectIndex::new(sync_base.clone()));

    // Map existing entries and count stale ones in a single pass
    let mut stale_count = 0;
    let existing: std::collections::HashMap<String, (Option<String>, u32)> = index
        .projects
        .into_iter()
        .filter_map(|p| {
            if !p.path.join(".wl").exists() {
                println!("removed: {} (directory gone)", p.name);
                stale_count += 1;
                None
            } else {
                Some((p.name.clone(), (p.last_opened, p.open_count)))
            }
        })
        .collect();

    // OPTIMIZATION: Use Walkdir + Rayon parallel bridge to find and parse .wl files concurrently
    let new_projects: Vec<_> = WalkDir::new(sync_base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name() == ".wl")
        .par_bridge()
        .filter_map(|entry| {
            let wl_path = entry.path();
            let path = wl_path.parent().unwrap(); // Safe because we matched a file named ".wl"
            
            parse_wl(wl_path).ok().map(|wl| {
                let name = wl.name.unwrap_or_else(|| {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                });
                let lang = wl.lang.unwrap_or_else(|| "txt".to_string());

                (name, lang, path.to_path_buf(), wl.includes)
            })
        })
        .collect();

    // Map new projects back into our index, preserving state and diffing includes
    let updated: Vec<ProjectEntry> = new_projects
        .into_iter()
        .map(|(name, lang, project_path, _includes)| {
            let (last_opened, open_count) = existing
                .get(&name)
                .cloned()
                .unwrap_or((None, 0));

            // Diff includes against applied-includes and run missing setups
            if let Err(e) = verify_and_diff(&project_path, config) {
                eprintln!("warning: {}: {} — skipping include sync", project_path.display(), e);
            }

            ProjectEntry {
                name,
                lang,
                path: project_path,
                added_at: now_iso(),
                last_opened,
                open_count,
            }
        })
        .collect();

    let new_count = updated.len();
    index.projects = updated;
    index_mod::save_index(&index)?;

    println!("synced {} projects (removed {})", new_count, stale_count);

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Streams JSON directly to a BufWriter instead of holding giant Strings in memory
fn write_json_stream<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = File::create(path).context(format!("failed to create {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data)
        .context(format!("failed to serialize JSON to {}", path.display()))?;
    Ok(())
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now.to_string()
}
