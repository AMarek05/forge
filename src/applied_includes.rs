//! Track which includes have been applied to a project.
//!
//! Each project gets a `.forge/applied-includes` file listing includes whose
//! setup.sh has already been run. On sync/edit, we diff the project's
//! current `includes` field against this list and run any missing setups.

use std::fs;
use std::path::Path;

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

// ═══════════════════════════════════════════════════════════════════════════
// Unit tests — run with: cargo test --lib
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    fn temp_project() -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("forge-applied-test-{}", ts));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ─── load ─────────────────────────────────────────────────────────────

    #[test]
    fn load_returns_empty_when_file_missing() {
        let dir = temp_project();
        let result = crate::applied_includes::load(&dir).unwrap();
        assert_eq!(result, Vec::<String>::new());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reads_single_include() {
        let dir = temp_project();
        fs::create_dir_all(dir.join(".forge")).unwrap();
        fs::write(dir.join(".forge").join("applied-includes"), "git\n").unwrap();
        let result = crate::applied_includes::load(&dir).unwrap();
        assert_eq!(result, vec!["git"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reads_multiple_includes() {
        let dir = temp_project();
        fs::create_dir_all(dir.join(".forge")).unwrap();
        fs::write(dir.join(".forge").join("applied-includes"), "git\noverseer\nsomething\n").unwrap();
        let result = crate::applied_includes::load(&dir).unwrap();
        assert_eq!(result, vec!["git", "overseer", "something"]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_ignores_empty_lines() {
        let dir = temp_project();
        fs::create_dir_all(dir.join(".forge")).unwrap();
        fs::write(dir.join(".forge").join("applied-includes"), "git\n\noverseer\n\n").unwrap();
        let result = crate::applied_includes::load(&dir).unwrap();
        assert_eq!(result, vec!["git", "overseer"]);
        fs::remove_dir_all(&dir).ok();
    }

    // ─── save / roundtrip ─────────────────────────────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        let dir = temp_project();
        let inc = vec!["git".to_string(), "overseer".to_string()];
        crate::applied_includes::save(&dir, &inc).unwrap();
        let loaded = crate::applied_includes::load(&dir).unwrap();
        assert_eq!(loaded, inc);
        fs::remove_dir_all(&dir).ok();
    }

    // ─── diff_applied ─────────────────────────────────────────────────────

    #[test]
    fn diff_applied_returns_new_includes() {
        let current = vec!["git".to_string(), "overseer".to_string()];
        let applied = vec!["git".to_string()];
        let result = crate::applied_includes::diff_applied(&current, &applied);
        assert_eq!(result, vec!["overseer"]);
    }

    #[test]
    fn diff_applied_empty_when_all_applied() {
        let current = vec!["git".to_string()];
        let applied = vec!["git".to_string()];
        let result = crate::applied_includes::diff_applied(&current, &applied);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_applied_empty_when_no_current() {
        let current: Vec<String> = vec![];
        let applied = vec!["git".to_string()];
        let result = crate::applied_includes::diff_applied(&current, &applied);
        assert!(result.is_empty());
    }
}