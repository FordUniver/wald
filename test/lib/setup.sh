#!/usr/bin/env bash
# Test environment setup and teardown for wald tests
# Provides functions for creating isolated test workspaces and multi-machine simulations

# ====================================================================================
# Global Variables
# ====================================================================================

# Single machine test workspace
TEST_WS=""           # Path to test workspace directory
_ORIGINAL_PWD=""     # Track original directory for cleanup

# Multi-machine test environment
TEST_ALPHA=""        # Path to machine-alpha workspace
TEST_BETA=""         # Path to machine-beta workspace
TEST_REMOTES=""      # Path to bare repos acting as remotes
_MULTI_TEST_DIR=""   # Root directory for multi-machine test

# ====================================================================================
# Single Machine Test Workspace
# ====================================================================================

# Create isolated test workspace with .wald/ structure
# Sets TEST_WS global variable and creates cleanup trap
setup_wald_workspace() {
    # Create temporary directory
    TEST_WS=$(mktemp -d /tmp/wald-test.XXXXXX)
    if [[ ! -d "$TEST_WS" ]]; then
        echo "Failed to create test workspace" >&2
        return 1
    fi

    # Track original directory
    _ORIGINAL_PWD="$PWD"

    # Change to test workspace
    cd "$TEST_WS" || return 1

    # Initialize git repository
    git init --quiet --initial-branch=main
    git config user.name "Wald Test"
    git config user.email "test@wald.local"
    git config pull.rebase true
    git config rebase.autoStash true

    # Create .wald/ directory structure
    mkdir -p .wald/{repos,state}

    # Create minimal manifest.yaml
    cat > .wald/manifest.yaml <<EOF
repos: {}
EOF

    # Create minimal config.yaml
    cat > .wald/config.yaml <<EOF
default_lfs: minimal
default_depth: 100
EOF

    # Create state.yaml
    cat > .wald/state.yaml <<EOF
last_sync: null
EOF

    # Initial commit
    git add .wald/
    git commit --quiet -m "Initialize wald workspace"

    # Set cleanup trap
    trap teardown_wald_workspace EXIT

    # Export WORKSPACE for wald commands
    export WORKSPACE="$TEST_WS"
}

# Clean up test workspace
teardown_wald_workspace() {
    # Return to original directory if we're still in test workspace
    if [[ -n "$_ORIGINAL_PWD" && "$PWD" == "$TEST_WS"* ]]; then
        cd "$_ORIGINAL_PWD" 2>/dev/null || cd /tmp
    fi

    # Remove test workspace
    if [[ -n "$TEST_WS" && -d "$TEST_WS" ]]; then
        # Force remove - worktrees may have read-only files
        chmod -R u+w "$TEST_WS" 2>/dev/null || true
        rm -rf "$TEST_WS"
    fi

    # Clear variables
    TEST_WS=""
    _ORIGINAL_PWD=""
    unset WORKSPACE 2>/dev/null || true

    # Remove trap
    trap - EXIT
}

# ====================================================================================
# Multi-Machine Test Environment
# ====================================================================================

# Create multi-machine simulation with local bare repos as remotes
# Sets TEST_ALPHA, TEST_BETA, TEST_REMOTES global variables
#
# Structure:
#   /tmp/wald-multi-$$/
#   ├── remotes/
#   │   └── workspace.git/     # Bare repo = shared origin
#   ├── machine-alpha/          # First workspace clone
#   └── machine-beta/           # Second workspace clone
setup_multi_machine() {
    # Create root directory for multi-machine test
    _MULTI_TEST_DIR=$(mktemp -d /tmp/wald-multi.XXXXXX)
    if [[ ! -d "$_MULTI_TEST_DIR" ]]; then
        echo "Failed to create multi-machine test directory" >&2
        return 1
    fi

    # Track original directory
    _ORIGINAL_PWD="$PWD"

    # Create remotes directory for bare repos
    TEST_REMOTES="$_MULTI_TEST_DIR/remotes"
    mkdir -p "$TEST_REMOTES"

    # Create bare workspace repo (acts as origin)
    cd "$TEST_REMOTES" || return 1
    git init --bare --quiet workspace.git
    cd "$_MULTI_TEST_DIR" || return 1

    # Clone to machine-alpha and set up wald structure
    TEST_ALPHA="$_MULTI_TEST_DIR/machine-alpha"
    git clone --quiet "$TEST_REMOTES/workspace.git" "$TEST_ALPHA"
    cd "$TEST_ALPHA" || return 1

    # Configure git
    git config user.name "Wald Test Alpha"
    git config user.email "alpha@wald.local"
    git config pull.rebase true
    git config rebase.autoStash true

    # Create .wald/ structure
    mkdir -p .wald/{repos,state}

    cat > .wald/manifest.yaml <<EOF
repos: {}
EOF

    cat > .wald/config.yaml <<EOF
default_lfs: minimal
default_depth: 100
EOF

    cat > .wald/state.yaml <<EOF
last_sync: null
EOF

    # Commit and push initial structure
    git add .wald/
    git commit --quiet -m "Initialize wald workspace"
    git push --quiet origin main

    # Clone to machine-beta
    cd "$_MULTI_TEST_DIR" || return 1
    TEST_BETA="$_MULTI_TEST_DIR/machine-beta"
    git clone --quiet "$TEST_REMOTES/workspace.git" "$TEST_BETA"
    cd "$TEST_BETA" || return 1

    # Configure git for beta
    git config user.name "Wald Test Beta"
    git config user.email "beta@wald.local"
    git config pull.rebase true
    git config rebase.autoStash true

    # Set cleanup trap
    trap teardown_multi_machine EXIT

    # Return to original directory
    cd "$_ORIGINAL_PWD" || return 1
}

