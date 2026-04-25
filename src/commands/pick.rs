//! `forge pick` — interactive fzf session picker.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::index::{self as index_mod, ProjectIndex};

pub fn run(tags: Option<String>) -> Result<()> {
    let config = ForgeConfig::load()?;
    let index = index_mod::load_index()?;

    let filter_tags: Vec<String> = if let Some(ref tags_str) = tags {
        tags_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };

    let projects: Vec<_> = index.projects.iter()
        .filter(|p| {
            if filter_tags.is_empty() {
                true
            } else {
                filter_tags.iter().all(|t| p.tags.contains(t))
            }
        })
        .collect();

    if projects.is_empty() {
        anyhow::bail!("no projects found");
    }

    // Build input for fzf: "name\tpath\tdesc"
    let input: String = projects.iter()
        .map(|p| {
            let desc = p.desc.as_deref().unwrap_or("");
            let tags = p.tags.join(",");
            format!("{}\t{}\t{} {}", p.name, p.path.to_string_lossy(), desc, tags)
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
            // Edit .wl in $EDITOR
            let wl_path = format!("{}/.wl", project_path);
            Command::new("sh")
                .args(["-c", &format!("{} {}", config.editor, wl_path)])
                .status()?;
        }
        "ctrl-o" => {
            // Open project directory in $EDITOR
            Command::new("sh")
                .args(["-c", &format!("{} {}", config.editor, project_path)])
                .status()?;
        }
        "ctrl-d" => {
            // Dry run — just print what would happen
            println!("[dry-run] session: {}", session_name_for(project_name));
            println!("[dry-run] path: {}", project_path);
        }
        "ctrl-s" => {
            // Toggle setup flag and run setup
            let setup_sh = std::path::PathBuf::from(project_path).join("setup.sh");
            if setup_sh.exists() {
                Command::new("bash")
                    .arg(&setup_sh)
                    .current_dir(project_path)
                    .status()?;
            }
            // Run direnv allow
            Command::new("direnv")
                .arg("allow")
                .current_dir(project_path)
                .status()
                .ok();
        }
        _ => {}
    }

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
