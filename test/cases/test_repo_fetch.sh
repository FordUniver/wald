#!/usr/bin/env bash
# Tests for 'wald repo fetch' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic fetch tests
# ====================================================================================

begin_test "wald repo fetch reports no repos when none cloned"
    setup_wald_workspace

    # Add repo to manifest but don't create bare repo
    add_repo_to_manifest "github.com/test/repo"

    # Fetch should report no repos
    _result=$($WALD_BIN repo fetch 2>&1)
    assert_contains "$_result" "No repositories"

    teardown_wald_workspace
end_test

begin_test "wald repo fetch fails on missing repo"
    setup_wald_workspace

    # Try to fetch non-existent repo
    _result=$($WALD_BIN repo fetch github.com/nonexistent/repo 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

begin_test "wald repo fetch starts fetching cloned repos"
    setup_wald_workspace

    # Create bare repo (note: fetch will fail because no real remote,
    # but we verify the command recognizes the repo)
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    # Fetch should attempt to fetch (will fail due to no remote, but shows repo)
    _result=$($WALD_BIN repo fetch 2>&1 || true)
    assert_contains "$_result" "Fetching"
    assert_contains "$_result" "github.com/test/repo"

    teardown_wald_workspace
end_test

# ====================================================================================
# Fetch --full tests (partial clone conversion)
# ====================================================================================

begin_test "wald repo fetch --full on non-partial clone reports already full"
    setup_wald_workspace

    # Create regular (non-partial) bare repo
    create_bare_repo "github.com/test/repo" with_commits
    add_repo_to_manifest "github.com/test/repo"

    # Fetch --full should report already full (may fail fetch due to no remote)
    _result=$($WALD_BIN repo fetch --full github.com/test/repo 2>&1 || true)
    assert_contains "$_result" "already full"

    teardown_wald_workspace
end_test

begin_test "wald repo fetch --full detects partial clone"
    setup_wald_workspace

    # Create bare repo
    create_bare_repo "github.com/test/repo" with_commits

    # Use CLI to add repo (creates proper manifest format)
    $WALD_BIN repo add --no-clone --filter=blob-none github.com/test/repo

    # Simulate partial clone by setting git config
    _bare_path=$(get_bare_repo_path "github.com/test/repo")
    git -C "$_bare_path" config remote.origin.promisor true
    git -C "$_bare_path" config remote.origin.partialclonefilter "blob:none"

    # Fetch --full should detect and attempt conversion
    _result=$($WALD_BIN repo fetch --full github.com/test/repo 2>&1 || true)
    assert_contains "$_result" "Converting to full clone"

    # Verify git config was removed (conversion happened)
    _promisor=$(git -C "$_bare_path" config remote.origin.promisor 2>&1 || echo "not set")
    assert_contains "$_promisor" "not set"

    teardown_wald_workspace
end_test

begin_test "wald repo fetch --full removes promisor config"
    setup_wald_workspace

    # Create bare repo
    create_bare_repo "github.com/test/repo" with_commits

    # Add with filter
    $WALD_BIN repo add --no-clone --filter=blob-none github.com/test/repo

    # Mark as partial clone
    _bare_path=$(get_bare_repo_path "github.com/test/repo")
    git -C "$_bare_path" config remote.origin.promisor true
    git -C "$_bare_path" config remote.origin.partialclonefilter "blob:none"

    # Verify promisor is set before conversion
    _promisor_before=$(git -C "$_bare_path" config remote.origin.promisor 2>&1)
    assert_eq "true" "$_promisor_before" "promisor should be set before conversion"

    # Fetch --full will fail (no real remote) but should still remove promisor config
    $WALD_BIN repo fetch --full github.com/test/repo 2>&1 || true

    # Verify promisor config was removed
    _promisor_after=$(git -C "$_bare_path" config remote.origin.promisor 2>&1 || echo "not set")
    assert_contains "$_promisor_after" "not set"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
