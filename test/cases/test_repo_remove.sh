#!/usr/bin/env bash
# Tests for 'wald repo remove' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic remove tests
# ====================================================================================

begin_test "wald repo remove deletes manifest entry"
    setup_wald_workspace

    # Add repo
    $WALD_BIN repo add --no-clone github.com/test/repo

    # Verify it exists
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo"

    # Remove repo
    _result=$($WALD_BIN repo remove github.com/test/repo 2>&1)
    assert_contains "$_result" "Removed"

    # Verify it's gone from manifest
    if grep -q "github.com/test/repo" .wald/manifest.yaml 2>/dev/null; then
        _fail "repo should be removed from manifest"
    fi

    teardown_wald_workspace
end_test

begin_test "wald repo remove via alias"
    setup_wald_workspace

    # Add repo with alias
    $WALD_BIN repo add --no-clone --alias=dots github.com/user/dotfiles

    # Remove via alias
    _result=$($WALD_BIN repo remove dots 2>&1)
    assert_contains "$_result" "Removed"

    # Verify it's gone
    if grep -q "github.com/user/dotfiles" .wald/manifest.yaml 2>/dev/null; then
        _fail "repo should be removed from manifest"
    fi

    teardown_wald_workspace
end_test

begin_test "wald repo remove preserves bare repo"
    setup_wald_workspace

    # Create bare repo
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    _bare_path=$(get_bare_repo_path "github.com/test/repo")
    assert_dir_exists "$_bare_path"

    # Remove from manifest
    $WALD_BIN repo remove github.com/test/repo

    # Bare repo should still exist (intentional - shared resources)
    assert_dir_exists "$_bare_path"

    teardown_wald_workspace
end_test

begin_test "wald repo remove fails on non-existent repo"
    setup_wald_workspace

    # Try to remove non-existent repo
    _result=$($WALD_BIN repo remove github.com/nonexistent/repo 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

begin_test "wald repo remove with multiple repos"
    setup_wald_workspace

    # Add multiple repos
    $WALD_BIN repo add --no-clone github.com/test/repo1
    $WALD_BIN repo add --no-clone github.com/test/repo2
    $WALD_BIN repo add --no-clone github.com/test/repo3

    # Remove one
    $WALD_BIN repo remove github.com/test/repo2

    # Verify only the correct one is removed
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo1"
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo3"

    if grep -q "github.com/test/repo2" .wald/manifest.yaml 2>/dev/null; then
        _fail "repo2 should be removed from manifest"
    fi

    teardown_wald_workspace
end_test

begin_test "wald repo remove with subgroups"
    setup_wald_workspace

    # Add repo with deep path (GitLab subgroups)
    $WALD_BIN repo add --no-clone git.zib.de/iol/research/project

    # Remove it
    _result=$($WALD_BIN repo remove git.zib.de/iol/research/project 2>&1)
    assert_contains "$_result" "Removed"

    # Verify it's gone
    if grep -q "git.zib.de/iol/research/project" .wald/manifest.yaml 2>/dev/null; then
        _fail "repo should be removed from manifest"
    fi

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
