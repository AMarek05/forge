use std::path::PathBuf;

mod cli;
mod commands;
mod config;
mod index;
mod paths;
mod wl_parser;

use anyhow::Result;

fn main() -> Result<()> {
    println!("forge");
    Ok(())
}