# Claude Code Cleaner

A terminal UI (TUI) tool for interactively cleaning up the `~/.claude/` directory. Inspired by CleanMyMac, it provides a visual, guided workflow to reclaim disk space from accumulated Claude Code data.

After extended use, `~/.claude/` can grow to **1.8 GB+** across **10,000+ files** — old session data, debug logs, telemetry, orphaned project caches, and more. This tool helps you see exactly what's there, choose what to clean, and safely reclaim space.

## Screenshots

![Scan](screenshot/scan.jpeg)

![Config](screenshot/config.png)

![Preview](screenshot/preview.png)

## Features

- **5-screen guided workflow**: Scan → Select → Projects → Preview → Clean
- **Smart scanning**: Detects all cleanable data categories with per-file age tracking
- **Orphan project detection**: Identifies project caches where the original project path no longer exists
- **Expiry-based filtering**: Only clean files older than a configurable threshold (default: 30 days)
- **Surgical `~/.claude.json` cleanup**: Remove orphan project entries, session metrics, and stale caches from the config file without deleting it
- **3-segment preview bar**: Visualize what will be cleaned (green), what's matchable but unselected (yellow), and what's kept (red)
- **Progress bar**: Real-time progress tracking during cleaning with per-category breakdown
- **Persistent preferences**: Your selections and settings are saved and restored between sessions
- **Dry run mode**: Preview exactly what would be deleted without touching any files
- **Protected paths**: `settings.json`, `CLAUDE.md`, `skills/`, `commands/`, `agents/`, `ide/`, and `credentials.json` are never cleaned

## Cleanable Categories

| Category | Directory/File | Description |
|---|---|---|
| Projects | `projects/` | Per-project session data (typically the largest) |
| Debug Logs | `debug/` | Debug log files |
| File History | `file-history/` | File version snapshots |
| Telemetry | `telemetry/` | Telemetry data |
| Shell Snapshots | `shell-snapshots/` | Shell environment snapshots |
| Plugins | `plugins/` | Plugin cache |
| Transcripts | `transcripts/` | Old session transcripts |
| Todos | `todos/` | Todo items |
| Plans | `plans/` | Plan documents |
| Usage Data | `usage-data/` | Usage analytics |
| Tasks | `tasks/` | Task management data |
| Paste Cache | `paste-cache/` | Clipboard cache |
| Config Backups | `~/.claude.json.backup*` | Config backup files |
| History | `history.jsonl` | Command history (trimmed, not deleted) |
| Config JSON | `~/.claude.json` | Surgical field cleanup (orphans, metrics, caches) |

## Installation

### Quick install (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/GarrickZ2/claude-code-cleaner/master/install.sh | bash
```

Or install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/GarrickZ2/claude-code-cleaner/master/install.sh | bash -s v0.1.0
```

Custom install directory:

```bash
curl -fsSL https://raw.githubusercontent.com/GarrickZ2/claude-code-cleaner/master/install.sh | INSTALL_DIR=~/.local/bin bash
```

### Via cargo

```bash
cargo install claude-code-cleaner
```

### From GitHub Releases

Download the latest binary for your platform from [Releases](https://github.com/GarrickZ2/claude-code-cleaner/releases), extract, and place it in your `$PATH`.

### From source

```bash
git clone https://github.com/GarrickZ2/claude-code-cleaner.git
cd claude-code-cleaner
cargo build --release
```

The binary will be at `target/release/claude-code-cleaner`.

## Usage

```bash
claude-code-cleaner
```

The tool launches a full-screen TUI with 5 screens:

### 1. Dashboard (Scan)

Displays an overview of your `~/.claude/` directory — total size, file count, and per-category breakdown with usage bars. A scan runs automatically on startup.

### 2. Select

Choose which categories to clean with checkboxes. This screen has three sections:

- **Categories**: Toggle individual data categories on/off
- **Config JSON**: Select surgical cleanup options for `~/.claude.json` (orphan entries, metrics, caches)
- **Settings**: Adjust expiry threshold (days) and dry run mode

File counts and sizes update dynamically as you change the expiry threshold.

### 3. Projects

Browse all projects with their status:

- **ORPHAN** (red): Original project path no longer exists — entire cache will be deleted
- **active** (green): Project still exists — only files older than the expiry threshold are cleaned

Use `/` to search/filter, `a` to select all, `o` to select orphans only.

### 4. Preview

Review the full clean plan before executing. The 3-segment bar shows:

- **Green**: Space that will be cleaned this run
- **Yellow**: Space that matches rules but isn't selected
- **Red**: Space that doesn't match any cleanup rules

Press `Enter` to confirm and start cleaning.

### 5. Clean

Executes the clean plan with a real-time progress bar and per-category log. Shows total freed space and any errors on completion.

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `1`-`5` | Jump to screen |
| `Tab` / `Shift-Tab` | Next / previous screen |
| `j`/`k` or `Up`/`Down` | Navigate lists |
| `Space` | Toggle selection |
| `Enter` | Confirm / proceed |
| `Esc` | Go back |
| `a` | Select all |
| `n` | Unselect all |
| `d` | Reset to defaults |
| `o` | Select orphan projects only (Projects screen) |
| `/` | Search/filter (Projects screen) |
| `Left`/`Right` | Adjust settings values |
| `s` | Rescan (after cleaning) |
| `q` / `Ctrl-C` | Quit |
| `?` | Help overlay |

## Safety

- **Protected paths** are never modified or deleted
- **History** is trimmed (keeps last 500 lines), not deleted
- **Config JSON** is cleaned surgically with atomic writes (write to temp file, then rename)
- **Active projects** only have expired files removed; the project directory and recent files are preserved
- Failed deletions are collected and reported — they never interrupt the process
- Preferences are saved to `~/.claude/cleaner-preferences.json`

## Project Structure

```
src/
  main.rs             # Entry point, terminal setup, event loop, key handling
  app.rs              # App state machine, Screen enum, preferences
  event.rs            # Event loop: crossterm input + tick + async channels
  ui/
    mod.rs            # Render dispatcher
    dashboard.rs      # Scan overview screen
    categories.rs     # Unified Select screen (categories + config json + settings)
    projects.rs       # Project browser with orphan/active status
    preview.rs        # Clean plan preview with 3-segment bar
    cleaning.rs       # Cleaning progress with progress bar
    widgets/          # Shared UI helpers (format_size, format_age, etc.)
  scanner/
    mod.rs            # Async scan dispatcher
    categories.rs     # Category scanning + config JSON analysis
    projects.rs       # Project scanning + orphan detection
  cleaner/
    mod.rs            # Async clean executor with progress reporting
  model/
    mod.rs            # Shared data types
    category.rs       # Category enum, CategoryInfo, protected paths
    project.rs        # ProjectInfo with expiry-based filtering
    scan_result.rs    # ScanResult with reclaimable/matchable calculations
    config_json.rs    # ConfigJsonInfo for surgical JSON cleanup
    settings.rs       # CleanSettings + UserPreferences persistence
    clean_plan.rs     # Clean plan types
```

## Requirements

- Rust 1.70+
- A terminal with Unicode support

## License

MIT
