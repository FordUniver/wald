use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::naming::worktree_dir_name;
use crate::output::Output;
use crate::workspace::{is_baum, Workspace};
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::gitignore::add_worktree_to_gitignore;

/// Options for branch command
pub struct BranchOptions {
    pub baum_path: PathBuf,
    pub branch: String,
}

/// Add a worktree for a branch to an existing baum
pub fn branch(ws: &Workspace, opts: BranchOptions, out: &Output) -> Result<()> {
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

    // Check if branch already has a worktree
    if baum_manifest.worktrees.iter().any(|wt| wt.branch == opts.branch) {
        bail!(
            "worktree for branch '{}' already exists in baum",
            opts.branch
        );
    }

    // Get bare repo path
    let bare_path = ws.bare_repo_path(&baum_manifest.repo_id)?;
    if !bare_path.exists() {
        bail!("bare repo not found: {}", bare_path.display());
    }

    // Create worktree
    let worktree_name = worktree_dir_name(&opts.branch);
    let worktree_path = container.join(&worktree_name);

    out.status("Adding worktree", &format!("{} -> {}", opts.branch, worktree_name));

    git::add_worktree(&bare_path, &worktree_path, &opts.branch)?;

    // Update baum manifest
    baum_manifest.add_worktree(&opts.branch, &worktree_name);
    save_baum(&container, &baum_manifest)?;

    // Add to .gitignore
    add_worktree_to_gitignore(&container, &worktree_name)?;

    out.success(&format!("Added worktree for branch: {}", opts.branch));

    Ok(())
}
