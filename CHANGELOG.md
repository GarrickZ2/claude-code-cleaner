# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-11

### Added

- **5-screen TUI workflow**: Dashboard → Select → Projects → Preview → Clean
- **Dashboard**: Auto-scan on startup with per-category size/file breakdown and usage bars
- **Select screen**: Unified 3-section interface combining category selection, config JSON cleanup options, and settings (expiry threshold, dry run)
- **Category scanning**: 14 cleanable categories — Projects, Debug Logs, File History, Telemetry, Shell Snapshots, Plugins, Transcripts, Todos, Plans, Usage Data, Tasks, Paste Cache, Config Backups, History
- **Project browser**: Browse all projects with orphan (ORPHAN) vs active status, search/filter, bulk select
- **Orphan project detection**: Identifies project caches where the original path no longer exists
- **Expiry-based filtering**: Per-file age tracking with configurable threshold; file counts and sizes update dynamically as threshold changes
- **Active project cleaning**: For non-orphan projects, only files older than the expiry threshold are deleted (not the entire directory)
- **Surgical `~/.claude.json` cleanup**: Remove orphan project entries, session metrics, and stale cache keys without deleting the file; uses atomic writes
- **3-segment preview bar**: Green (will clean), yellow (matchable but unselected), red (not matched/kept)
- **Progress bar**: Real-time progress tracking during cleaning with percentage, freed/expected sizes, and per-category log
- **Persistent preferences**: Settings and selections saved to `~/.claude/cleaner-preferences.json` after each clean, auto-loaded on next startup
- **Dry run mode**: Simulate cleaning without deleting any files
- **Protected paths**: `settings.json`, `CLAUDE.md`, `skills/`, `commands/`, `agents/`, `ide/`, `credentials.json` are never touched
- **History trimming**: `history.jsonl` is trimmed to last 500 lines (not deleted)
- **Config backup cleanup**: Detects `~/.claude.json.backup*` files in home directory
- **Event batching**: Coalesces rapid input events to prevent UI freeze during fast scrolling
- **Keyboard navigation**: Full keyboard-driven interface with vi-style keys, number keys for screen jumping, search/filter in project list
- **Help overlay**: Press `?` for context-sensitive help
- **Confirm dialog**: Safety confirmation before executing clean
