use std::path::PathBuf;

use anyhow::Result;
use walkdir::WalkDir;

use crate::output::{Output, OutputFormat};
use crate::workspace::baum::load_baum;
use crate::workspace::{Workspace, is_baum, validate_workspace_path};

/// Options for worktrees command
pub struct WorktreesOptions {
    pub filter: Option<PathBuf>,
}

/// List all worktrees in the workspace
pub fn worktrees(ws: &Workspace, opts: WorktreesOptions, out: &Output) -> Result<()> {
    let search_root = if let Some(filter) = opts.filter {
        // Validate filter path (with path traversal protection)
        validate_workspace_path(&ws.root, &filter)?
    } else {
        ws.root.clone()
    };

    // Find all baums
    let mut all_worktrees: Vec<WorktreeDisplay> = Vec::new();

    for entry in WalkDir::new(&search_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip .git directories, .wald/repos, and _*.wt worktree directories
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return false;
            }
            if name == "repos"
                && e.path()
                    .parent()
                    .map(|p| p.ends_with(".wald"))
                    .unwrap_or(false)
            {
                return false;
            }
            // Skip worktree directories (no need to descend into them)
            if e.file_type().is_dir() && name.starts_with('_') && name.ends_with(".wt") {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_dir() && is_baum(entry.path()) {
            // Load baum and get worktrees
            if let Ok(baum) = load_baum(entry.path()) {
                let container_path = entry
                    .path()
                    .strip_prefix(&ws.root)
                    .unwrap_or(entry.path())
                    .to_path_buf();

                for wt in &baum.worktrees {
                    all_worktrees.push(WorktreeDisplay {
                        repo_id: baum.repo_id.clone(),
                        container: container_path.to_string_lossy().to_string(),
                        branch: wt.branch.clone(),
                        path: wt.path.clone(),
                    });
                }
            }
        }
    }

    if all_worktrees.is_empty() {
        out.info("No worktrees found");
        return Ok(());
    }

    // Sort for deterministic output: by container, then by branch
    all_worktrees.sort_by(|a, b| (&a.container, &a.branch).cmp(&(&b.container, &b.branch)));

    match out.format {
        OutputFormat::Human => {
            // Group by container
            let mut current_container = String::new();
            for wt in &all_worktrees {
                if wt.container != current_container {
                    if !current_container.is_empty() {
                        println!();
                    }
                    println!("{} ({})", wt.container, wt.repo_id);
                    current_container = wt.container.clone();
                }
                println!("  {} -> {}", wt.branch, wt.path);
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&all_worktrees)?;
            println!("{}", json);
        }
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct WorktreeDisplay {
    repo_id: String,
    container: String,
    branch: String,
    path: String,
}
