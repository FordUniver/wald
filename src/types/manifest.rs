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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DepthPolicy {
    /// Full clone (all history)
    #[serde(rename = "full")]
    #[default]
    Full,
    /// Shallow clone with N commits
    Depth(u32),
}

/// Partial clone filter policy
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilterPolicy {
    /// Full clone (no filter)
    #[default]
    None,
    /// Blobless clone: fetches commits and trees, blobs on demand
    /// Fast initial clone, good for worktree workflows
    BlobNone,
    /// Treeless clone: fetches only commits, trees and blobs on demand
    /// Even faster but more network requests when navigating history
    TreeZero,
}

impl FilterPolicy {
    /// Return the git --filter argument value, or None for full clone
    pub fn as_git_arg(&self) -> Option<&'static str> {
        match self {
            FilterPolicy::None => None,
            FilterPolicy::BlobNone => Some("blob:none"),
            FilterPolicy::TreeZero => Some("tree:0"),
        }
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

    /// Partial clone filter
    #[serde(default)]
    pub filter: FilterPolicy,

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
    /// Registered repositories keyed by repo_id (host/path)
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

    /// Resolve a reference to a repo ID
    ///
    /// Resolution order:
    /// 1. Exact repo ID match
    /// 2. Explicit alias match
    /// 3. Fuzzy match by repo name (last path segment)
    /// 4. Fuzzy match by owner/repo pattern
    ///
    /// Returns None if no match found, or if multiple matches (ambiguous).
    pub fn resolve_alias(&self, reference: &str) -> Option<&str> {
        // First check if it's a direct repo ID
        if let Some((repo_id, _)) = self.repos.get_key_value(reference) {
            return Some(repo_id.as_str());
        }

        // Then check explicit aliases
        for (repo_id, entry) in &self.repos {
            if entry.aliases.contains(&reference.to_string()) {
                return Some(repo_id.as_str());
            }
        }

        // Fuzzy resolution
        match self.resolve_fuzzy(reference) {
            FuzzyResult::Unique(repo_id) => Some(repo_id),
            FuzzyResult::Ambiguous(_) => None,
            FuzzyResult::None => None,
        }
    }

    /// Resolve a reference with detailed result for error messages
    ///
    /// Use this when you need to distinguish between "not found" and "ambiguous".
    pub fn resolve_with_details(&self, reference: &str) -> ResolveResult<'_> {
        // First check if it's a direct repo ID
        if let Some((repo_id, _)) = self.repos.get_key_value(reference) {
            return ResolveResult::Found(repo_id.as_str());
        }

        // Then check explicit aliases
        for (repo_id, entry) in &self.repos {
            if entry.aliases.contains(&reference.to_string()) {
                return ResolveResult::Found(repo_id.as_str());
            }
        }

        // Fuzzy resolution
        match self.resolve_fuzzy(reference) {
            FuzzyResult::Unique(repo_id) => ResolveResult::Found(repo_id),
            FuzzyResult::Ambiguous(matches) => ResolveResult::Ambiguous(matches),
            FuzzyResult::None => ResolveResult::NotFound,
        }
    }

    /// Fuzzy resolution by repo name or owner/repo pattern
    fn resolve_fuzzy(&self, reference: &str) -> FuzzyResult<'_> {
        let mut matches: Vec<&str> = Vec::new();

        // Check if reference looks like owner/repo pattern
        let parts: Vec<&str> = reference.split('/').collect();

        if parts.len() == 2 {
            // Owner/repo pattern: user/repo → github.com/user/repo
            let (owner, repo) = (parts[0], parts[1]);
            for repo_id in self.repos.keys() {
                let id_parts: Vec<&str> = repo_id.split('/').collect();
                // Match host/owner/repo where last two parts match
                if id_parts.len() >= 3
                    && id_parts[id_parts.len() - 2] == owner
                    && id_parts[id_parts.len() - 1] == repo
                {
                    matches.push(repo_id.as_str());
                }
            }
        } else if parts.len() == 1 && !reference.is_empty() {
            // Repo name only: dotfiles → github.com/user/dotfiles
            for repo_id in self.repos.keys() {
                let id_parts: Vec<&str> = repo_id.split('/').collect();
                if let Some(last) = id_parts.last()
                    && *last == reference
                {
                    matches.push(repo_id.as_str());
                }
            }
        }

        match matches.len() {
            0 => FuzzyResult::None,
            1 => FuzzyResult::Unique(matches[0]),
            _ => {
                matches.sort();
                FuzzyResult::Ambiguous(matches)
            }
        }
    }
}

