use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::types::{BaumManifest, Config, Manifest, SyncState};
use crate::workspace::baum::{BAUM_DIR, is_baum, load_baum};
use crate::workspace::gitignore::ensure_gitignore_section;

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

        let manifest =
            Manifest::load(&wald_dir.join("manifest.yaml")).context("failed to load manifest")?;

        let config = Config::load(&wald_dir.join("config.yaml")).unwrap_or_default();

        let state = SyncState::load(&wald_dir.join("state.yaml")).unwrap_or_default();

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

    /// Initialize a new workspace at the given path
    ///
    /// Creates the .wald/ directory structure with:
    /// - manifest.yaml (empty repos)
    /// - config.yaml (default settings)
    /// - state.yaml (null last_sync)
    /// - repos/ directory
    ///
    /// Also adds wald-managed section to .gitignore
    pub fn init(root: &Path, force: bool) -> Result<()> {
        let wald_dir = root.join(WALD_DIR);

        // Check if we're inside an existing wald workspace (no nesting allowed)
        if let Ok(existing_root) = find_workspace_root_from(root) {
            if existing_root != root {
                bail!(
                    "cannot create nested workspace: already inside workspace at {}",
                    existing_root.display()
                );
            }
            // existing_root == root means .wald/ exists at this location
            if !force {
                bail!(
                    "workspace already exists at {} (use --force to recreate)",
                    root.display()
                );
            }
        }

        // Handle existing .wald/ directory
        if wald_dir.exists() {
            if !force {
                bail!(
                    ".wald/ already exists at {} (use --force to recreate)",
                    root.display()
                );
            }
            // Remove existing .wald/ for recreation
            fs::remove_dir_all(&wald_dir).with_context(|| {
                format!("failed to remove existing .wald/: {}", wald_dir.display())
            })?;
        }

        // Create .wald/ directory structure
        fs::create_dir_all(&wald_dir)
            .with_context(|| format!("failed to create .wald/: {}", wald_dir.display()))?;

        fs::create_dir_all(wald_dir.join("repos"))
            .with_context(|| "failed to create .wald/repos/")?;

        // Create manifest.yaml with empty repos
        let manifest = Manifest::default();
        manifest.save(&wald_dir.join("manifest.yaml"))?;

        // Create config.yaml with defaults
        let config = Config::default();
        config.save(&wald_dir.join("config.yaml"))?;

        // Create state.yaml
        let state = SyncState::default();
        state.save(&wald_dir.join("state.yaml"))?;

        // Add wald-managed section to .gitignore
        ensure_gitignore_section(root)?;

        Ok(())
    }

    /// Check if a directory is a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").exists()
    }

    /// Find all baums in the workspace
    ///
    /// Returns a list of (path, manifest) pairs for all discovered baums.
    pub fn find_all_baums(&self) -> Vec<(PathBuf, BaumManifest)> {
        find_all_baums(&self.root)
    }

    /// Collect all baum IDs in the workspace
    ///
    /// Returns a set of IDs for all baums that have them assigned.
    pub fn collect_baum_ids(&self) -> HashSet<String> {
        collect_baum_ids(&self.root)
    }
}

/// Find all baums in a workspace directory
///
/// Returns a list of (path, manifest) pairs for all discovered baums.
pub fn find_all_baums(workspace_root: &Path) -> Vec<(PathBuf, BaumManifest)> {
    let mut baums = Vec::new();

    for entry in WalkDir::new(workspace_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip .git directories, .wald/repos, and _*.wt worktree directories
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return false;
            }
            if name == "repos"
                && e.path()
                    .parent()
                    .map(|p| p.ends_with(".wald"))
                    .unwrap_or(false)
            {
                return false;
            }
            // Skip worktree directories (no need to descend into them)
            if e.file_type().is_dir() && name.starts_with('_') && name.ends_with(".wt") {
                return false;
            }
            // Skip .baum directories themselves
            if name == BAUM_DIR {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_dir()
            && is_baum(entry.path())
            && let Ok(manifest) = load_baum(entry.path())
        {
            baums.push((entry.path().to_path_buf(), manifest));
        }
    }

    baums
}

/// Collect all baum IDs in a workspace directory
///
/// Returns a set of IDs for all baums that have them assigned.
pub fn collect_baum_ids(workspace_root: &Path) -> HashSet<String> {
    find_all_baums(workspace_root)
        .into_iter()
        .filter_map(|(_, manifest)| manifest.id)
        .collect()
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
        fs::write(
            wald.join("config.yaml"),
            "default_lfs: minimal\ndefault_depth: 100",
        )
        .unwrap();
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

    #[test]
    fn test_workspace_init_creates_structure() {
        let dir = TempDir::new().unwrap();

        Workspace::init(dir.path(), false).unwrap();

        // Verify .wald/ structure
        assert!(dir.path().join(".wald").exists());
        assert!(dir.path().join(".wald/repos").exists());
        assert!(dir.path().join(".wald/manifest.yaml").exists());
        assert!(dir.path().join(".wald/config.yaml").exists());
        assert!(dir.path().join(".wald/state.yaml").exists());
        assert!(dir.path().join(".gitignore").exists());

        // Verify we can load the workspace
        let ws = Workspace::load_from(dir.path().to_path_buf()).unwrap();
        assert!(ws.manifest.repos.is_empty());
    }

    #[test]
    fn test_workspace_init_fails_without_force() {
        let dir = TempDir::new().unwrap();

        // First init succeeds
        Workspace::init(dir.path(), false).unwrap();

        // Second init fails without force
        let result = Workspace::init(dir.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_workspace_init_with_force() {
        let dir = TempDir::new().unwrap();

        // First init
        Workspace::init(dir.path(), false).unwrap();

        // Add a marker file
        fs::write(dir.path().join(".wald/marker.txt"), "test").unwrap();
        assert!(dir.path().join(".wald/marker.txt").exists());

        // Force reinit
        Workspace::init(dir.path(), true).unwrap();

        // Marker should be gone
        assert!(!dir.path().join(".wald/marker.txt").exists());

        // But structure should be valid
        assert!(dir.path().join(".wald/manifest.yaml").exists());
    }

    #[test]
    fn test_workspace_init_no_nesting() {
        let dir = TempDir::new().unwrap();

        // Create parent workspace
        Workspace::init(dir.path(), false).unwrap();

        // Try to create child workspace
        let child = dir.path().join("child");
        fs::create_dir_all(&child).unwrap();

        let result = Workspace::init(&child, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nested"));
    }

    #[test]
    fn test_is_git_repo() {
        let dir = TempDir::new().unwrap();

        // Not a git repo initially
        assert!(!Workspace::is_git_repo(dir.path()));

        // Create .git directory
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        assert!(Workspace::is_git_repo(dir.path()));
    }
}
