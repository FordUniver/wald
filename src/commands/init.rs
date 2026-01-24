use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};

use crate::output::Output;
use crate::workspace::Workspace;

/// Options for the init command
pub struct InitOptions {
    /// Path to initialize (default: current directory)
    pub path: Option<PathBuf>,
    /// Recreate .wald/ if it exists
    pub force: bool,
    /// Don't run git init (error if not already a git repo)
    pub no_git: bool,
}

/// Initialize a new wald workspace
pub fn init(opts: InitOptions, out: &Output) -> Result<()> {
    out.require_human("init")?;

    // Determine target path
    let target = opts
        .path
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let target = target.canonicalize().unwrap_or_else(|_| target.clone());

    // Check if target is a git repository
    if !Workspace::is_git_repo(&target) {
        if opts.no_git {
            bail!(
                "{} is not a git repository. Remove --no-git to initialize one.",
                target.display()
            );
        }

        // Run git init
        out.info(&format!("Initializing git repository at {}", target.display()));
        let status = Command::new("git")
            .args(["init"])
            .current_dir(&target)
            .status()?;

        if !status.success() {
            bail!("git init failed");
        }
    }

    // Initialize workspace
    Workspace::init(&target, opts.force)?;

    out.success(&format!(
        "Initialized wald workspace at {}",
        target.display()
    ));
    out.info("");
    out.info("Next steps:");
    out.info("  1. Add repositories: wald repo add github.com/user/repo");
    out.info("  2. Create a baum:    wald plant <repo> <container> <branch>");

    Ok(())
}
