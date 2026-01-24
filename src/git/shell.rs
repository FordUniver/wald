use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

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

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    /// Create a test repository with an initial commit
    fn create_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Configure user for commits
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }

        // Create initial commit
        {
            let sig = Signature::now("Test User", "test@test.com").unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                // Create a file
                std::fs::write(dir.path().join("README.md"), "# Test").unwrap();
                index.add_path(std::path::Path::new("README.md")).unwrap();
                index.write().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    /// Create a bare repository with initial commit
    fn create_bare_repo_with_commit() -> (TempDir, Repository) {
        // First create a regular repo with commits
        let (temp_dir, _repo) = create_test_repo();

        // Clone to bare using git command (more reliable than git2 for bare clone)
        let bare_dir = TempDir::new().unwrap();
        let output = Command::new("git")
            .arg("clone")
            .arg("--bare")
            .arg(temp_dir.path())
            .arg(bare_dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git clone --bare failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let bare_repo = Repository::open_bare(bare_dir.path()).unwrap();

        (bare_dir, bare_repo)
    }

    /// Get the default branch name from a repository
    fn get_default_branch(repo_path: &Path) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("symbolic-ref")
            .arg("--short")
            .arg("HEAD")
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn test_get_head_commit_returns_hash() {
        let (dir, _repo) = create_test_repo();

        let commit = get_head_commit(dir.path()).unwrap();

        // Should be a 40-char hex string
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_get_head_commit_fails_on_empty_repo() {
        let dir = TempDir::new().unwrap();
        let _repo = Repository::init(dir.path()).unwrap();

        // No commits yet, should fail
        let result = get_head_commit(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_worktree_prune_succeeds_on_clean_repo() {
        let (dir, _repo) = create_bare_repo_with_commit();

        // Prune should succeed even when there's nothing to prune
        let result = worktree_prune(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_worktree_prune_removes_stale_entries() {
        let (bare_dir, _bare_repo) = create_bare_repo_with_commit();
        let wt_dir = TempDir::new().unwrap();
        let wt_path = wt_dir.path().join("worktree");

        // Get the default branch name (may be "main" or "master" depending on git config)
        let branch = get_default_branch(bare_dir.path());

        // Add a worktree using git command
        let output = Command::new("git")
            .arg("-C")
            .arg(bare_dir.path())
            .arg("worktree")
            .arg("add")
            .arg(&wt_path)
            .arg(&branch)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git worktree add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Verify worktree was created
        assert!(wt_path.exists(), "worktree should exist at {:?}", wt_path);

        // Manually remove the worktree directory (simulating stale entry)
        std::fs::remove_dir_all(&wt_path).unwrap();

        // Prune should clean up the stale entry
        let result = worktree_prune(bare_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_git_mv_fails_on_missing_source() {
        let (dir, _repo) = create_test_repo();

        // Try to move non-existent file
        let result = git_mv(
            dir.path(),
            Path::new("nonexistent.txt"),
            Path::new("target.txt"),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_git_mv_stages_rename() {
        let (dir, _repo) = create_test_repo();

        // Create source file
        let source = dir.path().join("source.txt");
        std::fs::write(&source, "content").unwrap();

        // Stage it first
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("add")
            .arg("source.txt")
            .output()
            .unwrap();

        // Commit it
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .arg("commit")
            .arg("-m")
            .arg("Add source")
            .output()
            .unwrap();

        // Now git mv
        let result = git_mv(dir.path(), Path::new("source.txt"), Path::new("target.txt"));

        assert!(result.is_ok());
        assert!(dir.path().join("target.txt").exists());
        assert!(!dir.path().join("source.txt").exists());
    }
}
