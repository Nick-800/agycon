use crate::db::{ConversationRecord, TimeframeFilter};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct SystemStats {
    pub total_conversations: usize,
    pub total_steps: i64,
    pub avg_steps: f64,
    pub max_steps_title: String,
    pub max_steps: i64,
    pub last_24h_count: usize,
    pub last_7d_count: usize,
    pub last_30d_count: usize,
    pub top_workspaces: Vec<(String, usize)>,
    pub db_size_bytes: u64,
    pub convs_size_bytes: u64,
    pub brain_size_bytes: u64,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn calculate_stats(conversations: &[ConversationRecord]) -> SystemStats {
    let total_conversations = conversations.len();
    let total_steps: i64 = conversations.iter().map(|c| c.step_count).sum();
    let avg_steps = if total_conversations > 0 {
        total_steps as f64 / total_conversations as f64
    } else {
        0.0
    };

    let max_conv = conversations.iter().max_by_key(|c| c.step_count);
    let (max_steps_title, max_steps) = match max_conv {
        Some(c) => (c.title.clone(), c.step_count),
        None => ("None".to_string(), 0),
    };

    let last_24h_count = conversations
        .iter()
        .filter(|c| c.matches_timeframe(TimeframeFilter::Last24Hours))
        .count();

    let last_7d_count = conversations
        .iter()
        .filter(|c| c.matches_timeframe(TimeframeFilter::Last7Days))
        .count();

    let last_30d_count = conversations
        .iter()
        .filter(|c| c.matches_timeframe(TimeframeFilter::Last30Days))
        .count();

    let mut workspace_counts: HashMap<String, usize> = HashMap::new();
    for c in conversations {
        let ws = c.primary_workspace_display();
        *workspace_counts.entry(ws).or_insert(0) += 1;
    }

    let mut top_workspaces: Vec<(String, usize)> = workspace_counts.into_iter().collect();
    top_workspaces.sort_by(|a, b| b.1.cmp(&a.1));
    top_workspaces.truncate(5);

    let mut db_size_bytes = 0;
    let mut convs_size_bytes = 0;
    let mut brain_size_bytes = 0;

    if let Some(home) = dirs::home_dir() {
        let summaries = home.join(".gemini/antigravity-cli/conversation_summaries.db");
        if let Ok(meta) = fs::metadata(&summaries) {
            db_size_bytes += meta.len();
        }

        let convs_dir = home.join(".gemini/antigravity-cli/conversations");
        convs_size_bytes = dir_size(&convs_dir);

        let brain_dir = home.join(".gemini/antigravity-cli/brain");
        brain_size_bytes = dir_size(&brain_dir);
    }

    SystemStats {
        total_conversations,
        total_steps,
        avg_steps,
        max_steps_title,
        max_steps,
        last_24h_count,
        last_7d_count,
        last_30d_count,
        top_workspaces,
        db_size_bytes,
        convs_size_bytes,
        brain_size_bytes,
    }
}

pub fn print_stats(conversations: &[ConversationRecord]) {
    let stats = calculate_stats(conversations);

    println!();
    println!("================================================================================");
    println!("                         AGYCON ANALYTICS & STATS                               ");
    println!("================================================================================");
    println!();

    println!("Activity Overview:");
    println!("  Total Conversations:    {}", stats.total_conversations);
    println!("  Total Turn Steps:       {}", stats.total_steps);
    println!("  Average Steps / Conv:   {:.1}", stats.avg_steps);
    println!("  Longest Session:        {} ({} steps)", stats.max_steps_title, stats.max_steps);
    println!();

    println!("Time Breakdown:");
    println!("  Active in last 24h:     {}", stats.last_24h_count);
    println!("  Active in last 7d:      {}", stats.last_7d_count);
    println!("  Active in last 30d:     {}", stats.last_30d_count);
    println!();

    println!("Top Workspaces:");
    for (idx, (ws, count)) in stats.top_workspaces.iter().enumerate() {
        let bar_len = (count * 20).checked_div(stats.total_conversations).unwrap_or(0);
        let bar = "█".repeat(bar_len.max(1));
        println!("  {}. {:<40} {:>3} sessions  {}", idx + 1, ws, count, bar);
    }
    println!();

    println!("Storage Footprint:");
    println!("  Master Index (SQLite):  {}", format_bytes(stats.db_size_bytes));
    println!("  Trajectories Database:  {}", format_bytes(stats.convs_size_bytes));
    println!("  Brain Logs & Artifacts: {}", format_bytes(stats.brain_size_bytes));
    println!("  Total Storage Used:     {}", format_bytes(stats.db_size_bytes + stats.convs_size_bytes + stats.brain_size_bytes));
    println!();
    println!("================================================================================");
    println!();
}
