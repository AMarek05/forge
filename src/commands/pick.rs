//! `forge pick` — interactive fzf session picker.
//!
//! Reads .wl directly for desc/tags/includes (index holds only structural fields).

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::config::ForgeConfig;
use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(tags: Option<String>) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = index_mod::load_index()?;

    let filter_tags: Vec<String> = if let Some(ref tags_str) = tags {
        tags_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };

    let mut projects: Vec<(String, String, String, Vec<String>)> = vec![];

    for p in &index.projects {
        let wl_path = p.path.join(".wl");
        let wl = parse_wl(&wl_path).ok();
        let desc = wl.as_ref().and_then(|w| w.desc.clone()).unwrap_or_default();
        let project_tags = wl.as_ref().map(|w| w.tags.clone()).unwrap_or_default();

        // Tag filtering — uses .wl tags directly
        if !filter_tags.is_empty() && !filter_tags.iter().all(|t| project_tags.contains(t)) {
            continue;
        }

        projects.push((p.name.clone(), p.path.to_string_lossy().to_string(), desc, project_tags));
    }

    if projects.is_empty() {
        anyhow::bail!("no projects found");
    }

    // Build input for fzf: "name\tpath\tdesc\ttags"
    let input: String = projects.iter()
        .map(|(name, path, desc, tags)| {
            let tags_str = tags.join(",");
            format!("{}\t{}\t{}\t{}", name, path, desc, tags_str)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // fzf expects keys: Enter=open, Ctrl+R=remove, Ctrl+E=edit, Ctrl+O=open-dir, Ctrl+D=dry-run, Ctrl+S=toggle-setup
    let mut fzf_output = Command::new("fzf")
        .args([
            "--ansi",
            "--preview-window=right:60%",
            "--preview=cat {2}/.wl",
            "--expect=enter,ctrl-r,ctrl-e,ctrl-o,ctrl-d,ctrl-s",
            "--delimiter=\t",
            "--with-nth=1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    // Write input to fzf's stdin
    if let Some(mut stdin) = fzf_output.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }

    let output = fzf_output.wait_with_output()?;

    if !output.status.success() {
        // User cancelled or no selection
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        return Ok(());
    }

    let key = lines[0].trim();
    let rest = &lines[1..];

    if rest.is_empty() {
        return Ok(());
    }

    let selected = rest[0];
    let fields: Vec<&str> = selected.split('\t').collect();
    if fields.len() < 2 {
        anyhow::bail!("invalid fzf output");
    }

    let project_name = fields[0];
    let project_path = fields[1];

    match key {
        "enter" | "" => {
            // Switch or create session
            let session_name = format!("forge-{}", project_name);
            let tmux_bin = &config.tmux_binary;

            // Check if session exists
            if Command::new(tmux_bin)
                .args(["has-session", "-t", &session_name])
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                Command::new(tmux_bin)
                    .args(["switch-client", "-t", &session_name])
                    .status()?;
            } else {
                Command::new(tmux_bin)
                    .args(["new-session", "-d", "-s", &session_name, "-c", project_path])
                    .status()?;
                Command::new(tmux_bin)
                    .args(["switch-client", "-t", &session_name])
                    .status()?;
            }

            update_last_opened(project_name)?;
        }
        "ctrl-r" => {
            // Remove
            let mut index = index_mod::load_index()?;
            index.projects.retain(|p| p.name != project_name);
            index_mod::save_index(&index)?;
            println!("removed: {}", project_name);
        }
        "ctrl-e" => {
            // Edit .wl in $EDITOR — re-read .wl for new includes after editor close
            let wl_path = std::path::PathBuf::from(&project_path).join(".wl");
            std::env::set_current_dir(&project_path)?;
            let editor = &config.editor;
            let cmd = if editor.contains("nvim") {
                format!("{} -c Oil {}", editor, wl_path.to_string_lossy())
            } else {
                format!("{} {}", editor, wl_path.to_string_lossy())
            };
            Command::new("sh")
                .args(["-c", &cmd])
                .status()?;

            // Diff includes after edit
            let wl = parse_wl(&wl_path).ok();
            if let Some(ref w) = wl {
                diff_and_sync_includes(&project_path, &w.includes, &config)?;
            }
        }
        "ctrl-o" => {
            // Open project directory in $EDITOR
            std::env::set_current_dir(&project_path)?;
            let editor = &config.editor;
            let cmd = if editor.contains("nvim") {
                format!("{} -c Oil", editor)
            } else {
                editor.clone()
            };
            Command::new("sh")
                .args(["-c", &cmd])
                .status()?;
        }
        "ctrl-d" => {
            // Dry run — just print what would happen
            println!("[dry-run] session: {}", session_name_for(project_name));
            println!("[dry-run] path: {}", project_path);
        }
        "ctrl-s" => {
            // Toggle setup flag and run setup
            let setup_sh = std::path::PathBuf::from(&project_path).join("setup.sh");
            if setup_sh.exists() {
                Command::new("bash")
                    .arg(&setup_sh)
                    .current_dir(&project_path)
                    .status()?;
            }
            // Run direnv allow
            Command::new("direnv")
                .arg("allow")
                .current_dir(&project_path)
                .status()
                .ok();
        }
        _ => {}
    }

    Ok(())
}

/// Diff project includes against .forge/applied-includes and run any new setups.
fn diff_and_sync_includes(project_path: &str, current_includes: &[String], config: &ForgeConfig) -> Result<()> {
    use crate::applied_includes::{diff_applied, load as load_applied, save as save_applied};

    let path = std::path::PathBuf::from(project_path);
    let applied = load_applied(&path)?;
    let new_includes = diff_applied(current_includes, &applied);

    for inc_name in &new_includes {
        run_include_setup(inc_name, &path, config)?;
    }

    if !new_includes.is_empty() {
        let mut all_applied = applied;
        all_applied.extend(new_includes);
        save_applied(&path, &all_applied)?;
    }

    Ok(())
}

fn run_include_setup(inc_name: &str, project_path: &std::path::PathBuf, config: &ForgeConfig) -> Result<()> {
    let setup_sh = config.base.join("includes").join(inc_name).join("setup.sh");
    if !setup_sh.exists() {
        eprintln!("warning: include '{}' not found, skipping", inc_name);
        return Ok(());
    }

    let project_name = project_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let status = Command::new("bash")
        .arg(&setup_sh)
        .env("FORGE_PROJECT_NAME", project_name)
        .env("FORGE_PROJECT_PATH", project_path.to_str().unwrap_or(""))
        .env("FORGE_BASE", config.base.to_str().unwrap_or(""))
        .env("FORGE_SYNC_BASE", config.sync_base.to_str().unwrap_or(""))
        .env("FORGE_GITHUB_USER", &config.github_user)
        .env("FORGE_EDITOR", &config.editor)
        .env("FORGE_DRY_RUN", "0")
        .current_dir(project_path)
        .status()
        .with_context(|| format!("include setup '{}' failed", inc_name))?;

    if !status.success() {
        anyhow::bail!("include '{}' setup.sh exited with non-zero status", inc_name);
    }

    println!("applied include: {} for {}", inc_name, project_name);

    Ok(())
}

fn session_name_for(name: &str) -> String {
    format!("forge-{}", name)
}

fn update_last_opened(name: &str) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::index::{self as index_mod};

    let mut index = index_mod::load_index()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for p in &mut index.projects {
        if p.name == name {
            p.last_opened = Some(format!("{}", now));
            p.open_count += 1;
            break;
        }
    }

    index_mod::save_index(&index)?;
    Ok(())
}