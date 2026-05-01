//! `forge create` — create a new project.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use crate::applied_includes::save as save_applied;
use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectEntry, ProjectIndex};
use crate::paths::resolve_project_path;
use crate::verify_and_diff::verify_and_diff;
use crate::wl_parser::{parse_lang_wl, parse_wl, Language};

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}

pub fn run(name: String, lang: String, no_open: bool, _setup: bool, include: Option<String>, path: Option<String>, _run: Option<String>, editor: bool, dry_run: bool) -> Result<()> {
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

    let wl_content = build_wl_content(&name, &lang, wl_exists.then_some(&wl_path), &includes)?;

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

    // When editor runs: verify_and_diff handles index (adds if new)
    // When no editor: add_to_index adds the new project to index
    if !no_open || editor {
        verify_and_diff(&project_path, &config)?;
    } else {
        add_to_index(&name, &lang.name, &project_path)?;
        save_applied(&project_path, &includes)?;
    }

    Ok(())
}

fn load_language(lang_name: &str, config: &ForgeConfig) -> Result<Language> {
    // Try default/ first (HM-controlled, symlink to Nix store),
    // then custom/ (user-created languages).
    for base in [config.lang_default_dir(), config.lang_custom_dir()] {
        let lang_path = base.join(lang_name).join("lang.wl");
        if lang_path.exists() {
            let mut lang = parse_lang_wl(&lang_path)
                .with_context(|| format!("language '{}' not found in registry", lang_name))?;
            // Enrich with flake path from langs.json if available
            if let Ok(Some(flake)) = get_flake_from_langs_json(lang_name, config) {
                lang.flake = Some(flake);
            }
            return Ok(lang);
        }
    }
    anyhow::bail!(
        "language '{}' not found in registry (checked default/ and custom/ under {})",
        lang_name,
        config.lang_dir.display()
    )
}

/// Load langs.json and return the flake path for a given language name.
fn get_flake_from_langs_json(lang_name: &str, config: &ForgeConfig) -> Result<Option<String>> {
    let langs_path = config.config_dir().join("langs.json");
    if !langs_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&langs_path)
        .context("failed to read langs.json")?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&content)
        .context("failed to parse langs.json")?;
    for entry in entries {
        if entry.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s == lang_name)
            .unwrap_or(false)
        {
            if let Some(flake) = entry.get("flake").and_then(|v| v.as_str()) {
                return Ok(Some(flake.to_string()));
            }
        }
    }
    Ok(None)
}

/// Load includes.json and return the setup_sh for a given include name.
fn get_setup_sh_from_includes_json(inc_name: &str, config: &ForgeConfig) -> Result<Option<String>> {
    let includes_path = config.config_dir().join("includes.json");
    if !includes_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&includes_path)
        .context("failed to read includes.json")?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&content)
        .context("failed to parse includes.json")?;
    for entry in entries {
        if entry.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s == inc_name)
            .unwrap_or(false)
        {
            if let Some(setup_sh) = entry.get("setup_sh").and_then(|v| v.as_str()) {
                return Ok(Some(setup_sh.to_string()));
            }
        }
    }
    Ok(None)
}

