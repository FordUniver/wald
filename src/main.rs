use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

use wald::commands;
use wald::output::{print_error, Output, OutputFormat};
use wald::types::{DepthPolicy, LfsPolicy};
use wald::workspace::Workspace;

#[derive(Parser)]
#[command(name = "wald")]
#[command(about = "Git workspace manager: bare repos, worktrees, and cross-machine sync")]
#[command(version)]
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
    },

    /// Remove worktrees for branches from a baum
    Prune {
        /// Path to the baum container
        baum: PathBuf,

        /// Branches to remove
        #[arg(required = true)]
        branches: Vec<String>,

        /// Force removal even with uncommitted changes
        #[arg(short, long)]
        force: bool,
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
        Commands::Init { path, force } => {
            let opts = commands::init::InitOptions {
                path: path.clone(),
                force: *force,
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
                upstream,
                aliases,
                no_clone,
            } => {
                let opts = commands::repo::RepoAddOptions {
                    repo_id,
                    lfs,
                    depth,
                    upstream,
                    aliases,
                    clone: !no_clone, // Clone by default, --no-clone skips
                };
                commands::repo_add(&mut ws, opts, out)
            }
            RepoAction::List => commands::repo_list(&ws, out),
            RepoAction::Remove { repo } => commands::repo_remove(&mut ws, &repo, out),
            RepoAction::Fetch { repo } => commands::repo_fetch(&ws, repo.as_deref(), out),
        },

        Commands::Plant {
            repo,
            container,
            branches,
        } => {
            let opts = commands::plant::PlantOptions {
                repo_ref: repo,
                container,
                branches,
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

        Commands::Branch { baum, branch } => {
            let opts = commands::branch::BranchOptions {
                baum_path: baum,
                branch,
            };
            commands::branch(&ws, opts, out)
        }

        Commands::Prune {
            baum,
            branches,
            force,
        } => {
            let opts = commands::prune::PruneOptions {
                baum_path: baum,
                branches,
                force,
            };
            commands::prune(&ws, opts, out)
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
