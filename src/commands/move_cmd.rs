use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::git::worktree_move;
use crate::output::Output;
use crate::types::WorktreeEntry;
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::{is_baum, validate_workspace_path, Workspace};

/// Options for move command
pub struct MoveOptions {
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

/// Move a baum to a new location
pub fn move_baum(ws: &Workspace, opts: MoveOptions, out: &Output) -> Result<()> {
    out.require_human("move")?;

    // Resolve paths relative to workspace (with path traversal protection)
    let old_container = validate_workspace_path(&ws.root, &opts.old_path)?;
    let new_container = validate_workspace_path(&ws.root, &opts.new_path)?;

    // Check source exists
    if !old_container.exists() {
        bail!("source path not found: {}", old_container.display());
    }

    // Check source is a baum
    if !is_baum(&old_container) {
        bail!(
            "source is not a baum: {} (.baum directory not found)",
            old_container.display()
        );
    }

    // Check destination doesn't exist
    if new_container.exists() {
        bail!("destination already exists: {}", new_container.display());
    }

    // Ensure parent of destination exists
    if let Some(parent) = new_container.parent() {
        fs::create_dir_all(parent)?;
    }

    // Load baum manifest for info
    let mut baum_manifest = load_baum(&old_container)?;

    out.status(
        "Moving",
        &format!("{} -> {}", opts.old_path.display(), opts.new_path.display()),
    );

    // Get bare repo path
    let bare_path = ws.bare_repo_path(&baum_manifest.repo_id)?;

    // Create new container directory first (git worktree move needs parent to exist)
    fs::create_dir_all(&new_container)?;

    // Move each worktree using git worktree move
    let mut updated_worktrees = Vec::new();
    for wt in &baum_manifest.worktrees {
        let old_wt_path = old_container.join(&wt.path);
        let new_wt_path = new_container.join(&wt.path);

        if old_wt_path.exists() {
            out.verbose(&format!(
                "Moving worktree: {} -> {}",
                wt.path,
                new_wt_path.display()
            ));

            // Use git worktree move to properly update git's internal references
            worktree_move(&bare_path, &old_wt_path, &new_wt_path)
                .with_context(|| format!("failed to move worktree {}", wt.branch))?;
        }

        updated_worktrees.push(WorktreeEntry {
            branch: wt.branch.clone(),
            path: wt.path.clone(),
        });
    }

    // Update manifest with worktree info
    baum_manifest.worktrees = updated_worktrees;

    // Create new container's .baum directory
    let new_baum_dir = new_container.join(".baum");
    fs::create_dir_all(&new_baum_dir)?;

    // Save manifest to new location
    save_baum(&new_container, &baum_manifest)?;

    // Copy .gitignore if it exists
    let old_gitignore = old_container.join(".gitignore");
    let new_gitignore = new_container.join(".gitignore");
    if old_gitignore.exists() {
        fs::copy(&old_gitignore, &new_gitignore)?;
    }

    // Remove old .baum directory (worktrees already moved)
    let old_baum_dir = old_container.join(".baum");
    if old_baum_dir.exists() {
        fs::remove_dir_all(&old_baum_dir)?;
    }

    // Remove old .gitignore
    if old_gitignore.exists() {
        fs::remove_file(&old_gitignore)?;
    }

    // Remove old container if empty
    if old_container.exists() && old_container.read_dir()?.next().is_none() {
        fs::remove_dir(&old_container)?;
    }

    // Stage the changes in git for proper rename detection
    // Since we've manually moved files, use git add/rm to stage the changes
    stage_baum_move(&ws.root, &old_container, &new_container)?;

    out.success(&format!(
        "Moved {} ({} worktree(s))",
        baum_manifest.repo_id,
        baum_manifest.worktrees.len()
    ));

    Ok(())
}

/// Stage a baum move in git for proper rename detection
/// Uses git add/rm to stage the changes since files are already moved
fn stage_baum_move(repo: &Path, old: &Path, new: &Path) -> Result<()> {
    // Stage the new location
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("add")
        .arg(new.join(".baum"))
        .output();

    // Also stage the new .gitignore if it exists
    let new_gitignore = new.join(".gitignore");
    if new_gitignore.exists() {
        let _ = Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("add")
            .arg(&new_gitignore)
            .output();
    }

    // Stage removal of old location (if anything remains tracked)
    let old_baum = old.join(".baum");
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rm")
        .arg("-r")
        .arg("--cached")
        .arg("--ignore-unmatch")
        .arg(&old_baum)
        .output();

    let old_gitignore = old.join(".gitignore");
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rm")
        .arg("--cached")
        .arg("--ignore-unmatch")
        .arg(&old_gitignore)
        .output();

    Ok(())
}
