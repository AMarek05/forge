//! Path resolution for forge projects.
//!
//! Priority order:
//! 1. CLI `--path` flag - relsolves ~ and / absolute path natively
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
        // Handle Tilde Expansion
        let expanded_path = if p.starts_with("~/") || p == "~" {
            if let Some(mut home) = dirs::home_dir() {
                // Strip the "~/" and append the rest to the home directory
                let remainder = p.trim_start_matches("~/").trim_start_matches('~');
                if !remainder.is_empty() {
                    home.push(remainder);
                }
                home
            } else {
                // Fallback if we somehow can't find the home directory
                PathBuf::from(p)
            }
        } else {
            PathBuf::from(p)
        };

        // Handle Absolute vs Relative (after expanding tilde)
        if expanded_path.is_absolute() {
            expanded_path
        } else {
            config.sync_base.join(expanded_path)
        }
    } else {
        config.sync_base.join(&lang.path)
    };

    root.join(name)
}
