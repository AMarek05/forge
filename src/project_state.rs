//! Per-project state stored in `project/.forge/state`.
//!
//! Tracks all .wl fields so we can diff on editor close and update the index
//! without re-parsing everything. State file is written after every verified
//! edit/create so it stays in sync.
//!
//! Format of .forge/state (shell-var style, same as .wl):
//!   name="myproject"
//!   lang="rust"
//!   desc="A cool project"
//!   tags=["cli","rust"]
//!   includes=["git","overseer"]
//!   build="cargo build"
//!   run="cargo run"
//!   test="cargo test"
//!   check="cargo clippy"
//!   last_wl_mtime="1234567890"

use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use crate::wl_parser::{parse_json_array, strip_quotes, WlFile};

/// Fields we track per project (mirrors .wl surface fields).
#[derive(Debug, Clone, Default)]
pub struct ProjectState {
    pub name: String,
    pub lang: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub includes: Vec<String>,
    pub build: String,
    pub run: String,
    pub test: String,
    pub check: String,
    pub last_wl_mtime: u64,
}

impl ProjectState {
    /// Load existing state from project's .forge/state file.
    pub fn load(project_path: &PathBuf) -> Result<Self> {
        let path = project_path.join(".forge").join("state");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        parse_state_file(&content)
    }

    /// Save state to project's .forge/state file.
    pub fn save(&self, project_path: &PathBuf) -> Result<()> {
        let dir = project_path.join(".forge");
        fs::create_dir_all(&dir)?;
        let path = dir.join("state");
        fs::write(&path, self.to_string())?;
        Ok(())
    }

    /// Snapshot from a parsed .wl file and the .wl file's mtime.
    pub fn from_wl(wl: &WlFile, mtime: u64) -> Self {
        ProjectState {
            name: wl.name.clone().unwrap_or_default(),
            lang: wl.lang.clone().unwrap_or_default(),
            desc: wl.desc.clone().unwrap_or_default(),
            tags: wl.tags.clone(),
            includes: wl.includes.clone(),
            build: wl.build.clone().unwrap_or_default(),
            run: wl.run.clone().unwrap_or_default(),
            test: wl.test.clone().unwrap_or_default(),
            check: wl.check.clone().unwrap_or_default(),
            last_wl_mtime: mtime,
        }
    }

    /// Diff two states and return field names that changed.
    pub fn diff(&self, other: &ProjectState) -> Vec<&'static str> {
        let mut changed = vec![];
        if self.name != other.name { changed.push("name"); }
        if self.lang != other.lang { changed.push("lang"); }
        if self.desc != other.desc { changed.push("desc"); }
        if self.tags != other.tags { changed.push("tags"); }
        if self.includes != other.includes { changed.push("includes"); }
        if self.build != other.build { changed.push("build"); }
        if self.run != other.run { changed.push("run"); }
        if self.test != other.test { changed.push("test"); }
        if self.check != other.check { changed.push("check"); }
        changed
    }

    fn to_string(&self) -> String {
        let mut lines = vec![
            format!("name=\"{}\"", self.name),
            format!("lang=\"{}\"", self.lang),
            format!("desc=\"{}\"", self.desc),
        ];
        if !self.tags.is_empty() {
            let tags_str = self.tags.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
            lines.push(format!("tags=[{}]", tags_str));
        } else {
            lines.push(String::from("tags=[]"));
        }
        if !self.includes.is_empty() {
            let inc_str = self.includes.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
            lines.push(format!("includes=[{}]", inc_str));
        } else {
            lines.push(String::from("includes=[]"));
        }
        lines.push(format!("build=\"{}\"", self.build));
        lines.push(format!("run=\"{}\"", self.run));
        lines.push(format!("test=\"{}\"", self.test));
        lines.push(format!("check=\"{}\"", self.check));
        lines.push(format!("last_wl_mtime=\"{}\"", self.last_wl_mtime));
        lines.join("\n")
    }
}

fn parse_state_file(content: &str) -> Result<ProjectState> {
    let mut state = ProjectState::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => state.name = strip_quotes(value),
                "lang" => state.lang = strip_quotes(value),
                "desc" => state.desc = strip_quotes(value),
                "tags" => state.tags = parse_json_array(value),
                "includes" => state.includes = parse_json_array(value),
                "build" => state.build = strip_quotes(value),
                "run" => state.run = strip_quotes(value),
                "test" => state.test = strip_quotes(value),
                "check" => state.check = strip_quotes(value),
                "last_wl_mtime" => state.last_wl_mtime = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    Ok(state)
}

