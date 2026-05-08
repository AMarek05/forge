//! Path resolution for forge projects.
//!
//! Priority order:
//! 1. CLI `--path` flag - relsolves ~ and / absolute path natively
//! 2. `lang.path` from language registry

use std::path::{Path, PathBuf};

use crate::config::ForgeConfig;
use crate::wl_parser::Language;

pub fn resolve_project_path(
    name: &str,
    lang: &Language,
    config: &ForgeConfig,
    explicit_path: Option<&str>,
) -> PathBuf {
    if let Some(p) = explicit_path {
        let expanded_path = expand_tilde(Path::new(p));

        // If they provided a path, it is the FINAL destination.
        // We do NOT join(name) here.
        if expanded_path.is_absolute() {
            expanded_path
        } else {
            // If they pass a relative path (e.g. `--path my_dir`),
            // append it to sync_base, but still don't add the name!
            config.sync_base.join(expanded_path)
        }
    } else {
        // Default behavior: create it under sync_base / lang.path / project_name
        config.sync_base.join(&lang.path).join(name)
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(mut home) = dirs::home_dir() {
                let remainder = path_str.trim_start_matches("~/").trim_start_matches('~');
                if !remainder.is_empty() {
                    home.push(remainder);
                }
                return home;
            }
        }
    }
    path.to_path_buf()
}
