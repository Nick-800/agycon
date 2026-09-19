mod banner;
mod batch;
mod config;
mod db;
mod launcher;
mod search;
mod stats;
mod transcript;
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

    /// Automatically approve tool permissions (passes --dangerously-skip-permissions to agy)
    #[arg(short = 'd', short_alias = 'y', long = "dangerously-skip-permissions", visible_alias = "skip-permissions")]
    dangerously_skip_permissions: bool,

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
    /// View conversation usage statistics and storage footprint
    Stats,
    /// Search full transcripts for keywords
    Search {
        /// Keyword query to search for
        query: Option<String>,
    },
    /// Manage multiple conversations in bulk (export, delete)
    Batch,
    /// Generate shell auto-completion scripts (bash, zsh, fish, powershell, elvish)
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Display the agycon logo in the terminal
    Logo,
}

const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.jpg");

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut config = AppConfig::load();

    // Handle subcommands
    match cli.command {
        Some(Commands::Config { set_default }) => {
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
        Some(Commands::Stats) => {
            let db_path = db::locate_db_path()?;
            let conversations = db::load_conversations(&db_path)?;
            stats::print_stats(&conversations);
            return Ok(());
        }
        Some(Commands::Search { query }) => {
            let db_path = db::locate_db_path()?;
            let conversations = db::load_conversations(&db_path)?;
            if let Some(q) = query {
                let hits = search::search_all(&q, &conversations);
                println!("Found {} matching turn(s) for '{}':\n", hits.len(), q);
                for hit in hits {
                    println!("{}", hit);
                }
            } else {
                match search::run_interactive_search(&conversations)? {
                    search::SearchMenuResult::Resume { id, dangerously_skip_permissions } => {
                        launcher::launch_conversation(&id, dangerously_skip_permissions || cli.dangerously_skip_permissions);
                    }
                    search::SearchMenuResult::Back => {}
                }
            }
            return Ok(());
        }
        Some(Commands::Batch) => {
            let db_path = db::locate_db_path()?;
            let mut conversations = db::load_conversations(&db_path)?;
            batch::run_batch_menu(&mut conversations, &db_path)?;
            return Ok(());
        }
        Some(Commands::Completion { shell }) => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "agycon", &mut std::io::stdout());
            return Ok(());
        }
        Some(Commands::Logo) => {
            display_logo()?;
            return Ok(());
        }
        None => {}
    }

    // Direct actions
    if cli.new {
        launcher::launch_new(cli.dangerously_skip_permissions);
    }

    if cli.continue_latest {
        launcher::launch_continue(cli.dangerously_skip_permissions);
    }

    let cwd = env::current_dir()?;
    let db_path = db::locate_db_path()?;
    let mut conversations = db::load_conversations(&db_path)?;

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
        banner::print_banner();
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
    banner::print_banner();
    let result = ui::run_interactive_menu(&mut conversations, &cwd, &db_path, filter_current, &mut config)?;

    match result {
        ui::SelectionResult::StartNew { dangerously_skip_permissions } => {
            launcher::launch_new(dangerously_skip_permissions || cli.dangerously_skip_permissions)
        }
        ui::SelectionResult::ContinueLatest { dangerously_skip_permissions } => {
            launcher::launch_continue(dangerously_skip_permissions || cli.dangerously_skip_permissions)
        }
        ui::SelectionResult::ResumeConversation { id, dangerously_skip_permissions } => {
            launcher::launch_conversation(&id, dangerously_skip_permissions || cli.dangerously_skip_permissions)
        }
        ui::SelectionResult::Exit => Ok(()),
    }
}

fn display_logo() -> Result<(), Box<dyn Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Try chafa first
    let mut child = Command::new("chafa")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn();

    if let Ok(ref mut proc) = child {
        if let Some(ref mut stdin) = proc.stdin {
            let _ = stdin.write_all(LOGO_BYTES);
        }
        let _ = proc.wait();
        return Ok(());
    }

    println!("agycon: Antigravity Conversation Launcher");
    println!("(Install 'chafa' to render graphic images directly in the terminal)");
    Ok(())
}
