//! Parse `.wl` project files and `lang.wl` language registry files.
//!
//! `.wl` format:
//!   key="value"       → String
//!   key=[array]       → Vec<String> (JSON array)
//!   # comment         → ignored
//!
//! `lang.wl` format is similar but has different required fields.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Language {
    pub name: String,
    pub desc: String,
    pub path: String,
    pub direnv: String,
    pub build: Option<String>,
    pub run: Option<String>,
    pub test: Option<String>,
    pub check: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WlFile {
    pub name: Option<String>,
    pub lang: Option<String>,
    pub desc: Option<String>,
    pub tags: Vec<String>,
    pub includes: Vec<String>,
    pub build: Option<String>,
    pub run: Option<String>,
    pub test: Option<String>,
    pub check: Option<String>,
}

pub fn parse_lang_wl(path: &Path) -> Result<Language> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let fields = parse_fields(&content, path)?;

    Ok(Language {
        name: fields.get("name").cloned().unwrap_or_default(),
        desc: fields.get("desc").cloned().unwrap_or_default(),
        path: fields.get("path").cloned().unwrap_or_default(),
        direnv: fields.get("direnv").cloned().unwrap_or_default(),
        build: fields.get("build").cloned(),
        run: fields.get("run").cloned(),
        test: fields.get("test").cloned(),
        check: fields.get("check").cloned(),
    })
}

pub fn parse_wl(path: &Path) -> Result<WlFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let fields = parse_fields(&content, path)?;

    Ok(WlFile {
        name: fields.get("name").cloned(),
        lang: fields.get("lang").cloned(),
        desc: fields.get("desc").cloned(),
        tags: fields
            .get("tags")
            .cloned()
            .map(|s| parse_json_array(&s))
            .unwrap_or_default(),
        includes: fields
            .get("includes")
            .cloned()
            .map(|s| parse_json_array(&s))
            .unwrap_or_default(),
        build: fields.get("build").cloned(),
        run: fields.get("run").cloned(),
        test: fields.get("test").cloned(),
        check: fields.get("check").cloned(),
    })
}

fn parse_fields(content: &str, path: &Path) -> Result<HashMap<String, String>> {
    let mut fields = HashMap::new();
    let key_re = Regex::new(r#"^([a-z_]+)\s*=\s*(.+)$"#).unwrap();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(caps) = key_re.captures(line) {
            let key = caps.get(1).unwrap().as_str().to_string();
            let raw_value = caps.get(2).unwrap().as_str();

            let value = if raw_value.starts_with('[') {
                // JSON array — keep as-is for later parsing
                raw_value.to_string()
            } else {
                // Strip inline comment before processing
                let clean = raw_value.split('#').next().unwrap_or(raw_value).trim();
                if !(clean.starts_with('"') && clean.ends_with('"')
                    || clean.starts_with('\'') && clean.ends_with('\'')) {
                    anyhow::bail!("string value must be quoted: {}", raw_value);
                }
                strip_quotes(raw_value.split('#').next().unwrap_or(raw_value))
            };

            fields.insert(key, value);
        } else {
            anyhow::bail!("failed to parse line in {}: {}", path.display(), line);
        }
    }

    Ok(fields)
}

pub fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_json_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return vec![];
    }
    if !is_bracket_balanced(s) {
        return vec![];
    }

    let inner = &s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return vec![];
    }
    let mut result = vec![];
    for part in inner.split(',') {
        let part = part.trim();
        result.push(strip_quotes(part));
    }
    result
}


fn is_bracket_balanced(s: &str) -> bool {
    let mut depth = 0;
    let mut in_string = false;
    for c in s.chars() {
        match c {
            '"' => in_string = !in_string,
            '[' if !in_string => depth += 1,
            ']' if !in_string => {
                if depth == 0 { return false; }
                depth -= 1;
            }
            _ => {}
        }
    }
    !in_string && depth == 0
}

