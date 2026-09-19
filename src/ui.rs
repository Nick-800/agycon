use crate::config::{AppConfig, DefaultFilter};
use crate::db::{self, ConversationRecord, TimeframeFilter};
use crate::search;
use crate::stats;
use crate::transcript;
use inquire::{Confirm, Select, Text};
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum MenuItem {
    StartNew,
    ContinueLatest,
    SearchTranscripts,
    ViewStats,
    ToggleView { showing_current_only: bool },
    ToggleTimeframe { current: TimeframeFilter },
    ChangeDefaultFilter,
    Conversation(ConversationRecord),
}

impl fmt::Display for MenuItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuItem::StartNew => write!(f, "[+] Start a new conversation"),
            MenuItem::ContinueLatest => write!(f, "[>] Continue most recent conversation (agy -c)"),
            MenuItem::SearchTranscripts => write!(f, "[?] Search conversation transcripts (full-text)"),
            MenuItem::ViewStats => write!(f, "[%] View usage statistics and storage footprint"),
            MenuItem::ToggleView { showing_current_only: true } => {
                write!(f, "[~] View all workspaces (currently: current dir only)")
            }
            MenuItem::ToggleView { showing_current_only: false } => {
                write!(f, "[~] Filter to current directory (currently: all workspaces)")
            }
            MenuItem::ToggleTimeframe { current } => {
                write!(f, "[#] Timeframe: {} (click to cycle)", current)
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

enum SubmenuResult {
    Resume(String),
    Back,
}

pub fn run_interactive_menu(
    conversations: &mut Vec<ConversationRecord>,
    cwd: &Path,
    db_path: &Path,
    initial_filter_current: bool,
    config: &mut AppConfig,
) -> Result<SelectionResult, Box<dyn std::error::Error>> {
    let mut filter_current = initial_filter_current;
    let mut timeframe_filter = TimeframeFilter::AllTime;

    loop {
        let filtered: Vec<ConversationRecord> = conversations
            .iter()
            .filter(|c| !filter_current || c.matches_workspace(cwd))
            .filter(|c| c.matches_timeframe(timeframe_filter))
            .cloned()
            .collect();

        let mut items = Vec::new();
        items.push(MenuItem::StartNew);
        items.push(MenuItem::ContinueLatest);
        items.push(MenuItem::SearchTranscripts);
        items.push(MenuItem::ViewStats);
        items.push(MenuItem::ToggleView {
            showing_current_only: filter_current,
        });
        items.push(MenuItem::ToggleTimeframe {
            current: timeframe_filter,
        });
        items.push(MenuItem::ChangeDefaultFilter);

        for conv in &filtered {
            items.push(MenuItem::Conversation(conv.clone()));
        }

        let prompt_text = if filter_current {
            format!(
                "Select a conversation (Dir: {}, timeframe: {}, matches: {}):",
                cwd.display(),
                timeframe_filter,
                filtered.len()
            )
        } else {
            format!(
                "Select a conversation (All workspaces, timeframe: {}, total: {}):",
                timeframe_filter,
                filtered.len()
            )
        };

        let selection = Select::new(&prompt_text, items)
            .with_page_size(15)
            .with_help_message("Navigate with arrows, type to filter, Enter to select, Esc to exit")
            .prompt();

        match selection {
            Ok(MenuItem::StartNew) => return Ok(SelectionResult::StartNew),
            Ok(MenuItem::ContinueLatest) => return Ok(SelectionResult::ContinueLatest),
            Ok(MenuItem::SearchTranscripts) => {
                match search::run_interactive_search(conversations)? {
                    search::SearchMenuResult::Resume(id) => return Ok(SelectionResult::ResumeConversation(id)),
                    search::SearchMenuResult::Back => continue,
                }
            }
            Ok(MenuItem::ViewStats) => {
                stats::print_stats(conversations);
                let _ = Text::new("Press Enter to continue...").prompt();
                continue;
            }
            Ok(MenuItem::ToggleView { .. }) => {
                filter_current = !filter_current;
                continue;
            }
            Ok(MenuItem::ToggleTimeframe { current }) => {
                timeframe_filter = current.next();
                continue;
            }
            Ok(MenuItem::ChangeDefaultFilter) => {
                prompt_change_settings(config)?;
                continue;
            }
            Ok(MenuItem::Conversation(conv)) => {
                match run_conversation_action_menu(&conv, db_path, conversations)? {
                    SubmenuResult::Resume(id) => return Ok(SelectionResult::ResumeConversation(id)),
                    SubmenuResult::Back => continue,
                }
            }
            Err(_) => return Ok(SelectionResult::Exit),
        }
    }
}

#[derive(Debug, Clone)]
enum ActionChoice {
    Resume,
    Preview,
    ViewPager,
    ExportMarkdown,
    Rename,
    Delete,
    Back,
}

impl fmt::Display for ActionChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionChoice::Resume => write!(f, "[>] Resume conversation in agy"),
            ActionChoice::Preview => write!(f, "[?] View preview & latest turns"),
            ActionChoice::ViewPager => write!(f, "[#] View full transcript in terminal pager"),
            ActionChoice::ExportMarkdown => write!(f, "[v] Export transcript to Markdown file"),
            ActionChoice::Rename => write!(f, "[e] Rename conversation title"),
            ActionChoice::Delete => write!(f, "[x] Delete conversation"),
            ActionChoice::Back => write!(f, "[<] Back to conversation list"),
        }
    }
}

