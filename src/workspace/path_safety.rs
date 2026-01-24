//! Path safety utilities for workspace operations.
//!
//! Ensures user-provided paths cannot escape the workspace root via
//! path traversal attacks (e.g., using `..` components).

use std::env;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Validate and resolve a user-provided path relative to a workspace root.
///
/// This function:
/// - Handles paths starting with `.` as relative to current directory
/// - Treats other relative paths as relative to workspace root
/// - Rejects paths that would escape the workspace
/// - Returns the resolved absolute path
///
/// # Errors
///
/// Returns an error if:
/// - The resolved path is outside the workspace root
/// - The path cannot be resolved
///
/// # Example
///
/// ```ignore
/// // From workspace root:
/// let resolved = validate_workspace_path(&ws.root, "tools/repo")?;
/// // OK: resolved = /workspace/root/tools/repo
///
/// // From /workspace/root/infrastructure/tools with "./dotfiles":
/// let resolved = validate_workspace_path(&ws.root, "./dotfiles")?;
/// // OK: resolved = /workspace/root/infrastructure/tools/dotfiles
///
/// let err = validate_workspace_path(&ws.root, "../outside");
/// // Error: path escapes workspace
/// ```
pub fn validate_workspace_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let resolved = if path.is_absolute() {
        // Absolute path: use as-is but verify it's in workspace
        path.to_path_buf()
    } else {
        // Check if path starts with . or .. (relative to cwd)
        let first_component = path.components().next();
        let is_cwd_relative = matches!(
            first_component,
            Some(Component::CurDir) | Some(Component::ParentDir)
        );

        if is_cwd_relative {
            // Resolve relative to current working directory
            let cwd = env::current_dir().context("failed to get current directory")?;
            normalize_path(&cwd.join(path))
        } else {
            // Resolve relative to workspace root
            normalize_path(&root.join(path))
        }
    };

    // Verify the resolved path is within the workspace
    let canonical_root = normalize_path(root);
    if !resolved.starts_with(&canonical_root) {
        bail!(
            "path escapes workspace root: {} is not under {}",
            resolved.display(),
            canonical_root.display()
        );
    }

    Ok(resolved)
}

/// Normalize a path by resolving `.` and `..` components without requiring the path to exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Go up one level if possible
                normalized.pop();
            }
            Component::CurDir => {
                // Skip current dir markers
            }
            component => {
                normalized.push(component);
            }
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_valid_simple_path() {
        let dir = TempDir::new().unwrap();
        let result = validate_workspace_path(dir.path(), Path::new("tools/repo"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.path().join("tools/repo"));
    }

    #[test]
    fn test_valid_nested_path() {
        let dir = TempDir::new().unwrap();
        let result = validate_workspace_path(dir.path(), Path::new("research/2025/project/repo"));
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            dir.path().join("research/2025/project/repo")
        );
    }

    #[test]
    fn test_rejects_escape_via_dotdot() {
        let dir = TempDir::new().unwrap();
        // This path would escape the workspace
        let result = validate_workspace_path(dir.path(), Path::new("tools/../../outside"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("escapes"));
    }

    #[test]
    fn test_allows_dotdot_within_workspace() {
        let dir = TempDir::new().unwrap();
        // tools/../research stays within workspace
        let result = validate_workspace_path(dir.path(), Path::new("tools/../research/repo"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dir.path().join("research/repo"));
    }

    #[test]
    fn test_rejects_absolute_path_outside() {
        let dir = TempDir::new().unwrap();
        let result = validate_workspace_path(dir.path(), Path::new("/etc/passwd"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("escapes"));
    }

    #[test]
    fn test_allows_absolute_path_inside() {
        let dir = TempDir::new().unwrap();
        let inside = dir.path().join("tools/repo");
        let result = validate_workspace_path(dir.path(), &inside);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), inside);
    }

    #[test]
    fn test_cwd_relative_path() {
        // Create workspace structure
        let dir = TempDir::new().unwrap();
        // Canonicalize to handle macOS /tmp -> /private/tmp symlink
        let root = dir.path().canonicalize().unwrap();
        let subdir = root.join("infrastructure/tools");
        fs::create_dir_all(&subdir).unwrap();

        // Change to subdir and use ./dotfiles
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&subdir).unwrap();

        let result = validate_workspace_path(&root, Path::new("./dotfiles"));

        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok(), "expected ok, got {:?}", result);
        assert_eq!(
            result.unwrap(),
            root.join("infrastructure/tools/dotfiles")
        );
    }

    #[test]
    fn test_cwd_relative_parent() {
        // Create workspace structure
        let dir = TempDir::new().unwrap();
        // Canonicalize to handle macOS /tmp -> /private/tmp symlink
        let root = dir.path().canonicalize().unwrap();
        let subdir = root.join("infrastructure/tools");
        fs::create_dir_all(&subdir).unwrap();

        // Change to subdir and use ../dotfiles (goes to infrastructure/dotfiles)
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(&subdir).unwrap();

        let result = validate_workspace_path(&root, Path::new("../dotfiles"));

        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok(), "expected ok, got {:?}", result);
        assert_eq!(
            result.unwrap(),
            root.join("infrastructure/dotfiles")
        );
    }

    #[test]
    fn test_cwd_relative_escapes_workspace() {
        let dir = TempDir::new().unwrap();

        // Change to workspace root and use ../../outside
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let result = validate_workspace_path(dir.path(), Path::new("../../outside"));

        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("escapes"));
    }

    #[test]
    fn test_real_directory() {
        let dir = TempDir::new().unwrap();
        let result = validate_workspace_path(dir.path(), Path::new("tools/repo"));
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(dir.path()));
    }
}
