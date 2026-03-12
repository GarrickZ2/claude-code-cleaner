use crate::model::category::{is_protected, Category};
use crate::model::*;
use std::path::Path;
use tokio::sync::mpsc;

/// Default number of lines to keep when trimming history.jsonl
const HISTORY_KEEP_LINES: usize = 500;

#[derive(Debug, Clone)]
pub enum CleanMessage {
    Progress {
        category: String,
        current_file: String,
        #[allow(dead_code)]
        bytes_freed: u64,
        files_done: usize,
        total_files: usize,
    },
    CategoryDone {
        category: String,
        freed: u64,
        errors: Vec<String>,
    },
    Complete {
        total_freed: u64,
        total_errors: Vec<String>,
    },
    #[allow(dead_code)]
    Error(String),
}

pub async fn execute_clean(
    scan_result: &ScanResult,
    settings: &CleanSettings,
    tx: mpsc::UnboundedSender<CleanMessage>,
) {
    let claude_dir = &scan_result.claude_dir;
    let mut total_freed: u64 = 0;
    let mut all_errors: Vec<String> = Vec::new();

    let now = chrono::Local::now();
    let expiry_threshold = chrono::Duration::days(settings.expiry_days as i64);

    // Clean selected categories (except Projects, handled separately)
    for cat_info in &scan_result.categories {
        if !cat_info.selected || cat_info.category == Category::Projects {
            continue;
        }

        let cat_name = cat_info.category.to_string();
        let mut freed: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        if cat_info.category.is_trim_only() {
            if settings.dry_run {
                // Dry run: estimate how much would be saved
                match dry_run_trim_history(claude_dir, HISTORY_KEEP_LINES).await {
                    Ok(saved) => freed = saved,
                    Err(e) => errors.push(format!("history trim: {}", e)),
                }
            } else {
                match trim_history(claude_dir, HISTORY_KEEP_LINES).await {
                    Ok(saved) => freed = saved,
                    Err(e) => errors.push(format!("history trim: {}", e)),
                }
            }
        } else if cat_info.category.is_prefix_match() {
            let target_dir = if cat_info.category.is_home_dir() {
                claude_dir.parent().unwrap_or(claude_dir)
            } else {
                claude_dir
            };
            if settings.dry_run {
                match dry_run_prefix_files(target_dir, cat_info.category.dir_name()).await {
                    Ok(f) => freed = f,
                    Err(e) => errors.push(format!("{}: {}", cat_name, e)),
                }
            } else {
                match clean_prefix_files(target_dir, cat_info.category.dir_name()).await {
                    Ok(f) => freed = f,
                    Err(e) => errors.push(format!("{}: {}", cat_name, e)),
                }
            }
        } else {
            // Directory
            let dir_path = claude_dir.join(cat_info.category.dir_name());
            if dir_path.is_dir() {
                match clean_directory(
                    &dir_path,
                    claude_dir,
                    settings,
                    now,
                    expiry_threshold,
                    &tx,
                    &cat_name,
                )
                .await
                {
                    Ok((f, errs)) => {
                        freed = f;
                        errors = errs;
                    }
                    Err(e) => errors.push(format!("{}: {}", cat_name, e)),
                }
            }
        }

        total_freed += freed;
        all_errors.extend(errors.clone());
        let _ = tx.send(CleanMessage::CategoryDone {
            category: cat_name,
            freed,
            errors,
        });
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    }

    // Clean selected projects
    for proj in &scan_result.projects {
        if !proj.selected {
            continue;
        }
        if is_protected(&proj.data_path, claude_dir) {
            continue;
        }

        let cat_name = format!("Project: {}", proj.original_path.display());

        if proj.is_orphan {
            // Orphan project: delete entire directory
            let freed_size = proj.size;
            if settings.dry_run {
                total_freed += freed_size;
                let _ = tx.send(CleanMessage::CategoryDone {
                    category: cat_name,
                    freed: freed_size,
                    errors: vec![],
                });
            } else {
                match std::fs::remove_dir_all(&proj.data_path) {
                    Ok(_) => {
                        total_freed += freed_size;
                        let _ = tx.send(CleanMessage::CategoryDone {
                            category: cat_name,
                            freed: freed_size,
                            errors: vec![],
                        });
                    }
                    Err(e) => {
                        let err = format!("Failed to remove {}: {}", proj.data_path.display(), e);
                        all_errors.push(err.clone());
                        let _ = tx.send(CleanMessage::CategoryDone {
                            category: cat_name,
                            freed: 0,
                            errors: vec![err],
                        });
                    }
                }
            }
        } else {
            // Active project: delete only expired files
            match clean_directory(
                &proj.data_path,
                claude_dir,
                settings,
                now,
                expiry_threshold,
                &tx,
                &cat_name,
            )
            .await
            {
                Ok((freed, errs)) => {
                    total_freed += freed;
                    all_errors.extend(errs.clone());
                    let _ = tx.send(CleanMessage::CategoryDone {
                        category: cat_name,
                        freed,
                        errors: errs,
                    });
                }
                Err(e) => {
                    let err = format!("{}: {}", cat_name, e);
                    all_errors.push(err.clone());
                    let _ = tx.send(CleanMessage::CategoryDone {
                        category: cat_name,
                        freed: 0,
                        errors: vec![err],
                    });
                }
            }
        }
    }

    // Clean ~/.claude.json if any config_json options selected
    let cj = &scan_result.config_json;
    if cj.orphan_projects_selected || cj.metrics_selected || cj.cache_selected {
        let home_dir = claude_dir.parent().unwrap_or(claude_dir);
        let json_path = home_dir.join(".claude.json");
        match clean_config_json(
            &json_path,
            settings.dry_run,
            cj.orphan_projects_selected,
            cj.metrics_selected,
            cj.cache_selected,
        )
        .await
        {
            Ok(saved) => {
                total_freed += saved;
                let _ = tx.send(CleanMessage::CategoryDone {
                    category: "Config JSON".into(),
                    freed: saved,
                    errors: vec![],
                });
            }
            Err(e) => {
                let err = format!("config json: {}", e);
                all_errors.push(err.clone());
                let _ = tx.send(CleanMessage::CategoryDone {
                    category: "Config JSON".into(),
                    freed: 0,
                    errors: vec![err],
                });
            }
        }
    }

    let _ = tx.send(CleanMessage::Complete {
        total_freed,
        total_errors: all_errors,
    });
}

