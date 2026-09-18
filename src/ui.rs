use crate::config::{AppConfig, DefaultFilter};
use crate::db::ConversationRecord;
use inquire::Select;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum MenuItem {
    StartNew,
    ContinueLatest,
    ToggleView { showing_current_only: bool },
    ChangeDefaultFilter,
    Conversation(ConversationRecord),
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::StartNew => write!(f, "[+] Start a new conversation"),
            MenuItem::ContinueLatest => write!(f, "[>] Continue most recent conversation (agy -c)"),
            MenuItem::ToggleView { showing_current_only: true } => {
                write!(f, "[~] View all conversations (currently filtered to current dir)")
            }
            MenuItem::ToggleView { showing_current_only: false } => {
                write!(f, "[~] Filter to current directory (currently showing all)")
            }
            MenuItem::ChangeDefaultFilter => {
                write!(f, "[*] Configure default filter setting")
            }
            MenuItem::Conversation(conv) => {
                let time_tag = format!("[{}]", conv.relative_time());
                let title_trimmed = if conv.title.len() > 42 {
                    format!("{}...", &conv.title[..39])
                } else {
                    conv.title.clone()
                };
                let steps_tag = format!("({} steps)", conv.step_count);
                let ws = conv.primary_workspace_display();

                write!(f, "{:<12} {:<42} {:<11} | {}", time_tag, title_trimmed, steps_tag, ws)
            }
        }
    }
}

pub enum SelectionResult {
    StartNew,
    ContinueLatest,
    ResumeConversation(String),
    Exit,
}

pub fn run_interactive_menu(
    all_conversations: &[ConversationRecord],
    cwd: &Path,
    initial_filter_current: bool,
    config: &mut AppConfig,
) -> Result<SelectionResult, Box<dyn std::error::Error>> {
    let mut filter_current = initial_filter_current;

    loop {
        let filtered: Vec<ConversationRecord> = if filter_current {
            all_conversations
                .iter()
                .filter(|c| c.matches_workspace(cwd))
                .cloned()
                .collect()
        } else {
            all_conversations.to_vec()
        };

        let mut items = Vec::new();
        items.push(MenuItem::StartNew);
        items.push(MenuItem::ContinueLatest);
        items.push(MenuItem::ToggleView {
            showing_current_only: filter_current,
        });
        items.push(MenuItem::ChangeDefaultFilter);

        for conv in filtered {
            items.push(MenuItem::Conversation(conv));
        }

        let prompt_text = if filter_current {
            format!(
                "Select a conversation (Directory: {}, matches: {}):",
                cwd.display(),
                items.len() - 4
            )
        } else {
            format!("Select a conversation (All workspaces, total: {}):", items.len() - 4)
        };

        let selection = Select::new(&prompt_text, items)
            .with_page_size(15)
            .with_help_message("Use arrow keys to navigate, type to filter, Enter to select, Esc/Ctrl+C to quit")
            .prompt();

        match selection {
            Ok(MenuItem::StartNew) => return Ok(SelectionResult::StartNew),
            Ok(MenuItem::ContinueLatest) => return Ok(SelectionResult::ContinueLatest),
            Ok(MenuItem::ToggleView { .. }) => {
                filter_current = !filter_current;
                continue;
            }
            Ok(MenuItem::ChangeDefaultFilter) => {
                prompt_change_settings(config)?;
                continue;
            }
            Ok(MenuItem::Conversation(conv)) => {
                return Ok(SelectionResult::ResumeConversation(conv.id));
            }
            Err(_) => return Ok(SelectionResult::Exit),
        }
    }
}

pub fn prompt_change_settings(config: &mut AppConfig) -> Result<(), Box<dyn std::error::Error>> {
    let options = vec![
        "Filter to current directory by default",
        "Show all conversations by default",
    ];

    let current_setting = match config.default_filter {
        DefaultFilter::Current => "Current Directory",
        DefaultFilter::All => "All Workspaces",
    };

    println!("\nCurrent default filter is set to: {}\n", current_setting);

    let choice = Select::new("Choose default filter mode:", options).prompt();

    match choice {
        Ok("Filter to current directory by default") => {
            config.default_filter = DefaultFilter::Current;
            config.save()?;
            println!("Saved: default filter is now 'current directory'.\n");
        }
        Ok("Show all conversations by default") => {
            config.default_filter = DefaultFilter::All;
            config.save()?;
            println!("Saved: default filter is now 'all workspaces'.\n");
        }
        _ => {}
    }

    Ok(())
}
