# CONTEXT.md

## Domain Vocabulary

- **agy**: Google Antigravity CLI binary (`/home/nick/.gemini/antigravity-cli/bin/agy`).
- **agycon**: Standalone Rust CLI launcher and session selector for Antigravity conversations.
- **Conversation Summaries DB**: SQLite database located at `~/.gemini/antigravity-cli/conversation_summaries.db`.
- **Trajectory DB**: Individual SQLite databases per conversation at `~/.gemini/antigravity-cli/conversations/<id>.db`.
- **Brain**: Per-conversation workspace containing transcripts, scratch scripts, and artifacts at `~/.gemini/antigravity-cli/brain/<id>/`.
- **Process Replacement**: Utilizing Unix `exec` (`std::os::unix::process::CommandExt::exec`) to replace the `agycon` process with `agy`.
