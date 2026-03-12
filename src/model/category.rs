use std::fmt;
use std::path::Path;

/// Protected paths that should never be cleaned
pub const PROTECTED: &[&str] = &[
    "settings.json",
    "settings.local.json",
    "CLAUDE.md",
    "skills",
    "commands",
    "agents",
    "ide",
    "credentials.json",
    "statsig",
    ".credentials.json",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Projects,
    DebugLogs,
    FileHistory,
    Telemetry,
    ShellSnapshots,
    Plugins,
    Transcripts,
    Todos,
    Plans,
    UsageData,
    Tasks,
    PasteCache,
    ConfigBackups,
    History,
}

impl Category {
    pub const ALL: &'static [Category] = &[
        Category::Projects,
        Category::DebugLogs,
        Category::FileHistory,
        Category::Telemetry,
        Category::ShellSnapshots,
        Category::Plugins,
        Category::Transcripts,
        Category::Todos,
        Category::Plans,
        Category::UsageData,
        Category::Tasks,
        Category::PasteCache,
        Category::ConfigBackups,
        Category::History,
    ];

    /// Default recommended categories for cleaning (all selected)
    pub const DEFAULT_SELECTED: &'static [Category] = Self::ALL;

    /// The directory/file name under ~/.claude/
    pub fn dir_name(&self) -> &'static str {
        match self {
            Category::Projects => "projects",
            Category::DebugLogs => "debug",
            Category::FileHistory => "file-history",
            Category::Telemetry => "telemetry",
            Category::ShellSnapshots => "shell-snapshots",
            Category::Plugins => "plugins",
            Category::Transcripts => "transcripts",
            Category::Todos => "todos",
            Category::Plans => "plans",
            Category::UsageData => "usage-data",
            Category::Tasks => "tasks",
            Category::PasteCache => "paste-cache",
            Category::ConfigBackups => ".claude.json.backup",
            Category::History => "history.jsonl",
        }
    }

    /// Whether this category is a special file (not a directory)
    pub fn is_file(&self) -> bool {
        matches!(self, Category::History)
    }

    /// Whether this category uses prefix matching (backup files)
    pub fn is_prefix_match(&self) -> bool {
        matches!(self, Category::ConfigBackups)
    }

    /// Whether this category lives in the parent (home) directory rather than ~/.claude/
    pub fn is_home_dir(&self) -> bool {
        matches!(self, Category::ConfigBackups)
    }

    /// Whether this is a trim-only category (don't delete, just truncate)
    pub fn is_trim_only(&self) -> bool {
        matches!(self, Category::History)
    }

}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Projects => write!(f, "Projects"),
            Category::DebugLogs => write!(f, "Debug Logs"),
            Category::FileHistory => write!(f, "File History"),
            Category::Telemetry => write!(f, "Telemetry"),
            Category::ShellSnapshots => write!(f, "Shell Snapshots"),
            Category::Plugins => write!(f, "Plugins"),
            Category::Transcripts => write!(f, "Transcripts"),
            Category::Todos => write!(f, "Todos"),
            Category::Plans => write!(f, "Plans"),
            Category::UsageData => write!(f, "Usage Data"),
            Category::Tasks => write!(f, "Tasks"),
            Category::PasteCache => write!(f, "Paste Cache"),
            Category::ConfigBackups => write!(f, "Config Backups"),
            Category::History => write!(f, "History"),
        }
    }
}

/// Runtime info about a category after scanning
#[derive(Debug, Clone)]
pub struct CategoryInfo {
    pub category: Category,
    pub size: u64,
    pub file_count: usize,
    pub oldest_modified: Option<chrono::DateTime<chrono::Local>>,
    pub selected: bool,
    /// Per-file (modification_time, size) for expiry-based filtering.
    /// Only populated for directory-type categories.
    pub file_ages: Vec<(chrono::DateTime<chrono::Local>, u64)>,
}

impl CategoryInfo {
    pub fn new(category: Category) -> Self {
        Self {
            category,
            size: 0,
            file_count: 0,
            oldest_modified: None,
            selected: Category::DEFAULT_SELECTED.contains(&category),
            file_ages: Vec::new(),
        }
    }

    /// Size of files older than `expiry_days` (what would actually be cleaned).
    /// Projects category is cleaned by orphan status, not age — returns full size.
    /// Non-directory categories (prefix, file, trim) also return full size.
    pub fn expired_size(&self, expiry_days: u32) -> u64 {
        if self.file_ages.is_empty() || self.category == Category::Projects {
            return self.size;
        }
        let now = chrono::Local::now();
        let threshold = chrono::Duration::days(expiry_days as i64);
        self.file_ages
            .iter()
            .filter(|(dt, _)| now - *dt >= threshold)
            .map(|(_, sz)| sz)
            .sum()
    }

    /// Count of files older than `expiry_days`.
    /// Projects category returns full count (not time-based).
    pub fn expired_count(&self, expiry_days: u32) -> usize {
        if self.file_ages.is_empty() || self.category == Category::Projects {
            return self.file_count;
        }
        let now = chrono::Local::now();
        let threshold = chrono::Duration::days(expiry_days as i64);
        self.file_ages
            .iter()
            .filter(|(dt, _)| now - *dt >= threshold)
            .count()
    }

    pub fn size_percentage(&self, total: u64) -> f64 {
        if total == 0 {
            0.0
        } else {
            self.size as f64 / total as f64 * 100.0
        }
    }
}

/// Check if a path component is protected
pub fn is_protected(path: &Path, claude_dir: &Path) -> bool {
    if let Ok(rel) = path.strip_prefix(claude_dir) {
        if let Some(first) = rel.components().next() {
            let name = first.as_os_str().to_string_lossy();
            return PROTECTED.iter().any(|p| *p == name.as_ref());
        }
    }
    false
}
