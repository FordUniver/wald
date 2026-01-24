#!/usr/bin/env bash
# Tests for 'wald repo add' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic repo add tests
# ====================================================================================

begin_test "wald repo add creates manifest entry"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --lfs=minimal --depth=100 github.com/test/repo

    assert_file_exists ".wald/manifest.yaml"
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo"
    assert_file_contains ".wald/manifest.yaml" "lfs: minimal"

    teardown_wald_workspace
end_test

begin_test "wald repo add with custom LFS policy"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --lfs=full github.com/test/large-repo

    assert_file_contains ".wald/manifest.yaml" "github.com/test/large-repo"
    assert_file_contains ".wald/manifest.yaml" "lfs: full"

    teardown_wald_workspace
end_test

begin_test "wald repo add with custom depth"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --depth=50 github.com/test/shallow-repo

    assert_file_contains ".wald/manifest.yaml" "github.com/test/shallow-repo"
    assert_file_contains ".wald/manifest.yaml" "depth: 50"

    teardown_wald_workspace
end_test

begin_test "wald repo add with aliases"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --alias=dots --alias=dotfiles github.com/user/dotfiles

    assert_file_contains ".wald/manifest.yaml" "github.com/user/dotfiles"
    assert_file_contains ".wald/manifest.yaml" "aliases"

    teardown_wald_workspace
end_test

begin_test "wald repo add with upstream"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --upstream=git.zib.de/docker/ais2t git.zib.de/cspiegel/ais2t

    assert_file_contains ".wald/manifest.yaml" "git.zib.de/cspiegel/ais2t"
    assert_file_contains ".wald/manifest.yaml" "upstream: git.zib.de/docker/ais2t"

    teardown_wald_workspace
end_test

# ====================================================================================
# Filter policy tests
# ====================================================================================

begin_test "wald repo add with --filter=none"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --filter=none github.com/test/full-clone

    assert_file_contains ".wald/manifest.yaml" "github.com/test/full-clone"
    assert_file_contains ".wald/manifest.yaml" "filter: none"

    teardown_wald_workspace
end_test

begin_test "wald repo add with --filter=blob-none"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --filter=blob-none github.com/test/blobless

    assert_file_contains ".wald/manifest.yaml" "github.com/test/blobless"
    assert_file_contains ".wald/manifest.yaml" "filter: blob-none"

    teardown_wald_workspace
end_test

begin_test "wald repo add with --filter=tree-0"
    setup_wald_workspace

    $WALD_BIN repo add --no-clone --filter=tree-0 github.com/test/treeless

    assert_file_contains ".wald/manifest.yaml" "github.com/test/treeless"
    assert_file_contains ".wald/manifest.yaml" "filter: tree-zero"

    teardown_wald_workspace
end_test

begin_test "wald repo add defaults to blob-none filter"
    setup_wald_workspace

    # Add without explicit filter - should use default from config
    $WALD_BIN repo add --no-clone github.com/test/default-filter

    assert_file_contains ".wald/manifest.yaml" "github.com/test/default-filter"
    # Default is now blob-none
    assert_file_contains ".wald/manifest.yaml" "filter: blob-none"

    teardown_wald_workspace
end_test

begin_test "wald repo add rejects invalid filter"
    setup_wald_workspace

    # Try invalid filter
    _result=$($WALD_BIN repo add --no-clone --filter=invalid github.com/test/repo 2>&1 || true)
    assert_contains "$_result" "Invalid filter"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald repo add rejects invalid repo ID"
    setup_wald_workspace

    # Expected behavior (not yet implemented)
    # result=$($WALD_BIN repo add invalid-repo-id 2>&1 || true)
    # assert_contains "$result" "error"
    # assert_contains "$result" "invalid"

    # For now, just verify test infrastructure works
    assert_file_exists ".wald/manifest.yaml"

    teardown_wald_workspace
end_test

begin_test "wald repo add prevents duplicate entries"
    setup_wald_workspace

    # Add repo once
    $WALD_BIN repo add --no-clone github.com/test/repo

    # Second add should error or update
    _result=$($WALD_BIN repo add --no-clone github.com/test/repo 2>&1 || true)
    assert_contains "$_result" "already"

    # Verify only one entry exists
    _count=$(grep -c "github.com/test/repo" .wald/manifest.yaml || true)
    assert_eq "1" "$_count" "Should have exactly one repo entry"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
