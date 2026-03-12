use crate::model::{Category, CategoryInfo, ConfigJsonInfo};
use chrono::{DateTime, Local};
use std::path::Path;
use walkdir::WalkDir;

/// Top-level keys in ~/.claude.json that are caches and safe to remove
const JSON_REMOVABLE_TOP_KEYS: &[&str] = &[
    "cachedGrowthBookFeatures",
    "cachedStatsigGates",
    "cachedDynamicConfigs",
    "cachedExtraUsageDisabledReason",
    "cachedChromeExtensionInstalled",
    "skillUsage",
    "tipsHistory",
    "toolUsage",
    "s1mAccessCache",
    "groveConfigCache",
    "metricsStatusCache",
    "passesEligibilityCache",
    "clientDataCache",
    "feedbackSurveyState",
    "reactVulnerabilityCache",
];

/// Per-project keys that are session metrics (safe to strip)
const PROJECT_METRIC_KEYS: &[&str] = &[
    "lastModelUsage",
    "lastSessionMetrics",
    "lastSessionId",
    "lastCost",
    "lastAPIDuration",
    "lastAPIDurationWithoutRetries",
    "lastToolDuration",
    "lastDuration",
    "lastLinesAdded",
    "lastLinesRemoved",
    "lastTotalInputTokens",
    "lastTotalOutputTokens",
    "lastTotalCacheCreationInputTokens",
    "lastTotalCacheReadInputTokens",
    "lastTotalWebSearchRequests",
    "lastFpsAverage",
    "lastFpsLow1Pct",
    "exampleFiles",
    "exampleFilesGeneratedAt",
    "reactVulnerabilityCache",
];

/// Clean ~/.claude.json based on selection flags
pub fn clean_json_value(
    data: &serde_json::Value,
    remove_orphans: bool,
    strip_metrics: bool,
    remove_caches: bool,
) -> serde_json::Value {
    let Some(obj) = data.as_object() else {
        return data.clone();
    };

    let mut result = serde_json::Map::new();

    for (key, value) in obj {
        // Skip removable top-level cache keys
        if remove_caches && JSON_REMOVABLE_TOP_KEYS.iter().any(|k| k == key) {
            continue;
        }

        // Special handling for "projects" object
        if key == "projects" {
            if let Some(projects) = value.as_object() {
                let mut cleaned_projects = serde_json::Map::new();
                for (path, proj_value) in projects {
                    // Skip orphan projects
                    if remove_orphans && !std::path::Path::new(path).exists() {
                        continue;
                    }
                    // Strip session metric keys from existing projects
                    if strip_metrics {
                        if let Some(proj_obj) = proj_value.as_object() {
                            let mut cleaned_proj = serde_json::Map::new();
                            for (pk, pv) in proj_obj {
                                if !PROJECT_METRIC_KEYS.iter().any(|k| k == pk) {
                                    cleaned_proj.insert(pk.clone(), pv.clone());
                                }
                            }
                            cleaned_projects
                                .insert(path.clone(), serde_json::Value::Object(cleaned_proj));
                        } else {
                            cleaned_projects.insert(path.clone(), proj_value.clone());
                        }
                    } else {
                        cleaned_projects.insert(path.clone(), proj_value.clone());
                    }
                }
                result.insert(key.clone(), serde_json::Value::Object(cleaned_projects));
            } else {
                result.insert(key.clone(), value.clone());
            }
            continue;
        }

        result.insert(key.clone(), value.clone());
    }

    serde_json::Value::Object(result)
}

/// Scan ~/.claude.json and compute per-group reclaimable sizes
pub fn scan_config_json(claude_dir: &Path) -> ConfigJsonInfo {
    let home_dir = claude_dir.parent().unwrap_or(claude_dir);
    let json_path = home_dir.join(".claude.json");
    let mut info = ConfigJsonInfo::default();

    let content = match std::fs::read_to_string(&json_path) {
        Ok(c) => c,
        Err(_) => return info,
    };
    info.total_size = content.len() as u64;

    let data: serde_json::Value = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(_) => return info,
    };

    let Some(obj) = data.as_object() else {
        return info;
    };

    // 1. Top-level cache keys
    for (key, value) in obj {
        if JSON_REMOVABLE_TOP_KEYS.iter().any(|k| k == key) {
            info.cache_keys_count += 1;
            info.cache_size += serde_json::to_string(value)
                .map(|s| s.len() as u64)
                .unwrap_or(0);
        }
    }

    // 2. Projects analysis
    if let Some(projects) = obj.get("projects").and_then(|v| v.as_object()) {
        for (path, proj_value) in projects {
            let entry_size = serde_json::to_string(proj_value)
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            if !std::path::Path::new(path).exists() {
                // Orphan project
                info.orphan_projects_count += 1;
                info.orphan_projects_size += entry_size;
            } else if let Some(proj_obj) = proj_value.as_object() {
                // Existing project — count metric fields
                for (pk, pv) in proj_obj {
                    if PROJECT_METRIC_KEYS.iter().any(|k| k == pk) {
                        info.metrics_entries_count += 1;
                        info.metrics_size += serde_json::to_string(pv)
                            .map(|s| s.len() as u64)
                            .unwrap_or(0);
                    }
                }
            }
        }
    }

    // Default all selected
    info.orphan_projects_selected = info.orphan_projects_size > 0;
    info.metrics_selected = info.metrics_size > 0;
    info.cache_selected = info.cache_size > 0;

    info
}

pub async fn scan_category(
    claude_dir: &Path,
    category: Category,
) -> color_eyre::Result<CategoryInfo> {
    let mut info = CategoryInfo::new(category);

    if category.is_prefix_match() {
        // Handle backup files: match in the appropriate directory
        let scan_dir = if category.is_home_dir() {
            claude_dir.parent().unwrap_or(claude_dir)
        } else {
            claude_dir
        };
        scan_prefix_files(scan_dir, category.dir_name(), &mut info)?;
    } else if category.is_file() {
        // Single file
        let path = claude_dir.join(category.dir_name());
        if path.exists() {
            if let Ok(meta) = std::fs::metadata(&path) {
                info.size = meta.len();
                info.file_count = 1;
                info.oldest_modified = meta.modified().ok().map(DateTime::<Local>::from);
            }
        }
    } else {
        // Directory
        let dir_path = claude_dir.join(category.dir_name());
        if dir_path.is_dir() {
            scan_directory(&dir_path, &mut info)?;
        }
    }

    Ok(info)
}

fn scan_directory(dir_path: &Path, info: &mut CategoryInfo) -> color_eyre::Result<()> {
    for entry in WalkDir::new(dir_path)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                let file_size = meta.len();
                info.size += file_size;
                info.file_count += 1;
                if let Ok(modified) = meta.modified() {
                    let dt = DateTime::<Local>::from(modified);
                    info.file_ages.push((dt, file_size));
                    match &info.oldest_modified {
                        None => info.oldest_modified = Some(dt),
                        Some(oldest) if dt < *oldest => info.oldest_modified = Some(dt),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn scan_prefix_files(
    claude_dir: &Path,
    prefix: &str,
    info: &mut CategoryInfo,
) -> color_eyre::Result<()> {
    if let Ok(entries) = std::fs::read_dir(claude_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        info.size += meta.len();
                        info.file_count += 1;
                        if let Ok(modified) = meta.modified() {
                            let dt = DateTime::<Local>::from(modified);
                            match &info.oldest_modified {
                                None => info.oldest_modified = Some(dt),
                                Some(oldest) if dt < *oldest => info.oldest_modified = Some(dt),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
