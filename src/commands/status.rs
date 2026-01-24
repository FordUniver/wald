use std::process::Command;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::output::{Output, OutputFormat};
use crate::workspace::baum::load_baum;
use crate::workspace::{Workspace, is_baum};

/// Show workspace status
pub fn status(ws: &Workspace, out: &Output) -> Result<()> {
    // Get git status
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&ws.root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .context("failed to check git status")?;

    let git_status = String::from_utf8_lossy(&status_output.stdout);
    let is_clean = git_status.trim().is_empty();

    // Check ahead/behind
    let ab_output = Command::new("git")
        .arg("-C")
        .arg(&ws.root)
        .arg("rev-list")
        .arg("--left-right")
        .arg("--count")
        .arg("HEAD...@{upstream}")
        .output();

    let (ahead, behind) = if let Ok(ab) = ab_output {
        if ab.status.success() {
            let ab_str = String::from_utf8_lossy(&ab.stdout);
            let parts: Vec<&str> = ab_str.trim().split('\t').collect();
            if parts.len() == 2 {
                (
                    parts[0].parse::<u32>().unwrap_or(0),
                    parts[1].parse::<u32>().unwrap_or(0),
                )
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    // Count baums and worktrees
    let mut baum_count = 0;
    let mut worktree_count = 0;

    for entry in WalkDir::new(&ws.root)
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
        .flatten()
    {
        if entry.file_type().is_dir() && is_baum(entry.path()) {
            baum_count += 1;
            if let Ok(baum) = load_baum(entry.path()) {
                worktree_count += baum.worktrees.len();
            }
        }
    }

    match out.format {
        OutputFormat::Human => {
            // Workspace status
            if is_clean {
                println!("Workspace: clean");
            } else {
                println!("Workspace: has uncommitted changes");
            }

            // Sync status
            match (ahead, behind) {
                (0, 0) => println!("Sync: up to date"),
                (a, 0) => println!("Sync: {} commit(s) ahead of remote", a),
                (0, b) => println!("Sync: {} commit(s) behind remote", b),
                (a, b) => println!("Sync: diverged ({} ahead, {} behind)", a, b),
            }

            // Last sync
            if let Some(last) = &ws.state.last_sync {
                println!("Last sync: {}", &last[..8.min(last.len())]);
            } else {
                println!("Last sync: never");
            }

            // Counts
            println!("Repos: {} registered", ws.manifest.repos.len());
            println!(
                "Baums: {} planted ({} worktrees)",
                baum_count, worktree_count
            );
        }
        OutputFormat::Json => {
            let status = serde_json::json!({
                "workspace": {
                    "clean": is_clean,
                    "ahead": ahead,
                    "behind": behind,
                },
                "last_sync": ws.state.last_sync,
                "repos_count": ws.manifest.repos.len(),
                "baums_count": baum_count,
                "worktrees_count": worktree_count,
            });
            println!("{}", serde_json::to_string_pretty(&status)?);
        }
    }

    Ok(())
}
