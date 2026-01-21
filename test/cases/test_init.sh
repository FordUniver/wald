#!/usr/bin/env bash
# Tests for 'wald init' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic init tests
# ====================================================================================

begin_test "wald init creates .wald/ structure"
    # Create a fresh temp directory with git repo
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Run init
    $WALD_BIN init

    # Verify structure
    assert_dir_exists ".wald"
    assert_dir_exists ".wald/repos"
    assert_file_exists ".wald/manifest.yaml"
    assert_file_exists ".wald/config.yaml"
    assert_file_exists ".wald/state.yaml"
    assert_file_exists ".gitignore"

    # Verify manifest has empty repos
    assert_file_contains ".wald/manifest.yaml" "repos"

    # Verify config has defaults
    assert_file_contains ".wald/config.yaml" "default_lfs"
    assert_file_contains ".wald/config.yaml" "default_depth"

    # Verify gitignore has wald section
    assert_file_contains ".gitignore" "wald:start"
    assert_file_contains ".gitignore" ".wald/repos/"
    assert_file_contains ".gitignore" ".wald/state.yaml"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

begin_test "wald init fails if .wald/ exists without --force"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initialize once
    $WALD_BIN init

    # Second init should fail
    _result=$($WALD_BIN init 2>&1 || true)
    assert_contains "$_result" "already exists"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

begin_test "wald init --force recreates existing .wald/"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initialize once and add a marker
    $WALD_BIN init
    echo "test_marker" >> .wald/test_marker.txt
    assert_file_exists ".wald/test_marker.txt"

    # Force reinit
    $WALD_BIN init --force

    # Marker should be gone
    assert_file_not_exists ".wald/test_marker.txt"
    # Structure should still be valid
    assert_dir_exists ".wald"
    assert_file_exists ".wald/manifest.yaml"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

begin_test "wald init works with explicit path"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    _target_dir="$_test_dir/workspace"
    mkdir -p "$_target_dir"

    cd "$_target_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"
    cd "$_test_dir"

    # Init with explicit path
    $WALD_BIN init "$_target_dir"

    # Verify structure created in target
    assert_dir_exists "$_target_dir/.wald"
    assert_file_exists "$_target_dir/.wald/manifest.yaml"

    # Cleanup
    rm -rf "$_test_dir"
end_test

begin_test "wald init warns if not a git repo"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"

    # Init without git repo
    _result=$($WALD_BIN init 2>&1)
    assert_contains "$_result" "not a git repository"

    # But should still create workspace
    assert_dir_exists ".wald"
    assert_file_exists ".wald/manifest.yaml"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

begin_test "wald init fails inside existing wald workspace (no nesting)"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initialize parent
    $WALD_BIN init

    # Try to init in subdirectory
    mkdir -p sub/deep
    cd sub/deep

    _result=$($WALD_BIN init 2>&1 || true)
    assert_contains "$_result" "nested"

    # .wald/ should not be created in subdirectory
    assert_dir_not_exists ".wald"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

begin_test "wald init is idempotent with --force"
    _test_dir=$(mktemp -d /tmp/wald-init-test.XXXXXX)
    cd "$_test_dir"
    git init --quiet --initial-branch=main
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Init multiple times with force
    $WALD_BIN init
    $WALD_BIN init --force
    $WALD_BIN init --force

    # Should have exactly one wald section in gitignore
    _count=$(grep -c "wald:start" .gitignore || true)
    assert_eq "1" "$_count" "Should have exactly one wald section"

    # Structure should be valid
    assert_dir_exists ".wald"
    assert_file_exists ".wald/manifest.yaml"

    # Cleanup
    cd /tmp
    rm -rf "$_test_dir"
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
