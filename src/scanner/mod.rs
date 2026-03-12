pub mod categories;
pub mod projects;

use crate::model::*;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ScanMessage {
    #[allow(dead_code)]
    Progress { category: String, scanned: usize },
    Complete(ScanResult),
    Error(String),
}

/// Quick scan: only reads top-level directory entries + stat for sizes.
/// Used for initial dashboard display.
pub async fn quick_scan(claude_dir: PathBuf) -> color_eyre::Result<ScanResult> {
    let mut result = ScanResult::empty(claude_dir.clone());
    let mut total_size: u64 = 0;
    let mut total_files: usize = 0;

    for cat in Category::ALL {
        let info = categories::scan_category(&claude_dir, *cat).await?;
        total_size += info.size;
        total_files += info.file_count;
        result.categories.push(info);
    }

    result.total_size = total_size;
    result.total_files = total_files;

    // Quick project scan
    result.projects = projects::scan_projects(&claude_dir).await?;

    // Scan ~/.claude.json
    result.config_json = categories::scan_config_json(&claude_dir);

    Ok(result)
}

/// Deep scan with progress reporting via channel.
pub async fn deep_scan(
    claude_dir: PathBuf,
    tx: mpsc::UnboundedSender<ScanMessage>,
) {
    match quick_scan(claude_dir).await {
        Ok(result) => {
            let _ = tx.send(ScanMessage::Complete(result));
        }
        Err(e) => {
            let _ = tx.send(ScanMessage::Error(format!("{}", e)));
        }
    }
}