// ═══════════════════════════════════════════════════════════════════════════
// Unit tests — run with: cargo test --lib
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    fn temp_wl(content: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("forge-test-{}.wl", ts));
        fs::write(&path, content).unwrap();
        path
    }

    fn cleanup(path: &PathBuf) {
        fs::remove_file(path).ok();
    }

    // ─── strip_quotes ─────────────────────────────────────────────────────

    #[test]
    fn strip_quotes_double() {
        assert_eq!(crate::wl_parser::strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_single() {
        assert_eq!(crate::wl_parser::strip_quotes("'world'"), "world");
    }

    #[test]
    fn strip_quotes_no_quotes() {
        assert_eq!(crate::wl_parser::strip_quotes("plain"), "plain");
    }

    #[test]
    fn strip_quotes_with_whitespace() {
        assert_eq!(crate::wl_parser::strip_quotes("  \"spacy\"  "), "spacy");
    }

    // ─── parse_json_array ─────────────────────────────────────────────────

    #[test]
    fn parse_json_array_empty() {
        assert_eq!(crate::wl_parser::parse_json_array("[]"), Vec::<String>::new());
    }

    #[test]
    fn parse_json_array_single() {
        assert_eq!(crate::wl_parser::parse_json_array("[\"rust\"]"), vec!["rust"]);
    }

    #[test]
    fn parse_json_array_multiple() {
        assert_eq!(
            crate::wl_parser::parse_json_array("[\"cli\",\"wasm\",\"test\"]"),
            vec!["cli", "wasm", "test"]
        );
    }

    #[test]
    fn parse_json_array_with_whitespace() {
        assert_eq!(crate::wl_parser::parse_json_array("[  \"a\", \"b\"  ]"), vec!["a", "b"]);
    }

    #[test]
    fn parse_json_array_not_array() {
        assert_eq!(crate::wl_parser::parse_json_array("\"not\""), Vec::<String>::new());
    }

    #[test]
    fn parse_json_array_single_quotes() {
        assert_eq!(crate::wl_parser::parse_json_array("['rust','python']"), vec!["rust", "python"]);
    }

    // ─── parse_wl ─────────────────────────────────────────────────────────

    #[test]
    fn parse_wl_minimal() {
        let p = temp_wl("name=\"myproject\"\nlang=\"rust\"");
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(wl.name, Some("myproject".into()));
        assert_eq!(wl.lang, Some("rust".into()));
    }

    #[test]
    fn parse_wl_all_fields() {
        let p = temp_wl(
            r#"name="test"
lang="rust"
desc="A test project"
tags=["cli","wasm"]
includes=["git","overseer"]
build="cargo build"
run="cargo run"
test="cargo test"
check="cargo clippy"
"#,
        );
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(wl.name, Some("test".into()));
        assert_eq!(wl.desc, Some("A test project".into()));
        assert_eq!(wl.tags, vec!["cli", "wasm"]);
        assert_eq!(wl.includes, vec!["git", "overseer"]);
        assert_eq!(wl.build, Some("cargo build".into()));
    }

    #[test]
    fn parse_wl_empty_arrays() {
        let p = temp_wl("name=\"test\"\ntags=[]\nincludes=[]");
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        assert!(wl.tags.is_empty());
        assert!(wl.includes.is_empty());
    }

    #[test]
    fn parse_wl_ignores_comments() {
        let p = temp_wl("# comment\nname=\"test\"\nlang=\"rust\" # inline");
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(wl.name, Some("test".into()));
    }

    #[test]
    fn parse_wl_malformed_line() {
        let p = temp_wl("name=\"test\"\nthis line is malformed\nlang=\"rust\"");
        let result = crate::wl_parser::parse_wl(&p);
        cleanup(&p);
        assert!(result.is_err());
    }

    #[test]
    fn parse_wl_unclosed_bracket() {
        let p = temp_wl("name=\"test\"\ntags=[\"cli");
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        // Unbalanced bracket → parse_json_array returns [], test passes
        assert_eq!(wl.name, Some("test".into()));
        assert_eq!(wl.tags, Vec::<String>::new());
    }

    #[test]
    fn parse_wl_unquoted_string() {
        let p = temp_wl("name=test");
        let result = crate::wl_parser::parse_wl(&p);
        cleanup(&p);
        assert!(result.is_err());
    }

    #[test]
    fn parse_wl_duplicate_key_keeps_last() {
        let p = temp_wl("name=\"first\"\nname=\"second\"");
        let wl = crate::wl_parser::parse_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(wl.name, Some("second".into()));
    }

    // ─── parse_lang_wl ────────────────────────────────────────────────────

    #[test]
    fn parse_lang_wl_basic() {
        let p = temp_wl(
            r#"name="rust"
desc="Rust language support"
path="languages/rust"
direnv="use nix"
setup_priority="100"
build="cargo build"
run="cargo run"
"#,
        );
        let lang = crate::wl_parser::parse_lang_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(lang.name, "rust");
        assert_eq!(lang.build, Some("cargo build".into()));
    }

    #[test]
    fn parse_lang_wl_optional_fields_absent() {
        let p = temp_wl("name=\"python\"\ndesc=\"Python\"\npath=\"python\"\ndirenv=\"use poetry\"");
        let lang = crate::wl_parser::parse_lang_wl(&p).unwrap();
        cleanup(&p);
        assert_eq!(lang.name, "python");
        assert_eq!(lang.build, None);
    }
}