use std::path::PathBuf;

use anyhow::{Result, bail};

use crate::git;
use crate::naming::worktree_dir_name;
use crate::output::Output;
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::gitignore::{add_worktree_to_gitignore, ensure_gitignore_section};
use crate::workspace::{Workspace, collect_baum_ids, is_baum, validate_workspace_path};

/// Options for branch command
pub struct BranchOptions {
    pub baum_path: PathBuf,
    pub branch: String,
    pub force: bool,
    pub reuse: bool,
}

impl BranchOptions {
    pub fn branch_mode(&self) -> git::BranchMode {
        if self.force {
            git::BranchMode::Force
        } else if self.reuse {
            git::BranchMode::Reuse
        } else {
            git::BranchMode::Default
        }
    }
}

/// Add a worktree for a branch to an existing baum
pub fn branch(ws: &Workspace, opts: BranchOptions, out: &Output) -> Result<()> {
    out.require_human("branch")?;

    // Resolve path relative to workspace (with path traversal protection)
    let container = validate_workspace_path(&ws.root, &opts.baum_path)?;

    // Check if it's a baum
    if !is_baum(&container) {
        bail!(
            "not a baum: {} (.baum directory not found)",
            container.display()
        );
    }

    // Ensure workspace-level .gitignore has wald section
    ensure_gitignore_section(&ws.root)?;

    // Load baum manifest
    let mut baum_manifest = load_baum(&container)?;

    // Check if branch already has a worktree
    if baum_manifest
        .worktrees
        .iter()
        .any(|wt| wt.branch == opts.branch)
    {
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

    out.status(
        "Adding worktree",
        &format!("{} -> {}", opts.branch, worktree_name),
    );

    // Ensure the baum has an ID (generate if legacy baum)
    let existing_ids = collect_baum_ids(&ws.root);
    let baum_id = baum_manifest.ensure_id(&existing_ids).to_string();

    // Add worktree with tracking branch (wald/<baum_id>/<branch>)
    let local_branch = git::add_worktree_with_tracking_mode(
        &bare_path,
        &worktree_path,
        &opts.branch,
        &baum_id,
        opts.branch_mode(),
    )?;

    // Update baum manifest with local branch info
    baum_manifest.add_worktree_with_local(&opts.branch, &worktree_name, &local_branch);
    save_baum(&container, &baum_manifest)?;

    // Add to .gitignore
    add_worktree_to_gitignore(&container, &worktree_name)?;

    out.success(&format!("Added worktree for branch: {}", opts.branch));

    Ok(())
}
