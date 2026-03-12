use crate::model::*;
use crate::cleaner::CleanMessage;
use crate::scanner::ScanMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Categories,
    Projects,
    Preview,
    Cleaning,
}

impl Screen {
    pub const ALL: &'static [Screen] = &[
        Screen::Dashboard,
        Screen::Categories,
        Screen::Projects,
        Screen::Preview,
        Screen::Cleaning,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Screen::Dashboard => "Scan",
            Screen::Categories => "Select",
            Screen::Projects => "Projects",
            Screen::Preview => "Preview",
            Screen::Cleaning => "Clean",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Screen::Dashboard => 0,
            Screen::Categories => 1,
            Screen::Projects => 2,
            Screen::Preview => 3,
            Screen::Cleaning => 4,
        }
    }

    pub fn from_index(i: usize) -> Option<Screen> {
        Screen::ALL.get(i).copied()
    }
}

/// Which section the unified cursor is in on the Select screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectSection {
    Categories,
    ConfigJson,
    Settings,
}

pub struct App {
    pub screen: Screen,
    pub running: bool,
    pub scan_result: Option<ScanResult>,
    pub settings: CleanSettings,
    pub preferences: UserPreferences,
    pub show_help: bool,
    pub show_confirm: bool,

    // Select screen unified cursor
    pub category_cursor: usize,

    // Project screen state
    pub project_cursor: usize,
    pub project_scroll: usize,
    pub project_filter: String,
    pub project_filtering: bool,

    // Scanning state
    pub scanning: bool,
    pub scan_progress: Option<String>,

    // Cleaning state
    pub cleaning: bool,
    pub clean_messages: Vec<CleanMessage>,
    pub clean_complete: bool,
    pub clean_total_freed: u64,
    pub clean_total_errors: Vec<String>,
    pub clean_expected_size: u64,
    pub clean_freed_so_far: u64,
}

impl App {
    pub fn new(claude_dir: &std::path::Path) -> Self {
        let prefs = UserPreferences::load(claude_dir);
        let settings = prefs.settings.clone();
        Self {
            screen: Screen::Dashboard,
            running: true,
            scan_result: None,
            settings,
            preferences: prefs,
            show_help: false,
            show_confirm: false,
            category_cursor: 0,
            project_cursor: 0,
            project_scroll: 0,
            project_filter: String::new(),
            project_filtering: false,
            scanning: false,
            scan_progress: None,
            cleaning: false,
            clean_messages: Vec::new(),
            clean_complete: false,
            clean_total_freed: 0,
            clean_total_errors: Vec::new(),
            clean_expected_size: 0,
            clean_freed_so_far: 0,
        }
    }

    pub fn next_screen(&mut self) {
        let idx = self.screen.index();
        if idx < Screen::ALL.len() - 1 {
            self.screen = Screen::from_index(idx + 1).unwrap();
        }
    }

    pub fn prev_screen(&mut self) {
        let idx = self.screen.index();
        if idx > 0 {
            self.screen = Screen::from_index(idx - 1).unwrap();
        }
    }

    pub fn handle_scan_message(&mut self, msg: ScanMessage) {
        match msg {
            ScanMessage::Progress { category, scanned } => {
                self.scan_progress = Some(format!("Scanning {}: {} files...", category, scanned));
            }
            ScanMessage::Complete(mut result) => {
                // Apply saved preferences to scan result
                self.apply_preferences(&mut result);
                self.scan_result = Some(result);
                self.scanning = false;
                self.scan_progress = None;
            }
            ScanMessage::Error(e) => {
                self.scanning = false;
                self.scan_progress = Some(format!("Error: {}", e));
            }
        }
    }

    pub fn handle_clean_message(&mut self, msg: CleanMessage) {
        match &msg {
            CleanMessage::CategoryDone { freed, .. } => {
                self.clean_freed_so_far += freed;
            }
            CleanMessage::Complete { total_freed, total_errors } => {
                self.clean_complete = true;
                self.cleaning = false;
                self.clean_total_freed = *total_freed;
                self.clean_total_errors = total_errors.clone();
            }
            _ => {}
        }
        self.clean_messages.push(msg);
    }

    /// Returns indices of projects, optionally filtered by search string.
    /// Shows all projects (orphans + active with expired data).
    pub fn filtered_projects(&self) -> Vec<usize> {
        if let Some(ref result) = self.scan_result {
            result
                .projects
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    if self.project_filter.is_empty() {
                        true
                    } else {
                        let path_str = p.original_path.to_string_lossy().to_lowercase();
                        path_str.contains(&self.project_filter.to_lowercase())
                    }
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            Vec::new()
        }
    }

    // ── Select screen cursor helpers ──
    // Layout: [categories...] [config_json x3] [settings x2]
    // category_cursor is a unified index across all three sections.

    fn cat_count(&self) -> usize {
        self.scan_result.as_ref().map(|r| r.categories.len()).unwrap_or(0)
    }

    /// Total navigable items on Select screen
    pub fn select_total_items(&self) -> usize {
        self.cat_count() + ConfigJsonInfo::ITEM_COUNT + CleanSettings::FIELD_COUNT
    }

    /// Which section the cursor is in and the local index within that section
    pub fn select_cursor_section(&self) -> (SelectSection, usize) {
        let cc = self.cat_count();
        if self.category_cursor < cc {
            (SelectSection::Categories, self.category_cursor)
        } else if self.category_cursor < cc + ConfigJsonInfo::ITEM_COUNT {
            (SelectSection::ConfigJson, self.category_cursor - cc)
        } else {
            (SelectSection::Settings, self.category_cursor - cc - ConfigJsonInfo::ITEM_COUNT)
        }
    }

    /// Apply saved preferences to a fresh scan result
    fn apply_preferences(&self, result: &mut ScanResult) {
        if self.preferences.categories.is_empty() {
            return; // No saved prefs, use defaults
        }
        for cat in &mut result.categories {
            let key = cat.category.to_string();
            if let Some(&selected) = self.preferences.categories.get(&key) {
                cat.selected = selected;
            }
        }
        if let Some(v) = self.preferences.config_json_orphans {
            result.config_json.orphan_projects_selected = v;
        }
        if let Some(v) = self.preferences.config_json_metrics {
            result.config_json.metrics_selected = v;
        }
        if let Some(v) = self.preferences.config_json_caches {
            result.config_json.cache_selected = v;
        }
    }

    /// Capture current selections into preferences and save to disk
    pub fn save_preferences(&mut self, claude_dir: &std::path::Path) {
        if let Some(ref result) = self.scan_result {
            self.preferences.settings = self.settings.clone();
            self.preferences.categories.clear();
            for cat in &result.categories {
                self.preferences.categories.insert(cat.category.to_string(), cat.selected);
            }
            self.preferences.config_json_orphans = Some(result.config_json.orphan_projects_selected);
            self.preferences.config_json_metrics = Some(result.config_json.metrics_selected);
            self.preferences.config_json_caches = Some(result.config_json.cache_selected);
            self.preferences.save(claude_dir);
        }
    }

    /// Ensure project_cursor is visible within the scroll window.
    pub fn adjust_project_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.project_cursor < self.project_scroll {
            self.project_scroll = self.project_cursor;
        }
        if self.project_cursor >= self.project_scroll + visible_height {
            self.project_scroll = self.project_cursor - visible_height + 1;
        }
    }
}
