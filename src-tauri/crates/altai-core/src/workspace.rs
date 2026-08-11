//! Workspace path resolution shared by the desktop and terminal adapters.

use std::fmt;
use std::path::{Path, PathBuf};

/// Canonical locations owned by a single ALTAI workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    /// The canonical project directory.
    pub root: PathBuf,
    /// IsanAgent's durable state directory for this workspace.
    pub isanagent_state: PathBuf,
}

impl WorkspacePaths {
    /// Path to the durable SQLite event journal shared by every host adapter
    /// (desktop and CLI) that runs agent turns for this workspace.
    pub fn agent_event_journal_db(&self) -> PathBuf {
        self.isanagent_state
            .join(".system_generated")
            .join("agent_event_journal.db")
    }

    /// User/workspace Work OS database (projects, work_items, attempts, reviews).
    pub fn work_db(&self) -> PathBuf {
        self.isanagent_state
            .join(".system_generated")
            .join("work.db")
    }
}

/// A user-correctable workspace resolution failure.
#[derive(Debug)]
pub enum WorkspaceError {
    CurrentDirectory(std::io::Error),
    Missing(PathBuf),
    Canonicalize {
        path: PathBuf,
        source: std::io::Error,
    },
    NoParent(PathBuf),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirectory(error) => {
                write!(f, "could not determine the current directory: {error}")
            }
            Self::Missing(path) => write!(f, "workspace path does not exist: {}", path.display()),
            Self::Canonicalize { path, source } => write!(
                f,
                "could not resolve workspace {}: {source}",
                path.display()
            ),
            Self::NoParent(path) => write!(
                f,
                "could not determine a workspace directory for {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for WorkspaceError {}

/// Resolve a user-provided path, accepting a file by selecting its parent.
pub fn resolve_workspace(path: Option<&Path>) -> Result<WorkspacePaths, WorkspaceError> {
    let current = std::env::current_dir().map_err(WorkspaceError::CurrentDirectory)?;
    resolve_workspace_from(path, &current)
}

/// Resolve a workspace with a caller-supplied current directory.
///
/// This is public to keep deterministic tests and desktop adapters free from
/// process-global current-directory mutation.
pub fn resolve_workspace_from(
    path: Option<&Path>,
    current_dir: &Path,
) -> Result<WorkspacePaths, WorkspaceError> {
    let candidate = match path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => current_dir.join(path),
        None => current_dir.to_path_buf(),
    };
    if !candidate.exists() {
        return Err(WorkspaceError::Missing(candidate));
    }

    let directory = if candidate.is_dir() {
        candidate.as_path()
    } else {
        candidate
            .parent()
            .ok_or_else(|| WorkspaceError::NoParent(candidate.clone()))?
    };
    let root = directory
        .canonicalize()
        .map_err(|source| WorkspaceError::Canonicalize {
            path: directory.to_path_buf(),
            source,
        })?;

    Ok(WorkspacePaths {
        isanagent_state: root.join(".isanagent"),
        root,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn directory_resolves_to_canonical_root_and_state_directory() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let paths = resolve_workspace_from(Some(temp.path()), Path::new("/unused"))
            .expect("workspace should resolve");

        assert_eq!(paths.root, temp.path().canonicalize().unwrap());
        assert_eq!(paths.isanagent_state, paths.root.join(".isanagent"));
    }

    #[test]
    fn file_resolves_to_its_parent_workspace() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let file = temp.path().join("README.md");
        fs::write(&file, "ALTAI").expect("fixture file");

        let paths = resolve_workspace_from(Some(&file), Path::new("/unused"))
            .expect("parent workspace should resolve");
        assert_eq!(paths.root, temp.path().canonicalize().unwrap());
    }

    #[test]
    fn relative_file_resolves_from_the_supplied_current_directory() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let file = temp.path().join("README.md");
        fs::write(&file, "ALTAI").expect("fixture file");

        let paths = resolve_workspace_from(Some(Path::new("README.md")), temp.path())
            .expect("relative parent workspace should resolve");
        assert_eq!(paths.root, temp.path().canonicalize().unwrap());
    }

    #[test]
    fn agent_event_journal_db_lives_under_system_generated_state() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let paths = resolve_workspace_from(Some(temp.path()), Path::new("/unused"))
            .expect("workspace should resolve");

        assert_eq!(
            paths.agent_event_journal_db(),
            paths
                .isanagent_state
                .join(".system_generated")
                .join("agent_event_journal.db")
        );
        assert_eq!(
            paths.work_db(),
            paths
                .isanagent_state
                .join(".system_generated")
                .join("work.db")
        );
    }

    #[test]
    fn missing_path_is_user_correctable_error() {
        let missing = Path::new("/this/path/does/not/exist");
        assert!(matches!(
            resolve_workspace_from(Some(missing), Path::new("/unused")),
            Err(WorkspaceError::Missing(path)) if path == missing
        ));
    }
}