fn run_conversation_action_menu(
    conv: &ConversationRecord,
    db_path: &Path,
    conversations: &mut Vec<ConversationRecord>,
) -> Result<SubmenuResult, Box<dyn std::error::Error>> {
    let mut current_conv = conv.clone();

    loop {
        println!();
        println!("================================================================================");
        println!("Session:   {}", current_conv.title);
        println!("ID:        {}", current_conv.id);
        println!("Workspace: {}", current_conv.primary_workspace_display());
        println!("Modified:  {} ({})", current_conv.last_modified_time, current_conv.relative_time());
        println!("Steps:     {}", current_conv.step_count);
        println!("================================================================================");

        let options = vec![
            ActionChoice::Resume,
            ActionChoice::Preview,
            ActionChoice::ViewPager,
            ActionChoice::ExportMarkdown,
            ActionChoice::Rename,
            ActionChoice::Delete,
            ActionChoice::Back,
        ];

        let action = Select::new("Choose an action:", options).prompt();

        match action {
            Ok(ActionChoice::Resume) => return Ok(SubmenuResult::Resume(current_conv.id.clone())),
            Ok(ActionChoice::Preview) => {
                println!("\n--- Summary Preview ---");
                if current_conv.preview.trim().is_empty() {
                    println!("(No preview text available)");
                } else {
                    println!("{}", current_conv.preview.trim());
                }

                if let Ok(turns) = transcript::load_transcript(&current_conv.id) {
                    let total = turns.len();
                    let start = total.saturating_sub(4);
                    let recent = &turns[start..];

                    println!("\n--- Recent Turns ({}-{} of {}) ---", start + 1, total, total);
                    for t in recent {
                        let content_preview = if t.content.len() > 200 {
                            format!("{}...", &t.content[..197])
                        } else {
                            t.content.clone()
                        };
                        println!("[{} ({}):] {}", t.role, t.timestamp, content_preview.replace('\n', " "));
                        if !t.tool_summary.is_empty() {
                            println!("  Tools called: {}", t.tool_summary.join(", "));
                        }
                    }
                }
                println!("--------------------------------------------------------------------------------");

                let post_preview = Select::new("Action:", vec!["Resume conversation", "Back to session menu"]).prompt();
                if let Ok("Resume conversation") = post_preview {
                    return Ok(SubmenuResult::Resume(current_conv.id.clone()));
                }
            }
            Ok(ActionChoice::ViewPager) => {
                if let Ok(turns) = transcript::load_transcript(&current_conv.id) {
                    let mut buffer = String::new();
                    buffer.push_str(&format!("=== {} ===\nID: {}\nWorkspace: {}\n\n", current_conv.title, current_conv.id, current_conv.primary_workspace_display()));
                    for t in &turns {
                        buffer.push_str(&format!("--- {} ({}) ---\n", t.role, t.timestamp));
                        buffer.push_str(&t.content);
                        buffer.push_str("\n\n");
                        if !t.tool_summary.is_empty() {
                            buffer.push_str(&format!("Tools: {}\n\n", t.tool_summary.join(", ")));
                        }
                    }
                    let _ = transcript::view_in_pager(&buffer);
                } else {
                    println!("Unable to locate or read transcript for this conversation.");
                }
            }
            Ok(ActionChoice::ExportMarkdown) => {
                if let Ok(turns) = transcript::load_transcript(&current_conv.id) {
                    let clean_title = current_conv.title
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                        .collect::<String>();
                    let default_path = format!("{clean_title}.md");
                    let dest_input = Text::new("Export file path:")
                        .with_default(&default_path)
                        .prompt();

                    if let Ok(dest_str) = dest_input {
                        let dest_path = Path::new(&dest_str);
                        match transcript::export_markdown(&current_conv, &turns, dest_path) {
                            Ok(_) => println!("Successfully exported transcript to {}", dest_path.display()),
                            Err(e) => eprintln!("Export failed: {e}"),
                        }
                    }
                } else {
                    println!("Unable to read transcript to export.");
                }
            }
            Ok(ActionChoice::Rename) => {
                let new_title_input = Text::new("Enter new conversation title:")
                    .with_default(&current_conv.title)
                    .prompt();

                if let Ok(new_title) = new_title_input {
                    let trimmed = new_title.trim();
                    if !trimmed.is_empty() && trimmed != current_conv.title {
                        match db::rename_conversation(db_path, &current_conv.id, trimmed) {
                            Ok(_) => {
                                current_conv.title = trimmed.to_string();
                                if let Some(rec) = conversations.iter_mut().find(|c| c.id == current_conv.id) {
                                    rec.title = trimmed.to_string();
                                }
                                println!("Updated title to: '{}'", trimmed);
                            }
                            Err(e) => eprintln!("Error renaming conversation: {e}"),
                        }
                    }
                }
            }
            Ok(ActionChoice::Delete) => {
                let confirm = Confirm::new("Are you sure you want to delete this conversation?")
                    .with_default(false)
                    .prompt();

                if let Ok(true) = confirm {
                    let purge = Confirm::new("Also purge conversation database and brain logs from disk?")
                        .with_default(true)
                        .prompt()
                        .unwrap_or(false);

                    match db::delete_conversation(db_path, &current_conv.id, purge) {
                        Ok(_) => {
                            conversations.retain(|c| c.id != current_conv.id);
                            println!("Conversation deleted successfully.");
                            return Ok(SubmenuResult::Back);
                        }
                        Err(e) => eprintln!("Error deleting conversation: {e}"),
                    }
                }
            }
            Ok(ActionChoice::Back) | Err(_) => return Ok(SubmenuResult::Back),
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
