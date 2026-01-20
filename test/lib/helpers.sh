#!/usr/bin/env bash
# Wald-specific test helpers
# Functions for creating test repos, manipulating manifests, and validating state

# ====================================================================================
# Manifest Manipulation
# ====================================================================================

# Add repo entry to manifest.yaml
# Usage: add_repo_to_manifest <repo_id> [lfs_policy] [depth]
#
# Example: add_repo_to_manifest "github.com/user/repo" "full" "100"
add_repo_to_manifest() {
    local repo_id="$1"
    local lfs="${2:-minimal}"
    local depth="${3:-100}"

    # Read current manifest
    local manifest_file=".wald/manifest.yaml"

    # Use yq to add repo entry
    if command -v yq >/dev/null 2>&1; then
        yq -i ".repos[\"$repo_id\"].lfs = \"$lfs\"" "$manifest_file"
        yq -i ".repos[\"$repo_id\"].depth = $depth" "$manifest_file"
    else
        # Fallback: manual append (less robust but works without yq)
        if grep -q "^repos: {}" "$manifest_file" 2>/dev/null; then
            # Replace empty repos object
            cat > "$manifest_file" <<EOF
repos:
  $repo_id:
    lfs: $lfs
    depth: $depth
EOF
        else
            # Append to existing repos
            cat >> "$manifest_file" <<EOF
  $repo_id:
    lfs: $lfs
    depth: $depth
EOF
        fi
    fi
}

# Add repo with upstream to manifest.yaml
# Usage: add_repo_with_upstream <repo_id> <upstream_id> [lfs] [depth]
add_repo_with_upstream() {
    local repo_id="$1"
    local upstream_id="$2"
    local lfs="${3:-minimal}"
    local depth="${4:-100}"

    local manifest_file=".wald/manifest.yaml"

    if command -v yq >/dev/null 2>&1; then
        yq -i ".repos[\"$repo_id\"].upstream = \"$upstream_id\"" "$manifest_file"
        yq -i ".repos[\"$repo_id\"].lfs = \"$lfs\"" "$manifest_file"
        yq -i ".repos[\"$repo_id\"].depth = $depth" "$manifest_file"
    else
        # Fallback: manual append
        cat >> "$manifest_file" <<EOF
  $repo_id:
    upstream: $upstream_id
    lfs: $lfs
    depth: $depth
EOF
    fi
}

# Add repo with aliases to manifest.yaml
# Usage: add_repo_with_aliases <repo_id> <alias1> [alias2] [alias3]
add_repo_with_aliases() {
    local repo_id="$1"
    shift
    local aliases=("$@")

    local manifest_file=".wald/manifest.yaml"

    if command -v yq >/dev/null 2>&1; then
        yq -i ".repos[\"$repo_id\"].lfs = \"minimal\"" "$manifest_file"
        yq -i ".repos[\"$repo_id\"].depth = 100" "$manifest_file"
        for alias in "${aliases[@]}"; do
            yq -i ".repos[\"$repo_id\"].aliases += [\"$alias\"]" "$manifest_file"
        done
    else
        # Fallback: manual append
        local aliases_yaml=""
        for alias in "${aliases[@]}"; do
            aliases_yaml="$aliases_yaml, $alias"
        done
        aliases_yaml="${aliases_yaml#, }"  # Remove leading comma

        cat >> "$manifest_file" <<EOF
  $repo_id:
    lfs: minimal
    depth: 100
    aliases: [$aliases_yaml]
EOF
    fi
}

# ====================================================================================
# Bare Repository Creation
# ====================================================================================

# Create bare repo with realistic commit history
# Usage: create_bare_repo <repo_id> [with_commits]
#
# Creates bare repo in .wald/repos/<host>/<owner>/<name>.git
# If second arg is "with_commits", creates branches and commits
create_bare_repo() {
    local repo_id="$1"
    local with_commits="${2:-}"

    # Parse repo_id into host/owner/name
    local host owner name
    if [[ "$repo_id" =~ ^([^/]+)/([^/]+)/([^/]+)$ ]]; then
        host="${BASH_REMATCH[1]}"
        owner="${BASH_REMATCH[2]}"
        name="${BASH_REMATCH[3]}"
    else
        echo "Invalid repo_id format: $repo_id" >&2
        return 1
    fi

    # Use setup.sh function
    create_bare_repo_in_workspace "$host" "$owner" "$name" "$with_commits"
}

