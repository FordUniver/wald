use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::output::Output;
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::{is_baum, Workspace};

/// Options for prune command
pub struct PruneOptions {
    pub baum_path: PathBuf,
    pub branches: Vec<String>,
    pub force: bool,
}

/// Remove worktrees for branches from a baum
pub fn prune(ws: &Workspace, opts: PruneOptions, out: &Output) -> Result<()> {
    out.require_human("prune")?;

    // Resolve path relative to workspace
    let container = if opts.baum_path.is_absolute() {
        opts.baum_path.clone()
    } else {
        ws.root.join(&opts.baum_path)
    };

    // Check if it's a baum
    if !is_baum(&container) {
        bail!(
            "not a baum: {} (.baum directory not found)",
            container.display()
        );
    }

    // Load baum manifest
    let mut baum_manifest = load_baum(&container)?;

    // Get bare repo path
    let bare_path = ws.bare_repo_path(&baum_manifest.repo_id)?;

    let mut removed_count = 0;

    for branch in &opts.branches {
        // Find worktree entry
        let wt_idx = baum_manifest
            .worktrees
            .iter()
            .position(|wt| &wt.branch == branch);

        if let Some(idx) = wt_idx {
            let wt = &baum_manifest.worktrees[idx];
            let worktree_path = container.join(&wt.path);

            out.status("Removing worktree", branch);

            // Remove worktree from git
            if worktree_path.exists() {
                git::remove_worktree(&bare_path, &worktree_path, opts.force)?;
            }

            // Also remove directory if it still exists
            if worktree_path.exists() {
                fs::remove_dir_all(&worktree_path)?;
            }

            // Remove from manifest
            baum_manifest.worktrees.remove(idx);
            removed_count += 1;
        } else {
            out.warn(&format!("No worktree found for branch: {}", branch));
        }
    }

    // Save updated manifest
    save_baum(&container, &baum_manifest)?;

    if removed_count > 0 {
        out.success(&format!("Removed {} worktree(s)", removed_count));
    } else {
        out.info("No worktrees removed");
    }

    Ok(())
}
