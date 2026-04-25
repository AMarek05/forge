//! `forge cd` — cd into project directory, or print path.
//! `forge path` — print project path to stdout.

use anyhow::Result;

use crate::index::{self as index_mod};

pub fn run(name: String, emit_cd: bool) -> Result<()> {
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    if emit_cd {
        println!("cd {}", project.path.display());
    } else {
        println!("{}", project.path.display());
    }

    Ok(())
}
