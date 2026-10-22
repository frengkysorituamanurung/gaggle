use gaggle_core::{WorkspacePathError, WorkspacePathPolicy};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Default maximum file size allowed for `fs.read` (512 KiB).
pub const DEFAULT_MAX_READ_BYTES: u64 = 512 * 1024;
/// Default maximum entries returned by `fs.list` (500 entries).
pub const DEFAULT_MAX_LIST_ENTRIES: usize = 500;
/// Default maximum matching lines returned by `fs.search` (200 matches).
pub const DEFAULT_MAX_SEARCH_MATCHES: usize = 200;

/// Directories skipped automatically during `fs.list` and `fs.search`.
const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules", ".DS_Store"];

#[derive(Debug, Error)]
pub enum FsToolError {
    #[error(transparent)]
    Policy(#[from] WorkspacePathError),

    #[error("Path '{path}' is not a regular file")]
    NotRegularFile { path: String },

    #[error("Path '{path}' is not a directory")]
    NotDirectory { path: String },

    #[error("File '{path}' ({size_bytes} bytes) exceeds maximum read limit of {max_bytes} bytes")]
    FileTooLarge {
        path: String,
        size_bytes: u64,
        max_bytes: u64,
    },

    #[error("File '{path}' appears to be binary and cannot be read as UTF-8 text")]
    BinaryFileRejected { path: String },

    #[error("Search query must not be empty")]
    EmptySearchQuery,

    #[error("Filesystem I/O error on '{path}': {reason}")]
    Io { path: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsEntry {
    pub relative_path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsReadOutput {
    pub relative_path: String,
    pub content: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsSearchMatch {
    pub relative_path: String,
    pub line_number: usize,
    pub line_preview: String,
}

/// Lists immediate entries inside a workspace directory (relative to the workspace root).
/// Rejects paths outside the workspace and skips sensitive credential files.
pub fn fs_list(
    policy: &WorkspacePathPolicy,
    relative_dir: Option<&str>,
    max_entries: usize,
) -> Result<Vec<FsEntry>, FsToolError> {
    let dir_arg = relative_dir.unwrap_or(".");
    let resolved_dir = policy.resolve_existing_for_read(dir_arg)?;

    if !resolved_dir.is_dir() {
        return Err(FsToolError::NotDirectory {
            path: dir_arg.to_string(),
        });
    }

    let read_dir = std::fs::read_dir(&resolved_dir).map_err(|e| FsToolError::Io {
        path: dir_arg.to_string(),
        reason: e.to_string(),
    })?;

    let mut entries = Vec::new();
    for entry_res in read_dir {
        if entries.len() >= max_entries {
            break;
        }
        let entry = entry_res.map_err(|e| FsToolError::Io {
            path: dir_arg.to_string(),
            reason: e.to_string(),
        })?;

        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && IGNORED_DIRS.contains(&name)
        {
            continue;
        }

        // Only include entries that pass the workspace & sensitive-file policy
        let Ok(validated_path) = policy.resolve_existing_for_read(&path) else {
            continue;
        };

        let metadata = std::fs::metadata(&validated_path).map_err(|e| FsToolError::Io {
            path: validated_path.display().to_string(),
            reason: e.to_string(),
        })?;

        let rel = validated_path
            .strip_prefix(policy.canonical_root())
            .unwrap_or(&validated_path)
            .to_string_lossy()
            .replace('\\', "/");

        entries.push(FsEntry {
            relative_path: rel,
            is_dir: metadata.is_dir(),
            size_bytes: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    Ok(entries)
}

/// Reads a UTF-8 text file within the workspace, enforcing `WorkspacePathPolicy`,
/// `max_bytes` size limit, and binary-content rejection.
pub fn fs_read(
    policy: &WorkspacePathPolicy,
    candidate_path: impl AsRef<Path>,
    max_bytes: u64,
) -> Result<FsReadOutput, FsToolError> {
    let raw_display = candidate_path.as_ref().display().to_string();
    let resolved = policy.resolve_existing_for_read(candidate_path)?;

    let metadata = std::fs::metadata(&resolved).map_err(|e| FsToolError::Io {
        path: raw_display.clone(),
        reason: e.to_string(),
    })?;

    if !metadata.is_file() {
        return Err(FsToolError::NotRegularFile { path: raw_display });
    }

    let size_bytes = metadata.len();
    if size_bytes > max_bytes {
        return Err(FsToolError::FileTooLarge {
            path: raw_display,
            size_bytes,
            max_bytes,
        });
    }

    let bytes = std::fs::read(&resolved).map_err(|e| FsToolError::Io {
        path: raw_display.clone(),
        reason: e.to_string(),
    })?;

    // Check first 8 KiB for NUL bytes to detect binary files early
    let sample_len = bytes.len().min(8192);
    if bytes[..sample_len].contains(&0) {
        return Err(FsToolError::BinaryFileRejected { path: raw_display });
    }

    let content = String::from_utf8(bytes)
        .map_err(|_| FsToolError::BinaryFileRejected { path: raw_display })?;

    let relative_path = resolved
        .strip_prefix(policy.canonical_root())
        .unwrap_or(&resolved)
        .to_string_lossy()
        .replace('\\', "/");

    Ok(FsReadOutput {
        relative_path,
        content,
        size_bytes,
    })
}

/// Searches text files inside the workspace for a literal substring, skipping ignored directories,
/// binary files, oversized files, and sensitive files.
pub fn fs_search(
    policy: &WorkspacePathPolicy,
    query: &str,
    max_matches: usize,
) -> Result<Vec<FsSearchMatch>, FsToolError> {
    if query.trim().is_empty() {
        return Err(FsToolError::EmptySearchQuery);
    }

    let mut matches = Vec::new();
    let mut stack = vec![policy.canonical_root().to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        if matches.len() >= max_matches {
            break;
        }

        let Ok(read_dir) = std::fs::read_dir(&current_dir) else {
            continue;
        };

        let mut dir_entries: Vec<_> = read_dir.flatten().collect();
        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            if matches.len() >= max_matches {
                break;
            }

            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if IGNORED_DIRS.contains(&name) {
                continue;
            }

            // Skip symlinks to prevent directory cycles or workspace escapes during traversal
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }

            let Ok(validated) = policy.resolve_existing_for_read(&path) else {
                continue;
            };

            if file_type.is_dir() {
                stack.push(validated);
            } else if file_type.is_file()
                && let Ok(read_out) = fs_read(policy, &validated, DEFAULT_MAX_READ_BYTES)
            {
                for (idx, line) in read_out.content.lines().enumerate() {
                    if line.contains(query) {
                        matches.push(FsSearchMatch {
                            relative_path: read_out.relative_path.clone(),
                            line_number: idx + 1,
                            line_preview: line.chars().take(240).collect(),
                        });
                        if matches.len() >= max_matches {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(matches)
}