async fn clean_directory(
    dir_path: &Path,
    claude_dir: &Path,
    settings: &CleanSettings,
    now: chrono::DateTime<chrono::Local>,
    expiry_threshold: chrono::Duration,
    tx: &mpsc::UnboundedSender<CleanMessage>,
    cat_name: &str,
) -> color_eyre::Result<(u64, Vec<String>)> {
    let mut freed: u64 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut files_done = 0;

    // Collect entries first
    let entries: Vec<_> = walkdir::WalkDir::new(dir_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    let total = entries.len();

    // Process in depth-first order (files first, then empty dirs)
    // First pass: delete files
    for entry in &entries {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        if is_protected(path, claude_dir) {
            continue;
        }

        // Check expiry
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                let dt = chrono::DateTime::<chrono::Local>::from(modified);
                let age = now - dt;

                if age < expiry_threshold {
                    continue;
                }

                let file_size = meta.len();
                if settings.dry_run {
                    // Dry run: report without deleting
                    freed += file_size;
                    files_done += 1;
                    let _ = tx.send(CleanMessage::Progress {
                        category: cat_name.to_string(),
                        current_file: path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        bytes_freed: freed,
                        files_done,
                        total_files: total,
                    });
                } else {
                    match std::fs::remove_file(path) {
                        Ok(_) => {
                            freed += file_size;
                            files_done += 1;
                            let _ = tx.send(CleanMessage::Progress {
                                category: cat_name.to_string(),
                                current_file: path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                bytes_freed: freed,
                                files_done,
                                total_files: total,
                            });
                        }
                        Err(e) => {
                            errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
            }
        }
    }

    // Second pass: clean up empty directories (bottom-up), skip in dry run
    if !settings.dry_run {
        let mut dirs: Vec<_> = entries
            .iter()
            .filter(|e| e.file_type().is_dir())
            .map(|e| e.path().to_path_buf())
            .collect();
        dirs.sort_by(|a, b| b.cmp(a)); // reverse order for bottom-up

        for dir in dirs {
            if dir == dir_path {
                continue; // Don't remove the category dir itself
            }
            // Only remove if empty
            if let Ok(mut entries) = std::fs::read_dir(&dir) {
                if entries.next().is_none() {
                    let _ = std::fs::remove_dir(&dir);
                }
            }
        }
    }

    Ok((freed, errors))
}

async fn trim_history(claude_dir: &Path, keep_lines: usize) -> color_eyre::Result<u64> {
    let history_path = claude_dir.join("history.jsonl");
    if !history_path.exists() {
        return Ok(0);
    }

    let content = tokio::fs::read_to_string(&history_path).await?;
    let original_size = content.len() as u64;
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= keep_lines {
        return Ok(0);
    }

    let kept: Vec<&str> = lines[lines.len() - keep_lines..].to_vec();
    let new_content = kept.join("\n") + "\n";
    let new_size = new_content.len() as u64;

    // Atomic write: write to temp file then rename
    let tmp_path = claude_dir.join("history.jsonl.tmp");
    tokio::fs::write(&tmp_path, &new_content).await?;
    tokio::fs::rename(&tmp_path, &history_path).await?;

    Ok(original_size.saturating_sub(new_size))
}

async fn dry_run_trim_history(claude_dir: &Path, keep_lines: usize) -> color_eyre::Result<u64> {
    let history_path = claude_dir.join("history.jsonl");
    if !history_path.exists() {
        return Ok(0);
    }
    let content = tokio::fs::read_to_string(&history_path).await?;
    let original_size = content.len() as u64;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= keep_lines {
        return Ok(0);
    }
    let kept: Vec<&str> = lines[lines.len() - keep_lines..].to_vec();
    let new_size = (kept.join("\n").len() + 1) as u64;
    Ok(original_size.saturating_sub(new_size))
}

async fn dry_run_prefix_files(claude_dir: &Path, prefix: &str) -> color_eyre::Result<u64> {
    let mut would_free: u64 = 0;
    let entries = std::fs::read_dir(claude_dir)?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    would_free += meta.len();
                }
            }
        }
    }
    Ok(would_free)
}

