#!/usr/bin/env bash
# Tests for 'wald uproot' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic uproot tests
# ====================================================================================

begin_test "wald uproot removes baum container completely"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Verify baum exists
    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"

    # Uproot
    $WALD_BIN uproot "tools/repo"

    # Verify baum container removed
    assert_dir_not_exists "tools/repo"

    teardown_wald_workspace
end_test

begin_test "wald uproot deregisters worktrees from bare repo"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Verify worktrees registered
    assert_bare_worktree_count "github.com/test/repo" 2

    # Uproot
    $WALD_BIN uproot "tools/repo"

    # Verify worktrees deregistered (bare repo only tracks the bare repo itself)
    assert_bare_worktree_count "github.com/test/repo" 0

    teardown_wald_workspace
end_test

begin_test "wald uproot preserves bare repo"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    local bare_path
    bare_path=$(get_bare_repo_path "github.com/test/repo")

    # Verify bare repo exists
    assert_dir_exists "$bare_path"

    # Uproot
    $WALD_BIN uproot "tools/repo"

    # Verify bare repo still exists
    assert_dir_exists "$bare_path"

    teardown_wald_workspace
end_test

begin_test "wald uproot with multiple worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Verify multiple worktrees
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"

    # Uproot
    $WALD_BIN uproot "tools/repo"

    # Verify all gone
    assert_dir_not_exists "tools/repo"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald uproot fails if path doesn't exist"
    setup_wald_workspace

    _result=$($WALD_BIN uproot "nonexistent/path" 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

begin_test "wald uproot fails if path is not a baum"
    setup_wald_workspace

    # Create regular directory (not a baum)
    mkdir -p "tools/not-a-baum"

    _result=$($WALD_BIN uproot "tools/not-a-baum" 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

begin_test "wald uproot fails with uncommitted changes without --force"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Create uncommitted changes
    create_uncommitted_changes "tools/repo/_main.wt"

    _result=$($WALD_BIN uproot "tools/repo" 2>&1 || true)
    # Should fail - git worktree remove fails on unclean worktrees
    assert_contains "$_result" "failed to remove"

    teardown_wald_workspace
end_test

begin_test "wald uproot --force removes despite uncommitted changes"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Create uncommitted changes
    create_uncommitted_changes "tools/repo/_main.wt"

    # Force uproot
    $WALD_BIN uproot --force "tools/repo"

    # Verify removed
    assert_dir_not_exists "tools/repo"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
