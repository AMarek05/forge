//! Path resolution for forge projects.
//!
//! Priority order:
//! 1. CLI `--path` flag
//! 2. `lang.path` from language registry

use std::path::PathBuf;

use crate::config::ForgeConfig;
use crate::wl_parser::Language;

pub fn resolve_project_path(
    name: &str,
    lang: &Language,
    config: &ForgeConfig,
    explicit_path: Option<&str>,
) -> PathBuf {
    let root = if let Some(p) = explicit_path {
        config.sync_base.join(p)
    } else {
        config.sync_base.join(&lang.path)
    };

    root.join(name)
}