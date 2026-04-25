use std::path::PathBuf;

mod cli;
mod commands;
mod config;
mod include;
mod index;
mod paths;
mod tmux;
mod wl_parser;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Create { name, lang, no_open, setup, include, path, run, editor, dry_run } =>
            commands::create(name, lang, no_open, setup, include, path, run, editor, dry_run),
        cli::Command::Remove { name } =>
            commands::remove(name),
        cli::Command::List { tags } =>
            commands::list(tags),
        cli::Command::Sync =>
            commands::sync(),
        cli::Command::Cd { name, print } =>
            commands::cd(name, print),
        cli::Command::Session { name, setup, open } =>
            commands::session(name, setup, open),
        cli::Command::Pick { tags } =>
            commands::pick(tags),
        cli::Command::Setup { name, dry_run } =>
            commands::setup(name, dry_run),
        cli::Command::Include { list, name } =>
            commands::include(list, name),
        cli::Command::Lang { list, add, lang_name, path, direnv } =>
            commands::lang(list, add, lang_name, path, direnv),
        cli::Command::OverseerDef { name } =>
            commands::overseer_def(name),
        cli::Command::Edit { name } =>
            commands::edit(name),
        cli::Command::Open { name } =>
            commands::open(name),
        cli::Command::Help { command } =>
            commands::help(command),
    }
}
