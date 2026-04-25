//! `forge cd` — print project path to stdout.

use anyhow::Result;

use crate::index::{self as index_mod};

pub fn run(name: String) -> Result<()> {
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    println!("{}", project.path.display());

    Ok(())
}
