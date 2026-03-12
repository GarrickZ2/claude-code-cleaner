use super::Category;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CleanPlan {
    pub items: Vec<CleanItem>,
    pub total_size: u64,
    pub total_files: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CleanItem {
    pub category: Category,
    pub path: PathBuf,
    pub size: u64,
    pub file_count: usize,
    /// For History: trim to N lines instead of delete
    pub trim_to: Option<usize>,
}

#[allow(dead_code)]
impl CleanPlan {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_size: 0,
            total_files: 0,
        }
    }

    pub fn add(&mut self, item: CleanItem) {
        self.total_size += item.size;
        self.total_files += item.file_count;
        self.items.push(item);
    }
}
