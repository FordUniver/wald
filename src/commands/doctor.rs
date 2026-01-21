use std::path::PathBuf;

use anyhow::Result;
use walkdir::WalkDir;

use crate::git;
use crate::output::Output;
use crate::workspace::{is_baum, Workspace};
use crate::workspace::baum::load_baum;

/// Options for doctor command
pub struct DoctorOptions {
    pub fix: bool,
}

/// Check workspace health and optionally repair issues
pub fn doctor(ws: &Workspace, opts: DoctorOptions, out: &Output) -> Result<()> {
    let mut issues = Vec::new();

    out.status("Checking", "workspace structure");

    // Check .wald directory structure
    let wald_dir = ws.wald_dir();
    if !wald_dir.join("manifest.yaml").exists() {
        issues.push(Issue {
            severity: Severity::Error,
            message: "Missing manifest.yaml".to_string(),
            fix: None,
        });
    }

    // Check repos directory
    let repos_dir = ws.repos_dir();
    if !repos_dir.exists() {
        issues.push(Issue {
            severity: Severity::Warning,
            message: "Missing repos directory".to_string(),
            fix: Some(FixAction::CreateDir(repos_dir.clone())),
        });
    }

    out.status("Checking", "registered repositories");

    // Check each registered repo
    for (repo_id, _entry) in &ws.manifest.repos {
        if let Ok(bare_path) = ws.bare_repo_path(repo_id) {
            if !bare_path.exists() {
                issues.push(Issue {
                    severity: Severity::Warning,
                    message: format!("Bare repo not cloned: {}", repo_id),
                    fix: None,
                });
            }
        }
    }

    out.status("Checking", "planted baums");

    // Find and check all baums
    for entry in WalkDir::new(&ws.root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip .git directories, .wald/repos, and _*.wt worktree directories
            let name = e.file_name().to_string_lossy();
            if name == ".git" {
                return false;
            }
            if name == "repos" && e.path().parent().map(|p| p.ends_with(".wald")).unwrap_or(false) {
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
            check_baum(ws, entry.path(), &mut issues, out)?;
        }
    }

    // Report findings
    println!();
    if issues.is_empty() {
        out.success("No issues found");
    } else {
        let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
        let warnings = issues.iter().filter(|i| i.severity == Severity::Warning).count();

        println!("Found {} issue(s) ({} errors, {} warnings)", issues.len(), errors, warnings);
        println!();

        for issue in &issues {
            let prefix = match issue.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARN",
            };
            println!("  [{}] {}", prefix, issue.message);

            if opts.fix {
                if let Some(fix) = &issue.fix {
                    match apply_fix(fix) {
                        Ok(_) => println!("         Fixed!"),
                        Err(e) => println!("         Failed to fix: {}", e),
                    }
                }
            }
        }

        if !opts.fix && issues.iter().any(|i| i.fix.is_some()) {
            println!();
            println!("Run with --fix to automatically repair fixable issues");
        }
    }

    Ok(())
}

fn check_baum(
    ws: &Workspace,
    baum_path: &std::path::Path,
    issues: &mut Vec<Issue>,
    _out: &Output,
) -> Result<()> {
    // Load baum manifest
    let baum = match load_baum(baum_path) {
        Ok(b) => b,
        Err(e) => {
            issues.push(Issue {
                severity: Severity::Error,
                message: format!(
                    "Invalid baum manifest at {}: {}",
                    baum_path.display(),
                    e
                ),
                fix: None,
            });
            return Ok(());
        }
    };

    // Check if repo is registered
    if !ws.manifest.has_repo(&baum.repo_id) {
        issues.push(Issue {
            severity: Severity::Warning,
            message: format!(
                "Baum {} references unregistered repo: {}",
                baum_path.display(),
                baum.repo_id
            ),
            fix: None,
        });
    }

    // Check bare repo exists
    if let Ok(bare_path) = ws.bare_repo_path(&baum.repo_id) {
        if !bare_path.exists() {
            issues.push(Issue {
                severity: Severity::Error,
                message: format!(
                    "Baum {} missing bare repo: {}",
                    baum_path.display(),
                    bare_path.display()
                ),
                fix: None,
            });
            return Ok(());
        }

        // Check worktrees
        let worktree_list = git::list_worktrees(&bare_path).unwrap_or_default();

        for wt in &baum.worktrees {
            let wt_path = baum_path.join(&wt.path);

            // Check worktree directory exists
            if !wt_path.exists() {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!(
                        "Missing worktree directory: {} (branch: {})",
                        wt_path.display(),
                        wt.branch
                    ),
                    fix: None,
                });
                continue;
            }

            // Check .git file exists
            if !wt_path.join(".git").exists() {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!(
                        "Invalid worktree (missing .git): {}",
                        wt_path.display()
                    ),
                    fix: None,
                });
            }

            // Check worktree is in git's list
            let wt_path_str = wt_path.to_string_lossy();
            if !worktree_list.iter().any(|w| w.path == wt_path_str) {
                issues.push(Issue {
                    severity: Severity::Warning,
                    message: format!(
                        "Worktree not in git's list: {}",
                        wt_path.display()
                    ),
                    fix: None,
                });
            }
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Severity {
    Error,
    Warning,
}

struct Issue {
    severity: Severity,
    message: String,
    fix: Option<FixAction>,
}

enum FixAction {
    CreateDir(PathBuf),
}

fn apply_fix(fix: &FixAction) -> Result<()> {
    match fix {
        FixAction::CreateDir(path) => {
            std::fs::create_dir_all(path)?;
            Ok(())
        }
    }
}
