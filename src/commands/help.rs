#![allow(dead_code)]
//! `forge help` — show help.

use anyhow::Result;

pub fn run(command: String) -> Result<()> {
    let cmd = command.trim();

    if cmd.is_empty() {
        print_global_help();
    } else {
        match cmd {
            "create" => print_create_help(),
            "remove" => print_remove_help(),
            "list" => print_list_help(),
            "sync" => print_sync_help(),
            "cd" => print_cd_help(),
            "session" => print_session_help(),
            "pick" => print_pick_help(),
            "setup" => print_setup_help(),
            "include" => print_include_help(),
            "lang" => print_lang_help(),
            "overseer" => print_overseer_help(),
            "overseer-def" => print_overseer_def_help(),
            "edit" => print_edit_help(),
            "open" => print_open_help(),
            _ => eprintln!("unknown command: {}", cmd),
        }
    }

    Ok(())
}

fn print_global_help() {
    println!(r#"forge — tmux sessionizer backed by ~/sync with plugin-style includes

USAGE:
    forge <COMMAND>

COMMANDS:
    create         Create a new project
    remove         Remove a project from the index
    list           List all projects
    sync           Re-scan FORGE_SYNC_BASE and rebuild the index
    cd             Print project path to stdout
    session        Switch to or create a tmux session
    pick           Interactive fzf session picker
    setup          Run setup scripts for a project
    include        List or show include modules
    lang           List or add language packs
    overseer       Run or manage overseer.nvim task templates
    overseer-def   Print JSON overseer task definition
    edit           Edit project's .wl in $EDITOR
    open           Open project directory in $EDITOR

Run 'forge help <COMMAND>' for details.
"#);
}

fn print_create_help() {
    println!(r#"forge-create — create a new project

USAGE:
    forge create <NAME> --lang <LANG> [flags]

FLAGS:
    --lang <lang>       Language (required)
    --no-open           Skip opening .wl in $EDITOR
    --setup             Run setup scripts after creating .wl
    --include <list>    Pre-populate includes field (comma-separated)
    --path <path>       Override project path
    --run <cmd>         Run arbitrary shell command after creation
    --editor            Open $EDITOR after full creation
    --dry-run           Print actions without executing
"#);
}

fn print_remove_help() {
    println!(r#"forge-remove — remove a project from the index

USAGE:
    forge remove <NAME>
"#);
}

fn print_list_help() {
    println!(r#"forge-list — list all projects

USAGE:
    forge list [--tags <tags>]

FLAGS:
    --tags <tags>    Filter by tags (comma-separated)
"#);
}

fn print_sync_help() {
    println!(r#"forge-sync — re-scan FORGE_SYNC_BASE and rebuild the index

USAGE:
    forge sync
"#);
}

fn print_cd_help() {
    println!(r#"forge-cd — print project path to stdout

USAGE:
    forge cd <NAME> [--print]
"#);
}

fn print_session_help() {
    println!(r#"forge-session — switch to or create a tmux session

USAGE:
    forge session [NAME] [--setup] [--open]

FLAGS:
    --setup    Run setup scripts in the session before switching
    --open     Open project in $EDITOR after switching
"#);
}

fn print_pick_help() {
    println!(r#"forge-pick — interactive fzf session picker

USAGE:
    forge pick [--tags <tags>]

KEYBINDINGS:
    Enter      Open session (switch-or-create)
    Ctrl+R     Remove selected project
    Ctrl+E     Edit .wl in $EDITOR
    Ctrl+O     Open project directory in $EDITOR
    Ctrl+D     Dry run
    Ctrl+S     Run setup scripts

FLAGS:
    --tags <tags>    Filter by tags (comma-separated)
"#);
}

fn print_setup_help() {
    println!(r#"forge-setup — run setup scripts for a project

USAGE:
    forge setup <NAME> [--dry-run]

FLAGS:
    --dry-run    Print actions without executing
"#);
}

fn print_include_help() {
    println!(r#"forge-include — list or show include modules

USAGE:
    forge include [--list]
    forge include <NAME>

FLAGS:
    --list    List all available includes
"#);
}

fn print_lang_help() {
    println!(r#"forge-lang — list or add language packs

USAGE:
    forge lang --list
    forge lang add [--path <path>] [--direnv <direnv>] <NAME>

FLAGS:
    --list             List all available languages
    --add              Add a new language pack
    --path <path>      Path under ~/sync (for lang add)
    --direnv <direnv>  direnv directive (for lang add)
"#);
}

fn print_overseer_help() {
    println!(r#"forge-overseer — run or manage overseer.nvim task templates

USAGE:
    forge overseer [flags]
    forge overseer <NAME> [flags]
    forge overseer --regen
    forge overseer --rm <NAME>

FLAGS:
    --regen             Regenerate all project templates
    --rm                Remove project's templates
    --setup             Run setup scripts for overseer include

DESCRIPTION:
    Without flags, opens the overseer.nvim task picker in nvim.
    Templates are written to:
    ~/.local/share/nvim/site/lua/overseer/template/forge/
"#);
}

fn print_overseer_def_help() {
    println!(r#"forge-overseer-def — print JSON overseer task definition

USAGE:
    forge overseer-def <NAME>
"#);
}

fn print_edit_help() {
    println!(r#"forge-edit — edit project's .wl in $EDITOR

USAGE:
    forge edit <NAME>
"#);
}

fn print_open_help() {
    println!(r#"forge-open — open project directory in $EDITOR

USAGE:
    forge open <NAME>
"#);
}