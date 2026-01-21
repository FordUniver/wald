//! Path safety utilities for workspace operations.
//!
//! Ensures user-provided paths cannot escape the workspace root via
//! path traversal attacks (e.g., using `..` components).

use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};

/// Validate and resolve a user-provided path relative to a workspace root.
///
/// This function:
/// - Rejects absolute paths
/// - Rejects paths containing `..` components
/// - Returns the resolved path (root.join(path))
///
/// # Errors
///
/// Returns an error if:
/// - The path is absolute
/// - The path contains `..` components
///
/// # Example
///
/// ```ignore
/// let resolved = validate_workspace_path(&ws.root, "tools/repo")?;
/// // OK: resolved = /workspace/root/tools/repo
///
/// let err = validate_workspace_path(&ws.root, "../outside");
/// // Error: path contains '..' component
/// ```
pub fn validate_workspace_path(root: &Path, path: &Path) -> Result<PathBuf> {
    // Reject absolute paths
    if path.is_absolute() {
        bail!(
            "path must be relative to workspace root, got absolute path: {}",
            path.display()
        );
    }

    // Check for .. components
    for component in path.components() {
        match component {
            Component::ParentDir => {
                bail!(
                    "path contains '..' component which could escape workspace: {}",
                    path.display()
                );
            }
            Component::Normal(_)
            | Component::CurDir
            | Component::RootDir
            | Component::Prefix(_) => {
                // Normal components and . are fine
            }
        }
    }

    // Return the resolved path
    Ok(root.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_valid_simple_path() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("tools/repo"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/tools/repo"));
    }

    #[test]
    fn test_valid_nested_path() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("research/2025/project/repo"));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PathBuf::from("/workspace/research/2025/project/repo")
        );
    }

    #[test]
    fn test_rejects_dotdot() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("../outside"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".."));
    }

    #[test]
    fn test_rejects_dotdot_in_middle() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("tools/../../../etc/passwd"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".."));
    }

    #[test]
    fn test_rejects_dotdot_at_end() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("tools/repo/.."));
        assert!(result.is_err());
    }

    #[test]
    fn test_rejects_absolute_path() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("/etc/passwd"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("absolute"));
    }

    #[test]
    fn test_allows_single_dot() {
        let root = PathBuf::from("/workspace");
        let result = validate_workspace_path(&root, Path::new("./tools/repo"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_real_directory() {
        let dir = TempDir::new().unwrap();
        let result = validate_workspace_path(dir.path(), Path::new("tools/repo"));
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(dir.path()));
    }
}
