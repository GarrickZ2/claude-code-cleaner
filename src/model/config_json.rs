/// Breakdown of reclaimable data in ~/.claude.json
#[derive(Debug, Clone, Default)]
pub struct ConfigJsonInfo {
    pub total_size: u64,
    /// Orphan project entries (paths no longer exist)
    pub orphan_projects_count: usize,
    pub orphan_projects_size: u64,
    pub orphan_projects_selected: bool,
    /// Session metrics in existing projects (last*, exampleFiles, etc.)
    pub metrics_entries_count: usize,
    pub metrics_size: u64,
    pub metrics_selected: bool,
    /// Top-level cache keys (cachedGrowthBookFeatures, skillUsage, etc.)
    pub cache_keys_count: usize,
    pub cache_size: u64,
    pub cache_selected: bool,
}

impl ConfigJsonInfo {
    pub const ITEM_COUNT: usize = 3;

    pub fn reclaimable_size(&self) -> u64 {
        let mut total = 0;
        if self.orphan_projects_selected {
            total += self.orphan_projects_size;
        }
        if self.metrics_selected {
            total += self.metrics_size;
        }
        if self.cache_selected {
            total += self.cache_size;
        }
        total
    }
}