// ═══════════════════════════════════════════════════════════════════════════
// Unit tests — run with: cargo test --lib
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    fn temp_project() -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("forge-ps-test-{}", ts));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_wl() -> crate::wl_parser::WlFile {
        crate::wl_parser::WlFile {
            name: Some("myproject".into()),
            lang: Some("rust".into()),
            desc: Some("A test project".into()),
            tags: vec!["cli".into(), "wasm".into()],
            includes: vec!["git".into()],
            build: Some("cargo build".into()),
            run: Some("cargo run".into()),
            test: Some("cargo test".into()),
            check: Some("cargo clippy".into()),
            overseer_template: None,
            setup: None,
        }
    }

    // ─── from_wl ──────────────────────────────────────────────────────────

    #[test]
    fn from_wl_extracts_all_fields() {
        let wl = sample_wl();
        let state = crate::project_state::ProjectState::from_wl(&wl, 1234567890);
        assert_eq!(state.name, "myproject");
        assert_eq!(state.lang, "rust");
        assert_eq!(state.desc, "A test project");
        assert_eq!(state.tags, vec!["cli".into(), "wasm".into()]);
        assert_eq!(state.includes, vec!["git".into()]);
        assert_eq!(state.build, "cargo build");
        assert_eq!(state.last_wl_mtime, 1234567890);
    }

    // ─── diff ─────────────────────────────────────────────────────────────

    #[test]
    fn diff_detects_changed_fields() {
        let old = crate::project_state::ProjectState {
            name: "myproject".into(),
            lang: "rust".into(),
            desc: "old desc".into(),
            tags: vec![],
            includes: vec![],
            build: "cargo build".into(),
            run: "".into(),
            test: "".into(),
            check: "".into(),
            last_wl_mtime: 0,
        };
        let new = crate::project_state::ProjectState {
            name: "renamed".into(),
            lang: "rust".into(),
            desc: "new desc".into(),
            tags: vec!["cli".into()],
            includes: vec!["git".into()],
            build: "cargo build".into(),
            run: "".into(),
            test: "".into(),
            check: "".into(),
            last_wl_mtime: 999,
        };
        let changed = old.diff(&new);
        assert!(changed.contains(&"name"));
        assert!(changed.contains(&"desc"));
        assert!(changed.contains(&"tags"));
        assert!(changed.contains(&"includes"));
        assert!(changed.contains(&"last_wl_mtime"));
        assert!(!changed.contains(&"lang"));
        assert!(!changed.contains(&"build"));
    }

    #[test]
    fn diff_empty_when_identical() {
        let state = crate::project_state::ProjectState {
            name: "test".into(),
            lang: "rust".into(),
            desc: "".into(),
            tags: vec![],
            includes: vec![],
            build: "".into(),
            run: "".into(),
            test: "".into(),
            check: "".into(),
            last_wl_mtime: 0,
        };
        assert!(state.diff(&state).is_empty());
    }

    // ─── save / load roundtrip ─────────────────────────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_project();
        let state = crate::project_state::ProjectState {
            name: "myproject".into(),
            lang: "rust".into(),
            desc: "A cool project".into(),
            tags: vec!["cli".into(), "wasm".into()],
            includes: vec!["git".into(), "overseer".into()],
            build: "cargo build".into(),
            run: "cargo run".into(),
            test: "cargo test".into(),
            check: "cargo clippy".into(),
            last_wl_mtime: 1234567890,
        };
        state.save(&dir).unwrap();
        let loaded = crate::project_state::ProjectState::load(&dir).unwrap();
        assert_eq!(loaded.name, state.name);
        assert_eq!(loaded.lang, state.lang);
        assert_eq!(loaded.tags, state.tags);
        assert_eq!(loaded.includes, state.includes);
        assert_eq!(loaded.last_wl_mtime, state.last_wl_mtime);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_returns_default_when_no_state_file() {
        let dir = temp_project();
        let state = crate::project_state::ProjectState::load(&dir).unwrap();
        assert!(state.name.is_empty());
        assert!(state.lang.is_empty());
        fs::remove_dir_all(&dir).ok();
    }
}