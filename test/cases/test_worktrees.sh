#!/usr/bin/env bash
# Tests for 'wald worktrees' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic worktrees listing tests
# ====================================================================================

begin_test "wald worktrees lists all worktrees across workspace"
    setup_wald_workspace

    # Create two baums with multiple worktrees
    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"
    $WALD_BIN plant "github.com/test/repo1" "tools/repo1" main dev
    $WALD_BIN plant "github.com/test/repo2" "tools/repo2" main

    _result=$($WALD_BIN worktrees 2>&1)

    # Should list worktrees from both baums
    assert_contains "$_result" "main"
    assert_contains "$_result" "dev"
    assert_contains "$_result" "tools/repo1"
    assert_contains "$_result" "tools/repo2"

    teardown_wald_workspace
end_test

begin_test "wald worktrees with single baum"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    _result=$($WALD_BIN worktrees 2>&1)

    assert_contains "$_result" "main"
    assert_contains "$_result" "tools/repo"

    teardown_wald_workspace
end_test

begin_test "wald worktrees with multiple baums"
    setup_wald_workspace

    # Create three baums in different locations
    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    create_bare_repo "github.com/test/repo3" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"
    $WALD_BIN repo add "github.com/test/repo3"
    $WALD_BIN plant "github.com/test/repo1" "tools/repo1" main
    $WALD_BIN plant "github.com/test/repo2" "research/repo2" main dev
    $WALD_BIN plant "github.com/test/repo3" "admin/repo3" main

    _result=$($WALD_BIN worktrees 2>&1)

    assert_contains "$_result" "tools/repo1"
    assert_contains "$_result" "research/repo2"
    assert_contains "$_result" "admin/repo3"

    teardown_wald_workspace
end_test

begin_test "wald worktrees with filter limits to path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"
    $WALD_BIN plant "github.com/test/repo1" "tools/repo1" main
    $WALD_BIN plant "github.com/test/repo2" "research/repo2" main

    # Filter to only tools directory (positional argument)
    _result=$($WALD_BIN worktrees tools 2>&1)

    assert_contains "$_result" "tools/repo1"
    assert_not_contains "$_result" "research/repo2"

    teardown_wald_workspace
end_test

begin_test "wald worktrees --json produces valid JSON"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    _result=$($WALD_BIN worktrees --json 2>&1)

    # Should be valid JSON
    assert_json_valid "$_result"
    # Should contain expected fields
    assert_contains "$_result" "repo_id"
    assert_contains "$_result" "branch"
    assert_contains "$_result" "container"

    teardown_wald_workspace
end_test

begin_test "wald worktrees shows empty message when none exist"
    setup_wald_workspace

    # No baums planted
    _result=$($WALD_BIN worktrees 2>&1)

    assert_contains "$_result" "No worktrees"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
