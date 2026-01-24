use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::naming::worktree_dir_name;
use crate::output::Output;
use crate::workspace::gitignore::{add_worktree_to_gitignore, ensure_gitignore_section};
use crate::workspace::{create_baum, is_baum, validate_workspace_path, Workspace};

/// Options for plant command
pub struct PlantOptions {
    pub repo_ref: String,
    pub container: PathBuf,
    pub branches: Vec<String>,
}

/// Plant a baum (create container with worktrees)
pub fn plant(ws: &mut Workspace, opts: PlantOptions, out: &Output) -> Result<()> {
    out.require_human("plant")?;

    // Resolve repo reference to ID
    let repo_id = ws
        .resolve_repo(&opts.repo_ref)
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("repository not found in manifest: {}", opts.repo_ref))?;

    // Verify bare repo exists
    let bare_path = ws.bare_repo_path(&repo_id)?;
    if !bare_path.exists() {
        bail!(
            "bare repo not found: {}\nRun `wald repo add --clone {}` first",
            bare_path.display(),
            repo_id
        );
    }

    // Warn if partial clone (will need network to fetch blobs)
    if git::is_partial_clone(&bare_path)? {
        out.warn("Repository is a partial clone. Network access required to fetch file contents.");
        out.info("Use `wald repo fetch --full` to convert to a full clone for offline access.");
    }

    // Ensure workspace-level .gitignore has wald section
    ensure_gitignore_section(&ws.root)?;

    // Resolve container path (with path traversal protection)
    let container = validate_workspace_path(&ws.root, &opts.container)?;

    // Check if container path exists as file
    if container.exists() && !container.is_dir() {
        bail!(
            "container path exists but is not a directory: {}",
            container.display()
        );
    }

    // Check if already a baum
    if is_baum(&container) {
        bail!(
            "baum already planted at {}: use `wald branch` to add worktrees",
            container.display()
        );
    }

    // Determine branches to create
    let branches = if opts.branches.is_empty() {
        // Default to the default branch
        let default_branch = git::bare::get_default_branch(&bare_path)?;
        vec![default_branch]
    } else {
        opts.branches
    };

    out.status(
        "Planting",
        &format!("{} at {}", repo_id, opts.container.display()),
    );

    // Create baum
    let mut baum_manifest = create_baum(&container, &repo_id)?;

    // Create worktrees for each branch
    for branch in &branches {
        let worktree_name = worktree_dir_name(branch);
        let worktree_path = container.join(&worktree_name);

        out.status(
            "Creating worktree",
            &format!("{} -> {}", branch, worktree_name),
        );

        // Add worktree
        git::add_worktree(&bare_path, &worktree_path, branch)?;

        // Update baum manifest
        baum_manifest.add_worktree(branch, &worktree_name);

        // Add to container's .gitignore
        add_worktree_to_gitignore(&container, &worktree_name)?;
    }

    // Save updated baum manifest
    crate::workspace::baum::save_baum(&container, &baum_manifest)?;

    out.success(&format!(
        "Planted {} with {} worktree(s)",
        repo_id,
        branches.len()
    ));

    Ok(())
}
