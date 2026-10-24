//! Memory export to `<project>/.pal/memory/*.md` for Git sharing.

use anyhow::{Context, Result, bail};
use gaggle_core::WorkspacePathPolicy;
use gaggle_db::repositories::{memories::MemoryRepository, projects::ProjectRepository};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Default project-relative directory for per-memory Markdown exports.
pub const MEMORY_EXPORT_DIR: &str = ".pal/memory";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExportFile {
    pub file_name: String,
    pub path: PathBuf,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExportReport {
    pub project_id: String,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
    pub files: Vec<MemoryExportFile>,
}

/// Writes one Markdown file per active project + global memory.
///
/// `output_dir` is interpreted relative to the project root and must resolve
/// inside it; symlink escapes and `..` traversal are rejected by
/// [`WorkspacePathPolicy`]. Existing memory files are replaced atomically, but
/// unrelated files in the directory are never deleted or modified.
pub async fn export_memories(
    pool: &SqlitePool,
    project_id: &str,
    output_dir: Option<&Path>,
) -> Result<MemoryExportReport> {
    let projects = ProjectRepository::new(pool.clone());
    let project = projects
        .get_by_id(project_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Project {project_id} not found"))?;
    if project.archived_at.is_some() {
        bail!("Project {project_id} is archived; refusing to export memories");
    }

    let project_root = PathBuf::from(&project.root_path);
    let policy = WorkspacePathPolicy::new(&project_root).with_context(|| {
        format!(
            "Project {} workspace root is not accessible: {}",
            project_id, project.root_path
        )
    })?;

    let relative = output_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(MEMORY_EXPORT_DIR));
    let target = policy.resolve_for_write(&relative).with_context(|| {
        format!(
            "Refusing to export memories outside project workspace {}",
            policy.canonical_root().display()
        )
    })?;

    fs::create_dir_all(&target).with_context(|| {
        format!(
            "Failed to create memory export directory {}",
            target.display()
        )
    })?;
    let canonical_target = fs::canonicalize(&target).with_context(|| {
        format!(
            "Failed to resolve memory export directory {}",
            target.display()
        )
    })?;
    if !canonical_target.starts_with(policy.canonical_root()) {
        bail!(
            "Memory export directory {} escapes project workspace {}",
            canonical_target.display(),
            policy.canonical_root().display()
        );
    }

    let memories = MemoryRepository::new(pool.clone());
    let files = memories.export_markdown_files(project_id).await?;

    let mut exported = Vec::with_capacity(files.len());
    for file in files {
        if !is_safe_file_name(&file.file_name) {
            bail!(
                "Refusing to write memory export with unsafe file name {:?}",
                file.file_name
            );
        }
        let path = canonical_target.join(&file.file_name);
        let resolved = policy.resolve_for_write(&path)?;
        if resolved.parent() != Some(canonical_target.as_path()) {
            bail!(
                "Memory export path {} is outside the export directory",
                resolved.display()
            );
        }
        write_atomic(&resolved, file.markdown.as_bytes())?;
        exported.push(MemoryExportFile {
            file_name: file.file_name,
            path: resolved,
            bytes: file.markdown.len(),
        });
    }
    exported.sort_by(|left, right| left.file_name.cmp(&right.file_name));

    Ok(MemoryExportReport {
        project_id: project_id.to_string(),
        project_root: policy.canonical_root().to_path_buf(),
        output_dir: canonical_target,
        files: exported,
    })
}

fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', '\0'])
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to stage memory export in {}", parent.display()))?;
    temp.write_all(bytes)
        .context("Failed to write memory export")?;
    temp.as_file()
        .sync_all()
        .context("Failed to flush memory export to disk")?;
    temp.persist(path).map_err(|error| {
        anyhow::anyhow!(
            "Failed to write memory export to {}: {error}",
            path.display()
        )
    })?;
    Ok(())
}
