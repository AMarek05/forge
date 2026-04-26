//! `forge lang` — list or add language packs.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use anyhow::Result;

use crate::config::ForgeConfig;
use crate::wl_parser::parse_lang_wl;

pub fn run(_list: bool, add: bool, lang_name: Option<String>, path: Option<String>, direnv: Option<String>) -> Result<()> {
    if add {
        return run_add(lang_name.as_deref(), path.as_deref(), direnv.as_deref());
    }

    // --list
    let config = ForgeConfig::load()?;
    let langs_dir = config.base.join("languages");

    if !langs_dir.exists() {
        println!("no languages found");
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&langs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_string());

    println!("{:15} {:15} {:40}", "LANGUAGE", "PATH", "DESCRIPTION");
    println!("{}", "-".repeat(75));

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let wl_path = entry.path().join("lang.wl");

        if let Ok(lang) = parse_lang_wl(&wl_path) {
            println!("{:15} {:15} {:40}",
                name,
                lang.path,
                lang.desc.chars().take(38).collect::<String>()
            );
        } else {
            println!("{:15} {:15} {:40}", name, "", "");
        }
    }

    Ok(())
}

fn run_add(name: Option<&str>, path: Option<&str>, direnv: Option<&str>) -> Result<()> {
    // Non-interactive add — requires all args
    let name = name.ok_or_else(|| anyhow::anyhow!("lang add requires <name>"))?;
    let path = path.ok_or_else(|| anyhow::anyhow!("lang add requires --path"))?;
    let direnv = direnv.unwrap_or("none");

    let config = ForgeConfig::load()?;
    let langs_dir = config.base.join("languages").join(name);
    fs::create_dir_all(&langs_dir)?;

    // Write lang.wl
    let lang_wl = format!(r#"name="{}"
desc="{}"
path="{}"
direnv="{}"
requires=[]
setup_priority="10"
build="nix build"
run="nix run"
test="nix flake check"
check="nix flake check"
"#, name, name, path, direnv);

    fs::write(langs_dir.join("lang.wl"), lang_wl)?;
    fs::write(langs_dir.join("setup.sh"), "#!/bin/bash\n# Scaffold script\nset -e\necho 'TODO: implement setup'\n")?;
    fs::metadata(langs_dir.join("setup.sh"))?.permissions().set_mode(0o755);

    println!("created language '{}' at {}", name, langs_dir.display());
    Ok(())
}
