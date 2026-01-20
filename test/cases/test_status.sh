#!/usr/bin/env bash
# Tests for 'wald status' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic status tests
# ====================================================================================

begin_test "wald status shows clean workspace"
    setup_wald_workspace

    _result=$($WALD_BIN status 2>&1)

    # Should indicate workspace is clean
    assert_contains "$_result" "clean"

    teardown_wald_workspace
end_test

begin_test "wald status shows baum and worktree counts"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"
    $WALD_BIN plant "github.com/test/repo1" "tools/repo1" main dev
    $WALD_BIN plant "github.com/test/repo2" "tools/repo2" main

    _result=$($WALD_BIN status 2>&1)

    # Should show counts
    assert_contains "$_result" "2"  # 2 baums
    assert_contains "$_result" "3"  # 3 worktrees total

    teardown_wald_workspace
end_test

begin_test "wald status shows registered repos count"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"

    _result=$($WALD_BIN status 2>&1)

    # Should show 2 registered repos
    assert_contains "$_result" "2"
    assert_contains "$_result" "registered"

    teardown_wald_workspace
end_test

begin_test "wald status --json produces valid JSON"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    _result=$($WALD_BIN status --json 2>&1)

    # Should be valid JSON
    assert_json_valid "$_result"
    # Should contain expected fields
    assert_contains "$_result" "workspace"
    assert_contains "$_result" "baums_count"
    assert_contains "$_result" "worktrees_count"

    teardown_wald_workspace
end_test

begin_test "wald status with no baums planted"
    setup_wald_workspace

    # Add repos but don't plant any baums
    $WALD_BIN repo add "github.com/test/repo"

    _result=$($WALD_BIN status 2>&1)

    # Should show 0 baums
    assert_contains "$_result" "0"
    assert_contains "$_result" "Baums"

    teardown_wald_workspace
end_test

begin_test "wald status shows last sync info"
    setup_wald_workspace

    _result=$($WALD_BIN status 2>&1)

    # Should show last sync status (even if never synced)
    assert_contains "$_result" "Sync"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
