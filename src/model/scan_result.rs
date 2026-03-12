use super::{CategoryInfo, ConfigJsonInfo, ProjectInfo};

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub categories: Vec<CategoryInfo>,
    pub projects: Vec<ProjectInfo>,
    pub config_json: ConfigJsonInfo,
    pub total_size: u64,
    pub total_files: usize,
    pub claude_dir: std::path::PathBuf,
}

impl ScanResult {
    pub fn empty(claude_dir: std::path::PathBuf) -> Self {
        Self {
            categories: Vec::new(),
            projects: Vec::new(),
            config_json: ConfigJsonInfo::default(),
            total_size: 0,
            total_files: 0,
            claude_dir,
        }
    }

    pub fn reclaimable_size(&self, expiry_days: u32) -> u64 {
        let cat_size: u64 = self
            .categories
            .iter()
            .filter(|c| c.selected)
            .map(|c| c.expired_size(expiry_days))
            .sum();
        let proj_size: u64 = self
            .projects
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.expired_size(expiry_days))
            .sum();
        let projects_cat_selected = self
            .categories
            .iter()
            .any(|c| c.category == super::category::Category::Projects && c.selected);
        let config_json_size = self.config_json.reclaimable_size();
        if projects_cat_selected {
            cat_size + config_json_size
        } else {
            cat_size + proj_size + config_json_size
        }
    }

    /// Total size that *could* be cleaned if everything were selected.
    /// Mirrors `reclaimable_size` logic but ignores selection state.
    pub fn matchable_size(&self, expiry_days: u32) -> u64 {
        // All categories (non-Projects)
        let cat_size: u64 = self
            .categories
            .iter()
            .filter(|c| c.category != super::category::Category::Projects)
            .map(|c| c.expired_size(expiry_days))
            .sum();
        // All individual projects (orphan = full size, active = expired files)
        let proj_size: u64 = self
            .projects
            .iter()
            .map(|p| p.expired_size(expiry_days))
            .sum();
        // Projects category size (the entire projects/ directory)
        let projects_cat_size: u64 = self
            .categories
            .iter()
            .find(|c| c.category == super::category::Category::Projects)
            .map(|c| c.expired_size(expiry_days))
            .unwrap_or(0);
        // Use the larger of: Projects category size vs sum of individual projects
        // (category size may include files not tracked as individual projects)
        let effective_proj_size = projects_cat_size.max(proj_size);
        // All config_json fields
        let config_json_size = self.config_json.orphan_projects_size
            + self.config_json.metrics_size
            + self.config_json.cache_size;
        cat_size + effective_proj_size + config_json_size
    }

    pub fn selected_file_count(&self, expiry_days: u32) -> usize {
        let cat_files: usize = self
            .categories
            .iter()
            .filter(|c| c.selected && c.category != super::category::Category::Projects)
            .map(|c| c.expired_count(expiry_days))
            .sum();
        let proj_files: usize = self
            .projects
            .iter()
            .filter(|p| p.selected)
            .map(|p| p.expired_count(expiry_days))
            .sum();
        cat_files + proj_files
    }
}
