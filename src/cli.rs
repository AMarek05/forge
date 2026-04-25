//! Command-line argument parsing using clap.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "forge",
    about = "tmux sessionizer backed by ~/sync with plugin-style includes",
    version = "0.1.0"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new project
    Create {
        /// Project name
        name: String,

        /// Language (required)
        #[arg(long)]
        lang: String,

        /// Skip opening .wl in $EDITOR
        #[arg(long)]
        no_open: bool,

        /// Run setup scripts after creating .wl
        #[arg(long)]
        setup: bool,

        /// Pre-populate includes field (comma-separated)
        #[arg(long)]
        include: Option<String>,

        /// Override project path (ignores lang.path)
        #[arg(long)]
        path: Option<String>,

        /// Run arbitrary shell command after creation
        #[arg(long)]
        run: Option<String>,

        /// Open $EDITOR after full creation
        #[arg(long)]
        editor: bool,

        /// Print actions without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove a project from the index
    Remove {
        /// Project name
        name: String,
    },

    /// List all projects
    List {
        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// Re-scan FORGE_SYNC_BASE and rebuild the index
    Sync,

    /// cd into project directory. Use --print to print path only.
    Cd {
        /// Project name
        name: String,

        /// Print path instead of cd directive
        #[arg(long)]
        print: bool,
    },

    /// Switch to or create a tmux session
    Session {
        /// Project name
        name: Option<String>,

        /// Run setup scripts in the session
        #[arg(long)]
        setup: bool,

        /// Open project in $EDITOR after switching session
        #[arg(long)]
        open: bool,
    },

    /// Interactive fzf session picker
    Pick {
        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },

    /// Run setup scripts for a project
    Setup {
        /// Project name
        name: String,

        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },

    /// List or show include modules
    Include {
        /// Include name to show details for
        #[arg(long)]
        list: bool,

        /// Include name
        name: Option<String>,
    },

    /// List or add language packs
    Lang {
        /// List all available languages
        #[arg(long)]
        list: bool,

        /// Interactive wizard to add a new language
        #[arg(long)]
        add: bool,

        /// Language name (for lang add)
        lang_name: Option<String>,

        /// Path under ~/sync (for lang add)
        #[arg(long)]
        path: Option<String>,

        /// direnv directive (for lang add)
        #[arg(long)]
        direnv: Option<String>,
    },

    /// Print JSON overseer task definition for a project
    OverseerDef {
        /// Project name
        name: String,
    },

    /// Edit project's .wl in $EDITOR
    Edit {
        /// Project name
        name: String,
    },

    /// Open project directory in $EDITOR
    Open {
        /// Project name
        name: String,
    },

    /// Show help
    Help {
        /// Command to get help for
        #[arg(default_value = "")]
        command: String,
    },
}