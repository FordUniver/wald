use std::fs;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::git::history::detect_moves;
use crate::git::shell::get_head_commit;
use crate::output::Output;
use crate::workspace::{is_baum, Workspace};
use crate::workspace::baum::load_baum;

/// Options for sync command
pub struct SyncOptions {
    pub dry_run: bool,
    pub force: bool,
    pub push: bool,
}

/// Sync workspace with remote, replaying moves
pub fn sync(ws: &mut Workspace, opts: SyncOptions, out: &Output) -> Result<()> {
    // Check for uncommitted changes
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&ws.root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .context("failed to check git status")?;

    let status = String::from_utf8_lossy(&status_output.stdout);
    if !status.trim().is_empty() {
        bail!(
            "uncommitted changes in workspace\nCommit or stash changes before syncing"
        );
    }

    // Get current HEAD before pull
    let head_before = get_head_commit(&ws.root)?;

    // Get last sync point
    let last_sync = ws.state.last_sync.clone();

    out.status("Syncing", "pulling changes from remote");

    // Pull changes (rebase)
    if !opts.dry_run {
        let pull_output = Command::new("git")
            .arg("-C")
            .arg(&ws.root)
            .arg("pull")
            .arg("--rebase")
            .arg("--quiet")
            .output()
            .context("failed to pull changes")?;

        if !pull_output.status.success() {
            let stderr = String::from_utf8_lossy(&pull_output.stderr);
            if stderr.contains("diverged") && !opts.force {
                bail!(
                    "workspace has diverged from remote\nUse --force to force sync"
                );
            }
            bail!("git pull failed: {}", stderr);
        }
    }

    // Get HEAD after pull
    let head_after = get_head_commit(&ws.root)?;

    // Check if anything changed
    if head_before == head_after {
        out.info("Already up to date");

        // Push if requested and we have unpushed commits
        if opts.push {
            push_changes(ws, &opts, out)?;
        }

        // Update last sync (only if not dry-run)
        if !opts.dry_run {
            ws.state.update_last_sync(&head_after);
            ws.save_state()?;
        }

        return Ok(());
    }

    // Detect moves since last sync
    let from_commit = last_sync.as_deref().unwrap_or(&head_before);
    let moves = detect_moves(&ws.root, from_commit, &head_after)?;

    if !moves.is_empty() {
        out.status("Detected", &format!("{} baum move(s)", moves.len()));

        for mv in &moves {
            out.status("Move", &format!("{} -> {}", mv.old_path, mv.new_path));

            if !opts.dry_run {
                // Replay the move locally
                replay_move(ws, &mv.old_path, &mv.new_path, out)?;
            }
        }
    }

    // Push if requested
    if opts.push {
        push_changes(ws, &opts, out)?;
    }

    // Update last sync (only if not dry-run)
    if !opts.dry_run {
        ws.state.update_last_sync(&head_after);
        ws.save_state()?;
    }

    out.success("Sync complete");

    Ok(())
}

fn push_changes(ws: &Workspace, opts: &SyncOptions, out: &Output) -> Result<()> {
    if opts.dry_run {
        out.info("Would push changes to remote");
        return Ok(());
    }

    out.status("Pushing", "sending changes to remote");

    let push_output = Command::new("git")
        .arg("-C")
        .arg(&ws.root)
        .arg("push")
        .arg("--quiet")
        .output()
        .context("failed to push changes")?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        bail!("git push failed: {}", stderr);
    }

    Ok(())
}

fn replay_move(ws: &Workspace, old_path: &str, new_path: &str, out: &Output) -> Result<()> {
    let old_abs = ws.root.join(old_path);
    let new_abs = ws.root.join(new_path);

    // After git pull with rename detection:
    // - Old path may have orphaned worktrees (not git-tracked, so not removed)
    // - New path has .baum directory (git-tracked, so moved by git)
    //
    // We need to move the worktrees to the new location using `git worktree move`
    // to properly update the bare repo's worktree registry.

    let old_exists = old_abs.exists();
    let new_exists = new_abs.exists();
    let old_is_baum = is_baum(&old_abs);
    let new_is_baum = is_baum(&new_abs);

    if old_exists && new_exists {
        // Both paths exist - check if we can merge
        if !old_is_baum && new_is_baum {
            // Old has orphaned worktrees, new has .baum from git
            // Use git worktree move to relocate each worktree
            let baum = load_baum(&new_abs)?;
            let bare_path = ws.bare_repo_path(&baum.repo_id)?;

            move_worktrees_with_git(&bare_path, &old_abs, &new_abs, &baum.worktrees, out)?;

            // Clean up old directory if empty
            if old_abs.read_dir()?.next().is_none() {
                fs::remove_dir(&old_abs)?;
            }
        } else if old_is_baum && new_is_baum {
            // True conflict - both are complete baums
            out.warn(&format!(
                "Move conflict: both {} and {} are baums",
                old_path, new_path
            ));
        } else {
            // Some other case
            out.warn(&format!(
                "Move conflict: both {} and {} exist",
                old_path, new_path
            ));
        }
        return Ok(());
    }

    if old_exists && !new_exists {
        // Old exists but new doesn't - need to do the move
        if old_is_baum {
            // Get bare repo to update worktree references
            let baum = load_baum(&old_abs)?;
            let bare_path = ws.bare_repo_path(&baum.repo_id)?;

            // Ensure parent exists
            if let Some(parent) = new_abs.parent() {
                fs::create_dir_all(parent)?;
            }

            // Move the .baum directory (tracked content)
            let old_baum_dir = old_abs.join(".baum");
            let new_baum_dir = new_abs.join(".baum");
            fs::create_dir_all(&new_abs)?;
            fs::rename(&old_baum_dir, &new_baum_dir)?;

            // Move worktrees using git worktree move
            move_worktrees_with_git(&bare_path, &old_abs, &new_abs, &baum.worktrees, out)?;

            // Clean up old directory if empty
            if old_abs.exists() && old_abs.read_dir()?.next().is_none() {
                fs::remove_dir(&old_abs)?;
            }
        }
    }

    Ok(())
}

/// Move worktrees using `git worktree move` to properly update the registry
fn move_worktrees_with_git(
    bare_path: &std::path::Path,
    old_container: &std::path::Path,
    new_container: &std::path::Path,
    worktrees: &[crate::types::WorktreeEntry],
    out: &Output,
) -> Result<()> {
    use crate::git::shell::worktree_move;

    for wt in worktrees {
        let old_wt = old_container.join(&wt.path);
        let new_wt = new_container.join(&wt.path);

        if old_wt.exists() && !new_wt.exists() {
            // Use git worktree move to relocate and update registry
            match worktree_move(bare_path, &old_wt, &new_wt) {
                Ok(()) => {
                    out.status("Moved", &format!("worktree {} -> {}", old_wt.display(), new_wt.display()));
                }
                Err(e) => {
                    // Log warning but continue with other worktrees
                    out.warn(&format!(
                        "Failed to move worktree {}: {}",
                        wt.path, e
                    ));
                }
            }
        }
    }

    Ok(())
}
