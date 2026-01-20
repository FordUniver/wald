use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Move a worktree using `git worktree move`
///
/// Note: libgit2 doesn't support worktree move, so we shell out to git.
pub fn worktree_move(bare_repo: &Path, from: &Path, to: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("worktree")
        .arg("move")
        .arg(from)
        .arg(to)
        .output()
        .with_context(|| {
            format!(
                "failed to move worktree from {} to {}",
                from.display(),
                to.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to move worktree from {} to {}: {}",
            from.display(),
            to.display(),
            stderr.trim()
        );
    }

    Ok(())
}

/// Prune stale worktree entries
pub fn worktree_prune(bare_repo: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(bare_repo)
        .arg("worktree")
        .arg("prune")
        .output()
        .with_context(|| format!("failed to prune worktrees in {}", bare_repo.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to prune worktrees in {}: {}",
            bare_repo.display(),
            stderr.trim()
        );
    }

    Ok(())
}

/// Stage a file move with git mv for rename detection
pub fn git_mv(repo: &Path, from: &Path, to: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("mv")
        .arg(from)
        .arg(to)
        .output()
        .with_context(|| {
            format!(
                "failed to git mv from {} to {}",
                from.display(),
                to.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git mv failed from {} to {}: {}",
            from.display(),
            to.display(),
            stderr.trim()
        );
    }

    Ok(())
}

/// Get current HEAD commit hash
pub fn get_head_commit(repo: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .with_context(|| format!("failed to get HEAD commit in {}", repo.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "failed to get HEAD commit in {}: {}",
            repo.display(),
            stderr.trim()
        );
    }

    let commit = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    Ok(commit)
}
