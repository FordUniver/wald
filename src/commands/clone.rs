use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};

use crate::commands;
use crate::output::Output;
use crate::workspace::Workspace;

pub struct CloneOptions {
    pub url: String,
    pub dir: Option<PathBuf>,
}

pub fn clone(opts: CloneOptions, out: &Output) -> Result<()> {
    out.require_human("clone")?;

    // Determine target directory
    let dir = match &opts.dir {
        Some(d) => d.clone(),
        None => {
            // Extract repo name from URL
            let name = extract_repo_name(&opts.url)?;
            PathBuf::from(name)
        }
    };

    // Git clone the workspace
    out.status("Cloning workspace", &opts.url);
    let status = Command::new("git")
        .args(["clone", &opts.url])
        .arg(&dir)
        .status()?;

    if !status.success() {
        bail!("git clone failed");
    }

    // Load workspace and run sync
    let mut ws = Workspace::load_from(dir.clone())?;
    let sync_opts = commands::sync::SyncOptions {
        dry_run: false,
        force: false,
        push: false,
        offline: false,
    };

    out.status("Hydrating", "cloning missing repos");
    commands::sync(&mut ws, sync_opts, out)?;

    out.success(&format!(
        "Cloned and hydrated workspace at {}",
        dir.display()
    ));
    Ok(())
}

fn extract_repo_name(url: &str) -> Result<String> {
    // Handle git@host:path/repo.git and https://host/path/repo.git
    let name = url
        .trim_end_matches(".git")
        .rsplit(['/', ':'])
        .next()
        .ok_or_else(|| anyhow::anyhow!("cannot extract repo name from URL"))?;
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:user/repo.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_no_git_suffix() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_subgroup() {
        assert_eq!(
            extract_repo_name("git@git.zib.de:tools/subgroup/repo.git").unwrap(),
            "repo"
        );
    }
}
