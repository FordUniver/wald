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
# Creates bare repo in .wald/repos/<path>.git
# Supports subgroups: github.com/user/repo or git.zib.de/iol/research/project
# If second arg is "with_commits", creates branches and commits
create_bare_repo() {
    local repo_id="$1"
    local with_commits="${2:-}"

    # Get bare repo path using helper
    local bare_path
    bare_path=$(get_bare_repo_path "$repo_id") || return 1

    # Extract just the repo name for commit messages
    parse_repo_id "$repo_id" || return 1
    local name="$REPO_NAME"

    # Create directory structure
    mkdir -p "$(dirname "$bare_path")"

    if [[ "$with_commits" == "with_commits" ]]; then
        # Create temporary working directory to build history
        local temp_repo
        temp_repo=$(mktemp -d /tmp/wald-bare.XXXXXX)
        cd "$temp_repo" || return 1

        git init --quiet --initial-branch=main
        git config user.name "Wald Test"
        git config user.email "test@wald.local"

        # Create realistic commit history
        echo "# $name" > README.md
        git add README.md
        git commit --quiet -m "Initial commit"

        echo "Project description" >> README.md
        git add README.md
        git commit --quiet -m "Add description"

        # Create dev branch
        git checkout -b dev --quiet
        echo "Feature in progress" > feature.txt
        git add feature.txt
        git commit --quiet -m "Start feature development"

        # Return to main
        git checkout main --quiet

        # Clone to bare repo in workspace
        cd - >/dev/null || return 1
        git clone --bare --quiet "$temp_repo" "$bare_path"

        # Clean up temp repo
        rm -rf "$temp_repo"
    else
        # Create empty bare repo
        git init --bare --quiet "$bare_path"
    fi

    return 0
}

# ====================================================================================
# Baum Creation and Management
# ====================================================================================

