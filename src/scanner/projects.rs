use crate::model::ProjectInfo;
use chrono::{DateTime, Local};
use std::path::Path;
use walkdir::WalkDir;

pub async fn scan_projects(claude_dir: &Path) -> color_eyre::Result<Vec<ProjectInfo>> {
    let projects_dir = claude_dir.join("projects");
    if !projects_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    let entries = std::fs::read_dir(&projects_dir)?;
    for entry in entries.filter_map(|e| e.ok()) {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let original_path = ProjectInfo::decode_dir_name(&dir_name);
        let data_path = entry.path();

        let mut size: u64 = 0;
        let mut file_count: usize = 0;
        let mut last_modified: Option<DateTime<Local>> = None;
        let mut file_ages: Vec<(DateTime<Local>, u64)> = Vec::new();

        for walk_entry in WalkDir::new(&data_path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if walk_entry.file_type().is_file() {
                if let Ok(meta) = walk_entry.metadata() {
                    let file_size = meta.len();
                    size += file_size;
                    file_count += 1;
                    if let Ok(modified) = meta.modified() {
                        let dt = DateTime::<Local>::from(modified);
                        file_ages.push((dt, file_size));
                        match &last_modified {
                            None => last_modified = Some(dt),
                            Some(last) if dt > *last => last_modified = Some(dt),
                            _ => {}
                        }
                    }
                }
            }
        }

        let is_orphan = !original_path.exists();

        projects.push(ProjectInfo {
            dir_name,
            original_path,
            data_path,
            size,
            file_count,
            last_modified,
            is_orphan,
            selected: false,
            file_ages,
        });
    }

    // Sort by size descending
    projects.sort_by(|a, b| b.size.cmp(&a.size));
    Ok(projects)
}
