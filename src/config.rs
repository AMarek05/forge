//! Forge configuration — loaded from $FORGE_CONFIG_DIR/config.json (written by HM module).
//!
//! Layout:
//!   $FORGE_CONFIG_DIR/          (e.g. ~/.forge)
//!     config.json              — structured config (store symlink, immutable after build)
//!     index.json              — project index (mutable)
//!
//! The config file is a symlink to a Nix store path.
//! All other runtime state lives under $FORGE_CONFIG_DIR.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ForgeConfig {
    pub sync_base: PathBuf,
    pub editor: String,
    pub tmux_bin: String,
    pub github_user: String,
    pub lang_dir: PathBuf,
    pub include_dir: PathBuf,
    #[serde(skip_deserializing)]
    config_dir: PathBuf,
}

impl ForgeConfig {
    /// Load from $FORGE_CONFIG_DIR/config.json (set by HM module).
    /// No fallback — errors if env var absent or config file missing.
    pub fn load() -> Result<Self> {
        let config_dir = std::env::var("FORGE_CONFIG_DIR")
            .context("FORGE_CONFIG_DIR env var not set — HM module should set this")?;

        let config_dir = PathBuf::from(&config_dir);
        let config_path = config_dir.join("config.json");

        if !config_path.exists() {
            anyhow::bail!(
                "config file not found at {} — rebuild HM module to generate it",
                config_path.display()
            );
        }

        Self::load_from_path(&config_dir)
    }

    /// Load from an explicit config directory path.
    pub fn load_from_path(config_dir: &Path) -> Result<Self> {
        let config_path = config_dir.join("config.json");
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;

        let mut config: ForgeConfig = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", config_path.display()))?;

        // Resolve symlinks so paths are canonical
        config.config_dir = config_dir.canonicalize()
            .unwrap_or_else(|_| PathBuf::from(config_dir));
        config.lang_dir = config.lang_dir.canonicalize().unwrap_or(config.lang_dir);
        config.include_dir = config.include_dir.canonicalize().unwrap_or(config.include_dir);
        config.sync_base = config.sync_base.canonicalize().unwrap_or(config.sync_base);

        Ok(config)
    }

    /// Directory containing config.json and index.json (e.g. ~/.forge)
    pub fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    /// Project index file path
    pub fn index_path(&self) -> PathBuf {
        self.config_dir.join("index.json")
    }

    /// Per-project state directory (inside config_dir)
    pub fn state_dir(&self) -> PathBuf {
        self.config_dir.join("state")
    }

    /// Project root — same as sync_base
    pub fn projects_dir(&self) -> PathBuf {
        self.sync_base.clone()
    }
}