//! `forge overseer-def` — print JSON overseer task definition for a project.

use std::process::Command;

use anyhow::{Context, Result};

use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(name: String) -> Result<()> {
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    let wl_path = project.path.join(".wl");
    let wl = parse_wl(&wl_path)
        .with_context(|| format!("failed to read {}", wl_path.display()))?;

    let build_cmd = wl.build.as_ref()
        .or_else(|| Some(&project.build).and_then(|b| b.as_ref()))
        .map(|s: &String| s.as_str())
        .unwrap_or("nix build");

    let json = serde_json::json!({
        "name": format!("{}:build", name),
        "builder": "custom",
        "cmd": build_cmd,
        "cwd": project.path.to_string_lossy(),
    });

    println!("{}", serde_json::to_string_pretty(&json)?);

    Ok(())
}
