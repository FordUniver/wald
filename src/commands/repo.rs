use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::output::{Output, OutputFormat};
use crate::types::{DepthPolicy, LfsPolicy, RepoEntry, RepoId};
use crate::workspace::Workspace;

/// Options for repo add command
pub struct RepoAddOptions {
    pub repo_id: String,
    pub lfs: Option<LfsPolicy>,
    pub depth: Option<DepthPolicy>,
    pub upstream: Option<String>,
    pub aliases: Vec<String>,
    pub clone: bool,
}

/// Add a repository to the manifest
pub fn repo_add(ws: &mut Workspace, opts: RepoAddOptions, out: &Output) -> Result<()> {
    // Validate repo ID
    let id = RepoId::parse(&opts.repo_id)?;
    let repo_id = id.as_str();

    // Check for duplicates
    if ws.manifest.has_repo(&repo_id) {
        bail!("repository already registered: {}", repo_id);
    }

    // Check for alias conflicts
    for alias in &opts.aliases {
        if let Some(existing) = ws.manifest.resolve_alias(alias) {
            bail!(
                "alias '{}' already in use by repository: {}",
                alias, existing
            );
        }
    }

    // Create entry with defaults from config
    let entry = RepoEntry {
        lfs: opts.lfs.unwrap_or_else(|| ws.config.default_lfs.clone()),
        depth: opts.depth.unwrap_or_else(|| ws.config.default_depth.clone()),
        upstream: opts.upstream,
        aliases: opts.aliases,
    };

    // Get depth for cloning
    let clone_depth = match &entry.depth {
        DepthPolicy::Full => None,
        DepthPolicy::Depth(d) => Some(*d),
    };

    // Clone bare repo if requested
    if opts.clone {
        let bare_path = ws.repos_dir().join(id.to_bare_path());
        if !bare_path.exists() {
            out.status("Cloning", &repo_id);
            git::clone_bare(&id, &bare_path, clone_depth)?;
        }
    }

    // Add to manifest
    ws.manifest.repos.insert(repo_id.clone(), entry);
    ws.save_manifest()?;

    out.success(&format!("Added repository: {}", repo_id));

    Ok(())
}

/// List registered repositories
pub fn repo_list(ws: &Workspace, out: &Output) -> Result<()> {
    if ws.manifest.repos.is_empty() {
        out.info("No repositories registered");
        return Ok(());
    }

    match out.format {
        OutputFormat::Human => {
            for (repo_id, entry) in &ws.manifest.repos {
                let mut info = vec![];

                // LFS policy
                let lfs_str = match &entry.lfs {
                    LfsPolicy::Full => "lfs:full",
                    LfsPolicy::Minimal => "lfs:minimal",
                    LfsPolicy::Skip => "lfs:skip",
                };
                info.push(lfs_str.to_string());

                // Depth
                let depth_str = match &entry.depth {
                    DepthPolicy::Full => "depth:full".to_string(),
                    DepthPolicy::Depth(d) => format!("depth:{}", d),
                };
                info.push(depth_str);

                // Check if bare repo exists
                let bare_path = ws.bare_repo_path(repo_id).ok();
                let cloned = bare_path.map(|p| p.exists()).unwrap_or(false);
                if cloned {
                    info.push("cloned".to_string());
                }

                // Upstream
                if let Some(upstream) = &entry.upstream {
                    info.push(format!("upstream:{}", upstream));
                }

                // Aliases
                if !entry.aliases.is_empty() {
                    info.push(format!("aliases:{}", entry.aliases.join(",")));
                }

                println!("  {} ({})", repo_id, info.join(", "));
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&ws.manifest.repos)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Remove a repository from the manifest
pub fn repo_remove(ws: &mut Workspace, repo_ref: &str, out: &Output) -> Result<()> {
    // Resolve alias to repo ID
    let repo_id = ws
        .resolve_repo(repo_ref)
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("repository not found: {}", repo_ref))?;

    // Remove from manifest
    ws.manifest.repos.remove(&repo_id);
    ws.save_manifest()?;

    out.success(&format!("Removed repository: {}", repo_id));

    Ok(())
}

/// Fetch updates for repositories
pub fn repo_fetch(ws: &Workspace, repo_ref: Option<&str>, out: &Output) -> Result<()> {
    let repos: Vec<(String, PathBuf)> = if let Some(r) = repo_ref {
        // Fetch specific repo
        let repo_id = ws
            .resolve_repo(r)
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("repository not found: {}", r))?;
        let bare_path = ws.bare_repo_path(&repo_id)?;
        if !bare_path.exists() {
            bail!("bare repo not found: {}", bare_path.display());
        }
        vec![(repo_id, bare_path)]
    } else {
        // Fetch all cloned repos
        ws.manifest
            .repos
            .keys()
            .filter_map(|id| {
                let path = ws.bare_repo_path(id).ok()?;
                if path.exists() {
                    Some((id.clone(), path))
                } else {
                    None
                }
            })
            .collect()
    };

    if repos.is_empty() {
        out.info("No repositories to fetch");
        return Ok(());
    }

    for (repo_id, bare_path) in repos {
        out.status("Fetching", &repo_id);
        git::fetch_bare(&bare_path)?;
    }

    out.success("Fetch complete");

    Ok(())
}
