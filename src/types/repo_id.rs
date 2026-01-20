use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use thiserror::Error;

/// Canonical repository identifier: host/path/to/repo
///
/// Supports arbitrary path depth for GitLab subgroups:
/// - `github.com/user/repo` (traditional 3-segment)
/// - `git.zib.de/iol/research/project` (GitLab subgroups)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoId {
    /// Host part (e.g., "github.com", "git.zib.de")
    pub host: String,
    /// Path segments after the host (at least one required)
    /// e.g., ["user", "repo"] or ["iol", "research", "project"]
    pub path: Vec<String>,
}

#[derive(Error, Debug)]
pub enum RepoIdError {
    #[error("invalid repo ID format: expected host/path/to/repo, got '{0}'")]
    InvalidFormat(String),
    #[error("empty component in repo ID: '{0}'")]
    EmptyComponent(String),
    #[error("repo ID requires at least host and one path segment: '{0}'")]
    TooFewSegments(String),
}

impl RepoId {
    /// Parse a repo ID from a string like "github.com/user/repo" or "git.zib.de/iol/research/project"
    pub fn parse(s: &str) -> Result<Self, RepoIdError> {
        let parts: Vec<&str> = s.split('/').collect();

        if parts.len() < 2 {
            return Err(RepoIdError::TooFewSegments(s.to_string()));
        }

        let host = parts[0].trim();
        if host.is_empty() {
            return Err(RepoIdError::EmptyComponent(s.to_string()));
        }

        let path: Vec<String> = parts[1..]
            .iter()
            .map(|p| p.trim().to_string())
            .collect();

        // Verify no empty path segments
        if path.iter().any(|p| p.is_empty()) {
            return Err(RepoIdError::EmptyComponent(s.to_string()));
        }

        Ok(Self {
            host: host.to_string(),
            path,
        })
    }

    /// Get the path to the bare repo relative to .wald/repos/
    /// Returns: host/path/to/repo.git
    pub fn to_bare_path(&self) -> PathBuf {
        let mut p = PathBuf::from(&self.host);
        for segment in &self.path[..self.path.len() - 1] {
            p = p.join(segment);
        }
        // Last segment gets .git suffix
        if let Some(name) = self.path.last() {
            p = p.join(format!("{}.git", name));
        }
        p
    }

    /// Get the canonical string representation
    pub fn as_str(&self) -> String {
        format!("{}/{}", self.host, self.path.join("/"))
    }

