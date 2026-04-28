//! Track which includes have been applied to a project.
//!
//! Each project gets a `.forge/applied-includes` file listing includes whose
//! setup.sh has already been run. On sync/edit, we diff the project's
//! current `includes` field against this list and run any missing setups.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

/// Read `.forge/applied-includes` for a project, returning the set of
/// already-applied include names.
pub fn load(path: &Path) -> Result<Vec<String>> {
    let file = path.join(".forge").join("applied-includes");
    if !file.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&file)?;
    Ok(content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Save the applied-includes list back to the project's `.forge/` directory.
pub fn save(path: &Path, includes: &[String]) -> Result<()> {
    let dir = path.join(".forge");
    fs::create_dir_all(&dir)?;
    let file = dir.join("applied-includes");
    fs::write(&file, includes.join("\n"))?;
    Ok(())
}

/// Diff current includes against applied and return any that haven't been run yet.
pub fn diff_applied(current: &[String], applied: &[String]) -> Vec<String> {
    current
        .iter()
        .filter(|inc| !applied.contains(inc))
        .cloned()
        .collect()
}