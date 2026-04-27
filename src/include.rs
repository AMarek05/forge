#![allow(dead_code)]
//! Read include registry entries and merge fields into project `.wl` data.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct IncludeEntry {
    pub name: String,
    pub desc: String,
    pub provides: Vec<String>,
    pub version: String,
    /// Fields this include contributes (merged into .wl on create)
    pub fields: HashMap<String, String>,
    /// Absolute path to the include directory
    pub dir: PathBuf,
}

pub fn list_includes(base: &PathBuf) -> Result<Vec<IncludeEntry>> {
    let includes_dir = base.join("includes");
    if !includes_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = vec![];
    for entry in fs::read_dir(&includes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let wl_path = path.join("include.wl");
        if wl_path.exists() {
            if let Ok(inc) = parse_include_wl(&wl_path, name, path.clone()) {
                entries.push(inc);
            }
        }
    }
    Ok(entries)
}

fn parse_include_wl(path: &PathBuf, name: String, dir: PathBuf) -> Result<IncludeEntry> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let mut fields = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            let value = strip_quotes(value);
            fields.insert(key.to_string(), value);
        }
    }

    let provides = fields.get("provides")
        .cloned()
        .map(|s| parse_json_array(&s))
        .unwrap_or_default();

    Ok(IncludeEntry {
        name: fields.get("name").cloned().unwrap_or(name),
        desc: fields.get("desc").cloned().unwrap_or_default(),
        provides,
        version: fields.get("version").cloned().unwrap_or_default(),
        fields,
        dir,
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
    let mut result = vec![];
    for part in inner.split(',') {
        result.push(strip_quotes(part.trim()));
    }
    result
}

/// Merge fields from includes into a mutable HashMap of project fields.
/// Array fields are concatenated + deduplicated; string fields keep project value if set.
pub fn merge_include_fields(project_fields: &mut HashMap<String, String>, includes: &[IncludeEntry]) {
    for inc in includes {
        for (key, value) in &inc.fields {
            if key == "provides" || key == "desc" || key == "version" || key == "name" {
                continue;
            }
            if key == "includes" || key == "tags" {
                // Array fields — concatenate
                let existing = project_fields.get(key).cloned().unwrap_or_default();
                let existing_arr = parse_json_array(&existing);
                let new_arr = parse_json_array(value);
                let mut merged = existing_arr;
                for item in new_arr {
                    if !merged.contains(&item) {
                        merged.push(item);
                    }
                }
                let combined = format!("[{}]", merged.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(","));
                project_fields.insert(key.clone(), combined);
            } else {
                // String fields — only inherit if not already set
                if !project_fields.contains_key(key) {
                    project_fields.insert(key.clone(), value.clone());
                }
            }
        }
    }
}
