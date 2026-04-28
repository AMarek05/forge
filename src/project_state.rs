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

use crate::wl_parser::WlFile;

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