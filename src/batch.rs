use crate::db::{self, ConversationRecord};
use crate::transcript;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct BatchItem {
    conv: ConversationRecord,
}

impl fmt::Display for BatchItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_tag = format!("[{}]", self.conv.relative_time());
        let title_trimmed = if self.conv.title.len() > 36 {
            format!("{}...", &self.conv.title[..33])
        } else {
            self.conv.title.clone()
        };
        let ws = self.conv.primary_workspace_display();

        write!(f, "{:<10} {:<36} ({} steps) | {}", time_tag, title_trimmed, self.conv.step_count, ws)
    }
}

pub fn run_batch_menu(
    conversations: &mut Vec<ConversationRecord>,
    db_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("Select conversations using Spacebar to check/uncheck, Enter to confirm:");

    let items: Vec<BatchItem> = conversations
        .iter()
        .map(|c| BatchItem { conv: c.clone() })
        .collect();

    let selected = MultiSelect::new("Select conversations for batch action:", items)
        .with_page_size(15)
        .with_help_message("Space to toggle, Enter to confirm, Esc to cancel")
        .prompt();

    let Ok(chosen) = selected else {
        return Ok(());
    };

    if chosen.is_empty() {
        println!("No conversations selected.\n");
        return Ok(());
    }

    println!("\nSelected {} conversation(s).", chosen.len());

    let actions = vec![
        "[v] Bulk export selected to Markdown directory",
        "[x] Bulk delete selected conversations",
        "[<] Cancel",
    ];

    let action_choice = Select::new("Choose batch action:", actions).prompt();

    match action_choice {
        Ok("[v] Bulk export selected to Markdown directory") => {
            let default_dir = "./exported_conversations";
            let dir_input = Text::new("Destination directory:")
                .with_default(default_dir)
                .prompt()?;

            let dest_dir = PathBuf::from(dir_input.trim());
            fs::create_dir_all(&dest_dir)?;

            let mut exported_count = 0;
            for item in &chosen {
                if let Ok(turns) = transcript::load_transcript(&item.conv.id) {
                    let clean_title = item.conv.title
                        .chars()
                        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                        .collect::<String>();
                    let file_name = format!("{clean_title}.md");
                    let file_path = dest_dir.join(file_name);

                    if transcript::export_markdown(&item.conv, &turns, &file_path).is_ok() {
                        exported_count += 1;
                    }
                }
            }

            println!(
                "\nSuccessfully exported {} conversation(s) to '{}'.\n",
                exported_count,
                dest_dir.display()
            );
            let _ = Text::new("Press Enter to continue...").prompt();
        }
        Ok("[x] Bulk delete selected conversations") => {
            let confirm = Confirm::new(&format!(
                "Are you sure you want to permanently delete {} conversation(s)?",
                chosen.len()
            ))
            .with_default(false)
            .prompt()?;

            if confirm {
                let purge = Confirm::new("Also purge on-disk SQLite files and brain logs?")
                    .with_default(true)
                    .prompt()
                    .unwrap_or(false);

                let ids_to_delete: Vec<String> = chosen.iter().map(|item| item.conv.id.clone()).collect();
                let mut deleted_count = 0;

                for id in &ids_to_delete {
                    if db::delete_conversation(db_path, id, purge).is_ok() {
                        deleted_count += 1;
                    }
                }

                conversations.retain(|c| !ids_to_delete.contains(&c.id));
                println!("\nDeleted {} conversation(s) successfully.\n", deleted_count);
                let _ = Text::new("Press Enter to continue...").prompt();
            }
        }
        _ => {}
    }

    Ok(())
}
