#!/usr/bin/env bash
# Tests for 'wald move' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic move tests
# ====================================================================================

begin_test "wald move relocates baum to new path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Move using CLI
    $WALD_BIN move tools/repo admin/repo

    # Verify move
    assert_dir_not_exists "tools/repo"
    assert_dir_exists "admin/repo/.baum"
    assert_worktree_exists "admin/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald move preserves worktree functionality"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Move baum using CLI
    $WALD_BIN move tools/repo research/project/repo

    # Verify both worktrees still exist and are valid
    assert_worktree_exists "research/project/repo/_main.wt"
    assert_worktree_exists "research/project/repo/_dev.wt"
    assert_baum_has_worktree "research/project/repo" "main"
    assert_baum_has_worktree "research/project/repo" "dev"

    # Verify bare repo registry updated
    assert_bare_worktree_count "github.com/test/repo" 2

    teardown_wald_workspace
end_test

begin_test "wald move updates baum manifest paths"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Move baum using CLI
    $WALD_BIN move tools/repo infrastructure/repo

    # Verify baum manifest still valid
    assert_file_exists "infrastructure/repo/.baum/manifest.yaml"
    assert_file_contains "infrastructure/repo/.baum/manifest.yaml" "github.com/test/repo"

    teardown_wald_workspace
end_test

begin_test "wald move preserves uncommitted changes in worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Make uncommitted changes in worktree
    echo "uncommitted work" > "tools/repo/_main.wt/work.txt"

    # Move baum using CLI
    $WALD_BIN move tools/repo admin/repo

    # Verify uncommitted changes still present
    assert_file_exists "admin/repo/_main.wt/work.txt"
    assert_file_contains "admin/repo/_main.wt/work.txt" "uncommitted work"

    teardown_wald_workspace
end_test

# ====================================================================================
# Nested path moves
# ====================================================================================

begin_test "wald move handles deeply nested paths"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "research/25-project/nested/repo" main

    # Move using CLI
    $WALD_BIN move research/25-project/nested/repo admin/archived/old-projects/repo

    assert_dir_not_exists "research/25-project/nested/repo"
    assert_dir_exists "admin/archived/old-projects/repo/.baum"
    assert_worktree_exists "admin/archived/old-projects/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald move from nested to root level"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "research/25-project/repo" main

    # Move using CLI
    $WALD_BIN move research/25-project/repo repo

    assert_dir_not_exists "research/25-project/repo"
    assert_dir_exists "repo/.baum"
    assert_worktree_exists "repo/_main.wt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald move fails if source doesn't exist"
    setup_wald_workspace

    _result=$($WALD_BIN move nonexistent/path new/path 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

begin_test "wald move fails if source is not a baum"
    setup_wald_workspace

    # Create regular directory (not a baum)
    mkdir -p tools/regular-dir
    echo "not a baum" > tools/regular-dir/file.txt

    _result=$($WALD_BIN move tools/regular-dir admin/regular-dir 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

begin_test "wald move fails if destination already exists"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Create destination directory
    mkdir -p admin/repo
    echo "existing content" > admin/repo/file.txt

    _result=$($WALD_BIN move tools/repo admin/repo 2>&1 || true)
    assert_contains "$_result" "already exists"

    # Verify source still exists
    assert_dir_exists "tools/repo/.baum"
    # Verify destination wasn't overwritten
    assert_file_exists "admin/repo/file.txt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Git integration
# ====================================================================================

begin_test "wald move stages changes in git"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Commit initial state
    git add -A
    git commit -m "Plant repo"

    # Move using CLI (stages changes)
    $WALD_BIN move tools/repo admin/repo

    # Verify git sees the move
    _status=$(git status --short)
    assert_contains "$_status" "admin/repo"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
