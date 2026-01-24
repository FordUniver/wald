use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::id::parse_wald_branch;
use crate::output::Output;
use crate::workspace::baum::{load_baum, save_baum};
use crate::workspace::{find_all_baums, is_baum, validate_workspace_path, Workspace};

/// Options for prune command
pub struct PruneOptions {
    pub baum_path: PathBuf,
    pub branches: Vec<String>,
    pub force: bool,
}

/// Remove worktrees for branches from a baum
pub fn prune(ws: &Workspace, opts: PruneOptions, out: &Output) -> Result<()> {
    out.require_human("prune")?;

    // Resolve path relative to workspace (with path traversal protection)
    let container = validate_workspace_path(&ws.root, &opts.baum_path)?;

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

/// Clean up orphan wald/* branches across all repositories
///
/// A branch is considered orphan if:
/// - It matches the wald/<baum_id>/<branch> pattern
/// - No baum with that baum_id exists, OR
/// - The baum exists but doesn't have a worktree for that branch
pub fn prune_branches(ws: &Workspace, force: bool, out: &Output) -> Result<()> {
    out.require_human("prune --branches")?;

    // Collect all baum IDs and their worktrees
    let baums = find_all_baums(&ws.root);

    // Build a set of (baum_id, branch) pairs that are in use
    let mut in_use: HashSet<(String, String)> = HashSet::new();
    let mut baum_ids: HashSet<String> = HashSet::new();

    for (_, manifest) in &baums {
        if let Some(id) = &manifest.id {
            baum_ids.insert(id.clone());
            for wt in &manifest.worktrees {
                in_use.insert((id.clone(), wt.branch.clone()));
            }
        }
    }

    // Scan all repos for wald/* branches
    let mut total_removed = 0;
    let mut total_skipped = 0;

    for repo_id in ws.manifest.repos.keys() {
        let bare_path = match ws.bare_repo_path(repo_id) {
            Ok(p) if p.exists() => p,
            _ => continue,
        };

        let wald_branches = match git::list_wald_branches(&bare_path) {
            Ok(branches) => branches,
            Err(_) => continue,
        };

        for branch in wald_branches {
            // Parse the branch name to get baum_id and logical branch
            let Some((baum_id, logical_branch)) = parse_wald_branch(&branch) else {
                continue;
            };

            // Check if this branch is in use
            let key = (baum_id.to_string(), logical_branch.to_string());
            if in_use.contains(&key) {
                continue;
            }

            // Check if baum still exists (might be a renamed branch)
            let baum_exists = baum_ids.contains(baum_id);

            // Check for unpushed commits
            let has_unpushed = git::has_unpushed_commits(&bare_path, &branch).unwrap_or(false);

            if has_unpushed && !force {
                out.warn(&format!(
                    "{}: {} has unpushed commits, skipping (use --force to delete)",
                    repo_id, branch
                ));
                total_skipped += 1;
                continue;
            }

            // Delete the orphan branch
            let reason = if baum_exists {
                "worktree removed"
            } else {
                "baum not found"
            };

            out.status("Deleting", &format!("{}: {} ({})", repo_id, branch, reason));

            match git::delete_branch(&bare_path, &branch, force) {
                Ok(()) => total_removed += 1,
                Err(e) => {
                    out.warn(&format!("Failed to delete {}: {}", branch, e));
                    total_skipped += 1;
                }
            }
        }
    }

    if total_removed > 0 {
        out.success(&format!("Deleted {} orphan branch(es)", total_removed));
    }

    if total_skipped > 0 {
        out.info(&format!("Skipped {} branch(es)", total_skipped));
    }

    if total_removed == 0 && total_skipped == 0 {
        out.info("No orphan branches found");
    }

    Ok(())
}