    /// Get the repository name (last path segment)
    pub fn name(&self) -> &str {
        self.path.last().map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the owner/group path (all segments except the last)
    /// For "github.com/user/repo" returns "user"
    /// For "git.zib.de/iol/research/project" returns "iol/research"
    pub fn owner_path(&self) -> String {
        if self.path.len() <= 1 {
            String::new()
        } else {
            self.path[..self.path.len() - 1].join("/")
        }
    }

    /// Infer clone URL from repo ID
    /// Uses SSH by default for GitHub and GitLab hosts
    pub fn to_clone_url(&self) -> String {
        let path_str = self.path.join("/");
        match self.host.as_str() {
            "github.com" => format!("git@github.com:{}.git", path_str),
            "git.zib.de" => format!("git@git.zib.de:{}.git", path_str),
            "git.overleaf.com" => {
                // Overleaf uses HTTPS and only the project ID
                format!("https://git.overleaf.com/{}", self.name())
            }
            _ => format!("git@{}:{}.git", self.host, path_str),
        }
    }
}

impl FromStr for RepoId {
    type Err = RepoIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for RepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.host, self.path.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_three_segments() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.host, "github.com");
        assert_eq!(id.path, vec!["user", "repo"]);
        assert_eq!(id.name(), "repo");
        assert_eq!(id.owner_path(), "user");
    }

    #[test]
    fn test_parse_four_segments_subgroup() {
        let id = RepoId::parse("git.zib.de/iol/research/project").unwrap();
        assert_eq!(id.host, "git.zib.de");
        assert_eq!(id.path, vec!["iol", "research", "project"]);
        assert_eq!(id.name(), "project");
        assert_eq!(id.owner_path(), "iol/research");
    }

    #[test]
    fn test_parse_five_segments_deep_subgroup() {
        let id = RepoId::parse("git.zib.de/a/b/c/repo").unwrap();
        assert_eq!(id.host, "git.zib.de");
        assert_eq!(id.path, vec!["a", "b", "c", "repo"]);
        assert_eq!(id.name(), "repo");
        assert_eq!(id.owner_path(), "a/b/c");
    }

    #[test]
    fn test_parse_two_segments_minimal() {
        let id = RepoId::parse("example.com/repo").unwrap();
        assert_eq!(id.host, "example.com");
        assert_eq!(id.path, vec!["repo"]);
        assert_eq!(id.name(), "repo");
        assert_eq!(id.owner_path(), "");
    }

    #[test]
    fn test_parse_invalid_one_segment() {
        assert!(RepoId::parse("github.com").is_err());
    }

    #[test]
    fn test_parse_invalid_empty_segment() {
        assert!(RepoId::parse("github.com//repo").is_err());
        assert!(RepoId::parse("github.com/user/").is_err());
    }

    #[test]
    fn test_to_bare_path_three_segments() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.to_bare_path(), PathBuf::from("github.com/user/repo.git"));
    }

    #[test]
    fn test_to_bare_path_subgroup() {
        let id = RepoId::parse("git.zib.de/iol/research/project").unwrap();
        assert_eq!(id.to_bare_path(), PathBuf::from("git.zib.de/iol/research/project.git"));
    }

    #[test]
    fn test_to_clone_url_github() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.to_clone_url(), "git@github.com:user/repo.git");
    }

    #[test]
    fn test_to_clone_url_gitlab_subgroup() {
        let id = RepoId::parse("git.zib.de/iol/research/project").unwrap();
        assert_eq!(id.to_clone_url(), "git@git.zib.de:iol/research/project.git");
    }

    #[test]
    fn test_display_three_segments() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(format!("{}", id), "github.com/user/repo");
    }

    #[test]
    fn test_display_subgroup() {
        let id = RepoId::parse("git.zib.de/iol/research/project").unwrap();
        assert_eq!(format!("{}", id), "git.zib.de/iol/research/project");
    }

    // Edge case tests

    #[test]
    fn test_parse_with_port_in_host() {
        // Hosts with ports are valid (though unusual)
        let id = RepoId::parse("git.example.com:8443/user/repo").unwrap();
        assert_eq!(id.host, "git.example.com:8443");
        assert_eq!(id.path, vec!["user", "repo"]);
    }

    #[test]
    fn test_parse_whitespace_trimmed() {
        // Whitespace around components should be trimmed
        let id = RepoId::parse("  github.com / user / repo  ").unwrap();
        assert_eq!(id.host, "github.com");
        assert_eq!(id.path, vec!["user", "repo"]);
        assert_eq!(id.name(), "repo");
    }

    #[test]
    fn test_parse_trailing_slash() {
        // Trailing slash creates empty segment which should error
        let result = RepoId::parse("github.com/user/repo/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dots_in_path() {
        // Dots in path segments are valid (e.g., repo.fork)
        let id = RepoId::parse("github.com/user/repo.fork").unwrap();
        assert_eq!(id.host, "github.com");
        assert_eq!(id.path, vec!["user", "repo.fork"]);
        assert_eq!(id.name(), "repo.fork");
    }

    #[test]
    fn test_owner_path_single_segment() {
        // When path has only one segment, owner_path is empty
        let id = RepoId::parse("example.com/repo").unwrap();
        assert_eq!(id.name(), "repo");
        assert_eq!(id.owner_path(), "");
    }

    #[test]
    fn test_parse_leading_slash() {
        // Leading slash creates empty host which should error
        let result = RepoId::parse("/github.com/user/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_clone_url_overleaf() {
        // Overleaf uses HTTPS and only project ID
        let id = RepoId::parse("git.overleaf.com/abc123").unwrap();
        assert_eq!(id.to_clone_url(), "https://git.overleaf.com/abc123");
    }
}
