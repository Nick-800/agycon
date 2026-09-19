use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeframeFilter {
    AllTime,
    Last24Hours,
    Last7Days,
    Last30Days,
}

impl fmt::Display for TimeframeFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeframeFilter::AllTime => write!(f, "All Time"),
            TimeframeFilter::Last24Hours => write!(f, "Last 24 Hours"),
            TimeframeFilter::Last7Days => write!(f, "Last 7 Days"),
            TimeframeFilter::Last30Days => write!(f, "Last 30 Days"),
        }
    }
}

impl TimeframeFilter {
    pub fn next(&self) -> Self {
        match self {
            TimeframeFilter::AllTime => TimeframeFilter::Last24Hours,
            TimeframeFilter::Last24Hours => TimeframeFilter::Last7Days,
            TimeframeFilter::Last7Days => TimeframeFilter::Last30Days,
            TimeframeFilter::Last30Days => TimeframeFilter::AllTime,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConversationRecord {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub step_count: i64,
    pub last_modified_time: String,
    pub workspace_uris: Vec<String>,
    pub workspace_paths: Vec<PathBuf>,
}

impl ConversationRecord {
    pub fn matches_workspace(&self, cwd: &Path) -> bool {
        let Ok(canonical_cwd) = cwd.canonicalize().or_else(|_| Ok::<_, std::io::Error>(cwd.to_path_buf())) else {
            return false;
        };

        let home_dir = dirs::home_dir().and_then(|h| h.canonicalize().ok());

        for ws in &self.workspace_paths {
            let ws_canonical = ws.canonicalize().unwrap_or_else(|_| ws.clone());
            if ws_canonical == canonical_cwd {
                return true;
            }

            // If the workspace is the home directory itself, only match if cwd is also home
            if let Some(ref home) = home_dir {
                if &ws_canonical == home {
                    continue;
                }
            }

            // Check if current directory is inside the project workspace or vice-versa
            if canonical_cwd.starts_with(&ws_canonical) || ws_canonical.starts_with(&canonical_cwd) {
                return true;
            }
        }
        false
    }

    pub fn matches_timeframe(&self, filter: TimeframeFilter) -> bool {
        if filter == TimeframeFilter::AllTime {
            return true;
        }

        let ts_cleaned = self.last_modified_time.trim().replace(' ', "T");
        let parsed = DateTime::parse_from_rfc3339(&ts_cleaned)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                DateTime::parse_from_str(&self.last_modified_time, "%Y-%m-%d %H:%M:%S%.f%:z")
                    .map(|dt| dt.with_timezone(&Utc))
            });

        if let Ok(dt) = parsed {
            let duration = Utc::now().signed_duration_since(dt);
            match filter {
                TimeframeFilter::AllTime => true,
                TimeframeFilter::Last24Hours => duration.num_hours() <= 24 && duration.num_seconds() >= 0,
                TimeframeFilter::Last7Days => duration.num_days() <= 7 && duration.num_seconds() >= 0,
                TimeframeFilter::Last30Days => duration.num_days() <= 30 && duration.num_seconds() >= 0,
            }
        } else {
            true
        }
    }

    pub fn primary_workspace_display(&self) -> String {
        if let Some(ws) = self.workspace_paths.first() {
            let path_str = ws.to_string_lossy();
            if let Some(home) = dirs::home_dir() {
                let home_str = home.to_string_lossy();
                if path_str.starts_with(&*home_str) {
                    return format!("~{}", &path_str[home_str.len()..]);
                }
            }
            path_str.to_string()
        } else {
            "no workspace".to_string()
        }
    }

    pub fn relative_time(&self) -> String {
        let ts_cleaned = self.last_modified_time.trim().replace(' ', "T");
        let parsed = DateTime::parse_from_rfc3339(&ts_cleaned)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                DateTime::parse_from_str(&self.last_modified_time, "%Y-%m-%d %H:%M:%S%.f%:z")
                    .map(|dt| dt.with_timezone(&Utc))
            });

