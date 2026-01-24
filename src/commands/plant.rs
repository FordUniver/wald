use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::naming::worktree_dir_name;
use crate::output::Output;
use crate::types::ResolveResult;
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::gitignore::{add_worktree_to_gitignore, ensure_gitignore_section};
use crate::workspace::{collect_baum_ids, create_baum, is_baum, validate_workspace_path, Workspace};

/// Options for plant command
pub struct PlantOptions {
    pub repo_ref: String,
    pub container: PathBuf,
    pub branches: Vec<String>,
    pub force: bool,
    pub reuse: bool,
}

impl PlantOptions {
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

/// Plant a baum (create container with worktrees) or add worktrees to existing baum
pub fn plant(ws: &mut Workspace, opts: PlantOptions, out: &Output) -> Result<()> {
    out.require_human("plant")?;

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

    // Check if already a baum - if so, add worktrees to it
    let existing_baum = is_baum(&container);

    // Load existing baum or resolve repo for new baum
    let (mut baum_manifest, repo_id, is_new_baum) = if existing_baum {
        let manifest = load_baum(&container)?;
        let repo_id = manifest.repo_id.clone();

        // If repo_ref was provided and differs from existing baum, that's an error
        if !opts.repo_ref.is_empty() {
            match ws.manifest.resolve_with_details(&opts.repo_ref) {
                ResolveResult::Found(resolved_id) => {
                    if resolved_id != repo_id {
                        bail!(
                            "baum at {} is linked to {}, not {}",
                            container.display(),
                            repo_id,
                            resolved_id
                        );
                    }
                }
                ResolveResult::Ambiguous(matches) => {
                    bail!(
                        "'{}' is ambiguous, could be:\n  {}",
                        opts.repo_ref,
                        matches.join("\n  ")
                    );
                }
                ResolveResult::NotFound => {
                    // Ignore - the existing baum's repo_id will be used
                }
            }
        }

        (manifest, repo_id, false)
    } else {
        // Resolve repo reference to ID (required for new baum)
        if opts.repo_ref.is_empty() {
            bail!("repository reference required when creating a new baum");
        }

        let repo_id = match ws.manifest.resolve_with_details(&opts.repo_ref) {
            ResolveResult::Found(id) => id.to_string(),
            ResolveResult::Ambiguous(matches) => {
                bail!(
                    "'{}' is ambiguous, could be:\n  {}",
                    opts.repo_ref,
                    matches.join("\n  ")
                );
            }
            ResolveResult::NotFound => {
                bail!("repository not found in manifest: {}", opts.repo_ref);
            }
        };

        let manifest = create_baum(&container, &repo_id)?;
        (manifest, repo_id, true)
    };

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

    // Capture branch mode before moving branches
    let branch_mode = opts.branch_mode();

    // Determine branches to create
    let branches = if opts.branches.is_empty() {
        // Default to the default branch
        let default_branch = git::bare::get_default_branch(&bare_path)?;
        vec![default_branch]
    } else {
        opts.branches
    };

    // Check for duplicate branches if adding to existing baum
    if !is_new_baum {
        for branch in &branches {
            if baum_manifest.worktrees.iter().any(|wt| &wt.branch == branch) {
                bail!(
                    "worktree for branch '{}' already exists in baum at {}",
                    branch,
                    container.display()
                );
            }
        }
    }

    if is_new_baum {
        out.status(
            "Planting",
            &format!("{} at {}", repo_id, opts.container.display()),
        );
    } else {
        out.status(
            "Adding to baum",
            &format!("{} at {}", repo_id, opts.container.display()),
        );
    }

    // Collect existing baum IDs to avoid collisions
    let existing_ids = collect_baum_ids(&ws.root);

    // Ensure the baum has an ID before creating worktrees
    let baum_id = baum_manifest.ensure_id(&existing_ids).to_string();

    // Create worktrees for each branch using tracking branches
    let mut created_count = 0;
    for branch in &branches {
        let worktree_name = worktree_dir_name(branch);
        let worktree_path = container.join(&worktree_name);

        out.status(
            "Creating worktree",
            &format!("{} -> {}", branch, worktree_name),
        );

        // Add worktree with tracking branch (wald/<baum_id>/<branch>)
        let local_branch = git::add_worktree_with_tracking_mode(
            &bare_path,
            &worktree_path,
            branch,
            &baum_id,
            branch_mode,
        )?;

        // Update baum manifest with local branch info
        baum_manifest.add_worktree_with_local(branch, &worktree_name, &local_branch);

        // Add to container's .gitignore
        add_worktree_to_gitignore(&container, &worktree_name)?;

        created_count += 1;
    }

    // Save updated baum manifest (ID already set)
    save_baum(&container, &baum_manifest)?;

    if is_new_baum {
        out.success(&format!(
            "Planted {} with {} worktree(s)",
            repo_id, created_count
        ));
    } else {
        out.success(&format!("Added {} worktree(s) to baum", created_count));
    }

    Ok(())
}
