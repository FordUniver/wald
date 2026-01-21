# wald

Git workspace manager: bare repos, worktrees, and cross-machine sync.

## Overview

Wald ("forest") manages a personal workspace containing many git repositories:

- Centralized bare repo storage in `.wald/repos/`
- Worktrees placed throughout the workspace in baum containers
- Cross-machine synchronization via tracked manifests
- Health checks and diagnostic tools

## Installation

```bash
cargo install --path .
# or
make install  # installs to ~/.local/bin
```

## Commands

### Repository management

```bash
wald repo add <repo-id> [--clone]  # Register repo, optionally clone bare
wald repo list                      # List registered repos
wald repo remove <repo-id>          # Unregister repo
wald repo fetch [repo-id]           # Fetch updates (all if no repo specified)
```

Repo IDs use the format `host/path` (e.g., `github.com/user/repo` or `git.zib.de/group/subgroup/repo`).

### Baum management

```bash
wald plant <repo> <path> [branches...]  # Create baum with worktrees
wald uproot <path> [--force]            # Remove baum and all worktrees
wald move <old-path> <new-path>         # Move baum (updates manifests)
```

### Worktree management

```bash
wald branch <baum> <branch>    # Add worktree to existing baum
wald prune <baum> <branch...>  # Remove worktree(s) from baum
wald worktrees [path]          # List all worktrees (optionally filtered)
```

### Synchronization

```bash
wald sync [--dry-run] [--force]  # Pull workspace, replay moves
wald status                       # Show workspace sync status
wald doctor [--fix]               # Check health, optionally repair
```

## Directory structure

```
$WORKSPACE/
├── .wald/
│   ├── manifest.yaml      # Repo registry (tracked)
│   ├── config.yaml        # Settings (tracked)
│   ├── state.yaml         # Sync state (gitignored)
│   └── repos/             # Bare repos (gitignored)
│
└── path/to/project/       # Baum container
    ├── .baum/
    │   └── manifest.yaml  # Worktree declarations (tracked)
    ├── _main.wt/          # Worktree (gitignored)
    ├── _dev.wt/           # Worktree (gitignored)
    └── CLAUDE.md          # Personal metadata (tracked)
```

## Development

```bash
make test           # Run all tests (unit + integration)
make test-unit      # Run Rust unit tests only
make test-verbose   # Run with verbose output
cargo build         # Build debug binary
```

## Status

Core commands implemented. Not yet production-ready.

**Implemented:** repo, plant, uproot, move, branch, prune, worktrees, sync, status, doctor

**Not yet implemented:** backup/restore/export, local worktrees (`--local`), daemon mode
