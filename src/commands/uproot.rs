use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::git;
use crate::output::Output;
use crate::workspace::baum::load_baum;
use crate::workspace::{Workspace, is_baum, validate_workspace_path};

/// Options for uproot command
pub struct UprootOptions {
    pub path: PathBuf,
    pub force: bool,
}

/// Uproot a baum (remove container and worktrees)
pub fn uproot(ws: &Workspace, opts: UprootOptions, out: &Output) -> Result<()> {
    out.require_human("uproot")?;

    // Resolve path relative to workspace (with path traversal protection)
    let container = validate_workspace_path(&ws.root, &opts.path)?;

    // Check if it's a baum
    if !is_baum(&container) {
        bail!(
            "not a baum: {} (.baum directory not found)",
            container.display()
        );
    }

    // Load baum manifest to get worktree info
    let baum_manifest = load_baum(&container)?;

    // Get bare repo path
    let bare_path = ws.bare_repo_path(&baum_manifest.repo_id)?;

    out.status("Uprooting", &format!("{}", container.display()));

    // Remove each worktree from git
    for wt in &baum_manifest.worktrees {
        let worktree_path = container.join(&wt.path);
        if worktree_path.exists() {
            out.status("Removing worktree", &wt.branch);
            // Remove from git worktree list
            if let Err(e) = git::remove_worktree(&bare_path, &worktree_path, opts.force) {
                if opts.force {
                    // Force: just log and continue
                    out.warn(&format!("Failed to remove worktree {}: {}", wt.branch, e));
                } else {
                    return Err(e);
                }
            }
        }
    }

    // Remove the container directory
    fs::remove_dir_all(&container)?;

    out.success(&format!(
        "Uprooted {} ({} worktree(s) removed)",
        baum_manifest.repo_id,
        baum_manifest.worktrees.len()
    ));

    Ok(())
}
