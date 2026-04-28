//! `forge check` — validate .wl syntax and field integrity.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::config::ForgeConfig;
use crate::wl_parser::parse_json_array;
use crate::wl_parser::strip_quotes as wp_strip_quotes;

/// Result of validating a single .wl file
#[derive(Debug)]
pub struct CheckResult {
    pub path: String,
    pub errors: Vec<CheckError>,
    pub warnings: Vec<CheckWarning>,
}

#[derive(Debug, Clone)]
pub struct CheckError {
    pub line: Option<usize>,
    pub msg: String,
}

#[derive(Debug, Clone)]
pub struct CheckWarning {
    pub line: Option<usize>,
    pub msg: String,
}

/// Validate a single .wl file.
/// Returns errors (syntax, bad refs) and warnings (empty fields).
pub fn check_wl(path: &Path, lang_dir: Option<&Path>, include_dir: Option<&Path>) -> Result<CheckResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let mut errors = vec![];
    let mut warnings = vec![];

    // ─── Syntax validation ────────────────────────────────────────────────────
    let line_errors = validate_syntax(&content, path);
    errors.extend(line_errors);

    // If syntax is completely broken, stop here
    if !errors.is_empty() {
        return Ok(CheckResult {
            path: path.to_string_lossy().to_string(),
            errors,
            warnings,
        });
    }

    // ─── Field validation (safe to parse now) ─────────────────────────────────
    let fields = match parse_fields_only(&content) {
        Ok(f) => f,
        Err(e) => {
            errors.push(CheckError {
                line: None,
                msg: format!("parse error: {}", e),
            });
            return Ok(CheckResult {
                path: path.to_string_lossy().to_string(),
                errors,
                warnings,
            });
        }
    };

    // Check for duplicate keys
    let mut seen_keys = HashSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let key = trimmed[..eq].trim();
            if !seen_keys.insert(key.to_string()) {
                let line_num = line_number(&content, line);
                errors.push(CheckError {
                    line: Some(line_num),
                    msg: format!("duplicate field key \"{}\"", key),
                });
            }
        }
    }

    // Validate lang reference
    if let Some(lang) = fields.get("lang") {
        if let Some(dir) = lang_dir {
            let lang_path = dir.join(lang);
            if !lang_path.exists() {
                errors.push(CheckError {
                    line: None,
                    msg: format!("lang=\"{}\" — no such language (expected dir: {})", lang, dir.join(lang).display()),
                });
            }
        }
    }

    // Validate includes references
    if let Some(inc_val) = fields.get("includes") {
        let includes = parse_json_array(inc_val);
        if let Some(dir) = include_dir {
            for inc in &includes {
                let inc_path = dir.join(inc);
                if !inc_path.exists() {
                    errors.push(CheckError {
                        line: None,
                        msg: format!("includes=[\"{}\"] — no such include (expected dir: {})", inc, inc_path.display()),
                    });
                }
            }
        }
    }

    // Warn on empty build/run/test/check
    for field in ["build", "run", "test", "check"] {
        if let Some(val) = fields.get(field) {
            if val.is_empty() {
                warnings.push(CheckWarning {
                    line: None,
                    msg: format!("{}=\"\" — empty; overseer will fall back to \"nix build\"", field),
                });
            }
        }
    }

    Ok(CheckResult {
        path: path.to_string_lossy().to_string(),
        errors,
        warnings,
    })
}

/// Validate raw .wl syntax without building a full WlFile.
/// Collects line-level errors: malformed lines, bad array syntax.
fn validate_syntax(content: &str, _path: &Path) -> Vec<CheckError> {
    let mut errors = vec![];
    let key_re = regex::Regex::new(r#"^([a-z_]+)\s*=\s*(.+)$"#).unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !key_re.is_match(trimmed) {
            errors.push(CheckError {
                line: Some(line_num + 1),
                msg: format!("malformed line: expected key=\"value\" or key=[array], got: {}", trimmed),
            });
            continue;
        }

        let caps = key_re.captures(trimmed).unwrap();
        let raw_value = caps.get(2).unwrap().as_str();

        // Validate array syntax
        if raw_value.starts_with('[') {
            if let Err(e) = validate_array_syntax(raw_value) {
                errors.push(CheckError {
                    line: Some(line_num + 1),
                    msg: e.to_string(),
                });
            }
        } else {
            // Non-array value must be quoted
            let trimmed_val = raw_value.trim();
            if !(trimmed_val.starts_with('"') && trimmed_val.ends_with('"')
                || trimmed_val.starts_with('\'') && trimmed_val.ends_with('\''))
            {
                errors.push(CheckError {
                    line: Some(line_num + 1),
                    msg: format!("string value must be quoted: {}", trimmed_val),
                });
            }
        }
    }

    errors
}

