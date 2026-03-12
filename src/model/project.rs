use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// The encoded directory name under ~/.claude/projects/
    #[allow(dead_code)]
    pub dir_name: String,
    /// The decoded original project path
    pub original_path: PathBuf,
    /// Full path to this project's data directory
    pub data_path: PathBuf,
    /// Total size of this project's data
    pub size: u64,
    /// Number of files
    pub file_count: usize,
    /// Last modified time
    pub last_modified: Option<chrono::DateTime<chrono::Local>>,
    /// Whether the original project path still exists
    pub is_orphan: bool,
    /// Whether selected for cleaning
    pub selected: bool,
    /// Per-file (modification_time, size) for expiry-based filtering
    pub file_ages: Vec<(chrono::DateTime<chrono::Local>, u64)>,
}

impl ProjectInfo {
    /// For orphan projects, returns full size (delete everything).
    /// For active projects, returns size of files older than expiry_days.
    pub fn expired_size(&self, expiry_days: u32) -> u64 {
        if self.is_orphan {
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

    /// For orphan projects, returns full file count.
    /// For active projects, returns count of files older than expiry_days.
    pub fn expired_count(&self, expiry_days: u32) -> usize {
        if self.is_orphan {
            return self.file_count;
        }
        let now = chrono::Local::now();
        let threshold = chrono::Duration::days(expiry_days as i64);
        self.file_ages
            .iter()
            .filter(|(dt, _)| now - *dt >= threshold)
            .count()
    }

    /// Decode a claude projects directory name back to the original path.
    /// Encoding: leading `-` → `/`, `-` → `/`, `--` → literal `-`
    pub fn decode_dir_name(encoded: &str) -> PathBuf {
        let mut result = String::new();
        let chars: Vec<char> = encoded.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars[i] == '-' {
                if i + 1 < len && chars[i + 1] == '-' {
                    result.push('-');
                    i += 2;
                } else {
                    result.push('/');
                    i += 1;
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        PathBuf::from(result)
    }
}
