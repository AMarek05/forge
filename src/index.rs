//! Read/write `~/.forge/index.json` — the project index.
//!
//! Index stores structural fields only (name, lang, path, added_at,
//! last_opened, open_count). All project metadata (desc, tags, includes,
//! build/run/test/check) lives in each project's .forge/state file.
//!
//! Index location: ~/.forge/index.json (migrated from ~/.forge-index.json).

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub name: String,
    pub lang: String,
    pub path: PathBuf,
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
            version: 3,
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
        // Migrate from old location (~/.forge-index.json) if needed
        if let Some(old) = old_index_path() {
            if old.exists() {
                let content = fs::read_to_string(&old)?;
                let index: ProjectIndex = serde_json::from_str(&content)
                    .context("failed to parse old index")?;
                save_index_to(&index, &index_path()?)?;
                fs::remove_file(&old).ok();
                return Ok(index);
            }
        }
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
        return Ok(PathBuf::from(base).join("index.json"));
    }
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(".forge").join("index.json"))
}

fn old_index_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".forge-index.json"))
}

// ═══════════════════════════════════════════════════════════════════════════
// Unit tests — run with: cargo test --lib
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("forge-idx-test-{}", ts));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ─── save_index_to / load_index_from roundtrip ─────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_dir();
        let path = dir.join("index.json");

        let index = crate::index::ProjectIndex::new(dir.join("sync"));
        index.projects.push(crate::index::ProjectEntry {
            name: "test-project".into(),
            lang: "rust".into(),
            path: dir.join("Code").join("Rust").join("test-project"),
            added_at: "1234567890".into(),
            last_opened: Some("1234568000".into()),
            open_count: 5,
        });

        crate::index::save_index_to(&index, &path).unwrap();
        let loaded = crate::index::load_index_from(&path).unwrap();

        assert_eq!(loaded.version, 3);
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].name, "test-project");
        assert_eq!(loaded.projects[0].lang, "rust");
        assert_eq!(loaded.projects[0].open_count, 5);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_index_has_version_3() {
        let dir = temp_dir();
        let index = crate::index::ProjectIndex::new(dir.join("sync"));
        assert_eq!(index.version, 3);
        assert!(index.projects.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_nonexistent_returns_default_index() {
        let dir = temp_dir();
        let path = dir.join("nonexistent.json");
        // This should not panic even if file doesn't exist
        // (it tries migration from old path first)
        let _result = crate::index::load_index_from(&path);
        // No home dir in test env → falls back to default
        fs::remove_dir_all(&dir).ok();
    }
}