/// Validate array bracket matching and inner syntax.
fn validate_array_syntax(s: &str) -> Result<()> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        bail!("must be a JSON array [\"a\",\"b\"]");
    }

    let inner = &s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return Ok(()); // empty array is fine
    }

    // Check for matching quotes
    let mut in_string = false;
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if !in_string {
                    in_string = true;
                } else {
                    // Check it's not escaped
                    let _backslash_count = 0;
                    // count backslashes before this quote by peeking backwards
                    // (simplified: just require escaped quotes to be \")
                    in_string = false;
                }
            }
            '\\' => {
                // Expect exactly one char after backslash
                if chars.next().is_none() {
                    bail!("unescaped backslash at end of array");
                }
            }
            _ => {}
        }
    }

    if in_string {
        bail!("unclosed string in array");
    }

    Ok(())
}

/// Parse fields without building a WlFile — for use in early validation.
fn parse_fields_only(content: &str) -> Result<std::collections::HashMap<String, String>> {
    use std::collections::HashMap;
    let mut fields = HashMap::new();
    let key_re = regex::Regex::new(r#"^([a-z_]+)\s*=\s*(.+)$"#).unwrap();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(caps) = key_re.captures(line) {
            let key = caps.get(1).unwrap().as_str().to_string();
            let raw_value = caps.get(2).unwrap().as_str();
            let value = if raw_value.trim().starts_with('[') {
                raw_value.trim().to_string()
            } else {
                wp_strip_quotes(raw_value.trim())
            };
            fields.insert(key, value);
        } else {
            anyhow::bail!("malformed line: {}", line);
        }
    }

    Ok(fields)
}

/// Get 1-based line number for a specific line string in content.
fn line_number(content: &str, target_line: &str) -> usize {
    content.lines().position(|l| l == target_line).unwrap_or(0) + 1
}

/// Entry point: check one project or all.
pub fn run(name: Option<String>) -> Result<()> {
    match name {
        Some(n) => run_one(&n),
        None => run_all(),
    }
}

/// Run check on all projects in the index.
pub fn run_all() -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = crate::index::load_index()?;

    let lang_dir = config.lang_dir.as_deref();
    let include_dir = config.include_dir.as_deref();

    let mut had_error = false;

    for project in &index.projects {
        let wl_path = project.path.join(".wl");
        match check_wl(&wl_path, lang_dir, include_dir) {
            Ok(result) => {
                print_result(&result);
                if !result.errors.is_empty() {
                    had_error = true;
                }
            }
            Err(e) => {
                println!("❌ {}: failed to read — {}", project.name, e);
                had_error = true;
            }
        }
    }

    if had_error {
        std::process::exit(1);
    }
    Ok(())
}

/// Run check on a single project by name.
pub fn run_one(name: &str) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = crate::index::load_index()?;

    let project = index.projects.iter()
        .find(|p| p.name == name)
        .with_context(|| format!("project \"{}\" not found in index", name))?;

    let lang_dir = config.lang_dir.as_deref();
    let include_dir = config.include_dir.as_deref();
    let wl_path = project.path.join(".wl");

    let result = check_wl(&wl_path, lang_dir, include_dir)?;
    print_result(&result);

    if !result.errors.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn print_result(result: &CheckResult) {
    if result.errors.is_empty() && result.warnings.is_empty() {
        println!("✅ {}", result.path);
        return;
    }

    if result.errors.is_empty() {
        println!("⚠️  {}", result.path);
    } else {
        println!("❌ {}", result.path);
    }

    for err in &result.errors {
        let loc = err.line.map(|n| format!(":{}", n)).unwrap_or_default();
        println!("   error{}: {}", loc, err.msg);
    }

    for warn in &result.warnings {
        let loc = warn.line.map(|n| format!(":{}", n)).unwrap_or_default();
        println!("   warning{}: {}", loc, warn.msg);
    }
}
