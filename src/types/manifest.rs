use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// LFS fetch policy
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LfsPolicy {
    /// Fetch all LFS objects
    Full,
    /// Fetch only pointer files, pull objects on demand
    #[default]
    Minimal,
    /// Skip LFS entirely
    Skip,
}

/// Clone depth policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepthPolicy {
    /// Full clone (all history)
    #[serde(rename = "full")]
    Full,
    /// Shallow clone with N commits
    Depth(u32),
}

impl Default for DepthPolicy {
    fn default() -> Self {
        Self::Depth(100)
    }
}

/// Entry for a single repository in the manifest
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoEntry {
    /// LFS fetch policy
    #[serde(default)]
    pub lfs: LfsPolicy,

    /// Clone depth
    #[serde(default)]
    pub depth: DepthPolicy,

    /// Upstream repo ID for fork tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,

    /// Short aliases for this repo
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// Central manifest (.wald/manifest.yaml)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// Registered repositories keyed by repo_id (host/owner/name)
    #[serde(default)]
    pub repos: HashMap<String, RepoEntry>,
}

impl Manifest {
    /// Load manifest from a YAML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest: {}", path.display()))?;
        let manifest: Manifest = serde_yml::from_str(&content)
            .with_context(|| format!("failed to parse manifest: {}", path.display()))?;
        Ok(manifest)
    }

    /// Save manifest to a YAML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yml::to_string(self).context("failed to serialize manifest")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write manifest: {}", path.display()))?;
        Ok(())
    }

    /// Check if a repo ID exists in the manifest
    pub fn has_repo(&self, repo_id: &str) -> bool {
        self.repos.contains_key(repo_id)
    }

    /// Resolve an alias to a repo ID
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        // First check if it's a direct repo ID
        if let Some((repo_id, _)) = self.repos.get_key_value(alias) {
            return Some(repo_id.as_str());
        }

        // Then check aliases
        for (repo_id, entry) in &self.repos {
            if entry.aliases.contains(&alias.to_string()) {
                return Some(repo_id.as_str());
            }
        }

        None
    }
}

/// Entry for a worktree in a baum manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeEntry {
    /// Branch name
    pub branch: String,
    /// Relative path (e.g., "_main.wt")
    pub path: String,
}

/// Baum manifest (container/.baum/manifest.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaumManifest {
    /// Repository ID this baum is linked to
    pub repo_id: String,
    /// Worktrees in this baum
    #[serde(default)]
    pub worktrees: Vec<WorktreeEntry>,
}

impl BaumManifest {
    /// Load baum manifest from a YAML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read baum manifest: {}", path.display()))?;
        let manifest: BaumManifest = serde_yml::from_str(&content)
            .with_context(|| format!("failed to parse baum manifest: {}", path.display()))?;
        Ok(manifest)
    }

    /// Save baum manifest to a YAML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yml::to_string(self).context("failed to serialize baum manifest")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write baum manifest: {}", path.display()))?;
        Ok(())
    }

    /// Add a worktree entry
    pub fn add_worktree(&mut self, branch: &str, path: &str) {
        self.worktrees.push(WorktreeEntry {
            branch: branch.to_string(),
            path: path.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/repo".to_string(),
            RepoEntry {
                lfs: LfsPolicy::Full,
                depth: DepthPolicy::Depth(50),
                upstream: None,
                aliases: vec!["repo".to_string()],
            },
        );

        let yaml = serde_yml::to_string(&manifest).unwrap();
        let parsed: Manifest = serde_yml::from_str(&yaml).unwrap();

        assert!(parsed.repos.contains_key("github.com/user/repo"));
        assert_eq!(parsed.repos["github.com/user/repo"].lfs, LfsPolicy::Full);
    }

    #[test]
    fn test_resolve_alias() {
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/dotfiles".to_string(),
            RepoEntry {
                aliases: vec!["dots".to_string(), "dotfiles".to_string()],
                ..Default::default()
            },
        );

        assert_eq!(
            manifest.resolve_alias("dots"),
            Some("github.com/user/dotfiles")
        );
        assert_eq!(
            manifest.resolve_alias("github.com/user/dotfiles"),
            Some("github.com/user/dotfiles")
        );
        assert_eq!(manifest.resolve_alias("unknown"), None);
    }

    #[test]
    fn test_baum_manifest_roundtrip() {
        let mut baum = BaumManifest {
            repo_id: "github.com/user/repo".to_string(),
            worktrees: vec![],
        };
        baum.add_worktree("main", "_main.wt");
        baum.add_worktree("dev", "_dev.wt");

        let yaml = serde_yml::to_string(&baum).unwrap();
        let parsed: BaumManifest = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.repo_id, "github.com/user/repo");
        assert_eq!(parsed.worktrees.len(), 2);
        assert_eq!(parsed.worktrees[0].branch, "main");
    }

    // Edge case tests for alias resolution

    #[test]
    fn test_resolve_alias_nonexistent() {
        // Alias that doesn't exist should return None
        let manifest = Manifest::default();
        assert_eq!(manifest.resolve_alias("nonexistent"), None);
    }

    #[test]
    fn test_resolve_alias_collision_returns_first_match() {
        // When same alias is in multiple repos, returns first match found
        // (HashMap iteration order is non-deterministic, but should still return something)
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/repo1".to_string(),
            RepoEntry {
                aliases: vec!["shared".to_string()],
                ..Default::default()
            },
        );
        manifest.repos.insert(
            "github.com/user/repo2".to_string(),
            RepoEntry {
                aliases: vec!["shared".to_string()],
                ..Default::default()
            },
        );

        // Should resolve to one of them
        let resolved = manifest.resolve_alias("shared");
        assert!(resolved.is_some());
        let resolved_id = resolved.unwrap();
        assert!(
            resolved_id == "github.com/user/repo1" || resolved_id == "github.com/user/repo2"
        );
    }

    #[test]
    fn test_manifest_empty_repos() {
        // Empty repos map should work fine
        let yaml = "repos: {}";
        let manifest: Manifest = serde_yml::from_str(yaml).unwrap();
        assert!(manifest.repos.is_empty());
        assert!(!manifest.has_repo("anything"));
    }

    #[test]
    fn test_manifest_missing_repos_key() {
        // Missing repos key should use default (empty)
        let yaml = "";
        let manifest: Manifest = serde_yml::from_str(yaml).unwrap();
        assert!(manifest.repos.is_empty());
    }

    #[test]
    fn test_has_repo_with_direct_match() {
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/repo".to_string(),
            RepoEntry::default(),
        );

        assert!(manifest.has_repo("github.com/user/repo"));
        assert!(!manifest.has_repo("github.com/other/repo"));
    }

    #[test]
    fn test_resolve_alias_prefers_direct_repo_id() {
        // If the input is both a repo ID and an alias, prefer repo ID
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/dotfiles".to_string(),
            RepoEntry {
                aliases: vec!["dots".to_string()],
                ..Default::default()
            },
        );
        manifest.repos.insert(
            "github.com/other/repo".to_string(),
            RepoEntry {
                aliases: vec!["github.com/user/dotfiles".to_string()], // Weird but possible
                ..Default::default()
            },
        );

        // Direct repo ID takes precedence
        assert_eq!(
            manifest.resolve_alias("github.com/user/dotfiles"),
            Some("github.com/user/dotfiles")
        );
    }
}
