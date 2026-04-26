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
use clap_complete::Generator;

fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
    clap_complete::generate(gen, cmd, "forge", &mut std::io::stdout());
}

fn list_shells() -> String {
    let shells = clap_complete::shells::all();
    shells.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Handle completion generation early, before command dispatch
    if let Some(shell) = &cli.generate_completion {
        let mut cmd = cli::Cli::command();
        match shell.as_str() {
            "zsh" => print_completions(clap_complete::shells::Zsh, &mut cmd),
            "bash" => print_completions(clap_complete::shells::Bash, &mut cmd),
            "fish" => print_completions(clap_complete::shells::Fish, &mut cmd),
            "powershell" => print_completions(clap_complete::shells::PowerShell, &mut cmd),
            _ => anyhow::bail!("Unsupported shell: {}. Use: {}", shell, list_shells()),
        }
        return Ok(());
    }

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
        cli::Command::Overseer { regen, name, rm, setup } =>
            commands::overseer(regen, name, rm, setup),
        cli::Command::OverseerDef { name } =>
            commands::overseer_def(name),
        cli::Command::Edit { name } =>
            commands::edit(name),
        cli::Command::Open { name } =>
            commands::open(name),
    }
}