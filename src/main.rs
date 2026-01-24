use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

use wald::commands;
use wald::output::{print_error, Output, OutputFormat};
use wald::types::{DepthPolicy, FilterPolicy, LfsPolicy};
use wald::workspace::Workspace;

#[derive(Parser)]
#[command(name = "wald")]
#[command(about = "Git workspace manager: bare repos, worktrees, and cross-machine sync")]
#[command(version = env!("WALD_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new wald workspace
    Init {
        /// Path to initialize (default: current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Recreate .wald/ directory if it exists
        #[arg(long)]
        force: bool,

        /// Don't run git init (error if not already a git repo)
        #[arg(long)]
        no_git: bool,
    },

    /// Manage repository registry
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },

    /// Plant a baum (create container with worktrees)
    #[command(visible_alias = "create")]
    Plant {
        /// Repository ID or alias
        repo: String,

        /// Container path (relative to workspace root)
        container: PathBuf,

        /// Branches to create worktrees for (default: default branch)
        #[arg(trailing_var_arg = true)]
        branches: Vec<String>,

        /// Delete existing local branch, create fresh from origin
        #[arg(long, conflicts_with = "reuse")]
        force: bool,

        /// Use existing local branch as-is (skip if has unpushed commits)
        #[arg(long)]
        reuse: bool,
    },

    /// Uproot a baum (remove container and worktrees)
    #[command(visible_alias = "rm")]
    Uproot {
        /// Path to the baum container
        path: PathBuf,

        /// Force removal even with uncommitted changes
        #[arg(short, long)]
        force: bool,
    },

    /// Move a baum to a new location
    #[command(visible_alias = "graft", visible_alias = "mv")]
    Move {
        /// Current baum path
        old_path: PathBuf,

        /// New baum path
        new_path: PathBuf,
    },

    /// Add a worktree for a branch to an existing baum
    Branch {
        /// Path to the baum container
        baum: PathBuf,

        /// Branch name
        branch: String,

        /// Delete existing local branch, create fresh from origin
        #[arg(long, conflicts_with = "reuse")]
        force: bool,

        /// Use existing local branch as-is (skip if has unpushed commits)
        #[arg(long)]
        reuse: bool,
    },

    /// Remove worktrees for branches from a baum, or clean up orphan branches
    Prune {
        /// Path to the baum container (required unless --branches)
        #[arg(required_unless_present = "cleanup_branches")]
        baum: Option<PathBuf>,

        /// Branches to remove (required unless --branches)
        #[arg(required_unless_present = "cleanup_branches")]
        branches: Vec<String>,

        /// Force removal even with uncommitted changes
        #[arg(short, long)]
        force: bool,

        /// Clean up orphan wald/* branches (workspace-wide)
        #[arg(long = "branches", conflicts_with_all = ["baum", "branches"])]
        cleanup_branches: bool,
    },

    /// List all worktrees in the workspace
    Worktrees {
        /// Filter by path
        filter: Option<PathBuf>,
    },

    /// Sync workspace with remote
    Sync {
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,

        /// Force sync even if diverged
        #[arg(long)]
        force: bool,

        /// Push changes after syncing
        #[arg(long)]
        push: bool,
    },

    /// Show workspace status
    Status,

    /// Check workspace health and repair issues
    Doctor {
        /// Attempt to fix issues
        #[arg(long)]
        fix: bool,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    /// Add a repository to the registry
    Add {
        /// Repository ID (host/path, e.g., github.com/user/repo)
        repo_id: String,

        /// LFS fetch policy
        #[arg(long, value_parser = parse_lfs)]
        lfs: Option<LfsPolicy>,

        /// Clone depth (number or "full")
        #[arg(long, value_parser = parse_depth)]
        depth: Option<DepthPolicy>,

        /// Partial clone filter (blob-none for fast clone, blobs fetched on demand)
        #[arg(long, value_parser = parse_filter)]
        filter: Option<FilterPolicy>,

        /// Upstream repository for fork tracking
        #[arg(long)]
        upstream: Option<String>,

        /// Short aliases for this repo
        #[arg(long = "alias", action = clap::ArgAction::Append)]
        aliases: Vec<String>,

        /// Skip cloning (only add to manifest)
        #[arg(long)]
        no_clone: bool,
    },

    /// List registered repositories
    List,

    /// Remove a repository from the registry
    Remove {
        /// Repository ID or alias
        repo: String,
    },

    /// Fetch updates for repositories
    Fetch {
        /// Repository ID or alias (all if not specified)
        repo: Option<String>,

        /// Convert partial clones to full and fetch all objects
        #[arg(long)]
        full: bool,
    },

    /// Run garbage collection on repositories
    Gc {
        /// Repository ID or alias (all if not specified)
        repo: Option<String>,

        /// Aggressive garbage collection (slower but more thorough)
        #[arg(long)]
        aggressive: bool,
    },
}

