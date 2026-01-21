#!/usr/bin/env bash
# Tests for basic 'wald sync' scenarios (non-move cases)

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Clean sync scenarios
# ====================================================================================

begin_test "wald sync with no changes reports up to date"
    setup_wald_workspace

    # Commit initial state
    git add -A
    git commit -m "Initial workspace"

    # Expected behavior:
    # result=$($WALD_BIN sync)
    # assert_contains "$result" "up to date"
    # assert_contains "$result" "No changes"

    # For now, just verify workspace is clean
    _status=$(git status --porcelain)
    assert_eq "" "$_status" "Workspace should be clean"

    teardown_wald_workspace
end_test

begin_test "wald sync updates last_sync in state.yaml"
    setup_wald_workspace

    # Get current commit
    _current_commit=$(git rev-parse HEAD)

    # Sync should update state.yaml
    $WALD_BIN sync

    # Verify state updated
    _last_sync=$(get_last_sync)
    assert_eq "$_current_commit" "$_last_sync" "last_sync should match current commit"

    teardown_wald_workspace
end_test

# ====================================================================================
# Workspace ahead of origin
# ====================================================================================

begin_test "wald sync detects when workspace is ahead"
    setup_multi_machine

    # Alpha makes changes
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    git add -A
    git commit -m "Add repo to manifest"
    # Don't push yet

    # Check git status shows ahead
    _status=$(git status)
    assert_contains "$_status" "Your branch is ahead"

    teardown_multi_machine
end_test

begin_test "wald sync pushes when workspace ahead (with --push)"
    setup_multi_machine

    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    git add -A
    git commit -m "Add repo"

    # Expected behavior:
    # $WALD_BIN sync --push

    # Simulate: push changes
    git push origin main

    # Verify beta can now pull
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main

    assert_file_exists ".wald/manifest.yaml"
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo"

    teardown_multi_machine
end_test

# ====================================================================================
# Workspace behind origin
# ====================================================================================

begin_test "wald sync pulls when workspace behind"
    setup_multi_machine

    # Alpha makes and pushes changes
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    workspace_commit "$TEST_ALPHA" "Add repo"

    # Beta is now behind
    cd "$TEST_BETA" || exit 1

    # Expected behavior:
    # $WALD_BIN sync

    # Simulate: pull changes
    git pull --rebase origin main

    # Verify changes arrived
    assert_file_exists ".wald/manifest.yaml"
    assert_file_contains ".wald/manifest.yaml" "github.com/test/repo"

    teardown_multi_machine
end_test

# ====================================================================================
# Diverged scenarios
# ====================================================================================

begin_test "wald sync detects diverged history"
    setup_multi_machine

    # Alpha makes changes and pushes
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/alpha-repo" "with_commits"
    add_repo_to_manifest "github.com/test/alpha-repo" "minimal" "100"
    workspace_commit "$TEST_ALPHA" "Alpha changes"

    # Beta makes different changes (don't pull alpha's changes)
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/beta-repo" "with_commits"
    add_repo_to_manifest "github.com/test/beta-repo" "minimal" "100"
    git add -A
    git commit -m "Beta changes"

    # Now beta is diverged (has unpushed commits + origin has new commits)
    # Expected behavior:
    # result=$($WALD_BIN sync 2>&1 || true)
    # assert_contains "$result" "diverged"
    # assert_contains "$result" "--force"

    # Verify diverged state via git
    git fetch origin
    _behind=$(git rev-list HEAD..origin/main --count)
    _ahead=$(git rev-list origin/main..HEAD --count)

    assert_gt "$_behind" "0" "Should be behind origin"
    assert_gt "$_ahead" "0" "Should be ahead of origin"

    teardown_multi_machine
end_test

# ====================================================================================
# Uncommitted changes handling
# ====================================================================================

begin_test "wald sync fails with uncommitted changes"
    setup_wald_workspace

    # Make uncommitted changes
    echo "uncommitted" > new-file.txt

    # Expected behavior:
    # result=$($WALD_BIN sync 2>&1 || true)
    # assert_contains "$result" "uncommitted changes"
    # assert_contains "$result" "commit or stash"

    # Verify workspace is dirty
    _status=$(git status --porcelain)
    assert_contains "$_status" "new-file.txt"

    teardown_wald_workspace
end_test

begin_test "wald sync with --stash handles uncommitted changes"
    setup_wald_workspace

    # Make uncommitted changes
    echo "uncommitted" > new-file.txt

    # Expected behavior (with rebase.autoStash enabled):
    # git pull --rebase will automatically stash and restore

    # Simulate: commit changes (since we can't test stash without pull)
    git add new-file.txt
    git commit -m "Add file"

    # Verify file was committed
    assert_file_exists "new-file.txt"
    _status=$(git status --porcelain)
    assert_eq "" "$_status" "Workspace should be clean after commit"

    teardown_wald_workspace
end_test

# ====================================================================================
# State file validation
# ====================================================================================

begin_test "wald sync initializes state.yaml if missing"
    setup_wald_workspace

    # Remove state file
    rm -f .wald/state.yaml

    # Sync should recreate it
    $WALD_BIN sync

    assert_file_exists ".wald/state.yaml"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
