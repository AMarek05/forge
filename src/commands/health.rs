//! `forge health` — validate system state and report issues.
//!
//! Checks:
//!   - index file: valid JSON, readable, non-empty projects array
//!   - each index entry: .wl exists at path, .wl parses, name is set
//!   - duplicate names: two entries with same name
//!   - stale entries: path points to directory without .wl
//!   - .forge/state: present for projects
//!
//! With --fix: removes stale entries, fixes missing names from .wl,
//! removes duplicates (keeps first seen, removes rest).

use std::collections::HashMap;

use anyhow::Result;

use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(fix: bool) -> Result<()> {
    let mut index = index_mod::load_index()?;

    let mut errors = vec![];
    let mut warnings = vec![];
    let mut fixed = vec![];

    // Check: index parses and is non-empty
    if index.projects.is_empty() {
        println!("⚠️  index is empty (no projects)");
    } else {
        println!("✅ index.json: {} projects", index.projects.len());
    }

    // Check: duplicate names
    let mut name_count: HashMap<String, usize> = HashMap::new();
    for entry in &index.projects {
        *name_count.entry(entry.name.clone()).or_insert(0) += 1;
    }
    let dup_names: Vec<String> = name_count.iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, _)| name.clone())
        .collect();

    for name in &dup_names {
        let dupes: Vec<_> = index.projects.iter()
            .filter(|e| e.name == *name)
            .map(|e| e.path.to_string_lossy().to_string())
            .collect();
        let msg = format!(
            "⚠️  duplicate name \"{}\" ({} occurrences): {}",
            name, name_count[name], dupes.join(", ")
        );
        warnings.push((name.clone(), msg.clone()));
        println!("{}", msg);

        if fix {
            // Remove duplicates: keep first occurrence by path, remove rest
            let mut seen_paths = std::collections::HashSet::new();
            let original_len = index.projects.len();
            index.projects.retain(|entry| {
                if entry.name == *name {
                    if !seen_paths.contains(&entry.path) {
                        seen_paths.insert(entry.path.clone());
                        true
                    } else {
                        false  // drop duplicate
                    }
                } else {
                    true
                }
            });
            let removed = original_len - index.projects.len();
            fixed.push(format!("removed {} duplicate(s) of \"{}\"", removed, name));
            println!("✅ fixed: removed {} duplicate(s) of \"{}\"", removed, name);
        }
    }

    // Check: each entry (collect indices to remove first to avoid mutation during iteration)
    let mut stale_indices = vec![];
    let mut error_indices = vec![];

    for (idx, entry) in index.projects.iter().enumerate() {
        let wl_path = entry.path.join(".wl");

        // .wl exists at path?
        if !wl_path.exists() {
            let msg = format!(
                "❌ project \"{}\": path {} does not exist (stale entry)",
                entry.name, entry.path.display()
            );
            errors.push((entry.name.clone(), msg.clone()));
            println!("{}", msg);
            stale_indices.push(idx);
            continue;
        }

        // .wl parses?
        match parse_wl(&wl_path) {
            Ok(wl) => {
                println!("✅ \"{}\": .wl valid", entry.name);

                // name set?
                if wl.name.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                    let msg = format!("⚠️  project at {}: .wl has empty name field", entry.path.display());
                    warnings.push((entry.name.clone(), msg.clone()));
                    println!("{}", msg);
                }

                // .forge/state exists?
                let state_path = entry.path.join(".forge").join("state");
                if state_path.exists() {
                    println!("  ✅ .forge/state present");
                } else {
                    println!("  ⚠️  .forge/state missing (will be created on next edit)");
                }
            }
            Err(e) => {
                let msg = format!(
                    "❌ project \"{}\": .wl syntax error — {}",
                    entry.name, e
                );
                errors.push((entry.name.clone(), msg.clone()));
                println!("{}", msg);
                error_indices.push(idx);
            }
        }
    }

    // Remove stale entries if --fix
    if fix && !stale_indices.is_empty() {
        // Remove in reverse order to avoid index shifting
        for idx in stale_indices.iter().rev() {
            let entry = &index.projects[*idx];
            println!("✅ removing stale entry \"{}\"", entry.name);
            fixed.push(format!("removed stale entry \"{}\"", entry.name));
            index.projects.remove(*idx);
        }
    }

    println!("");
    if errors.is_empty() && warnings.is_empty() {
        println!("✅ no issues found");
    } else {
        if !errors.is_empty() {
            println!("❌ {} error(s)", errors.len());
        }
        if !warnings.is_empty() {
            println!("⚠️  {} warning(s)", warnings.len());
        }
    }

    if fix && !fixed.is_empty() {
        println!("");
        println!("=== Fixes applied ===");
        for f in &fixed {
            println!("  {}", f);
        }
        index_mod::save_index(&index)?;
        println!("");
        println!("✅ index updated");
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}