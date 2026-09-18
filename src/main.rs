mod config;
mod db;
mod launcher;
mod ui;

use clap::{Parser, Subcommand};
use config::{AppConfig, DefaultFilter};
use std::env;
use std::error::Error;

#[derive(Parser, Debug)]
#[command(name = "agycon", version, about = "Interactive launcher and manager for Antigravity (agy) conversations")]
struct Cli {
    /// Show all conversations regardless of current workspace
    #[arg(short = 'a', long = "all", conflicts_with = "current")]
    all: bool,

    /// Filter conversations strictly to the current directory
    #[arg(short = 'c', long = "current", conflicts_with = "all")]
    current: bool,

    /// Start a new conversation directly without interactive prompt
    #[arg(short = 'n', long = "new")]
    new: bool,

    /// Continue the most recent conversation directly
    #[arg(long = "continue")]
    continue_latest: bool,

    /// List conversations non-interactively and exit
    #[arg(short = 'l', long = "list")]
    list: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// View or configure agycon settings
    Config {
        /// Set the default filter mode: 'current' (current directory) or 'all' (all workspaces)
        #[arg(short = 's', long = "set")]
        set_default: Option<DefaultFilter>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut config = AppConfig::load();

    // Handle subcommands
    if let Some(Commands::Config { set_default }) = cli.command {
        if let Some(new_mode) = set_default {
            config.default_filter = new_mode;
            config.save()?;
            println!("Configuration updated: default filter is now '{}'.", new_mode);
            if let Some(path) = AppConfig::config_path() {
                println!("Saved to: {}", path.display());
            }
        } else {
            println!("agycon Configuration:");
            println!("  Default filter: {}", config.default_filter);
            if let Some(path) = AppConfig::config_path() {
                println!("  Config file:    {}", path.display());
            }
            println!("\nTo change: agycon config --set <current|all>");
        }
        return Ok(());
    }

    // Direct actions
    if cli.new {
        launcher::launch_new();
    }

    if cli.continue_latest {
        launcher::launch_continue();
    }

    let cwd = env::current_dir()?;
    let db_path = db::locate_db_path()?;
    let conversations = db::load_conversations(&db_path)?;

    // Determine initial filtering
    let filter_current = if cli.all {
        false
    } else if cli.current {
        true
    } else {
        config.default_filter == DefaultFilter::Current
    };

    // Non-interactive list mode
    if cli.list {
        println!("Loaded {} conversations from {}\n", conversations.len(), db_path.display());
        let filtered: Vec<_> = if filter_current {
            println!("Filter: Current directory ({})", cwd.display());
            conversations.iter().filter(|c| c.matches_workspace(&cwd)).collect()
        } else {
            println!("Filter: All workspaces");
            conversations.iter().collect()
        };

        println!("{:<12} {:<42} {:<11} | Workspace", "Time", "Title", "Steps");
        println!("{}", "-".repeat(90));

        for conv in &filtered {
            let time_tag = format!("[{}]", conv.relative_time());
            let title_trimmed = if conv.title.len() > 42 {
                format!("{}...", &conv.title[..39])
            } else {
                conv.title.clone()
            };
            let steps_tag = format!("({} steps)", conv.step_count);
            println!("{:<12} {:<42} {:<11} | {}", time_tag, title_trimmed, steps_tag, conv.primary_workspace_display());
        }

        println!("\nTotal matched: {}", filtered.len());
        return Ok(());
    }

    // Interactive UI
    let result = ui::run_interactive_menu(&conversations, &cwd, filter_current, &mut config)?;

    match result {
        ui::SelectionResult::StartNew => launcher::launch_new(),
        ui::SelectionResult::ContinueLatest => launcher::launch_continue(),
        ui::SelectionResult::ResumeConversation(id) => launcher::launch_conversation(&id),
        ui::SelectionResult::Exit => Ok(()),
    }
}