# ====================================================================================
# Baum Creation and Management
# ====================================================================================

# Plant a baum (create .baum/ and worktrees)
# Usage: plant_baum <repo_id> <container_path> <branch1> [branch2] [branch3]
#
# Example: plant_baum "github.com/test/repo" "tools/repo" "main" "dev"
plant_baum() {
    local repo_id="$1"
    local container_path="$2"
    shift 2
    local branches=("$@")

    # Create container directory
    mkdir -p "$container_path"

    # Create .baum/ directory
    mkdir -p "$container_path/.baum"

    # Parse repo_id for bare repo path
    local host owner name
    if [[ "$repo_id" =~ ^([^/]+)/([^/]+)/([^/]+)$ ]]; then
        host="${BASH_REMATCH[1]}"
        owner="${BASH_REMATCH[2]}"
        name="${BASH_REMATCH[3]}"
    else
        echo "Invalid repo_id format: $repo_id" >&2
        return 1
    fi

    local bare_repo=".wald/repos/$host/$owner/$name.git"

    if [[ ! -d "$bare_repo" ]]; then
        echo "Bare repo does not exist: $bare_repo" >&2
        return 1
    fi

    # Create baum manifest
    cat > "$container_path/.baum/manifest.yaml" <<EOF
repo_id: $repo_id
worktrees:
EOF

    # Create worktrees for each branch
    for branch in "${branches[@]}"; do
        local worktree_dir="$container_path/_${branch}.wt"

        # Create worktree
        git -C "$bare_repo" worktree add "$PWD/$worktree_dir" "$branch" 2>/dev/null || {
            # Branch might not exist, create it
            git -C "$bare_repo" worktree add -b "$branch" "$PWD/$worktree_dir" 2>/dev/null || {
                echo "Failed to create worktree for branch $branch" >&2
                return 1
            }
        }

        # Add to baum manifest
        cat >> "$container_path/.baum/manifest.yaml" <<EOF
  - branch: $branch
    path: _${branch}.wt
EOF
    done

    return 0
}

# ====================================================================================
# Worktree Validation
# ====================================================================================

# Assert worktree exists and is properly configured
# Usage: assert_worktree_exists <path>
assert_worktree_exists() {
    local path="$1"
    local msg="${2:-worktree should exist}"

    # Check directory exists
    if [[ ! -d "$path" ]]; then
        _fail "$msg: directory does not exist: $path"
        return 1
    fi

    # Check .git file exists (worktrees have .git file, not directory)
    if [[ ! -f "$path/.git" ]]; then
        _fail "$msg: .git file missing (not a worktree): $path"
        return 1
    fi

    # Verify .git file contains gitdir reference
    if ! grep -q "gitdir:" "$path/.git" 2>/dev/null; then
        _fail "$msg: .git file invalid (missing gitdir): $path"
        return 1
    fi

    return 0
}

# Assert baum manifest contains worktree entry
# Usage: assert_baum_has_worktree <baum_path> <branch>
assert_baum_has_worktree() {
    local baum_path="$1"
    local branch="$2"
    local msg="${3:-baum should have worktree for branch}"

    local manifest="$baum_path/.baum/manifest.yaml"

    if [[ ! -f "$manifest" ]]; then
        _fail "$msg: baum manifest does not exist: $manifest"
        return 1
    fi

    if ! grep -q "branch: $branch" "$manifest"; then
        _fail "$msg: branch $branch not in baum manifest"
        return 1
    fi

    return 0
}

# ====================================================================================
# Move Detection
# ====================================================================================

# Detect moved baums between two commits using git diff -M
# Usage: detect_moves <workspace_path> <from_commit> <to_commit>
#
# Returns: List of "old_path -> new_path" for .baum/manifest.yaml files
detect_moves() {
    local ws_path="$1"
    local from_commit="$2"
    local to_commit="$3"
    local original_dir="$PWD"

    cd "$ws_path" || return 1

    # Use git diff with rename detection
    # --first-parent: follow only first parent (ignore merge commits)
    # -M: detect renames
    # --name-status: show status (R for rename) and paths
    # --diff-filter=R: only show renames
    local moves
    moves=$(git diff -M --name-status --first-parent --diff-filter=R "$from_commit..$to_commit" | \
            grep ".baum/manifest.yaml" || true)

    cd "$original_dir" || return 1

    echo "$moves"
}

