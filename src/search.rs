use crate::db::{self, ConversationRecord};
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
        let title = db::truncate_with_ellipsis(&self.conv.title, 30);

        let snippet_clean = self.snippet.replace('\n', " ");
        let snippet_trimmed = db::truncate_with_ellipsis(&snippet_clean, 40);

        write!(f, "{:<10} {:<30} | {}: \"{}\"", time_tag, title, self.turn_role, snippet_trimmed)
    }
}

pub fn search_all(query: &str, conversations: &[ConversationRecord]) -> Vec<SearchHit> {
    let q_lower = query.to_lowercase();
    let mut hits = Vec::new();

    for conv in conversations {
        if let Ok(turns) = transcript::load_transcript(&conv.id) {
            for turn in turns {
                let content_chars: Vec<char> = turn.content.chars().collect();
                let content_lower: String = content_chars.iter().collect::<String>().to_lowercase();
                if let Some(byte_pos) = content_lower.find(&q_lower) {
                    let match_char_idx = content_lower[..byte_pos].chars().count();
                    let query_char_len = q_lower.chars().count();
                    let start_char = match_char_idx.saturating_sub(25);
                    let end_char = (match_char_idx + query_char_len + 35).min(content_chars.len());
                    let snippet: String = content_chars[start_char..end_char].iter().collect();
                    let snippet = snippet.trim().to_string();

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
    Resume {
        id: String,
        dangerously_skip_permissions: bool,
    },
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

                let action = Select::new(
                    "Action:",
                    vec![
                        "[>] Resume this conversation",
                        "[!] Resume with --dangerously-skip-permissions",
                        "[<] Back",
                    ],
                )
                .prompt();
                if let Ok("[>] Resume this conversation") = action {
                    return Ok(SearchMenuResult::Resume {
                        id: hit.conv.id.clone(),
                        dangerously_skip_permissions: false,
                    });
                } else if let Ok("[!] Resume with --dangerously-skip-permissions") = action {
                    return Ok(SearchMenuResult::Resume {
                        id: hit.conv.id.clone(),
                        dangerously_skip_permissions: true,
                    });
                }
            }
            Ok(SearchMenuResult::Back)
        }
        Err(_) => Ok(SearchMenuResult::Back),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_hit_display_multibyte() {
        let conv = ConversationRecord {
            id: "test-id".to_string(),
            title: "تطوير أداة جديدة لإدارة المحادثات والمشاريع البرمجية".to_string(),
            preview: "preview".to_string(),
            step_count: 5,
            last_modified_time: "2026-09-19 10:00:00".to_string(),
            workspace_uris: vec![],
            workspace_paths: vec![],
        };
        let hit = SearchHit {
            conv,
            turn_role: "user".to_string(),
            snippet: "هذا النص يحتوي على أحرف عربية ورموز خاصة تفوق الطول المحدد بكثير".to_string(),
        };
        let formatted = format!("{}", hit);
        assert!(!formatted.is_empty());
    }
}
