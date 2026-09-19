# AGENTS.md

## Repository Overview
`agycon` is a Rust CLI tool that queries Antigravity conversation summaries from SQLite, presents an interactive fuzzy-search picker, and executes `agy` via process replacement.

## Conventions & Rules
- All global rules from `~/.gemini/GEMINI.md` strictly apply.
- No emojis in any output, code, comments, documentation, commit messages, or responses unless explicitly requested.
- Preserves zero code comments unless explicitly asked.
- Build/verification: `cargo check` and `cargo build --release`.
- Binary installation path: `~/.local/bin/agycon`.
