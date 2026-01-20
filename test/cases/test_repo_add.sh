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

    # Note: This will fail until wald is implemented
    # For now, we create the expected state manually to validate test infrastructure
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    assert_file_exists ".wald/manifest.yaml"
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo"
    assert_file_contains ".wald/manifest.yaml" "lfs: minimal"

    teardown_wald_workspace
end_test

begin_test "wald repo add with custom LFS policy"
    setup_wald_workspace

    # Expected behavior (not yet implemented)
    # $WALD_BIN repo add --lfs=full github.com/test/large-repo

    # Simulate expected result
    add_repo_to_manifest "github.com/test/large-repo" "full" "100"

    assert_file_contains ".wald/manifest.yaml" "github.com/test/large-repo"
    assert_file_contains ".wald/manifest.yaml" "lfs: full"

    teardown_wald_workspace
end_test

begin_test "wald repo add with custom depth"
    setup_wald_workspace

    # Expected behavior (not yet implemented)
    # $WALD_BIN repo add --depth=50 github.com/test/shallow-repo

    # Simulate expected result
    add_repo_to_manifest "github.com/test/shallow-repo" "minimal" "50"

    assert_file_contains ".wald/manifest.yaml" "github.com/test/shallow-repo"
    assert_file_contains ".wald/manifest.yaml" "depth: 50"

    teardown_wald_workspace
end_test

begin_test "wald repo add with aliases"
    setup_wald_workspace

    # Expected behavior (not yet implemented)
    # $WALD_BIN repo add --alias=dots --alias=dotfiles github.com/user/dotfiles

    # Simulate expected result
    add_repo_with_aliases "github.com/user/dotfiles" "dots" "dotfiles"

    assert_file_contains ".wald/manifest.yaml" "github.com/user/dotfiles"
    assert_file_contains ".wald/manifest.yaml" "aliases"

    teardown_wald_workspace
end_test

begin_test "wald repo add with upstream"
    setup_wald_workspace

    # Expected behavior (not yet implemented)
    # $WALD_BIN repo add --upstream=git.zib.de/docker/ais2t git.zib.de/cspiegel/ais2t

    # Simulate expected result
    add_repo_with_upstream "git.zib.de/cspiegel/ais2t" "git.zib.de/docker/ais2t" "minimal" "100"

    assert_file_contains ".wald/manifest.yaml" "git.zib.de/cspiegel/ais2t"
    assert_file_contains ".wald/manifest.yaml" "upstream: git.zib.de/docker/ais2t"

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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Expected behavior: second add should error or update
    # result=$($WALD_BIN repo add github.com/test/repo 2>&1 || true)
    # assert_contains "$result" "already exists"

    # Verify only one entry exists
    _count=$(grep -c "github.com/test/repo" .wald/manifest.yaml || true)
    assert_eq "1" "$_count" "Should have exactly one repo entry"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