# Plant a baum (create .baum/ and worktrees)
# Usage: plant_baum <repo_id> <container_path> <branch1> [branch2] [branch3]
#
# Example: plant_baum "github.com/test/repo" "tools/repo" "main" "dev"
# Supports subgroups: plant_baum "git.zib.de/iol/research/project" "research/project" "main"
plant_baum() {
    local repo_id="$1"
    local container_path="$2"
    shift 2
    local branches=("$@")

    # Create container directory
    mkdir -p "$container_path"

    # Create .baum/ directory
    mkdir -p "$container_path/.baum"

    # Get bare repo path using helper
    local bare_repo
    bare_repo=$(get_bare_repo_path "$repo_id") || return 1

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

# Materialize a baum received via sync
# Creates worktrees for entries in baum manifest that don't exist on disk
# Usage: materialize_baum <baum_path>
#
# This handles the case where sync pulls a .baum directory but worktrees don't exist.
# Reads repo_id from baum manifest, creates worktrees from bare repo.
materialize_baum() {
    local baum_path="$1"
    local manifest="$baum_path/.baum/manifest.yaml"

    if [[ ! -f "$manifest" ]]; then
        echo "Baum manifest not found: $manifest" >&2
        return 1
    fi

    # Extract repo_id from manifest
    local repo_id
    repo_id=$(grep "^repo_id:" "$manifest" | cut -d' ' -f2)
    if [[ -z "$repo_id" ]]; then
        echo "Could not extract repo_id from manifest" >&2
        return 1
    fi

    # Get bare repo path
    local bare_repo
    bare_repo=$(get_bare_repo_path "$repo_id") || return 1

    if [[ ! -d "$bare_repo" ]]; then
        echo "Bare repo not found: $bare_repo" >&2
        return 1
    fi

    # Parse worktrees from manifest and create missing ones
    # Format: "  - branch: <name>\n    path: <path>"
    local branch=""
    local wt_path=""
    while IFS= read -r line; do
        if [[ "$line" =~ ^[[:space:]]*-[[:space:]]*branch:[[:space:]]*(.*) ]]; then
            branch="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^[[:space:]]*path:[[:space:]]*(.*) ]]; then
            wt_path="${BASH_REMATCH[1]}"
            # Got both branch and path, create worktree if missing
            if [[ -n "$branch" && -n "$wt_path" ]]; then
                local full_path="$baum_path/$wt_path"
                if [[ ! -d "$full_path" ]]; then
                    git -C "$bare_repo" worktree add "$PWD/$full_path" "$branch" 2>/dev/null || \
                    git -C "$bare_repo" worktree add -b "$branch" "$PWD/$full_path" 2>/dev/null || true
                fi
            fi
            branch=""
            wt_path=""
        fi
    done < "$manifest"

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

# Assert bare repo has worktree registered
# Usage: assert_bare_has_worktree <repo_id> <worktree_path>
#
# Checks that the bare repo's worktree registry contains an entry for the given path.
# The worktree registry is stored in <bare-repo>/worktrees/<name>/gitdir
assert_bare_has_worktree() {
    local repo_id="$1"
    local wt_path="$2"
    local msg="${3:-bare repo should have worktree registered}"

    local bare_repo
    bare_repo=$(get_bare_repo_path "$repo_id") || {
        _fail "$msg: could not get bare repo path for $repo_id"
        return 1
    }

    if [[ ! -d "$bare_repo" ]]; then
        _fail "$msg: bare repo does not exist: $bare_repo"
        return 1
    fi

    local worktrees_dir="$bare_repo/worktrees"
    if [[ ! -d "$worktrees_dir" ]]; then
        _fail "$msg: no worktrees registered in $bare_repo"
        return 1
    fi

    # Check if any worktree registry entry points to the given path
    local found=0
    local abs_wt_path
    abs_wt_path=$(cd "$(dirname "$wt_path")" 2>/dev/null && pwd)/$(basename "$wt_path") || abs_wt_path="$PWD/$wt_path"

    for wt_dir in "$worktrees_dir"/*; do
        if [[ -f "$wt_dir/gitdir" ]]; then
            local gitdir_content
            gitdir_content=$(cat "$wt_dir/gitdir")
            # gitdir contains path to worktree's .git file
            local registered_path
            registered_path=$(dirname "$gitdir_content")
            if [[ "$registered_path" == "$abs_wt_path" ]] || [[ "$gitdir_content" == *"$wt_path"* ]]; then
                found=1
                break
            fi
        fi
    done

    if [[ $found -eq 0 ]]; then
        _fail "$msg: worktree $wt_path not found in bare repo registry"
        return 1
    fi

    return 0
}

# Assert bare repo worktree count matches expected
# Usage: assert_bare_worktree_count <repo_id> <expected_count>
assert_bare_worktree_count() {
    local repo_id="$1"
    local expected="$2"
    local msg="${3:-bare repo worktree count should match}"

    local bare_repo
    bare_repo=$(get_bare_repo_path "$repo_id") || {
        _fail "$msg: could not get bare repo path for $repo_id"
        return 1
    }

    local worktrees_dir="$bare_repo/worktrees"
    local actual=0

    if [[ -d "$worktrees_dir" ]]; then
        # Count non-pruned worktree entries
        for wt_dir in "$worktrees_dir"/*; do
            if [[ -d "$wt_dir" && -f "$wt_dir/gitdir" ]]; then
                actual=$((actual + 1))
            fi
        done
    fi

    if [[ "$actual" -ne "$expected" ]]; then
        _fail "$msg: expected $expected worktrees, found $actual in $bare_repo"
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
# Returns: Sets REPO_HOST, REPO_PATH (array), REPO_NAME
#
# Supports arbitrary path depth for GitLab subgroups:
#   github.com/user/repo -> HOST=github.com, PATH=(user repo), NAME=repo
#   git.zib.de/iol/research/project -> HOST=git.zib.de, PATH=(iol research project), NAME=project
parse_repo_id() {
    local repo_id="$1"

    # Must have at least host/something
    if [[ ! "$repo_id" =~ / ]]; then
        echo "Invalid repo_id format: $repo_id" >&2
        return 1
    fi

    # Split on /
    IFS='/' read -ra parts <<< "$repo_id"

    if [[ ${#parts[@]} -lt 2 ]]; then
        echo "Invalid repo_id format: $repo_id" >&2
        return 1
    fi

    REPO_HOST="${parts[0]}"
    REPO_PATH=("${parts[@]:1}")
    REPO_NAME="${parts[${#parts[@]}-1]}"

    # Verify no empty segments
    for part in "${parts[@]}"; do
        if [[ -z "$part" ]]; then
            echo "Empty segment in repo_id: $repo_id" >&2
            return 1
        fi
    done

    return 0
}

# Get bare repo path from repo_id
# Usage: get_bare_repo_path <repo_id>
#
# Returns path like: .wald/repos/git.zib.de/iol/research/project.git
get_bare_repo_path() {
    local repo_id="$1"

    if parse_repo_id "$repo_id"; then
        # Join path segments and append .git to last one
        local path=".wald/repos/$REPO_HOST"
        for segment in "${REPO_PATH[@]}"; do
            path="$path/$segment"
        done
        # Replace last segment with .git suffix
        echo "${path}.git"
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

# ====================================================================================
# Additional Worktree Assertions
# ====================================================================================

# Assert worktree does NOT exist
# Usage: assert_worktree_not_exists <path>
assert_worktree_not_exists() {
    local path="$1"
    local msg="${2:-worktree should not exist}"

    # Check directory does not exist
    if [[ -d "$path" ]]; then
        _fail "$msg: directory still exists: $path"
        return 1
    fi

    return 0
}

# Assert baum has exact worktree count
# Usage: assert_baum_worktree_count <baum_path> <expected_count>
assert_baum_worktree_count() {
    local baum_path="$1"
    local expected="$2"
    local msg="${3:-baum should have expected worktree count}"

    local manifest="$baum_path/.baum/manifest.yaml"

    if [[ ! -f "$manifest" ]]; then
        _fail "$msg: baum manifest does not exist: $manifest"
        return 1
    fi

    # Count worktree entries (lines starting with "  - branch:")
    local actual
    actual=$(grep -c "^[[:space:]]*-[[:space:]]*branch:" "$manifest" 2>/dev/null || echo "0")

    if [[ "$actual" -ne "$expected" ]]; then
        _fail "$msg: expected $expected worktrees, found $actual"
        return 1
    fi

    return 0
}

# Assert baum does NOT have worktree for branch
# Usage: assert_baum_not_has_worktree <baum_path> <branch>
assert_baum_not_has_worktree() {
    local baum_path="$1"
    local branch="$2"
    local msg="${3:-baum should not have worktree for branch}"

    local manifest="$baum_path/.baum/manifest.yaml"

    if [[ ! -f "$manifest" ]]; then
        _fail "$msg: baum manifest does not exist: $manifest"
        return 1
    fi

    if grep -q "branch: $branch" "$manifest"; then
        _fail "$msg: branch $branch found in baum manifest"
        return 1
    fi

    return 0
}

# ====================================================================================
# Test Setup Helpers
# ====================================================================================

# Create uncommitted changes in a worktree
# Usage: create_uncommitted_changes <worktree_path>
create_uncommitted_changes() {
    local wt_path="$1"

    if [[ ! -d "$wt_path" ]]; then
        echo "Worktree does not exist: $wt_path" >&2
        return 1
    fi

    # Create a new file with uncommitted changes
    echo "Uncommitted change $(date +%s)" > "$wt_path/uncommitted.txt"
    git -C "$wt_path" add uncommitted.txt

    return 0
}

# Create uncommitted changes (untracked file) in a worktree
# Usage: create_untracked_file <worktree_path>
create_untracked_file() {
    local wt_path="$1"

    if [[ ! -d "$wt_path" ]]; then
        echo "Worktree does not exist: $wt_path" >&2
        return 1
    fi

    # Create untracked file
    echo "Untracked file $(date +%s)" > "$wt_path/untracked.txt"

    return 0
}

# Assert .gitignore contains entry
# Usage: assert_gitignore_contains <container_path> <entry>
assert_gitignore_contains() {
    local container="$1"
    local entry="$2"
    local msg="${3:-.gitignore should contain entry}"

    local gitignore="$container/.gitignore"

    if [[ ! -f "$gitignore" ]]; then
        _fail "$msg: .gitignore does not exist: $gitignore"
        return 1
    fi

    if ! grep -qF "$entry" "$gitignore"; then
        _fail "$msg: '$entry' not found in .gitignore"
        return 1
    fi

    return 0
}

# Assert .gitignore does NOT contain entry
# Usage: assert_gitignore_not_contains <container_path> <entry>
assert_gitignore_not_contains() {
    local container="$1"
    local entry="$2"
    local msg="${3:-.gitignore should not contain entry}"

    local gitignore="$container/.gitignore"

    if [[ ! -f "$gitignore" ]]; then
        # No .gitignore means entry definitely not present
        return 0
    fi

    if grep -qF "$entry" "$gitignore"; then
        _fail "$msg: '$entry' found in .gitignore"
        return 1
    fi

    return 0
}
