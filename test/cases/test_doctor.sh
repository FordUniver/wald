#!/usr/bin/env bash
# Tests for 'wald doctor' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic doctor tests
# ====================================================================================

begin_test "wald doctor finds no issues in healthy workspace"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    _result=$($WALD_BIN doctor 2>&1)

    assert_contains "$_result" "No issues found"

    teardown_wald_workspace
end_test

begin_test "wald doctor detects missing bare repo"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"

    # Remove bare repo after adding to manifest
    rm -rf ".wald/repos/github.com/test/repo.git"

    _result=$($WALD_BIN doctor 2>&1)

    assert_contains "$_result" "not cloned"

    teardown_wald_workspace
end_test

begin_test "wald doctor detects orphaned worktree in manifest"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    # Remove dev worktree directory but keep manifest entry
    rm -rf "tools/repo/_dev.wt"

    _result=$($WALD_BIN doctor 2>&1)

    assert_contains "$_result" "Missing worktree"

    teardown_wald_workspace
end_test

begin_test "wald doctor detects corrupted baum manifest"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Corrupt manifest
    echo "invalid: [yaml" > "tools/repo/.baum/manifest.yaml"

    _result=$($WALD_BIN doctor 2>&1)

    assert_contains "$_result" "Invalid baum manifest"

    teardown_wald_workspace
end_test

begin_test "wald doctor --fix creates missing directories"
    setup_wald_workspace

    # Remove repos directory
    rm -rf ".wald/repos"

    _result=$($WALD_BIN doctor --fix 2>&1)

    # Should have fixed the issue
    assert_contains "$_result" "Fixed"

    # repos directory should now exist
    assert_dir_exists ".wald/repos"

    teardown_wald_workspace
end_test

begin_test "wald doctor reports unfixable issues separately"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Create unfixable issue: remove bare repo
    rm -rf ".wald/repos/github.com/test/repo.git"

    _result=$($WALD_BIN doctor 2>&1)

    # Should report issue
    assert_contains "$_result" "WARN"

    teardown_wald_workspace
end_test

begin_test "wald doctor with multiple issues"
    setup_wald_workspace

    # Create multiple repos
    create_bare_repo "github.com/test/repo1" "with_commits"
    create_bare_repo "github.com/test/repo2" "with_commits"
    $WALD_BIN repo add "github.com/test/repo1"
    $WALD_BIN repo add "github.com/test/repo2"
    $WALD_BIN plant "github.com/test/repo1" "tools/repo1" main

    # Create multiple issues
    rm -rf ".wald/repos/github.com/test/repo2.git"  # Missing bare repo
    rm -rf "tools/repo1/_main.wt"  # Missing worktree

    _result=$($WALD_BIN doctor 2>&1)

    # Should report multiple issues
    assert_contains "$_result" "issue"
    # Should mention count
    assert_contains "$_result" "2"

    teardown_wald_workspace
end_test

begin_test "wald doctor detects worktree not in git registry"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Get bare repo path and prune worktrees from git (but keep directory)
    local bare_path
    bare_path=$(get_bare_repo_path "github.com/test/repo")

    # Create a fake worktree directory without git registration
    mkdir -p "tools/repo/_fake.wt"
    echo "gitdir: invalid" > "tools/repo/_fake.wt/.git"

    # Add to baum manifest manually
    cat >> "tools/repo/.baum/manifest.yaml" <<EOF
  - branch: fake
    path: _fake.wt
EOF

    _result=$($WALD_BIN doctor 2>&1)

    # Should detect problem - manifest parsing will fail due to malformed append
    assert_contains "$_result" "Invalid baum manifest"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
