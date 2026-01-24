#!/usr/bin/env bash
# Tests for 'wald repo gc' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic gc tests
# ====================================================================================

begin_test "wald repo gc succeeds on single repo"
    setup_wald_workspace

    # Create bare repo with commits
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    # GC should succeed
    _result=$($WALD_BIN repo gc github.com/test/repo 2>&1)
    assert_contains "$_result" "Cleaning"
    assert_contains "$_result" "github.com/test/repo"
    assert_contains "$_result" "Garbage collection complete"

    teardown_wald_workspace
end_test

begin_test "wald repo gc all repos"
    setup_wald_workspace

    # Create multiple bare repos
    create_bare_repo "github.com/test/repo1" with_commits
    create_bare_repo "github.com/test/repo2" with_commits
    add_repo_to_manifest "github.com/test/repo1"
    add_repo_to_manifest "github.com/test/repo2"

    # GC all should succeed
    _result=$($WALD_BIN repo gc 2>&1)
    assert_contains "$_result" "repo1"
    assert_contains "$_result" "repo2"
    assert_contains "$_result" "Garbage collection complete"

    teardown_wald_workspace
end_test

begin_test "wald repo gc --aggressive runs thorough cleanup"
    setup_wald_workspace

    # Create bare repo with commits
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    # Aggressive GC should succeed
    _result=$($WALD_BIN repo gc --aggressive github.com/test/repo 2>&1)
    assert_contains "$_result" "Cleaning"
    assert_contains "$_result" "Garbage collection complete"

    teardown_wald_workspace
end_test

begin_test "wald repo gc reports no repos when none cloned"
    setup_wald_workspace

    # Add repo to manifest but don't create bare repo
    add_repo_to_manifest "github.com/test/repo"

    # GC should report no repos
    _result=$($WALD_BIN repo gc 2>&1)
    assert_contains "$_result" "No repositories"

    teardown_wald_workspace
end_test

begin_test "wald repo gc fails on missing repo"
    setup_wald_workspace

    # Try to GC non-existent repo
    _result=$($WALD_BIN repo gc github.com/nonexistent/repo 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

begin_test "wald repo gc works with repo alias"
    setup_wald_workspace

    # Create bare repo
    create_bare_repo "github.com/user/dotfiles" with_commits

    # Add repo with alias
    $WALD_BIN repo add --no-clone --alias=dots github.com/user/dotfiles

    # GC via alias should succeed
    _result=$($WALD_BIN repo gc dots 2>&1)
    assert_contains "$_result" "Cleaning"
    assert_contains "$_result" "Garbage collection complete"

    teardown_wald_workspace
end_test

begin_test "wald repo gc on repo with worktrees"
    setup_wald_workspace

    # Create bare repo with commits
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    # Plant baum to create worktrees
    $WALD_BIN plant github.com/test/repo tools/repo main

    # GC should still succeed (worktrees are referenced)
    _result=$($WALD_BIN repo gc github.com/test/repo 2>&1)
    assert_contains "$_result" "Cleaning"
    assert_contains "$_result" "Garbage collection complete"

    # Verify worktree still works
    assert_worktree_exists "tools/repo/_main.wt"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
