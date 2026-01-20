#!/usr/bin/env bash
# Tests for 'wald branch' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic branch tests
# ====================================================================================

begin_test "wald branch adds worktree to existing baum"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Verify initial state
    assert_worktree_exists "tools/repo/_main.wt"
    assert_baum_worktree_count "tools/repo" 1

    # Add branch
    $WALD_BIN branch "tools/repo" feature

    # Verify new worktree
    assert_worktree_exists "tools/repo/_feature.wt"
    assert_baum_worktree_count "tools/repo" 2
    assert_baum_has_worktree "tools/repo" "feature"

    teardown_wald_workspace
end_test

begin_test "wald branch creates worktree for existing branch"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Add worktree for existing 'dev' branch (created by create_bare_repo with_commits)
    $WALD_BIN branch "tools/repo" dev

    # Verify worktree
    assert_worktree_exists "tools/repo/_dev.wt"
    assert_baum_has_worktree "tools/repo" "dev"

    teardown_wald_workspace
end_test

begin_test "wald branch updates baum manifest correctly"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Initial state
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_not_has_worktree "tools/repo" "feature"

    # Add branch
    $WALD_BIN branch "tools/repo" feature

    # Verify manifest updated
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "feature"

    teardown_wald_workspace
end_test

begin_test "wald branch updates .gitignore correctly"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Verify initial .gitignore
    assert_gitignore_contains "tools/repo" "_main.wt"

    # Add branch
    $WALD_BIN branch "tools/repo" feature

    # Verify .gitignore updated
    assert_gitignore_contains "tools/repo" "_main.wt"
    assert_gitignore_contains "tools/repo" "_feature.wt"

    teardown_wald_workspace
end_test

begin_test "wald branch with nested baum path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "research/25-project/repo" main

    # Add branch to nested baum
    $WALD_BIN branch "research/25-project/repo" feature

    # Verify worktree
    assert_worktree_exists "research/25-project/repo/_feature.wt"
    assert_baum_has_worktree "research/25-project/repo" "feature"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald branch fails if branch already has worktree"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Try to add worktree for branch that already has one
    _result=$($WALD_BIN branch "tools/repo" main 2>&1 || true)
    assert_contains "$_result" "already exists"

    teardown_wald_workspace
end_test

begin_test "wald branch fails if baum doesn't exist"
    setup_wald_workspace

    _result=$($WALD_BIN branch "nonexistent/path" feature 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

begin_test "wald branch fails if bare repo missing"
    setup_wald_workspace

    # Create baum manually without bare repo
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Remove bare repo
    rm -rf ".wald/repos/github.com/test/repo.git"

    _result=$($WALD_BIN branch "tools/repo" feature 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
