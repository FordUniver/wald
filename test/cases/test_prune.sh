#!/usr/bin/env bash
# Tests for 'wald prune' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic prune tests
# ====================================================================================

begin_test "wald prune removes single worktree"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Verify both worktrees exist
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"
    assert_baum_worktree_count "tools/repo" 2

    # Prune dev
    $WALD_BIN prune "tools/repo" dev

    # Verify dev removed, main remains
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_not_exists "tools/repo/_dev.wt"
    assert_baum_worktree_count "tools/repo" 1

    teardown_wald_workspace
end_test

begin_test "wald prune removes multiple worktrees in one call"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Add a third worktree
    $WALD_BIN branch "tools/repo" feature

    assert_baum_worktree_count "tools/repo" 3

    # Prune dev and feature together
    $WALD_BIN prune "tools/repo" dev feature

    # Only main should remain
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_not_exists "tools/repo/_dev.wt"
    assert_worktree_not_exists "tools/repo/_feature.wt"
    assert_baum_worktree_count "tools/repo" 1

    teardown_wald_workspace
end_test

begin_test "wald prune updates baum manifest correctly"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Verify both in manifest
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "dev"

    # Prune dev
    $WALD_BIN prune "tools/repo" dev

    # dev should be removed from manifest
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_not_has_worktree "tools/repo" "dev"

    teardown_wald_workspace
end_test

# ====================================================================================
# Warning cases
# ====================================================================================

begin_test "wald prune warns for non-existent branches, continues others"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Prune with mix of existing and non-existing
    _result=$($WALD_BIN prune "tools/repo" dev nonexistent 2>&1)

    # Should warn about nonexistent
    assert_contains "$_result" "No worktree found"

    # dev should still be removed
    assert_worktree_not_exists "tools/repo/_dev.wt"
    assert_baum_not_has_worktree "tools/repo" "dev"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald prune fails if baum doesn't exist"
    setup_wald_workspace

    _result=$($WALD_BIN prune "nonexistent/path" main 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

begin_test "wald prune fails with uncommitted changes without --force"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Create uncommitted changes in dev worktree
    create_uncommitted_changes "tools/repo/_dev.wt"

    _result=$($WALD_BIN prune "tools/repo" dev 2>&1 || true)
    # Should fail - git worktree remove fails on modified worktrees
    assert_contains "$_result" "failed to remove"

    teardown_wald_workspace
end_test

begin_test "wald prune --force removes despite uncommitted changes"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Create uncommitted changes in dev worktree
    create_uncommitted_changes "tools/repo/_dev.wt"

    # Force prune
    $WALD_BIN prune --force "tools/repo" dev

    # dev should be removed
    assert_worktree_not_exists "tools/repo/_dev.wt"

    teardown_wald_workspace
end_test

begin_test "wald prune fails if path is not a baum"
    setup_wald_workspace

    # Create regular directory (not a baum)
    mkdir -p "tools/not-a-baum"

    _result=$($WALD_BIN prune "tools/not-a-baum" main 2>&1 || true)
    assert_contains "$_result" "not a baum"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
