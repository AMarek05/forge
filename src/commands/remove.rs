//! `forge remove` — remove a project from the index.

use anyhow::Result;

use crate::index::{self as index_mod, ProjectIndex};

pub fn run(name: String) -> Result<()> {
    let mut index = index_mod::load_index()?;

    let original_len = index.projects.len();
    index.projects.retain(|p| p.name != name);

    if index.projects.len() == original_len {
        anyhow::bail!("project '{}' not found in index", name);
    }

    index_mod::save_index(&index)?;
    println!("removed: {}", name);

    Ok(())
}
