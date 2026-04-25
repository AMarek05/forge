//! `forge cd` — print cd directive for eval, or --print for bare path.
//! `forge path` — print bare project path.

use anyhow::Result;

use crate::index::{self as index_mod};

pub fn run(name: String, print_only: bool) -> Result<()> {
    let index = index_mod::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", name))?;

    if print_only {
        println!("{}", project.path.display());
    } else {
        println!("cd {}", project.path.display());
    }

    Ok(())
}
