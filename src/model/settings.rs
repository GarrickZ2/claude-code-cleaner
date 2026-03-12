use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Persisted user preferences (saved after each clean)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub settings: CleanSettings,
    /// Category name -> selected
    #[serde(default)]
    pub categories: HashMap<String, bool>,
    /// ConfigJson selections
    #[serde(default)]
    pub config_json_orphans: Option<bool>,
    #[serde(default)]
    pub config_json_metrics: Option<bool>,
    #[serde(default)]
    pub config_json_caches: Option<bool>,
}

const PREFS_FILENAME: &str = "cleaner-preferences.json";

impl UserPreferences {
    pub fn load(claude_dir: &Path) -> Self {
        let path = claude_dir.join(PREFS_FILENAME);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, claude_dir: &Path) {
        let path = claude_dir.join(PREFS_FILENAME);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanSettings {
    /// Days threshold: only clean files older than this
    pub expiry_days: u32,
    /// Dry run mode: simulate cleaning without actually deleting anything
    pub dry_run: bool,
}

impl Default for CleanSettings {
    fn default() -> Self {
        Self {
            expiry_days: 30,
            dry_run: false,
        }
    }
}

impl CleanSettings {
    pub const FIELD_COUNT: usize = 2;

    pub fn field_name(index: usize) -> &'static str {
        match index {
            0 => "Expiry Threshold (days)",
            1 => "Dry Run (simulate only)",
            _ => "",
        }
    }

    pub fn field_value(&self, index: usize) -> String {
        match index {
            0 => format!("{}", self.expiry_days),
            1 => {
                if self.dry_run {
                    "Yes".into()
                } else {
                    "No".into()
                }
            }
            _ => String::new(),
        }
    }

    pub fn increment(&mut self, index: usize) {
        match index {
            0 => self.expiry_days = self.expiry_days.saturating_add(1).min(365),
            1 => self.dry_run = !self.dry_run,
            _ => {}
        }
    }

    pub fn decrement(&mut self, index: usize) {
        match index {
            0 => self.expiry_days = self.expiry_days.saturating_sub(1).max(1),
            1 => self.dry_run = !self.dry_run,
            _ => {}
        }
    }
}
