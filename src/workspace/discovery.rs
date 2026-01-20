use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::types::{Config, Manifest, SyncState};

/// The wald directory name
pub const WALD_DIR: &str = ".wald";

/// Find the workspace root by walking up from the current directory
/// Returns the directory containing .wald/
pub fn find_workspace_root() -> Result<PathBuf> {
    let current = env::current_dir().context("failed to get current directory")?;
    find_workspace_root_from(&current)
}

/// Find the workspace root starting from a specific directory
pub fn find_workspace_root_from(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let wald_dir = current.join(WALD_DIR);
        if wald_dir.is_dir() {
            return Ok(current);
        }

        if !current.pop() {
            bail!(
                "not a wald workspace (no .wald/ directory found in {} or any parent)",
                start.display()
            );
        }
    }
}

/// Workspace context holding paths and loaded configurations
#[derive(Debug)]
pub struct Workspace {
    /// Root directory containing .wald/
    pub root: PathBuf,
    /// Central manifest
    pub manifest: Manifest,
    /// Workspace config
    pub config: Config,
    /// Sync state
    pub state: SyncState,
}

impl Workspace {
    /// Load workspace from the current directory
    pub fn load() -> Result<Self> {
        let root = find_workspace_root()?;
        Self::load_from(root)
    }

    /// Load workspace from a specific root
    pub fn load_from(root: PathBuf) -> Result<Self> {
        let wald_dir = root.join(WALD_DIR);

        let manifest = Manifest::load(&wald_dir.join("manifest.yaml"))
            .context("failed to load manifest")?;

        let config = Config::load(&wald_dir.join("config.yaml"))
            .unwrap_or_default();

        let state = SyncState::load(&wald_dir.join("state.yaml"))
            .unwrap_or_default();

        Ok(Self {
            root,
            manifest,
            config,
            state,
        })
    }

    /// Get the .wald directory path
    pub fn wald_dir(&self) -> PathBuf {
        self.root.join(WALD_DIR)
    }

    /// Get the repos directory path (.wald/repos/)
    pub fn repos_dir(&self) -> PathBuf {
        self.wald_dir().join("repos")
    }

    /// Get the manifest file path
    pub fn manifest_path(&self) -> PathBuf {
        self.wald_dir().join("manifest.yaml")
    }

    /// Get the state file path
    pub fn state_path(&self) -> PathBuf {
        self.wald_dir().join("state.yaml")
    }

    /// Save manifest to disk
    pub fn save_manifest(&self) -> Result<()> {
        self.manifest.save(&self.manifest_path())
    }

    /// Save state to disk
    pub fn save_state(&self) -> Result<()> {
        self.state.save(&self.state_path())
    }

    /// Get the bare repo path for a repo ID
    pub fn bare_repo_path(&self, repo_id: &str) -> Result<PathBuf> {
        let id = crate::types::RepoId::parse(repo_id)?;
        Ok(self.repos_dir().join(id.to_bare_path()))
    }

    /// Check if a bare repo exists
    pub fn has_bare_repo(&self, repo_id: &str) -> bool {
        self.bare_repo_path(repo_id)
            .map(|p| p.is_dir())
            .unwrap_or(false)
    }

    /// Resolve a repo reference (ID or alias) to a repo ID
    pub fn resolve_repo(&self, reference: &str) -> Option<&str> {
        self.manifest.resolve_alias(reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace() -> TempDir {
        let dir = TempDir::new().unwrap();
        let wald = dir.path().join(".wald");
        fs::create_dir_all(&wald).unwrap();
        fs::write(wald.join("manifest.yaml"), "repos: {}").unwrap();
        fs::write(wald.join("config.yaml"), "default_lfs: minimal\ndefault_depth: 100").unwrap();
        dir
    }

    #[test]
    fn test_find_workspace_root() {
        let dir = setup_workspace();
        let root = find_workspace_root_from(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn test_find_workspace_root_from_subdir() {
        let dir = setup_workspace();
        let subdir = dir.path().join("sub/deep/dir");
        fs::create_dir_all(&subdir).unwrap();
        let root = find_workspace_root_from(&subdir).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn test_workspace_load() {
        let dir = setup_workspace();
        let ws = Workspace::load_from(dir.path().to_path_buf()).unwrap();
        assert_eq!(ws.root, dir.path());
        assert!(ws.manifest.repos.is_empty());
    }
}