/// Result of fuzzy resolution
enum FuzzyResult<'a> {
    Unique(&'a str),
    Ambiguous(Vec<&'a str>),
    None,
}

/// Detailed result of resolve_with_details
pub enum ResolveResult<'a> {
    Found(&'a str),
    Ambiguous(Vec<&'a str>),
    NotFound,
}

/// Entry for a worktree in a baum manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeEntry {
    /// Branch name (the logical branch, e.g., "main")
    pub branch: String,
    /// Relative path (e.g., "_main.wt")
    pub path: String,
    /// Local tracking branch name (e.g., "wald/abc123/main")
    /// None for legacy worktrees that check out the remote branch directly
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_branch: Option<String>,
}

/// Baum manifest (container/.baum/manifest.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaumManifest {
    /// Unique baum ID (6-char hex)
    /// None for legacy baums; generated on first modification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
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

    /// Add a worktree entry (legacy style, no local tracking branch)
    pub fn add_worktree(&mut self, branch: &str, path: &str) {
        self.worktrees.push(WorktreeEntry {
            branch: branch.to_string(),
            path: path.to_string(),
            local_branch: None,
        });
    }

    /// Add a worktree entry with a local tracking branch
    pub fn add_worktree_with_local(&mut self, branch: &str, path: &str, local_branch: &str) {
        self.worktrees.push(WorktreeEntry {
            branch: branch.to_string(),
            path: path.to_string(),
            local_branch: Some(local_branch.to_string()),
        });
    }

    /// Get or generate the baum ID
    ///
    /// If the baum has no ID yet, generates one using the provided set
    /// of existing IDs to avoid collisions.
    pub fn ensure_id(&mut self, existing_ids: &std::collections::HashSet<String>) -> &str {
        if self.id.is_none() {
            self.id = Some(crate::id::generate_baum_id(existing_ids));
        }
        self.id.as_ref().unwrap()
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
                filter: FilterPolicy::BlobNone,
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
            id: Some("abc123".to_string()),
            repo_id: "github.com/user/repo".to_string(),
            worktrees: vec![],
        };
        baum.add_worktree("main", "_main.wt");
        baum.add_worktree_with_local("dev", "_dev.wt", "wald/abc123/dev");

        let yaml = serde_yml::to_string(&baum).unwrap();
        let parsed: BaumManifest = serde_yml::from_str(&yaml).unwrap();

        assert_eq!(parsed.id, Some("abc123".to_string()));
        assert_eq!(parsed.repo_id, "github.com/user/repo");
        assert_eq!(parsed.worktrees.len(), 2);
        assert_eq!(parsed.worktrees[0].branch, "main");
        assert_eq!(parsed.worktrees[0].local_branch, None);
        assert_eq!(
            parsed.worktrees[1].local_branch,
            Some("wald/abc123/dev".to_string())
        );
    }

    #[test]
    fn test_baum_manifest_legacy_compat() {
        // Legacy manifests without id or local_branch should still parse
        let yaml = r#"
repo_id: github.com/user/repo
worktrees:
  - branch: main
    path: _main.wt
"#;
        let parsed: BaumManifest = serde_yml::from_str(yaml).unwrap();
        assert_eq!(parsed.id, None);
        assert_eq!(parsed.worktrees[0].local_branch, None);
    }

    #[test]
    fn test_baum_ensure_id() {
        use std::collections::HashSet;

        let mut baum = BaumManifest {
            id: None,
            repo_id: "github.com/user/repo".to_string(),
            worktrees: vec![],
        };

        let existing = HashSet::new();
        let id = baum.ensure_id(&existing).to_string();
        assert_eq!(id.len(), 6);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));

        // Calling again should return the same ID
        let id2 = baum.ensure_id(&existing).to_string();
        assert_eq!(id, id2);
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
        assert!(resolved_id == "github.com/user/repo1" || resolved_id == "github.com/user/repo2");
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
        manifest
            .repos
            .insert("github.com/user/repo".to_string(), RepoEntry::default());

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

    // Fuzzy resolution tests

    #[test]
    fn test_fuzzy_resolve_by_repo_name() {
        let mut manifest = Manifest::default();
        manifest
            .repos
            .insert("github.com/user/dotfiles".to_string(), RepoEntry::default());
        manifest
            .repos
            .insert("github.com/user/other".to_string(), RepoEntry::default());

        // Resolve by repo name only
        assert_eq!(
            manifest.resolve_alias("dotfiles"),
            Some("github.com/user/dotfiles")
        );
        assert_eq!(
            manifest.resolve_alias("other"),
            Some("github.com/user/other")
        );
    }

    #[test]
    fn test_fuzzy_resolve_by_owner_repo() {
        let mut manifest = Manifest::default();
        manifest
            .repos
            .insert("github.com/alice/repo".to_string(), RepoEntry::default());
        manifest
            .repos
            .insert("gitlab.com/bob/repo".to_string(), RepoEntry::default());

        // Resolve by owner/repo pattern
        assert_eq!(
            manifest.resolve_alias("alice/repo"),
            Some("github.com/alice/repo")
        );
        assert_eq!(
            manifest.resolve_alias("bob/repo"),
            Some("gitlab.com/bob/repo")
        );
    }

    #[test]
    fn test_fuzzy_resolve_ambiguous_name_returns_none() {
        let mut manifest = Manifest::default();
        manifest
            .repos
            .insert("github.com/alice/repo".to_string(), RepoEntry::default());
        manifest
            .repos
            .insert("gitlab.com/bob/repo".to_string(), RepoEntry::default());

        // Both have same repo name "repo" - ambiguous
        assert_eq!(manifest.resolve_alias("repo"), None);
    }

    #[test]
    fn test_fuzzy_resolve_with_details_ambiguous() {
        let mut manifest = Manifest::default();
        manifest
            .repos
            .insert("github.com/alice/repo".to_string(), RepoEntry::default());
        manifest
            .repos
            .insert("gitlab.com/bob/repo".to_string(), RepoEntry::default());

        // Check detailed result for ambiguous match
        match manifest.resolve_with_details("repo") {
            super::ResolveResult::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2);
                assert!(matches.contains(&"github.com/alice/repo"));
                assert!(matches.contains(&"gitlab.com/bob/repo"));
            }
            _ => panic!("Expected ambiguous result"),
        }
    }

    #[test]
    fn test_fuzzy_resolve_explicit_alias_takes_precedence() {
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "github.com/user/dotfiles".to_string(),
            RepoEntry {
                aliases: vec!["dots".to_string()],
                ..Default::default()
            },
        );
        manifest
            .repos
            .insert("github.com/other/dots".to_string(), RepoEntry::default());

        // Explicit alias "dots" should resolve to first repo, not fuzzy to "dots" repo
        assert_eq!(
            manifest.resolve_alias("dots"),
            Some("github.com/user/dotfiles")
        );
    }

    #[test]
    fn test_fuzzy_resolve_subgroup_repos() {
        let mut manifest = Manifest::default();
        manifest.repos.insert(
            "git.zib.de/cspiegel/group/repo".to_string(),
            RepoEntry::default(),
        );

        // Resolve by owner/repo should match last two segments
        assert_eq!(
            manifest.resolve_alias("group/repo"),
            Some("git.zib.de/cspiegel/group/repo")
        );
        // Just repo name
        assert_eq!(
            manifest.resolve_alias("repo"),
            Some("git.zib.de/cspiegel/group/repo")
        );
    }
}
