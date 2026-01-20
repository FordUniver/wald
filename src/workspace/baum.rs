use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::types::BaumManifest;

/// The baum directory name within a container
pub const BAUM_DIR: &str = ".baum";

/// Check if a directory is a baum (has .baum/ subdirectory)
pub fn is_baum(path: &Path) -> bool {
    path.join(BAUM_DIR).is_dir()
}

/// Create a new baum in a container directory
/// Returns the BaumManifest for the new baum
pub fn create_baum(container: &Path, repo_id: &str) -> Result<BaumManifest> {
    // Check if container exists
    if container.exists() {
        // Check if it's a directory
        if !container.is_dir() {
            bail!(
                "container path exists but is not a directory: {}",
                container.display()
            );
        }

        // Check if already a baum
        if is_baum(container) {
            bail!(
                "baum already planted at {}: .baum directory exists",
                container.display()
            );
        }
    }

    // Create container and .baum directory
    let baum_dir = container.join(BAUM_DIR);
    fs::create_dir_all(&baum_dir)
        .with_context(|| format!("failed to create baum directory: {}", baum_dir.display()))?;

    // Create initial manifest
    let manifest = BaumManifest {
        repo_id: repo_id.to_string(),
        worktrees: vec![],
    };

    // Save manifest
    manifest.save(&baum_dir.join("manifest.yaml"))?;

    Ok(manifest)
}

/// Load a baum manifest from a container directory
pub fn load_baum(container: &Path) -> Result<BaumManifest> {
    let manifest_path = container.join(BAUM_DIR).join("manifest.yaml");
    BaumManifest::load(&manifest_path)
}

/// Save a baum manifest to a container directory
pub fn save_baum(container: &Path, manifest: &BaumManifest) -> Result<()> {
    let manifest_path = container.join(BAUM_DIR).join("manifest.yaml");
    manifest.save(&manifest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_baum() {
        let dir = TempDir::new().unwrap();

        // Not a baum initially
        assert!(!is_baum(dir.path()));

        // Create .baum directory
        fs::create_dir(dir.path().join(".baum")).unwrap();
        assert!(is_baum(dir.path()));
    }

    #[test]
    fn test_create_baum() {
        let dir = TempDir::new().unwrap();
        let container = dir.path().join("my-baum");

        let manifest = create_baum(&container, "github.com/user/repo").unwrap();

        assert_eq!(manifest.repo_id, "github.com/user/repo");
        assert!(manifest.worktrees.is_empty());
        assert!(is_baum(&container));
        assert!(container.join(".baum/manifest.yaml").exists());
    }

    #[test]
    fn test_create_baum_fails_if_exists() {
        let dir = TempDir::new().unwrap();
        let container = dir.path().join("my-baum");

        // Create first baum
        create_baum(&container, "github.com/user/repo").unwrap();

        // Second create should fail
        let result = create_baum(&container, "github.com/user/other");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_baum_fails_if_file_exists() {
        let dir = TempDir::new().unwrap();
        let container = dir.path().join("my-baum");

        // Create as file
        fs::write(&container, "not a directory").unwrap();

        let result = create_baum(&container, "github.com/user/repo");
        assert!(result.is_err());
    }
}
