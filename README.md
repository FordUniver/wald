# wald

Git workspace manager: bare repos, worktrees, and cross-machine sync.

## Overview

Wald ("forest") manages a personal workspace containing many git repositories:

- Centralized bare repo storage in `.wald/repos/`
- Worktrees placed throughout the workspace in baum containers
- Cross-machine synchronization via tracked manifests
- Background monitoring for stale repos and sync conflicts

## Planned features

- `wald repo add/remove/list/fetch` — manage registered repos
- `wald plant/uproot/move` — manage baum containers
- `wald branch/prune/worktrees` — manage worktrees within baum
- `wald sync/status/doctor` — synchronization and health checks
- `wald backup/restore/export` — git bundle-based backup
