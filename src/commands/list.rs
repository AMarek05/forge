//! `forge list` — list all projects.

use anyhow::Result;

use crate::index::{self as index_mod, ProjectIndex};

pub fn run(tags: Option<String>) -> Result<()> {
    let index = index_mod::load_index()?;

    let filter_tags: Vec<String> = if let Some(ref tags_str) = tags {
        tags_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![]
    };

    let filtered: Vec<_> = index.projects.iter()
        .filter(|p| {
            if filter_tags.is_empty() {
                true
            } else {
                filter_tags.iter().all(|t| p.tags.contains(t))
            }
        })
        .collect();

    if filtered.is_empty() {
        println!("no projects found");
        return Ok(());
    }

    // Header
    println!("{:25} {:10} {:35} {:30}", "NAME", "LANG", "DESC", "TAGS");
    println!("{}", "-".repeat(100));

    for p in filtered {
        let desc = p.desc.as_deref().unwrap_or("").chars().take(33).collect::<String>();
        let tags_str = p.tags.join(",");
        println!("{:25} {:10} {:35} {:30}", p.name, p.lang, desc, tags_str);
    }

    Ok(())
}