        if let Ok(dt) = parsed {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt);

            if duration.num_seconds() < 60 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else if duration.num_days() == 1 {
                "yesterday".to_string()
            } else if duration.num_days() < 30 {
                format!("{}d ago", duration.num_days())
            } else {
                format!("{}mo ago", duration.num_days() / 30)
            }
        } else {
            // Fallback: take YYYY-MM-DD
            if self.last_modified_time.len() >= 10 {
                self.last_modified_time[0..10].to_string()
            } else {
                self.last_modified_time.clone()
            }
        }
    }
}

pub fn locate_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine user home directory".to_string())?;
    let path = home.join(".gemini/antigravity-cli/conversation_summaries.db");
    if !path.exists() {
        return Err(format!(
            "Conversation summaries database not found at {}",
            path.display()
        ));
    }
    Ok(path)
}

pub fn load_conversations(db_path: &Path) -> Result<Vec<ConversationRecord>, Box<dyn std::error::Error>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut stmt = conn.prepare(
        "SELECT conversation_id, title, preview, step_count, last_modified_time, workspace_uris
         FROM conversation_summaries
         WHERE step_count > 0 AND last_modified_time > '2000-01-01'
         ORDER BY last_modified_time DESC",
    )?;

    let records_iter = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let preview: String = row.get(2)?;
        let step_count: i64 = row.get(3)?;
        let last_modified_time: String = row.get(4)?;
        let workspace_uris_raw: String = row.get(5)?;

        Ok((id, title, preview, step_count, last_modified_time, workspace_uris_raw))
    })?;

    let mut records = Vec::new();
    for row in records_iter {
        let (id, title, preview, step_count, last_modified_time, uris_raw) = row?;
        let workspace_uris: Vec<String> = serde_json::from_str(&uris_raw).unwrap_or_default();
        let mut workspace_paths = Vec::new();

        for uri in &workspace_uris {
            let path_str = uri.strip_prefix("file://").unwrap_or(uri);
            workspace_paths.push(PathBuf::from(path_str));
        }

        let display_title = if title.trim().is_empty() {
            if preview.trim().is_empty() {
                "Untitled conversation".to_string()
            } else {
                let first_line = preview.lines().next().unwrap_or("Untitled conversation");
                if first_line.len() > 60 {
                    format!("{}...", &first_line[..57])
                } else {
                    first_line.to_string()
                }
            }
        } else {
            title
        };

        records.push(ConversationRecord {
            id,
            title: display_title,
            preview,
            step_count,
            last_modified_time,
            workspace_uris,
            workspace_paths,
        });
    }

    Ok(records)
}

pub fn rename_conversation(
    db_path: &Path,
    conversation_id: &str,
    new_title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.execute(
        "UPDATE conversation_summaries SET title = ?1 WHERE conversation_id = ?2",
        [new_title, conversation_id],
    )?;
    Ok(())
}

pub fn delete_conversation(
    db_path: &Path,
    conversation_id: &str,
    purge_files: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conn.execute(
        "DELETE FROM conversation_summaries WHERE conversation_id = ?1",
        [conversation_id],
    )?;

    if purge_files {
        if let Some(home) = dirs::home_dir() {
            let conv_db = home.join(format!(".gemini/antigravity-cli/conversations/{conversation_id}.db"));
            let conv_shm = home.join(format!(".gemini/antigravity-cli/conversations/{conversation_id}.db-shm"));
            let conv_wal = home.join(format!(".gemini/antigravity-cli/conversations/{conversation_id}.db-wal"));
            let brain_dir = home.join(format!(".gemini/antigravity-cli/brain/{conversation_id}"));

            let _ = std::fs::remove_file(conv_db);
            let _ = std::fs::remove_file(conv_shm);
            let _ = std::fs::remove_file(conv_wal);
            let _ = std::fs::remove_dir_all(brain_dir);
        }
    }
    Ok(())
}
