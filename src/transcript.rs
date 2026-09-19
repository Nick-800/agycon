use crate::db::ConversationRecord;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RawTranscriptEntry {
    pub step_index: Option<i64>,
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub created_at: Option<String>,
    pub content: Option<String>,
    pub thinking: Option<String>,
    pub tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct RawToolCall {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranscriptTurn {
    pub role: String,
    pub timestamp: String,
    pub content: String,
    pub tool_summary: Vec<String>,
}

pub fn locate_transcript_path(conversation_id: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let base = home.join(format!(
        ".gemini/antigravity-cli/brain/{conversation_id}/.system_generated/logs"
    ));

    let full = base.join("transcript_full.jsonl");
    if full.exists() {
        return Some(full);
    }

    let compact = base.join("transcript.jsonl");
    if compact.exists() {
        return Some(compact);
    }

    None
}

pub fn load_transcript(conversation_id: &str) -> Result<Vec<TranscriptTurn>, Box<dyn std::error::Error>> {
    let Some(path) = locate_transcript_path(conversation_id) else {
        return Err("No transcript found for this conversation".into());
    };

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut turns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(entry) = serde_json::from_str::<RawTranscriptEntry>(&line) {
            let role = match entry.source.as_deref() {
                Some("USER_EXPLICIT") | Some("USER") => "User".to_string(),
                Some("MODEL") => "Assistant".to_string(),
                _ => continue,
            };

            let timestamp = entry.created_at.unwrap_or_default();
            let mut content = entry.content.unwrap_or_default();

            // Clean up prompt wrapping if present
            if role == "User" {
                if let Some(start) = content.find("<USER_REQUEST>") {
                    if let Some(end) = content.find("</USER_REQUEST>") {
                        content = content[start + 14..end].trim().to_string();
                    }
                }
            }

            let mut tool_summary = Vec::new();
            if let Some(calls) = entry.tool_calls {
                for c in calls {
                    if let Some(name) = c.name {
                        tool_summary.push(name);
                    }
                }
            }

            if !content.is_empty() || !tool_summary.is_empty() {
                turns.push(TranscriptTurn {
                    role,
                    timestamp,
                    content,
                    tool_summary,
                });
            }
        }
    }

    Ok(turns)
}

pub fn export_markdown(
    conv: &ConversationRecord,
    turns: &[TranscriptTurn],
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", conv.title));
    md.push_str(&format!("- **Conversation ID:** `{}`\n", conv.id));
    md.push_str(&format!("- **Last Modified:** {}\n", conv.last_modified_time));
    md.push_str(&format!("- **Workspace:** {}\n", conv.primary_workspace_display()));
    md.push_str(&format!("- **Total Turns:** {}\n\n", turns.len()));
    md.push_str("---\n\n");

    for turn in turns {
        md.push_str(&format!("### {} ({})\n\n", turn.role, turn.timestamp));
        if !turn.content.is_empty() {
            md.push_str(&turn.content);
            md.push_str("\n\n");
        }
        if !turn.tool_summary.is_empty() {
            md.push_str(&format!(
                "> *Tool calls:* `{}`\n\n",
                turn.tool_summary.join("`, `")
            ));
        }
    }

    fs::write(dest, md)?;
    Ok(())
}

pub fn view_in_pager(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less -R".to_string());
    let parts: Vec<&str> = pager.split_whitespace().collect();
    let cmd = parts.first().unwrap_or(&"less");
    let args = &parts[1..];

    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .or_else(|_| Command::new("less").arg("-R").stdin(Stdio::piped()).spawn())
        .or_else(|_| Command::new("cat").stdin(Stdio::piped()).spawn())?;

    if let Some(ref mut stdin) = child.stdin {
        let _ = stdin.write_all(content.as_bytes());
    }

    let _ = child.wait();
    Ok(())
}