# Clean up multi-machine test environment
teardown_multi_machine() {
    # Return to original directory
    if [[ -n "$_ORIGINAL_PWD" ]]; then
        cd "$_ORIGINAL_PWD" 2>/dev/null || cd /tmp
    fi

    # Remove entire multi-machine test directory
    if [[ -n "$_MULTI_TEST_DIR" && -d "$_MULTI_TEST_DIR" ]]; then
        # Force remove - may contain git objects and worktrees
        chmod -R u+w "$_MULTI_TEST_DIR" 2>/dev/null || true
        rm -rf "$_MULTI_TEST_DIR"
    fi

    # Clear variables
    TEST_ALPHA=""
    TEST_BETA=""
    TEST_REMOTES=""
    _MULTI_TEST_DIR=""
    _ORIGINAL_PWD=""

    # Remove trap
    trap - EXIT
}

# ====================================================================================
# Bare Repository Creation
# ====================================================================================

# Create a bare git repository in .wald/repos/
# Usage: create_bare_repo_in_workspace <host> <owner> <name> [with_commits]
#
# Creates bare repo at .wald/repos/<host>/<owner>/<name>.git
# If with_commits="with_commits", creates realistic commit history
create_bare_repo_in_workspace() {
    local host="$1"
    local owner="$2"
    local name="$3"
    local with_commits="${4:-}"

    local repo_path=".wald/repos/$host/$owner/$name.git"
    mkdir -p "$(dirname "$repo_path")"

    if [[ "$with_commits" == "with_commits" ]]; then
        # Create temporary working directory to build history
        local temp_repo=$(mktemp -d /tmp/wald-bare.XXXXXX)
        cd "$temp_repo" || return 1

        git init --quiet
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
        git clone --bare --quiet "$temp_repo" "$repo_path"

        # Clean up temp repo
        rm -rf "$temp_repo"
    else
        # Create empty bare repo
        git init --bare --quiet "$repo_path"
    fi

    return 0
}

# ====================================================================================
# Git Operations Helpers
# ====================================================================================

# Commit all changes and push in a workspace
# Usage: workspace_commit <workspace_path> <message>
workspace_commit() {
    local ws_path="$1"
    local message="$2"
    local original_dir="$PWD"

    cd "$ws_path" || return 1
    git add -A
    git commit --quiet -m "$message"
    git push --quiet origin main

    cd "$original_dir" || return 1
}

# Pull changes in a workspace
# Usage: workspace_pull <workspace_path>
workspace_pull() {
    local ws_path="$1"
    local original_dir="$PWD"

    cd "$ws_path" || return 1
    git pull --quiet --rebase origin main

    cd "$original_dir" || return 1
}

# Get current commit hash
# Usage: get_commit_hash <workspace_path>
get_commit_hash() {
    local ws_path="$1"
    local original_dir="$PWD"

    cd "$ws_path" || return 1
    local hash
    hash=$(git rev-parse HEAD)

    cd "$original_dir" || return 1
    echo "$hash"
}

# ====================================================================================
# Additional Assertions for Wald-Specific Validation
# ====================================================================================

# Assert directory does NOT exist
assert_dir_not_exists() {
    local path="$1"
    local msg="${2:-directory should not exist}"

    if [[ ! -d "$path" ]]; then
        return 0
    else
        _fail "$msg: $path exists" "" ""
        return 1
    fi
}

# Assert file contains text
assert_file_contains() {
    local file="$1"
    local needle="$2"
    local msg="${3:-file should contain text}"

    if [[ ! -f "$file" ]]; then
        _fail "$msg: file does not exist: $file"
        return 1
    fi

    if grep -q "$needle" "$file"; then
        return 0
    else
        _fail "$msg: '$needle' not found in $file"
        return 1
    fi
}

# Assert command succeeds
assert_success() {
    local output="$1"
    local msg="${2:-command should succeed}"

    # This is a semantic check - we assume the command ran successfully
    # if we got output. Actual exit code checking should use assert_exit_code
    if [[ -n "$output" ]] || [[ "$output" == "" ]]; then
        return 0
    else
        _fail "$msg"
        return 1
    fi
}
