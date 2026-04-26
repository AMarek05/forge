//! Read/write `~/.forge-index.json` — the cached project index.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub name: String,
    pub lang: String,
    pub path: PathBuf,
    pub desc: Option<String>,
    pub tags: Vec<String>,
    pub includes: Vec<String>,
    pub build: Option<String>,
    pub added_at: String,
    pub last_opened: Option<String>,
    pub open_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndex {
    pub version: u32,
    pub sync_base: PathBuf,
    pub projects: Vec<ProjectEntry>,
}

impl ProjectIndex {
    pub fn new(sync_base: PathBuf) -> Self {
        ProjectIndex {
            version: 1,
            sync_base,
            projects: vec![],
        }
    }
}

pub fn load_index() -> Result<ProjectIndex> {
    let index_path = index_path()?;
    load_index_from(&index_path)
}

pub fn load_index_from(path: &PathBuf) -> Result<ProjectIndex> {
    if !path.exists() {
        let home = dirs::home_dir().unwrap_or_default();
        return Ok(ProjectIndex::new(home.join("sync")));
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let index: ProjectIndex = serde_json::from_str(&content)
        .context("failed to parse index JSON")?;

    Ok(index)
}

pub fn save_index(index: &ProjectIndex) -> Result<()> {
    let index_path = index_path()?;
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create dir {}", parent.display()))?;
    }
    save_index_to(index, &index_path)
}

pub fn save_index_to(index: &ProjectIndex, path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(index)
        .context("failed to serialize index")?;

    fs::write(path, json)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

fn index_path() -> Result<PathBuf> {
    if let Ok(base) = std::env::var("FORGE_BASE") {
        return Ok(PathBuf::from(base).join(".forge-index.json"));
    }
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(".forge-index.json"))
}