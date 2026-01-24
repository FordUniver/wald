use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::id::format_wald_branch;

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

/// Branch handling mode for worktree creation
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BranchMode {
    /// Default: fail if local branch exists with unpushed commits
    #[default]
    Default,
    /// Delete existing local branch, create fresh from origin
    Force,
    /// Use existing local branch as-is
    Reuse,
}

/// Add a worktree with a local tracking branch (wald/<baum_id>/<branch>)
///
/// Creates a local branch `wald/<baum_id>/<branch>` tracking `origin/<branch>`,
/// then checks it out in the worktree. This allows multiple baums to have
/// worktrees for the same logical branch.
///
/// Returns the local branch name that was created.
pub fn add_worktree_with_tracking(
    bare_repo: &Path,
    worktree_path: &Path,
    branch: &str,
    baum_id: &str,
) -> Result<String> {
    add_worktree_with_tracking_mode(
        bare_repo,
        worktree_path,
        branch,
        baum_id,
        BranchMode::Default,
    )
}

/// Add a worktree with a local tracking branch, with configurable branch mode
pub fn add_worktree_with_tracking_mode(
    bare_repo: &Path,
    worktree_path: &Path,
    branch: &str,
    baum_id: &str,
    mode: BranchMode,
) -> Result<String> {
    let local_branch = format_wald_branch(baum_id, branch);
    let remote_branch = format!("origin/{}", branch);

    // Check if local branch already exists
    let branch_exists = check_branch_exists(bare_repo, &local_branch)?;

    if branch_exists {
        match mode {
            BranchMode::Force => {
                // Delete the existing branch and recreate
                delete_branch(bare_repo, &local_branch, true)?;
            }
            BranchMode::Reuse => {
                // Use existing branch as-is, but check for unpushed commits
                if has_unpushed_commits(bare_repo, &local_branch)? {
                    bail!(
                        "branch '{}' has unpushed commits; use --force to discard or push changes first",
                        local_branch
                    );
                }
                // Just add the worktree with the existing branch
                return add_worktree_for_existing_branch(bare_repo, worktree_path, &local_branch);
            }
            BranchMode::Default => {
                // Check for unpushed commits and fail if present
                if has_unpushed_commits(bare_repo, &local_branch)? {
                    bail!(
                        "branch '{}' exists with unpushed commits; use --force to overwrite or --reuse to keep",
                        local_branch
                    );
                }
                // Safe to overwrite
            }
        }
    }

    // Create the local branch tracking the remote
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("branch")
        .arg("-f")
        .arg(&local_branch)
        .arg(&remote_branch)
        .output()
        .with_context(|| format!("failed to create branch {}", local_branch))?;

    if !output.status.success() {
        // If origin/branch doesn't exist, try creating from the default branch
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a valid object name")
            || stderr.contains("not a valid reference")
            || stderr.contains("unknown revision")
        {
            // Try to find HEAD or default branch
            let fallback_output = Command::new("git")
                .arg("-C")
                .arg(bare_repo)
                .arg("branch")
                .arg("-f")
                .arg(&local_branch)
                .arg("HEAD")
                .output()
                .with_context(|| format!("failed to create branch {} from HEAD", local_branch))?;

            if !fallback_output.status.success() {
                let stderr = String::from_utf8_lossy(&fallback_output.stderr);
                bail!(
                    "failed to create branch {}: remote '{}' not found and no HEAD: {}",
                    local_branch,
                    remote_branch,
                    stderr.trim()
                );
            }
        } else {
            bail!(
                "failed to create branch {}: {}",
                local_branch,
                stderr.trim()
            );
        }
    }

    // Set up tracking (--set-upstream-to) - non-fatal if it fails
    let _ = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("branch")
        .arg("--set-upstream-to")
        .arg(&remote_branch)
        .arg(&local_branch)
        .output();

    // Add the worktree checking out the local branch
    add_worktree_for_existing_branch(bare_repo, worktree_path, &local_branch)?;

    Ok(local_branch)
}

/// Add a worktree for an existing branch
fn add_worktree_for_existing_branch(
    bare_repo: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<String> {
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to add worktree for branch {}: {}",
            branch,
            stderr.trim()
        );
    }

    Ok(branch.to_string())
}

/// Check if a local branch exists in the repository
pub fn check_branch_exists(bare_repo: &Path, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("rev-parse")
        .arg("--verify")
        .arg(format!("refs/heads/{}", branch))
        .output()
        .with_context(|| format!("failed to check branch {}", branch))?;

    Ok(output.status.success())
}

/// Delete a local branch
pub fn delete_branch(bare_repo: &Path, branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("branch")
        .arg(flag)
        .arg(branch)
        .output()
        .with_context(|| format!("failed to delete branch {}", branch))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to delete branch {}: {}", branch, stderr.trim());
    }

    Ok(())
}

/// List all branches matching the wald/* pattern
pub fn list_wald_branches(bare_repo: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("branch")
        .arg("--list")
        .arg("wald/*")
        .output()
        .with_context(|| "failed to list wald branches")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to list wald branches: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().trim_start_matches("* ").to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(branches)
}

/// Check if a branch has unpushed commits relative to its upstream
///
/// Returns true if the branch has commits not in the upstream, false otherwise.
/// Returns false if the branch has no upstream configured.
pub fn has_unpushed_commits(bare_repo: &Path, branch: &str) -> Result<bool> {
    // First check if upstream exists
    let upstream_output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg(format!("{}@{{upstream}}", branch))
        .output()
        .with_context(|| format!("failed to check upstream for {}", branch))?;

    if !upstream_output.status.success() {
        // No upstream configured
        return Ok(false);
    }

    let upstream = String::from_utf8_lossy(&upstream_output.stdout)
        .trim()
        .to_string();

    // Check if there are commits in branch that aren't in upstream
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("rev-list")
        .arg("--count")
        .arg(format!("{}..{}", upstream, branch))
        .output()
        .with_context(|| format!("failed to count unpushed commits for {}", branch))?;

    if !output.status.success() {
        // Error checking - assume there might be unpushed commits to be safe
        return Ok(true);
    }

    let count: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .unwrap_or(0);

    Ok(count > 0)
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
