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

        // Update last sync
        ws.state.update_last_sync(&head_after);
        ws.save_state()?;

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

    // Update last sync
    ws.state.update_last_sync(&head_after);
    ws.save_state()?;

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

    // Check if old path exists (shouldn't after git pull)
    if old_abs.exists() {
        // Check if new path also exists (conflict)
        if new_abs.exists() {
            out.warn(&format!(
                "Move conflict: both {} and {} exist",
                old_path, new_path
            ));
            return Ok(());
        }

        // Old exists but new doesn't - need to do the move
        if is_baum(&old_abs) {
            // Get bare repo to update worktree references
            let baum = load_baum(&old_abs)?;

            // Ensure parent exists
            if let Some(parent) = new_abs.parent() {
                fs::create_dir_all(parent)?;
            }

            // Move the directory
            fs::rename(&old_abs, &new_abs)?;

            // Update worktree paths in bare repo
            update_worktree_paths(ws, &baum.repo_id, old_path, new_path)?;
        }
    }

    Ok(())
}

fn update_worktree_paths(
    ws: &Workspace,
    repo_id: &str,
    _old_container: &str,
    _new_container: &str,
) -> Result<()> {
    // Get bare repo path
    let bare_path = ws.bare_repo_path(repo_id)?;

    // Prune stale worktree references
    crate::git::shell::worktree_prune(&bare_path)?;

    Ok(())
}
