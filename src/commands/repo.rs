use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::git;
use crate::output::{Output, OutputFormat};
use crate::types::{DepthPolicy, FilterPolicy, LfsPolicy, RepoEntry, RepoId};
use crate::workspace::Workspace;

/// Options for repo add command
pub struct RepoAddOptions {
    pub repo_id: String,
    pub lfs: Option<LfsPolicy>,
    pub depth: Option<DepthPolicy>,
    pub filter: Option<FilterPolicy>,
    pub upstream: Option<String>,
    pub aliases: Vec<String>,
    pub clone: bool,
}

/// Add a repository to the manifest
pub fn repo_add(ws: &mut Workspace, opts: RepoAddOptions, out: &Output) -> Result<()> {
    out.require_human("repo add")?;

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
                alias,
                existing
            );
        }
    }

    // Create entry with defaults from config
    let entry = RepoEntry {
        lfs: opts.lfs.unwrap_or_else(|| ws.config.default_lfs.clone()),
        depth: opts
            .depth
            .unwrap_or_else(|| ws.config.default_depth.clone()),
        filter: opts
            .filter
            .unwrap_or_else(|| ws.config.default_filter.clone()),
        upstream: opts.upstream,
        aliases: opts.aliases,
    };

    // Build clone options
    let clone_opts = git::CloneOptions {
        depth: match &entry.depth {
            DepthPolicy::Full => None,
            DepthPolicy::Depth(d) => Some(*d),
        },
        filter: entry.filter.as_git_arg().map(|s| s.to_string()),
    };

    // Clone bare repo if requested
    if opts.clone {
        let bare_path = ws.repos_dir().join(id.to_bare_path());
        if !bare_path.exists() {
            out.status("Cloning", &repo_id);
            git::clone_bare(&id, &bare_path, clone_opts)?;
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

    // Sort repo IDs for deterministic output
    let mut repo_ids: Vec<_> = ws.manifest.repos.keys().collect();
    repo_ids.sort();

    match out.format {
        OutputFormat::Human => {
            for repo_id in &repo_ids {
                let entry = &ws.manifest.repos[*repo_id];
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
            // Sort keys in JSON output for determinism
            let sorted: std::collections::BTreeMap<_, _> = ws.manifest.repos.iter().collect();
            let json = serde_json::to_string_pretty(&sorted)?;
            println!("{}", json);
        }
    }

    Ok(())
}

/// Remove a repository from the manifest
pub fn repo_remove(ws: &mut Workspace, repo_ref: &str, out: &Output) -> Result<()> {
    out.require_human("repo remove")?;

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

/// Options for repo fetch command
pub struct RepoFetchOptions {
    pub repo_ref: Option<String>,
    /// Convert partial clones to full and fetch all objects
    pub full: bool,
}

/// Fetch updates for repositories
pub fn repo_fetch(ws: &mut Workspace, opts: RepoFetchOptions, out: &Output) -> Result<()> {
    out.require_human("repo fetch")?;

    let repos: Vec<(String, PathBuf)> = if let Some(ref r) = opts.repo_ref {
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

    let mut updated_manifest = false;

    for (repo_id, bare_path) in repos {
        if opts.full {
            let is_partial = git::is_partial_clone(&bare_path)?;
            if is_partial {
                out.status("Converting to full clone", &repo_id);
                git::fetch_full(&bare_path)?;
                // Update manifest to reflect full clone
                if let Some(entry) = ws.manifest.repos.get_mut(&repo_id) {
                    entry.filter = FilterPolicy::None;
                    updated_manifest = true;
                }
            } else {
                out.status("Fetching", &format!("{} (already full)", repo_id));
                git::fetch_bare(&bare_path)?;
            }
        } else {
            out.status("Fetching", &repo_id);
            git::fetch_bare(&bare_path)?;
        }
    }

    if updated_manifest {
        ws.save_manifest()?;
    }

    out.success("Fetch complete");

    Ok(())
}

/// Options for repo gc command
pub struct RepoGcOptions {
    pub repo_ref: Option<String>,
    pub aggressive: bool,
}

/// Run garbage collection on repositories
pub fn repo_gc(ws: &Workspace, opts: RepoGcOptions, out: &Output) -> Result<()> {
    out.require_human("repo gc")?;

    let repos: Vec<(String, PathBuf)> = if let Some(ref r) = opts.repo_ref {
        // GC specific repo
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
        // GC all cloned repos
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
        out.info("No repositories to clean");
        return Ok(());
    }

    for (repo_id, bare_path) in repos {
        out.status("Cleaning", &repo_id);
        git::gc(&bare_path, opts.aggressive)?;
    }

    out.success("Garbage collection complete");

    Ok(())
}
