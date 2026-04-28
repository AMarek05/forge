//! `forge list` — list all projects.
//!
//! Reads .wl directly for desc, tags, includes (index holds only structural fields).

use anyhow::Result;

use crate::index::{self as index_mod};
use crate::wl_parser::parse_wl;

pub fn run(tags: Option<String>) -> Result<()> {
    let index = index_mod::load_index()?;

    let filter_tags: Vec<String> = if let Some(ref tags_str) = tags {
        tags_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };

    let mut rows: Vec<(String, String, String, String, Vec<String>)> = vec![];

    for p in &index.projects {
        let wl_path = p.path.join(".wl");
        let wl = parse_wl(&wl_path).ok();
        let desc = wl.as_ref().and_then(|w| w.desc.clone()).unwrap_or_default();
        let tags = wl.as_ref().map(|w| w.tags.clone()).unwrap_or_default();
        let includes = wl.as_ref().map(|w| w.includes.clone()).unwrap_or_default();

        // Tag filtering
        if !filter_tags.is_empty() && !filter_tags.iter().all(|t| tags.contains(t)) {
            continue;
        }

        rows.push((p.name.clone(), p.lang.clone(), desc, tags.join(","), includes));
    }

    if rows.is_empty() {
        println!("no projects found");
        return Ok(());
    }

    // Header
    println!("{:25} {:10} {:35} {:30}", "NAME", "LANG", "DESC", "TAGS");
    println!("{}", "-".repeat(100));

    for (name, lang, desc, tags_str, includes) in rows {
        let desc_trunc = desc.chars().take(33).collect::<String>();
        println!("{:25} {:10} {:35} {:30}", name, lang, desc_trunc, tags_str);
    }

    Ok(())
}