# ====================================================================================
# State File Manipulation
# ====================================================================================

# Update last_sync in state.yaml
# Usage: update_last_sync <commit_hash>
update_last_sync() {
    local commit="$1"
    local state_file=".wald/state.yaml"

    if command -v yq >/dev/null 2>&1; then
        yq -i ".last_sync = \"$commit\"" "$state_file"
    else
        # Fallback: sed replacement
        if [[ -f "$state_file" ]]; then
            sed -i.bak "s/last_sync: .*/last_sync: $commit/" "$state_file"
            rm -f "$state_file.bak"
        fi
    fi
}

# Get last_sync from state.yaml
# Usage: get_last_sync
get_last_sync() {
    local state_file=".wald/state.yaml"

    if command -v yq >/dev/null 2>&1; then
        yq -r ".last_sync" "$state_file"
    else
        # Fallback: grep
        grep "^last_sync:" "$state_file" | cut -d' ' -f2
    fi
}

# ====================================================================================
# Repository ID Parsing
# ====================================================================================

# Parse repo_id into components
# Usage: parse_repo_id <repo_id>
# Returns: Sets REPO_HOST, REPO_OWNER, REPO_NAME
parse_repo_id() {
    local repo_id="$1"

    if [[ "$repo_id" =~ ^([^/]+)/([^/]+)/([^/]+)$ ]]; then
        REPO_HOST="${BASH_REMATCH[1]}"
        REPO_OWNER="${BASH_REMATCH[2]}"
        REPO_NAME="${BASH_REMATCH[3]}"
        return 0
    else
        echo "Invalid repo_id format: $repo_id" >&2
        return 1
    fi
}

# Get bare repo path from repo_id
# Usage: get_bare_repo_path <repo_id>
get_bare_repo_path() {
    local repo_id="$1"

    if parse_repo_id "$repo_id"; then
        echo ".wald/repos/$REPO_HOST/$REPO_OWNER/$REPO_NAME.git"
    else
        return 1
    fi
}

# ====================================================================================
# Test Fixtures Loading
# ====================================================================================

# Load a git bundle into remotes directory (for multi-machine tests)
# Usage: load_test_bundle <bundle_file> <repo_name>
#
# Creates remotes/<repo_name>.git from bundle
load_test_bundle() {
    local bundle_file="$1"
    local repo_name="$2"

    if [[ -z "$TEST_REMOTES" ]]; then
        echo "TEST_REMOTES not set - must call setup_multi_machine first" >&2
        return 1
    fi

    if [[ ! -f "$bundle_file" ]]; then
        echo "Bundle file not found: $bundle_file" >&2
        return 1
    fi

    # Clone bundle to bare repo in remotes/
    git clone --bare --quiet "$bundle_file" "$TEST_REMOTES/$repo_name.git"
}

# ====================================================================================
# Debug Helpers
# ====================================================================================

# Print workspace structure (for debugging failed tests)
# Usage: debug_workspace [path]
debug_workspace() {
    local path="${1:-.}"

    echo "=== Workspace Structure ==="
    tree -L 3 -a "$path" 2>/dev/null || find "$path" -maxdepth 3 -print

    echo ""
    echo "=== .wald/manifest.yaml ==="
    cat "$path/.wald/manifest.yaml" 2>/dev/null || echo "Not found"

    echo ""
    echo "=== .wald/state.yaml ==="
    cat "$path/.wald/state.yaml" 2>/dev/null || echo "Not found"

    echo ""
    echo "=== Git Status ==="
    git -C "$path" status --short 2>/dev/null || echo "Not a git repo"
}

# Print multi-machine test state (for debugging)
debug_multi_machine() {
    echo "=== Multi-Machine Test State ==="
    echo "TEST_ALPHA=$TEST_ALPHA"
    echo "TEST_BETA=$TEST_BETA"
    echo "TEST_REMOTES=$TEST_REMOTES"

    if [[ -d "$TEST_ALPHA" ]]; then
        echo ""
        echo "=== Alpha Workspace ==="
        debug_workspace "$TEST_ALPHA"
    fi

    if [[ -d "$TEST_BETA" ]]; then
        echo ""
        echo "=== Beta Workspace ==="
        debug_workspace "$TEST_BETA"
    fi
}
