use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use thiserror::Error;

/// Canonical repository identifier: host/owner/name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoId {
    pub host: String,
    pub owner: String,
    pub name: String,
}

#[derive(Error, Debug)]
pub enum RepoIdError {
    #[error("invalid repo ID format: expected host/owner/name, got '{0}'")]
    InvalidFormat(String),
    #[error("empty component in repo ID: '{0}'")]
    EmptyComponent(String),
}

impl RepoId {
    /// Parse a repo ID from a string like "github.com/user/repo"
    pub fn parse(s: &str) -> Result<Self, RepoIdError> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return Err(RepoIdError::InvalidFormat(s.to_string()));
        }

        let host = parts[0].trim();
        let owner = parts[1].trim();
        let name = parts[2].trim();

        if host.is_empty() || owner.is_empty() || name.is_empty() {
            return Err(RepoIdError::EmptyComponent(s.to_string()));
        }

        Ok(Self {
            host: host.to_string(),
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }

    /// Get the path to the bare repo relative to .wald/repos/
    /// Returns: host/owner/name.git
    pub fn to_bare_path(&self) -> PathBuf {
        PathBuf::from(&self.host)
            .join(&self.owner)
            .join(format!("{}.git", self.name))
    }

    /// Get the canonical string representation
    pub fn as_str(&self) -> String {
        format!("{}/{}/{}", self.host, self.owner, self.name)
    }

    /// Infer clone URL from repo ID
    /// Uses SSH by default for GitHub and GitLab hosts
    pub fn to_clone_url(&self) -> String {
        match self.host.as_str() {
            "github.com" => format!("git@github.com:{}/{}.git", self.owner, self.name),
            "git.zib.de" => format!("git@git.zib.de:{}/{}.git", self.owner, self.name),
            "git.overleaf.com" => {
                format!("https://git.overleaf.com/{}", self.name)
            }
            _ => format!("git@{}:{}/{}.git", self.host, self.owner, self.name),
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
        write!(f, "{}/{}/{}", self.host, self.owner, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.host, "github.com");
        assert_eq!(id.owner, "user");
        assert_eq!(id.name, "repo");
    }

    #[test]
    fn test_parse_invalid_too_few_parts() {
        assert!(RepoId::parse("github.com/repo").is_err());
    }

    #[test]
    fn test_parse_invalid_too_many_parts() {
        assert!(RepoId::parse("github.com/org/sub/repo").is_err());
    }

    #[test]
    fn test_to_bare_path() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.to_bare_path(), PathBuf::from("github.com/user/repo.git"));
    }

    #[test]
    fn test_to_clone_url_github() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(id.to_clone_url(), "git@github.com:user/repo.git");
    }

    #[test]
    fn test_display() {
        let id = RepoId::parse("github.com/user/repo").unwrap();
        assert_eq!(format!("{}", id), "github.com/user/repo");
    }
}
