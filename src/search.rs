use crate::db::ConversationRecord;
use crate::transcript;
use inquire::{Select, Text};
use std::fmt;

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub conv: ConversationRecord,
    pub turn_role: String,
    pub snippet: String,
}

impl fmt::Display for SearchHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time_tag = format!("[{}]", self.conv.relative_time());
        let title = if self.conv.title.len() > 30 {
            format!("{}...", &self.conv.title[..27])
        } else {
            self.conv.title.clone()
        };

        let snippet_clean = self.snippet.replace('\n', " ");
        let snippet_trimmed = if snippet_clean.len() > 40 {
            format!("{}...", &snippet_clean[..37])
        } else {
            snippet_clean
        };

        write!(f, "{:<10} {:<30} | {}: \"{}\"", time_tag, title, self.turn_role, snippet_trimmed)
    }
}

pub fn search_all(query: &str, conversations: &[ConversationRecord]) -> Vec<SearchHit> {
    let q_lower = query.to_lowercase();
    let mut hits = Vec::new();

    for conv in conversations {
        if let Ok(turns) = transcript::load_transcript(&conv.id) {
            for turn in turns {
                if let Some(pos) = turn.content.to_lowercase().find(&q_lower) {
                    let start = pos.saturating_sub(25);
                    let end = (pos + query.len() + 35).min(turn.content.len());
                    let snippet = turn.content[start..end].trim().to_string();

                    hits.push(SearchHit {
                        conv: conv.clone(),
                        turn_role: turn.role,
                        snippet,
                    });

                    // Limit hits per conversation to avoid flooding
                    if hits.len() >= 100 {
                        return hits;
                    }
                }
            }
        }
    }

    hits
}

pub enum SearchMenuResult {
    Resume(String),
    Back,
}

pub fn run_interactive_search(conversations: &[ConversationRecord]) -> Result<SearchMenuResult, Box<dyn std::error::Error>> {
    let query_input = Text::new("Search transcripts for keyword:")
        .prompt();

    let Ok(query) = query_input else {
        return Ok(SearchMenuResult::Back);
    };

    let query_trimmed = query.trim();
    if query_trimmed.is_empty() {
        return Ok(SearchMenuResult::Back);
    }

    println!("\nSearching transcripts for '{}'...", query_trimmed);
    let hits = search_all(query_trimmed, conversations);

    if hits.is_empty() {
        println!("No matching transcript turns found for '{}'.\n", query_trimmed);
        let _ = Text::new("Press Enter to continue...").prompt();
        return Ok(SearchMenuResult::Back);
    }

    println!("Found {} matching turn(s):\n", hits.len());

    let mut options: Vec<String> = hits.iter().map(|h| h.to_string()).collect();
    options.push("[<] Back to conversation list".to_string());

    let choice = Select::new("Select a search result:", options)
        .with_page_size(15)
        .prompt();

    match choice {
        Ok(ref sel) if sel == "[<] Back to conversation list" => Ok(SearchMenuResult::Back),
        Ok(sel) => {
            if let Some(idx) = hits.iter().position(|h| h.to_string() == sel) {
                let hit = &hits[idx];
                println!();
                println!("Match Details:");
                println!("  Session:   {}", hit.conv.title);
                println!("  Workspace: {}", hit.conv.primary_workspace_display());
                println!("  Role:      {}", hit.turn_role);
                println!("  Snippet:   \"{}\"", hit.snippet);
                println!();

                let action = Select::new("Action:", vec!["[>] Resume this conversation", "[<] Back"]).prompt();
                if let Ok("[>] Resume this conversation") = action {
                    return Ok(SearchMenuResult::Resume(hit.conv.id.clone()));
                }
            }
            Ok(SearchMenuResult::Back)
        }
        Err(_) => Ok(SearchMenuResult::Back),
    }
}
