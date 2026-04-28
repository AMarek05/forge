//! `forge overseer` — manage overseer.nvim task templates for projects.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

const TEMPLATE_DIR: &str = ".local/share/nvim/site/lua/overseer/template/forge";

fn template_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    Ok(home.join(TEMPLATE_DIR))
}

fn escape_lua(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn write_task_template(
    dir: &PathBuf,
    project: &index_mod::ProjectEntry,
    task: &str,
    cmd: &str,
    tag: &str,
) -> Result<PathBuf> {
    let name = format!("{}_{}", project.name, task);
    let escaped = escape_lua(cmd);
    let content = format!(
        r#"return {{
  name = "{name}",
  builder = function()
    return {{
      cmd = {{ "bash", "-c", "{cmd}" }},
      cwd = "{cwd}",
      components = {{ "default" }},
    }}
  end,
  tags = {{ overseer.TAG.{tag} }},
  desc = "{name}",
}}
"#,
        name = name,
        cmd = escaped,
        cwd = project.path.to_string_lossy(),
        tag = tag
    );
    let path = dir.join(format!("{}.lua", name));
    fs::write(&path, &content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn write_project_templates(project: &index_mod::ProjectEntry) -> Result<()> {
    // Read build from index (set at project creation time), fall back to .wl parsing
    let build_cmd = parse_wl(&project.path.join(".wl"))
        .ok()
        .and_then(|w| w.build)
        .unwrap_or_else(|| "nix build".to_string());

    let run_cmd = parse_wl(&project.path.join(".wl"))
        .ok()
        .and_then(|w| w.run)
        .unwrap_or_else(|| "nix run".to_string());

    let check_cmd = parse_wl(&project.path.join(".wl"))
        .ok()
        .and_then(|w| w.test.or(w.check))
        .unwrap_or_else(|| "nix flake check".to_string());

    let dir = template_dir()?;
    fs::create_dir_all(&dir).context("failed to create template dir")?;

    write_task_template(&dir, project, "build", &build_cmd, "BUILD")?;
    write_task_template(&dir, project, "run", &run_cmd, "RUN")?;
    write_task_template(&dir, project, "check", &check_cmd, "TEST")?;

    println!("overseer: registered {} (build/run/check)", project.name);
    Ok(())
}

fn remove_project_templates(name: &str) -> Result<()> {
    let dir = template_dir()?;
    for task in ["build", "run", "check"] {
        let path = dir.join(format!("{}_{}.lua", name, task));
        if path.exists() {
            fs::remove_file(&path).context("failed to remove template")?;
        }
    }
    println!("overseer: removed {}", name);
    Ok(())
}

pub fn run(regen: bool, name: Option<String>, remove: bool, _setup: bool) -> Result<()> {
    if remove {
        if let Some(ref n) = name {
            remove_project_templates(n)?;
        } else {
            anyhow::bail!("--rm requires a project name");
        }
        return Ok(());
    }

    if !regen && name.is_none() {
        open_overseer_picker()?;
        return Ok(());
    }

    let index = index_mod::load_index()
        .context("failed to load index")?;

    if let Some(ref project_name) = name {
        let project = index.projects.iter()
            .find(|p| p.name == *project_name)
            .ok_or_else(|| anyhow::anyhow!("project '{}' not found in index", project_name))?;
        write_project_templates(project)?;
    } else {
        let mut ok = 0;
        for project in &index.projects {
            if write_project_templates(project).is_ok() {
                ok += 1;
            }
        }
        println!("overseer: registered {} projects", ok);
    }

    Ok(())
}

fn open_overseer_picker() -> Result<()> {
    let nvim = std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());
    let script = "lua require('overseer').toggle()";
    match std::process::Command::new(&nvim)
        .args(["--headless", "-c", script, "-c", "quitall!"])
        .status()
    {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("overseer: nvim not found (is nvim installed?)");
            Ok(())
        }
        Err(e) => Err(e).context("failed to run nvim"),
    }
}