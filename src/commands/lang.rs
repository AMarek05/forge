//! `forge lang` — list or add language packs.

use anyhow::Result;

pub fn run(_list: bool, _add: bool, _lang_name: Option<&str>, _path: Option<&str>, _direnv: Option<&str>) -> Result<()> {
    println!("lang not implemented");
    Ok(())
}