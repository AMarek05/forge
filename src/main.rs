mod applied_includes;
mod cli;
mod commands;
mod project_state;
mod verify_and_diff;
mod config;
mod include;
mod index;
mod paths;
mod tmux;
mod wl_parser;

use anyhow::Result;
use clap::{Parser, CommandFactory};

fn list_shells() -> String {
    ["zsh", "bash", "fish", "powershell"].join(", ")
}

fn main() -> Result<()> {
    // Handle --generate-completion and --print-lang-dir by intercepting args
    // before clap does its normal parsing (which requires a subcommand).
    let args: Vec<String> = std::env::args_os().map(|s| s.to_string_lossy().to_string()).collect();

    if let Some(idx) = args.iter().position(|a| a == "--generate-completion") {
        if let Some(shell) = args.get(idx + 1) {
            let mut cmd = cli::Cli::command();
            match shell.as_str() {
                "zsh" => clap_complete::generate(clap_complete::shells::Zsh, &mut cmd, "forge", &mut std::io::stdout()),
                "bash" => clap_complete::generate(clap_complete::shells::Bash, &mut cmd, "forge", &mut std::io::stdout()),
                "fish" => clap_complete::generate(clap_complete::shells::Fish, &mut cmd, "forge", &mut std::io::stdout()),
                "powershell" => clap_complete::generate(clap_complete::shells::PowerShell, &mut cmd, "forge", &mut std::io::stdout()),
                _ => anyhow::bail!("Unsupported shell: {}. Use: {}", shell, list_shells()),
            }
            return Ok(());
        }
    }

    if args.contains(&"--print-lang-dir".to_string()) {
        let config = config::ForgeConfig::load()?;
        println!("{}", config.lang_dir.display());
        return Ok(());
    }

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Command::Create { name, lang, no_open, setup, include, path, run, editor, dry_run } =>
            commands::create(name, lang, no_open, setup, include, path, run, editor, dry_run),
        cli::Command::Remove { name } =>
            commands::remove(name),
        cli::Command::List { tags } =>
            commands::list(tags),
        cli::Command::Sync { langs, includes } =>
            commands::sync::run(&commands::SyncFlags { langs, includes }),
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
        cli::Command::Check { name } =>
            commands::check(name),
        cli::Command::Health { fix } =>
            commands::health(fix),
    }
}
