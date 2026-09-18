# agycon

Fast, interactive CLI launcher and manager for Google Antigravity (`agy`) conversations, built in Rust.

## Features

- **Interactive Conversation Picker**: Fuzzy search across past Antigravity conversations using keyboard navigation and live search.
- **Direct Actions**:
  - `[+] Start a new conversation` (`agy`)
  - `[>] Continue most recent conversation` (`agy -c`)
  - Select any past conversation to resume (`agy --conversation <id>`)
- **Smart Workspace Filtering**:
  - Filters strictly to the current working directory by default.
  - Quick in-menu toggle or `--all` flag to browse conversations across all projects.
- **Configurable Default Behavior**:
  - Persistently toggle whether to filter to the current directory by default or show all workspaces.
  - Stored in `~/.config/agycon/config.json`.
- **Zero Overhead Process Replacement**:
  - Uses Unix `exec` to replace the `agycon` process with `agy`, ensuring seamless terminal control, signal handling, and zero memory overhead.
- **Embedded SQLite**:
  - Built with bundled SQLite, requiring no external `sqlite3` CLI or system libraries.

## Installation

Build and install the binary:

```bash
cargo build --release
cp target/release/agycon ~/.local/bin/
```

Or via Cargo:

```bash
cargo install --path .
```

## Usage

### Interactive Menu
Launch the interactive selector in your current workspace:

```bash
agycon
```

View all conversations across all projects:

```bash
agycon --all
```

Force filter strictly to current directory:

```bash
agycon --current
```

### Direct Actions
Start a new conversation immediately:

```bash
agycon --new
```

Continue the latest conversation:

```bash
agycon --continue
```

Print matching conversations non-interactively:

```bash
agycon --list
agycon --all --list
```

### Settings & Configuration
View current configuration:

```bash
agycon config
```

Change default filter mode:

```bash
# Filter to current workspace by default
agycon config --set current

# Show all workspaces by default
agycon config --set all
```

You can also change the default filter mode directly inside the interactive menu under `[*] Configure default filter setting`.