async fn clean_config_json(
    json_path: &Path,
    dry_run: bool,
    remove_orphans: bool,
    strip_metrics: bool,
    remove_caches: bool,
) -> color_eyre::Result<u64> {
    use crate::scanner::categories::clean_json_value;

    if !json_path.exists() {
        return Ok(0);
    }

    let content = tokio::fs::read_to_string(json_path).await?;
    let original_size = content.len() as u64;

    let data: serde_json::Value = serde_json::from_str(&content)?;
    let cleaned = clean_json_value(&data, remove_orphans, strip_metrics, remove_caches);
    let new_content = serde_json::to_string_pretty(&cleaned)? + "\n";
    let new_size = new_content.len() as u64;

    let saved = original_size.saturating_sub(new_size);

    if !dry_run && saved > 0 {
        // Atomic write: write to temp file then rename
        let tmp_path = json_path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, &new_content).await?;
        tokio::fs::rename(&tmp_path, json_path).await?;
    }

    Ok(saved)
}

async fn clean_prefix_files(claude_dir: &Path, prefix: &str) -> color_eyre::Result<u64> {
    let mut freed: u64 = 0;
    let entries = std::fs::read_dir(claude_dir)?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let size = meta.len();
                    if std::fs::remove_file(entry.path()).is_ok() {
                        freed += size;
                    }
                }
            }
        }
    }
    Ok(freed)
}
