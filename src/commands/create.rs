#![allow(dead_code)]
//! `forge create` — create a new project.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry, ProjectIndex};
use crate::paths::resolve_project_path;
use crate::wl_parser::{parse_lang_wl, parse_wl, Language};

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_json_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return vec![];
    }
    let inner = &s[1..s.len() - 1].trim();
    if inner.is_empty() {
        return vec![];
    }
    let mut result = vec![];
    for part in inner.split(',') {
        result.push(strip_quotes(part.trim()));
    }
    result
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple unix timestamp — caller can convert
    format!("{}", now)
}

pub fn run(name: String, lang: String, no_open: bool, _setup: bool, include: Option<String>, path: Option<String>, run: Option<String>, editor: bool, dry_run: bool) -> Result<()> {
    let config = ForgeConfig::load()?;
    let lang = load_language(&lang, &config)?;

    let project_path = resolve_project_path(&name, &lang, &config, path.as_deref());

    if dry_run {
        println!("[dry-run] create directory: {}", project_path.display());
        println!("[dry-run] write .wl: {}/.wl", project_path.display());
        return Ok(());
    }

    // Create directory if needed
    fs::create_dir_all(&project_path)
        .with_context(|| format!("failed to create directory {}", project_path.display()))?;

    // Generate or update .wl
    let wl_path = project_path.join(".wl");
    let wl_exists = wl_path.exists();

    if wl_exists {
        println!("project already exists, updating .wl");
    }

    let includes: Vec<String> = if let Some(ref inc_str) = include {
        inc_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };

    let wl_content = build_wl_content(&name, &lang.name, wl_exists.then_some(&wl_path), &includes)?;

    fs::write(&wl_path, &wl_content)
        .with_context(|| format!("failed to write {}", wl_path.display()))?;

    println!("created: {}", wl_path.display());

    // Open editor before setup
    if !no_open {
        let editor = &config.editor;
        std::process::Command::new("sh")
            .args(["-c", &format!("{} {}", editor, wl_path.display())])
            .status()
            .context("editor failed")?;
    }

    // Run language setup
    run_lang_setup(&lang, &project_path, &config)?;

    // Run include setups
    run_include_setups(&includes, &project_path, &config)?;

    // Run arbitrary command
    if let Some(ref cmd) = run {
        std::process::Command::new("sh")
            .args(["-c", cmd])
            .current_dir(&project_path)
            .status()?;
    }

    // Open editor again if --editor
    if editor {
        let editor = &config.editor;
        std::process::Command::new("sh")
            .args(["-c", &format!("{} {}", editor, wl_path.display())])
            .status()?;
    }

    // Add to index
    add_to_index(&name, &lang.name, &project_path, wl_exists.then_some(&wl_path))?;

    Ok(())
}

fn load_language(lang_name: &str, config: &ForgeConfig) -> Result<Language> {
    let lang_path = config.base.join("languages").join(lang_name).join("lang.wl");
    parse_lang_wl(&lang_path)
        .with_context(|| format!("language '{}' not found in registry", lang_name))
}

fn build_wl_content(name: &str, lang: &str, existing_wl: Option<&PathBuf>, includes: &[String]) -> Result<String> {
    let mut lines = vec![
        format!("name=\"{}\"", name),
        format!("lang=\"{}\"", lang),
    ];

    if !includes.is_empty() {
        let inc_str = includes.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
        lines.push(format!("includes=[{}]", inc_str));
    }

    // Carry over existing fields if .wl exists
    if let Some(ref path) = existing_wl {
        if let Ok(existing) = parse_wl(path) {
            if let Some(ref desc) = existing.desc {
                if !desc.is_empty() {
                    lines.insert(2, format!("desc=\"{}\"", desc));
                }
            }
            if !existing.tags.is_empty() {
                let tags_str = existing.tags.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
                lines.push(format!("tags=[{}]", tags_str));
            }
            if let Some(ref build) = existing.build {
                lines.push(format!("build=\"{}\"", build));
            }
            if let Some(ref run) = existing.run {
                lines.push(format!("run=\"{}\"", run));
            }
            if let Some(ref test) = existing.test {
                lines.push(format!("test=\"{}\"", test));
            }
            if let Some(ref check) = existing.check {
                lines.push(format!("check=\"{}\"", check));
            }
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn run_lang_setup(lang: &Language, project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    if lang.direnv == "none" {
        return Ok(());
    }

    let setup_sh = config.base.join("languages").join(&lang.name).join("setup.sh");
    if !setup_sh.exists() {
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let lang_name = &lang.name;
    let lang_template_dir = config.base.join("languages").join(lang_name);

    let env_vars: [(&str, &str); 9] = [
        ("FORGE_PROJECT_NAME", &project_name),
        ("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or("")),
        ("FORGE_LANG", &lang_name),
        ("FORGE_LANG_TEMPLATE_DIR", &lang_template_dir.to_string_lossy()),
        ("FORGE_BASE", config.base.to_str().unwrap_or("")),
        ("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or("")),
        ("FORGE_GITHUB_USER", &config.github_user),
        ("FORGE_EDITOR", &config.editor),
        ("FORGE_DRY_RUN", "0"),
    ];

    let mut cmd = std::process::Command::new("bash");
    cmd.arg(&setup_sh);
    for (k, v) in &env_vars {
        cmd.env(k, v);
    }
    cmd.current_dir(project_path);

    let status = cmd.status()
        .with_context(|| format!("language setup failed for {}", lang.name))?;

    if !status.success() {
        anyhow::bail!("setup.sh exited with non-zero status");
    }

    Ok(())
}

fn run_include_setups(includes: &[String], project_path: &PathBuf, config: &ForgeConfig) -> Result<()> {
    for inc_name in includes {
        let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
        if !setup_sh.exists() {
            eprintln!("warning: include '{}' not found, skipping", inc_name);
            continue;
        }

        let project_name = project_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let env_vars = [
            ("FORGE_PROJECT_NAME", project_name),
            ("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or("")),
            ("FORGE_BASE", config.base.to_str().unwrap_or("")),
            ("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or("")),
            ("FORGE_GITHUB_USER", &config.github_user),
            ("FORGE_EDITOR", &config.editor),
            ("FORGE_DRY_RUN", "0"),
        ];

        let mut cmd = std::process::Command::new("bash");
        cmd.arg(&setup_sh);
        for (k, v) in &env_vars {
            cmd.env(k, v);
        }
        cmd.current_dir(project_path);

        let status = cmd.status()
            .with_context(|| format!("include setup '{}' failed", inc_name))?;

        if !status.success() {
            anyhow::bail!("include '{}' setup.sh exited with non-zero status", inc_name);
        }
    }

    Ok(())
}

fn add_to_index(name: &str, lang: &str, path: &PathBuf, _wl_path: Option<&PathBuf>) -> Result<()> {
    let mut index = index_mod::load_index()
        .unwrap_or_else(|_| ProjectIndex::new(path.parent().map(|p| p.to_path_buf()).unwrap_or_default()));

    // Check for duplicate
    if index.projects.iter().any(|p| p.name == name) {
        eprintln!("warning: project '{}' already in index", name);
        return Ok(());
    }

    let added_at = now_iso();

    let entry = ProjectEntry {
        name: name.to_string(),
        lang: lang.to_string(),
        path: path.clone(),
        desc: None,
        tags: vec![],
        includes: vec![],
        build: None,
        added_at,
        last_opened: None,
        open_count: 0,
    };

    index.projects.push(entry);
    index_mod::save_index(&index)?;

    Ok(())
}
