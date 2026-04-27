#![allow(dead_code)]
//! Parse `.wl` project files and `lang.wl` language registry files.
//!
//! `.wl` format:
//!   key="value"       → String
//!   key=[array]       → Vec<String> (JSON array)
//!   # comment         → ignored
//!
//! `lang.wl` format is similar but has different required fields.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Language {
    pub name: String,
    pub desc: String,
    pub path: String,
    pub direnv: String,
    pub setup_priority: i32,
    pub build: Option<String>,
    pub run: Option<String>,
    pub test: Option<String>,
    pub check: Option<String>,
    pub overseer_template: Option<String>,
    pub setup: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WlFile {
    pub name: Option<String>,
    pub lang: Option<String>,
    pub desc: Option<String>,
    pub tags: Vec<String>,
    pub includes: Vec<String>,
    pub build: Option<String>,
    pub run: Option<String>,
    pub test: Option<String>,
    pub check: Option<String>,
    pub overseer_template: Option<String>,
    pub setup: Option<String>,
}

pub fn parse_lang_wl(path: &Path) -> Result<Language> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let fields = parse_fields(&content, path)?;

    Ok(Language {
        name: fields.get("name").cloned().unwrap_or_default(),
        desc: fields.get("desc").cloned().unwrap_or_default(),
        path: fields.get("path").cloned().unwrap_or_default(),
        direnv: fields.get("direnv").cloned().unwrap_or_default(),
        setup_priority: fields
            .get("setup_priority")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        build: fields.get("build").cloned(),
        run: fields.get("run").cloned(),
        test: fields.get("test").cloned(),
        check: fields.get("check").cloned(),
        overseer_template: fields.get("overseer_template").cloned(),
        setup: fields.get("setup").cloned(),
    })
}

pub fn parse_wl(path: &Path) -> Result<WlFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let fields = parse_fields(&content, path)?;

    Ok(WlFile {
        name: fields.get("name").cloned(),
        lang: fields.get("lang").cloned(),
        desc: fields.get("desc").cloned(),
        tags: fields
            .get("tags")
            .cloned()
            .map(|s| parse_json_array(&s))
            .unwrap_or_default(),
        includes: fields
            .get("includes")
            .cloned()
            .map(|s| parse_json_array(&s))
            .unwrap_or_default(),
        build: fields.get("build").cloned(),
        run: fields.get("run").cloned(),
        test: fields.get("test").cloned(),
        check: fields.get("check").cloned(),
        overseer_template: fields.get("overseer_template").cloned(),
        setup: fields.get("setup").cloned(),
    })
}

fn parse_fields(content: &str, path: &Path) -> Result<HashMap<String, String>> {
    let mut fields = HashMap::new();
    let key_re = Regex::new(r#"^([a-z_]+)\s*=\s*(.+)$"#).unwrap();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(caps) = key_re.captures(line) {
            let key = caps.get(1).unwrap().as_str().to_string();
            let raw_value = caps.get(2).unwrap().as_str();

            let value = if raw_value.starts_with('[') {
                // JSON array — keep as-is for later parsing
                raw_value.to_string()
            } else {
                // Quoted string — strip quotes
                strip_quotes(raw_value)
            };

            fields.insert(key, value);
        } else {
            anyhow::bail!("failed to parse line in {}: {}", path.display(), line);
        }
    }

    Ok(fields)
}

pub(crate) fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub(crate) fn parse_json_array(s: &str) -> Vec<String> {
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
        let part = part.trim();
        result.push(strip_quotes(part));
    }
    result
}