fn build_wl_content(name: &str, lang: &Language, existing_wl: Option<&PathBuf>, includes: &[String]) -> Result<String> {
    // Pre-declare all fields when creating a new project.
    // When re-editing an existing .wl, carry over user-modified values.
    let mut lines = vec![
        format!("name=\"{}\"", name),
        format!("lang=\"{}\"", lang.name),
        format!("desc=\"\""),
        String::from("tags=[]"),
    ];

    // includes
    if !includes.is_empty() {
        let inc_str = includes.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
        lines.push(format!("includes=[{}]", inc_str));
    } else {
        lines.push(String::from("includes=[]"));
    }

    // build/run/test/check from lang defaults
    if let Some(ref b) = lang.build {
        if !b.is_empty() {
            lines.push(format!("build=\"{}\"", b));
        }
    }
    if let Some(ref r) = lang.run {
        if !r.is_empty() {
            lines.push(format!("run=\"{}\"", r));
        }
    }
    if let Some(ref t) = lang.test {
        if !t.is_empty() {
            lines.push(format!("test=\"{}\"", t));
        }
    }
    if let Some(ref c) = lang.check {
        if !c.is_empty() {
            lines.push(format!("check=\"{}\"", c));
        }
    }

    // Carry over existing user-modified fields if .wl already exists
    if let Some(ref path) = existing_wl {
        if let Ok(existing) = parse_wl(path) {
            if let Some(ref desc) = existing.desc {
                if !desc.is_empty() {
                    // Replace the empty desc we added
                    if let Some(idx) = lines.iter().position(|l| l == "desc=\"\"") {
                        lines[idx] = format!("desc=\"{}\"", desc);
                    }
                }
            }
            if !existing.tags.is_empty() {
                // Replace empty tags
                if let Some(idx) = lines.iter().position(|l| l == "tags=[]") {
                    let tags_str = existing.tags.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(",");
                    lines[idx] = format!("tags=[{}]", tags_str);
                }
            }
            if let Some(ref build) = existing.build {
                // Replace if not default
                if let Some(idx) = lines.iter().position(|l| l.starts_with("build=\"")) {
                    lines[idx] = format!("build=\"{}\"", build);
                }
            }
            if let Some(ref run) = existing.run {
                if let Some(idx) = lines.iter().position(|l| l.starts_with("run=\"")) {
                    lines[idx] = format!("run=\"{}\"", run);
                }
            }
            if let Some(ref test) = existing.test {
                if let Some(idx) = lines.iter().position(|l| l.starts_with("test=\"")) {
                    lines[idx] = format!("test=\"{}\"", test);
                }
            }
            if let Some(ref check) = existing.check {
                if let Some(idx) = lines.iter().position(|l| l.starts_with("check=\"")) {
                    lines[idx] = format!("check=\"{}\"", check);
                }
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

    // Use the flake path from langs.json — it points directly to the store
    // e.g. ~/.forge/langs/default/rust/flake.nix
    let setup_sh = if let Some(ref flake) = lang.flake {
        let lang_dir = PathBuf::from(flake).parent().unwrap().to_path_buf();
        lang_dir.join("setup.sh")
    } else {
        // Fallback: scan default/ then custom/
        for base in [config.lang_default_dir(), config.lang_custom_dir()] {
            let candidate = base.join(&lang.name).join("setup.sh");
            if candidate.exists() {
                break;
            }
        }
        config.lang_dir.join(&lang.name).join("setup.sh")
    };
    if !setup_sh.exists() {
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let lang_name = &lang.name;
    // lang_dir for FORGE_LANG_DIR env var
    let lang_dir = if let Some(ref flake) = lang.flake {
        PathBuf::from(flake).parent().unwrap().to_path_buf()
    } else {
        config.lang_dir.join(&lang.name)
    };

    let env_vars: [(&str, &str); 8] = [
        ("FORGE_PROJECT_NAME", &project_name),
        ("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or("")),
        ("FORGE_LANG", &lang_name),
        ("FORGE_LANG_DIR", &lang_dir.to_string_lossy()),
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
        let setup_sh = match get_setup_sh_from_includes_json(inc_name, config) {
            Ok(Some(sh)) => sh,
            Ok(None) => {
                eprintln!("warning: include '{}' not found in includes.json, skipping", inc_name);
                continue;
            }
            Err(e) => {
                eprintln!("warning: could not read includes.json: {}, skipping '{}'", e, inc_name);
                continue;
            }
        };

        let project_name = project_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let env_vars = [
            ("FORGE_PROJECT_NAME", project_name),
            ("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or("")),
            ("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or("")),
            ("FORGE_GITHUB_USER", &config.github_user),
            ("FORGE_EDITOR", &config.editor),
            ("FORGE_DRY_RUN", "0"),
        ];

        let mut cmd = std::process::Command::new("bash");
        cmd.arg("-c");
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

fn add_to_index(name: &str, lang: &str, path: &PathBuf) -> Result<()> {
    let mut index = index_mod::load_index()
        .unwrap_or_else(|_| ProjectIndex::new(path.parent().map(|p| p.to_path_buf()).unwrap_or_default()));

    // Check for duplicate
    if index.projects.iter().any(|p| p.name == name) {
        eprintln!("warning: project '{}' already in index", name);
        return Ok(());
    }

    let entry = ProjectEntry {
        name: name.to_string(),
        lang: lang.to_string(),
        path: path.clone(),
        added_at: now_iso(),
        last_opened: None,
        open_count: 0,
    };

    index.projects.push(entry);
    index_mod::save_index(&index)?;

    Ok(())
}