use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkspacePathError {
    #[error("Workspace root path does not exist or cannot be canonicalized: {0}")]
    InvalidRootPath(String),

    #[error("Workspace root path must be a directory: {0}")]
    RootNotDirectory(String),

    #[error("Refusing to register restricted system or credential directory as workspace: {0}")]
    RestrictedRootPath(String),

    #[error("Target path cannot be empty")]
    EmptyTargetPath,

    #[error("Path escapes authorized workspace root '{root}': {target}")]
    OutsideWorkspace { root: String, target: String },

    #[error("Access to sensitive credential or key path is denied: {0}")]
    SensitivePathDenied(String),

    #[error("Failed to resolve path '{path}': {reason}")]
    ResolutionFailed { path: String, reason: String },
}

/// Sensitive filenames and directory components that require explicit protection against
/// automatic context collection or unapproved access.
const SENSITIVE_PATH_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    ".ssh",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    ".aws",
    ".gnupg",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "credentials.json",
];

/// System directories that must never be registered directly as a project workspace root.
const RESTRICTED_SYSTEM_ROOTS: &[&str] = &[
    "/",
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/private/etc",
    "/var",
    "/private/var",
    "/System",
    "/Library",
    "/proc",
    "/sys",
    "/dev",
];

/// Enforces canonicalized filesystem boundaries for a project workspace.
///
/// Defends against:
/// - `..` parent directory traversal
/// - Symlink escapes pointing outside the workspace root
/// - Registering root/system directories or sensitive credential directories
/// - Accessing sensitive credential files (e.g., `.env`, `.ssh/id_ed25519`) without explicit policy
#[derive(Debug, Clone)]
pub struct WorkspacePathPolicy {
    canonical_root: PathBuf,
}

impl WorkspacePathPolicy {
    /// Validates and canonicalizes a candidate project root directory.
    pub fn new(root_path: impl AsRef<Path>) -> Result<Self, WorkspacePathError> {
        let raw = root_path.as_ref();
        let canonical = std::fs::canonicalize(raw)
            .map_err(|e| WorkspacePathError::InvalidRootPath(format!("{}: {e}", raw.display())))?;

        if !canonical.is_dir() {
            return Err(WorkspacePathError::RootNotDirectory(
                canonical.display().to_string(),
            ));
        }

        let canonical_str = canonical.to_string_lossy();
        if RESTRICTED_SYSTEM_ROOTS.contains(&canonical_str.as_ref()) {
            return Err(WorkspacePathError::RestrictedRootPath(
                canonical_str.into_owned(),
            ));
        }

        for component in canonical.components() {
            if let Component::Normal(os_str) = component
                && let Some(name) = os_str.to_str()
                && matches!(name, ".ssh" | ".aws" | ".gnupg")
            {
                return Err(WorkspacePathError::RestrictedRootPath(
                    canonical.display().to_string(),
                ));
            }
        }

        Ok(Self {
            canonical_root: canonical,
        })
    }

    /// Returns the canonicalized workspace root path.
    pub fn canonical_root(&self) -> &Path {
        &self.canonical_root
    }

    /// Resolves a relative or absolute path for reading an existing file/directory inside the workspace,
    /// rejecting symlink escapes, `..` escapes, and sensitive credential files.
    pub fn resolve_existing_for_read(
        &self,
        candidate: impl AsRef<Path>,
    ) -> Result<PathBuf, WorkspacePathError> {
        let resolved = self.resolve_within_workspace(candidate.as_ref(), true)?;
        Self::check_not_sensitive(&resolved, &self.canonical_root)?;
        Ok(resolved)
    }

    /// Resolves a relative or absolute path for writing/creating a file inside the workspace,
    /// canonicalizing the deepest existing ancestor so non-existent leaf filenames are supported
    /// while still preventing symlink escapes on parent directories.
    pub fn resolve_for_write(
        &self,
        candidate: impl AsRef<Path>,
    ) -> Result<PathBuf, WorkspacePathError> {
        let resolved = self.resolve_within_workspace(candidate.as_ref(), false)?;
        Self::check_not_sensitive(&resolved, &self.canonical_root)?;
        Ok(resolved)
    }

    fn resolve_within_workspace(
        &self,
        candidate: &Path,
        must_exist: bool,
    ) -> Result<PathBuf, WorkspacePathError> {
        if candidate.as_os_str().is_empty() {
            return Err(WorkspacePathError::EmptyTargetPath);
        }

        let joined = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.canonical_root.join(candidate)
        };

        let canonical_target = if must_exist || joined.exists() {
            std::fs::canonicalize(&joined).map_err(|e| WorkspacePathError::ResolutionFailed {
                path: candidate.display().to_string(),
                reason: e.to_string(),
            })?
        } else {
            self.canonicalize_non_existing(&joined)?
        };

        if !canonical_target.starts_with(&self.canonical_root) {
            return Err(WorkspacePathError::OutsideWorkspace {
                root: self.canonical_root.display().to_string(),
                target: candidate.display().to_string(),
            });
        }

        Ok(canonical_target)
    }

    fn canonicalize_non_existing(&self, target: &Path) -> Result<PathBuf, WorkspacePathError> {
        let mut normalized = PathBuf::new();
        for comp in target.components() {
            match comp {
                Component::Prefix(p) => normalized.push(p.as_os_str()),
                Component::RootDir => normalized.push(Component::RootDir.as_os_str()),
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(WorkspacePathError::OutsideWorkspace {
                            root: self.canonical_root.display().to_string(),
                            target: target.display().to_string(),
                        });
                    }
                }
                Component::Normal(part) => normalized.push(part),
            }
        }

        let mut cursor = normalized.as_path();
        let mut tail_parts = Vec::new();
        while !cursor.exists() {
            if let Some(name) = cursor.file_name() {
                tail_parts.push(name.to_os_string());
            }
            cursor = cursor
                .parent()
                .ok_or_else(|| WorkspacePathError::OutsideWorkspace {
                    root: self.canonical_root.display().to_string(),
                    target: target.display().to_string(),
                })?;
        }

        let mut canonical_base =
            std::fs::canonicalize(cursor).map_err(|e| WorkspacePathError::ResolutionFailed {
                path: cursor.display().to_string(),
                reason: e.to_string(),
            })?;

        for part in tail_parts.into_iter().rev() {
            canonical_base.push(part);
        }

        Ok(canonical_base)
    }

    fn check_not_sensitive(
        resolved: &Path,
        canonical_root: &Path,
    ) -> Result<(), WorkspacePathError> {
        let rel = resolved.strip_prefix(canonical_root).unwrap_or(resolved);
        for comp in rel.components() {
            if let Component::Normal(os_str) = comp
                && let Some(name) = os_str.to_str()
            {
                let lower = name.to_ascii_lowercase();
                if SENSITIVE_PATH_NAMES.contains(&lower.as_str())
                    || lower.starts_with(".env.")
                    || lower.ends_with(".pem")
                    || lower.ends_with(".key")
                {
                    return Err(WorkspacePathError::SensitivePathDenied(
                        rel.display().to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}
