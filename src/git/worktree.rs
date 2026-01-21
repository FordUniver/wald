use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Add a worktree from a bare repository
///
/// If the branch doesn't exist locally, creates it tracking the remote branch.
pub fn add_worktree(bare_repo: &Path, worktree_path: &Path, branch: &str) -> Result<()> {
    // First, try to add worktree for existing branch
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("worktree")
        .arg("add")
        .arg(worktree_path)
        .arg(branch)
        .output()
        .with_context(|| {
            format!(
                "failed to add worktree at {} for branch {}",
                worktree_path.display(),
                branch
            )
        })?;

    if output.status.success() {
        return Ok(());
    }

    // If branch doesn't exist, try creating it
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not a valid reference") || stderr.contains("invalid reference") {
        // Try to create branch tracking origin
        let output = Command::new("git")
            .arg("-C")
            .arg(bare_repo)
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch)
            .arg(worktree_path)
            .arg(format!("origin/{}", branch))
            .output()
            .with_context(|| format!("failed to create branch {} for worktree", branch))?;

        if output.status.success() {
            return Ok(());
        }

        // If origin/branch doesn't exist either, create from HEAD
        let output = Command::new("git")
            .arg("-C")
            .arg(bare_repo)
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch)
            .arg(worktree_path)
            .output()
            .with_context(|| format!("failed to create new branch {} for worktree", branch))?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to add worktree for branch {}: {}",
            branch,
            stderr.trim()
        );
    }

    bail!(
        "failed to add worktree for branch {}: {}",
        branch,
        stderr.trim()
    );
}

/// Remove a worktree
pub fn remove_worktree(bare_repo: &Path, worktree_path: &Path, force: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(bare_repo).arg("worktree").arg("remove");

    if force {
        cmd.arg("--force");
    }

    cmd.arg(worktree_path);

    let output = cmd
        .output()
        .with_context(|| format!("failed to remove worktree at {}", worktree_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to remove worktree at {}: {}",
            worktree_path.display(),
            stderr.trim()
        );
    }

    Ok(())
}

/// List all worktrees for a bare repository
pub fn list_worktrees(bare_repo: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("worktree")
        .arg("list")
        .arg("--porcelain")
        .output()
        .with_context(|| format!("failed to list worktrees for {}", bare_repo.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to list worktrees for {}: {}",
            bare_repo.display(),
            stderr.trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_worktree_list(&stdout)
}

/// Information about a worktree
#[derive(Debug, Clone, Default)]
pub struct WorktreeInfo {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    pub locked: bool,
    pub prunable: bool,
}

fn parse_worktree_list(output: &str) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let mut current = WorktreeInfo::default();

    for line in output.lines() {
        if line.is_empty() {
            if !current.path.is_empty() {
                worktrees.push(current);
                current = WorktreeInfo::default();
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current.path = path.to_string();
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current.head = Some(head.to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            // branch refs/heads/main -> main
            if let Some(name) = branch.strip_prefix("refs/heads/") {
                current.branch = Some(name.to_string());
            } else {
                current.branch = Some(branch.to_string());
            }
        } else if line == "bare" {
            current.bare = true;
        } else if line == "detached" {
            current.detached = true;
        } else if line.starts_with("locked") {
            current.locked = true;
        } else if line.starts_with("prunable") {
            current.prunable = true;
        }
    }

    // Don't forget the last entry
    if !current.path.is_empty() {
        worktrees.push(current);
    }

    Ok(worktrees)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_worktree_list() {
        let output = r#"worktree /path/to/bare.git
HEAD abc123
bare

worktree /path/to/main
HEAD def456
branch refs/heads/main

worktree /path/to/feature
HEAD 789abc
branch refs/heads/feature
"#;

        let worktrees = parse_worktree_list(output).unwrap();
        assert_eq!(worktrees.len(), 3);

        assert_eq!(worktrees[0].path, "/path/to/bare.git");
        assert!(worktrees[0].bare);

        assert_eq!(worktrees[1].path, "/path/to/main");
        assert_eq!(worktrees[1].branch, Some("main".to_string()));
        assert!(!worktrees[1].bare);

        assert_eq!(worktrees[2].path, "/path/to/feature");
        assert_eq!(worktrees[2].branch, Some("feature".to_string()));
    }
}