fn parse_lfs(s: &str) -> Result<LfsPolicy, String> {
    match s.to_lowercase().as_str() {
        "full" => Ok(LfsPolicy::Full),
        "minimal" => Ok(LfsPolicy::Minimal),
        "skip" => Ok(LfsPolicy::Skip),
        _ => Err(format!(
            "Invalid LFS policy: {}. Use full, minimal, or skip",
            s
        )),
    }
}

fn parse_depth(s: &str) -> Result<DepthPolicy, String> {
    if s.to_lowercase() == "full" {
        Ok(DepthPolicy::Full)
    } else {
        s.parse::<u32>()
            .map(DepthPolicy::Depth)
            .map_err(|_| format!("Invalid depth: {}. Use a number or 'full'", s))
    }
}

fn parse_filter(s: &str) -> Result<FilterPolicy, String> {
    match s.to_lowercase().replace(':', "-").as_str() {
        "none" => Ok(FilterPolicy::None),
        "blob-none" => Ok(FilterPolicy::BlobNone),
        "tree-0" | "tree-zero" => Ok(FilterPolicy::TreeZero),
        _ => Err(format!(
            "Invalid filter: {}. Use none, blob-none, or tree-0",
            s
        )),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Human
    };

    let out = Output::new(format, cli.verbose);

    if let Err(e) = run(cli, &out) {
        print_error(&e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(cli: Cli, out: &Output) -> anyhow::Result<()> {
    // Handle commands that don't require an existing workspace
    match &cli.command {
        Commands::Completion { shell } => {
            generate_completions(*shell);
            return Ok(());
        }
        Commands::Init {
            path,
            force,
            no_git,
        } => {
            let opts = commands::init::InitOptions {
                path: path.clone(),
                force: *force,
                no_git: *no_git,
            };
            return commands::init(opts, out);
        }
        _ => {}
    }

    // Load workspace for all other commands
    let mut ws = Workspace::load()?;

    match cli.command {
        Commands::Repo { action } => match action {
            RepoAction::Add {
                repo_id,
                lfs,
                depth,
                filter,
                upstream,
                aliases,
                no_clone,
            } => {
                let opts = commands::repo::RepoAddOptions {
                    repo_id,
                    lfs,
                    depth,
                    filter,
                    upstream,
                    aliases,
                    clone: !no_clone, // Clone by default, --no-clone skips
                };
                commands::repo_add(&mut ws, opts, out)
            }
            RepoAction::List => commands::repo_list(&ws, out),
            RepoAction::Remove { repo } => commands::repo_remove(&mut ws, &repo, out),
            RepoAction::Fetch { repo, full } => {
                let opts = commands::repo::RepoFetchOptions {
                    repo_ref: repo,
                    full,
                };
                commands::repo_fetch(&mut ws, opts, out)
            }
            RepoAction::Gc { repo, aggressive } => {
                let opts = commands::repo::RepoGcOptions {
                    repo_ref: repo,
                    aggressive,
                };
                commands::repo_gc(&ws, opts, out)
            }
        },

        Commands::Plant {
            repo,
            container,
            branches,
            force,
            reuse,
        } => {
            let opts = commands::plant::PlantOptions {
                repo_ref: repo,
                container,
                branches,
                force,
                reuse,
            };
            commands::plant(&mut ws, opts, out)
        }

        Commands::Uproot { path, force } => {
            let opts = commands::uproot::UprootOptions { path, force };
            commands::uproot(&ws, opts, out)
        }

        Commands::Move { old_path, new_path } => {
            let opts = commands::move_cmd::MoveOptions { old_path, new_path };
            commands::move_baum(&ws, opts, out)
        }

        Commands::Branch {
            baum,
            branch,
            force,
            reuse,
        } => {
            let opts = commands::branch::BranchOptions {
                baum_path: baum,
                branch,
                force,
                reuse,
            };
            commands::branch(&ws, opts, out)
        }

        Commands::Prune {
            baum,
            branches,
            force,
            cleanup_branches,
        } => {
            if cleanup_branches {
                commands::prune_branches(&ws, force, out)
            } else {
                let opts = commands::prune::PruneOptions {
                    baum_path: baum.expect("baum required"),
                    branches,
                    force,
                };
                commands::prune(&ws, opts, out)
            }
        }

        Commands::Worktrees { filter } => {
            let opts = commands::worktrees::WorktreesOptions { filter };
            commands::worktrees(&ws, opts, out)
        }

        Commands::Sync {
            dry_run,
            force,
            push,
        } => {
            let opts = commands::sync::SyncOptions {
                dry_run,
                force,
                push,
            };
            commands::sync(&mut ws, opts, out)
        }

        Commands::Status => commands::status(&ws, out),

        Commands::Doctor { fix } => {
            let opts = commands::doctor::DoctorOptions { fix };
            commands::doctor(&ws, opts, out)
        }

        Commands::Init { .. } => unreachable!(),
        Commands::Completion { .. } => unreachable!(),
    }
}

fn generate_completions(shell: Shell) {
    use clap::CommandFactory;
    use clap_complete::generate;

    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut std::io::stdout());
}
