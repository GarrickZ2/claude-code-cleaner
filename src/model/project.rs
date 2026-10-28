use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

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

    /// Encode a path the way Claude Code names its project directories:
    /// every character other than an ASCII letter or digit becomes `-`
    /// (one `-` per UTF-16 code unit, matching the JS `replace`).
    pub fn encode_path(path: &str) -> String {
        let mut out = String::with_capacity(path.len());
        for c in path.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c);
            } else {
                for _ in 0..c.len_utf16() {
                    out.push('-');
                }
            }
        }
        out
    }

    /// Resolve the original project path for a directory under
    /// ~/.claude/projects/. Returns the path and whether it could be confirmed.
    ///
    /// The encoding is lossy (`/`, `-`, `_`, `.` and spaces all become `-`),
    /// so the name alone cannot be decoded. Instead:
    /// 1. use the `cwd` recorded in the project's session transcripts, or
    /// 2. search the filesystem for an existing path that encodes to the name.
    ///
    /// If neither works, falls back to a naive decode, returned as unconfirmed.
    pub fn resolve_original_path(dir_name: &str, data_path: &Path) -> (PathBuf, bool) {
        if let Some(cwd) = Self::cwd_from_transcripts(dir_name, data_path) {
            let exists = cwd.exists();
            return (cwd, exists);
        }
        if let Some(found) = Self::find_existing_path(dir_name) {
            return (found, true);
        }
        (Self::decode_dir_name(dir_name), false)
    }

    /// Read the `cwd` field from the project's `.jsonl` session files,
    /// newest first. Only accepts a `cwd` that encodes back to `dir_name`.
    fn cwd_from_transcripts(dir_name: &str, data_path: &Path) -> Option<PathBuf> {
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(data_path)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .map(|p| {
                let mtime = std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                (mtime, p)
            })
            .collect();
        files.sort_by_key(|(t, _)| std::cmp::Reverse(*t));

        for (_, file) in files {
            let Ok(f) = File::open(&file) else { continue };
            for line in BufReader::new(f).lines().map_while(Result::ok).take(100) {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                if let Some(cwd) = value.get("cwd").and_then(|c| c.as_str()) {
                    if Self::encode_path(cwd) == dir_name {
                        return Some(PathBuf::from(cwd));
                    }
                }
            }
        }
        None
    }

    /// Walk the filesystem from `/`, descending only into entries whose
    /// encoded name matches the next part of `dir_name`.
    fn find_existing_path(dir_name: &str) -> Option<PathBuf> {
        let rest = dir_name.strip_prefix('-')?;
        Self::find_existing_path_in(Path::new("/"), rest)
    }

    fn find_existing_path_in(dir: &Path, rest: &str) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let encoded = Self::encode_path(&name.to_string_lossy());
            let path = entry.path();
            if rest == encoded {
                if path.is_dir() {
                    return Some(path);
                }
            } else if let Some(sub) = rest
                .strip_prefix(encoded.as_str())
                .and_then(|r| r.strip_prefix('-'))
            {
                if path.is_dir() {
                    if let Some(found) = Self::find_existing_path_in(&path, sub) {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    /// Naive decode used only as a last resort for display: every `-` → `/`,
    /// `--` → `-`. This is lossy and must not be used to decide orphan status.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("ccc-test-{}-{}", name, std::process::id()))
            .canonicalize_or_self();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    trait CanonicalizeOrSelf {
        fn canonicalize_or_self(self) -> PathBuf;
    }

    impl CanonicalizeOrSelf for PathBuf {
        fn canonicalize_or_self(self) -> PathBuf {
            std::fs::create_dir_all(&self).ok();
            self.canonicalize().unwrap_or(self)
        }
    }

    #[test]
    fn encode_path_replaces_non_alphanumerics() {
        assert_eq!(
            ProjectInfo::encode_path("/home/user/my_project"),
            "-home-user-my-project"
        );
        assert_eq!(
            ProjectInfo::encode_path("/home/me/.config"),
            "-home-me--config"
        );
        assert_eq!(
            ProjectInfo::encode_path("/a/my-proj v1.2"),
            "-a-my-proj-v1-2"
        );
    }

    #[test]
    fn resolves_underscore_path_from_filesystem() {
        let root = temp_dir("fs");
        let project = root.join("my_project");
        std::fs::create_dir_all(&project).unwrap();
        let data = root.join("data");
        std::fs::create_dir_all(&data).unwrap();

        let dir_name = ProjectInfo::encode_path(&project.to_string_lossy());
        let (path, exists) = ProjectInfo::resolve_original_path(&dir_name, &data);
        assert!(exists);
        assert_eq!(path, project);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn resolves_path_from_transcript_cwd() {
        let root = temp_dir("cwd");
        let project = root.join("my-project.v2");
        std::fs::create_dir_all(&project).unwrap();
        let data = root.join("data");
        std::fs::create_dir_all(&data).unwrap();
        let cwd = project.to_string_lossy().to_string();
        std::fs::write(
            data.join("session.jsonl"),
            format!("{}\n", serde_json::json!({ "type": "user", "cwd": cwd })),
        )
        .unwrap();

        let dir_name = ProjectInfo::encode_path(&cwd);
        let (path, exists) = ProjectInfo::resolve_original_path(&dir_name, &data);
        assert!(exists);
        assert_eq!(path, project);

        std::fs::remove_dir_all(&project).unwrap();
        let (path, exists) = ProjectInfo::resolve_original_path(&dir_name, &data);
        assert!(!exists);
        assert_eq!(path, project);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_path_is_reported_as_not_existing() {
        let root = temp_dir("missing");
        let dir_name = ProjectInfo::encode_path(&root.join("gone_project").to_string_lossy());
        let (_, exists) = ProjectInfo::resolve_original_path(&dir_name, &root);
        assert!(!exists);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
