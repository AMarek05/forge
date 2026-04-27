//! `forge include` — list or show include modules.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::include::IncludeEntry;

pub fn run(list: bool, name: Option<String>) -> Result<()> {
    if list {
        return run_list();
    }

    if let Some(ref n) = name {
        return run_show(n);
    }

    // No args — show help
    run_list()
}

fn run_list() -> Result<()> {
    let config = ForgeConfig::load()?;
    let includes_dir = config.base.join("includes");

    if !includes_dir.exists() {
        println!("no includes found");
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&includes_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_string());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let wl_path = entry.path().join("include.wl");

        if let Ok(inc) = parse_include_wl(&wl_path) {
            println!("{}", name);
            println!("  {}", inc.desc);
            if !inc.provides.is_empty() {
                println!("  provides: {}", inc.provides.join(", "));
            }
            println!();
        }
    }

    Ok(())
}

fn run_show(name: &str) -> Result<()> {
    let config = ForgeConfig::load()?;
    let wl_path = config.base.join("includes").join(name).join("include.wl");

    if !wl_path.exists() {
        anyhow::bail!("include '{}' not found", name);
    }

    let inc = parse_include_wl(&wl_path)?;

    println!("include: {}", inc.name);
    println!("description: {}", inc.desc);
    println!("version: {}", inc.version);
    if !inc.provides.is_empty() {
        println!("provides: {}", inc.provides.join(", "));
    }

    let _setup_sh = wl_path.parent().unwrap().join("setup.sh");
    if _setup_sh.exists() {
        println!("\nsetup.sh exists");
        let content = fs::read_to_string(&_setup_sh)?;
        println!("---");
        println!("{}", content);
        println!("---");
    }

    Ok(())
}

fn parse_include_wl(path: &PathBuf) -> Result<IncludeEntry> {
    let content = fs::read_to_string(path)?;

    let mut fields = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim().to_string(), strip_quotes(value.trim()));
        }
    }

    let provides = fields.get("provides")
        .cloned()
        .map(|s| parse_json_array(&s))
        .unwrap_or_default();

    Ok(IncludeEntry {
        name: fields.get("name").cloned().unwrap_or_default(),
        desc: fields.get("desc").cloned().unwrap_or_default(),
        provides,
        version: fields.get("version").cloned().unwrap_or_default(),
        fields,
        dir: path.parent().unwrap_or(&PathBuf::new()).to_path_buf(),
    })
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_json_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return vec![];
    }
    let inner = &s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return vec![];
    }
    inner.split(',').map(|part| strip_quotes(part.trim())).collect()
}
