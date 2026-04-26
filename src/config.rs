#![allow(dead_code)]
//! Parse `~/.forge/config.sh` shell-style exports into a ForgeConfig struct.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ForgeConfig {
    pub github_user: String,
    pub default_remote_base: String,
    pub sync_base: PathBuf,
    pub base: PathBuf,
    pub editor: String,
    pub tmux_binary: String,
    pub path_override: Option<String>,
    pub lang_dir: Option<PathBuf>,
    pub include_dir: Option<PathBuf>,
}

impl ForgeConfig {
    pub fn load() -> Result<Self> {
        let config_path = dirs::home_dir()
            .context("no home dir")?
            .join(".forge/config.sh");

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;

        Self::parse(&content)
    }

    fn parse(content: &str) -> Result<Self> {
        let mut vars: HashMap<String, String> = HashMap::new();

        let export_re = Regex::new(r#"^\s*export\s+([A-Z_]+)="([^"]*)"\s*$"#).unwrap();
        let simple_re = Regex::new(r#"^\s*([A-Z_]+)="([^"]*)"\s*$"#).unwrap();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Try export pattern first
            if let Some(caps) = export_re.captures(line) {
                let key = caps.get(1).unwrap().as_str().to_string();
                let val = caps.get(2).unwrap().as_str().to_string();
                vars.insert(key, val);
            } else if let Some(caps) = simple_re.captures(line) {
                let key = caps.get(1).unwrap().as_str().to_string();
                let val = caps.get(2).unwrap().as_str().to_string();
                vars.insert(key, val);
            }
        }

        // Expand $HOME in paths
        let home = dirs::home_dir().unwrap_or_default();
        let expand = |s: &str| s.replace("$HOME", home.to_str().unwrap_or(""));

        let sync_base = expand(vars.get("FORGE_SYNC_BASE").unwrap_or(&format!("{}/sync", home.display())));
        let base = expand(vars.get("FORGE_BASE").unwrap_or(&format!("{}/.forge", home.display())));

        Ok(ForgeConfig {
            github_user: vars.get("FORGE_GITHUB_USER").cloned().unwrap_or_default(),
            default_remote_base: vars.get("FORGE_DEFAULT_REMOTE_BASE").cloned().unwrap_or_default(),
            sync_base: PathBuf::from(sync_base),
            base: PathBuf::from(base),
            editor: vars.get("FORGE_EDITOR").cloned().unwrap_or_else(|| "nvim".to_string()),
            tmux_binary: vars.get("FORGE_TMUX_BINARY").cloned().unwrap_or_else(|| "tmux".to_string()),
            path_override: vars.get("FORGE_PATH_OVERRIDE").cloned().filter(|s| !s.is_empty()),
            lang_dir: vars.get("FORGE_LANG_DIR").map(PathBuf::from),
            include_dir: vars.get("FORGE_INCLUDE_DIR").map(PathBuf::from),
        })
    }
}