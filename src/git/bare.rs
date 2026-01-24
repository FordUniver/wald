use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use git2::{BranchType, Repository};

use crate::types::RepoId;

/// Options for cloning a bare repo
#[derive(Default)]
pub struct CloneOptions {
    /// Shallow clone depth (None = full history)
    pub depth: Option<u32>,
    /// Partial clone filter (None = full clone)
    pub filter: Option<String>,
}

/// Clone a repository as a bare repo
pub fn clone_bare(repo_id: &RepoId, target: &Path, opts: CloneOptions) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    // Check if target already exists
    if target.exists() {
        bail!("bare repo already exists: {}", target.display());
    }

    let url = repo_id.to_clone_url();

    // Use git command for clone (libgit2 has limited shallow/partial clone support)
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--bare").arg("--quiet");

    if let Some(d) = opts.depth {
        cmd.arg(format!("--depth={}", d));
    }

    if let Some(ref f) = opts.filter {
        cmd.arg(format!("--filter={}", f));
    }

    cmd.arg(&url).arg(target);

    let output = cmd
        .output()
        .with_context(|| format!("failed to execute git clone for {}", repo_id))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git clone failed for {}: {}", repo_id, stderr);
    }

    Ok(())
}

/// Open an existing bare repository
pub fn open_bare(path: &Path) -> Result<Repository> {
    Repository::open_bare(path)
        .with_context(|| format!("failed to open bare repo: {}", path.display()))
}

/// Fetch updates in a bare repository
pub fn fetch_bare(path: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("fetch")
        .arg("--all")
        .arg("--prune")
        .arg("--quiet")
        .output()
        .with_context(|| format!("failed to execute git fetch in {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git fetch failed in {}: {}", path.display(), stderr);
    }

    Ok(())
}

/// Check if a bare repository is a partial clone
pub fn is_partial_clone(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("--get")
        .arg("remote.origin.promisor")
        .output()
        .with_context(|| format!("failed to check partial clone status: {}", path.display()))?;

    // If config exists and is "true", it's a partial clone
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout);
        return Ok(value.trim() == "true");
    }

    Ok(false)
}

/// Convert a partial clone to a full clone and fetch all objects
pub fn fetch_full(path: &Path) -> Result<()> {
    // Remove partial clone configuration
    // These may fail if not set, which is fine
    let _ = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("--unset")
        .arg("remote.origin.promisor")
        .output();

    let _ = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("--unset")
        .arg("remote.origin.partialclonefilter")
        .output();

    // Fetch all objects (--refetch ensures we get everything)
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("fetch")
        .arg("--all")
        .arg("--prune")
        .arg("--refetch")
        .output()
        .with_context(|| format!("failed to fetch full in {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git fetch --refetch failed in {}: {}",
            path.display(),
            stderr
        );
    }

    Ok(())
}

/// Run garbage collection on a bare repository
pub fn gc(path: &Path, aggressive: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(path).arg("gc");

    if aggressive {
        cmd.arg("--aggressive");
    }

    let output = cmd
        .output()
        .with_context(|| format!("failed to run git gc in {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git gc failed in {}: {}", path.display(), stderr);
    }

    Ok(())
}

/// List branches in a bare repository
pub fn list_branches(path: &Path) -> Result<Vec<String>> {
    let repo = open_bare(path)?;
    let mut branches = Vec::new();

    for branch_result in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch_result?;
        if let Some(name) = branch.name()? {
            branches.push(name.to_string());
        }
    }

    // Also check remote branches (for origin/main etc)
    for branch_result in repo.branches(Some(BranchType::Remote))? {
        let (branch, _) = branch_result?;
        if let Some(name) = branch.name()? {
            // Strip "origin/" prefix
            if let Some(stripped) = name.strip_prefix("origin/")
                && !branches.contains(&stripped.to_string())
            {
                branches.push(stripped.to_string());
            }
        }
    }

    Ok(branches)
}

/// Check if a branch exists in a bare repository
pub fn has_branch(path: &Path, branch: &str) -> Result<bool> {
    let repo = open_bare(path)?;

    // Check local branches
    if repo.find_branch(branch, BranchType::Local).is_ok() {
        return Ok(true);
    }

    // Check remote branches
    let remote_name = format!("origin/{}", branch);
    if repo.find_branch(&remote_name, BranchType::Remote).is_ok() {
        return Ok(true);
    }

    Ok(false)
}

/// Get the default branch name for a bare repository
pub fn get_default_branch(path: &Path) -> Result<String> {
    let repo = open_bare(path)?;

    // Try to find HEAD reference
    if let Ok(head) = repo.find_reference("HEAD")
        && let Some(target) = head.symbolic_target()
    {
        // refs/heads/main -> main
        if let Some(branch) = target.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: check for common default branch names
    for name in ["main", "master"] {
        if has_branch(path, name)? {
            return Ok(name.to_string());
        }
    }

    bail!("could not determine default branch for {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require network access and are marked as ignored
    // Run with: cargo test -- --ignored

    #[test]
    #[ignore]
    fn test_clone_bare() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let target = dir.path().join("test.git");

        let repo_id = RepoId::parse("github.com/octocat/Hello-World").unwrap();
        let opts = CloneOptions {
            depth: Some(1),
            filter: None,
        };
        clone_bare(&repo_id, &target, opts).unwrap();

        assert!(target.exists());
        assert!(target.join("HEAD").exists());
    }
}
