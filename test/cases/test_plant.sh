#!/usr/bin/env bash
# Tests for 'wald plant' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic plant tests
# ====================================================================================

begin_test "wald plant creates baum with single worktree"
    setup_wald_workspace

    # Create bare repo first
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Verify baum structure
    assert_dir_exists "tools/repo/.baum"
    assert_file_exists "tools/repo/.baum/manifest.yaml"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_baum_has_worktree "tools/repo" "main"

    # Verify bare repo registry
    assert_bare_worktree_count "github.com/test/repo" 1

    teardown_wald_workspace
end_test

begin_test "wald plant creates baum with multiple worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev

    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "dev"

    # Verify bare repo registry
    assert_bare_worktree_count "github.com/test/repo" 2

    teardown_wald_workspace
end_test

begin_test "wald plant with nested container path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "research/25-project/repo" main

    assert_dir_exists "research/25-project/repo/.baum"
    assert_worktree_exists "research/25-project/repo/_main.wt"

    # Verify bare repo registry
    assert_bare_worktree_count "github.com/test/repo" 1

    teardown_wald_workspace
end_test

begin_test "wald plant creates .gitignore for worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Verify .gitignore was created with worktree entry
    assert_dir_exists "tools/repo/.baum"
    assert_file_exists "tools/repo/.gitignore"
    assert_file_contains "tools/repo/.gitignore" "_main.wt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Using repo aliases
# ====================================================================================

begin_test "wald plant accepts repo alias"
    setup_wald_workspace

    create_bare_repo "github.com/user/dotfiles" "with_commits"
    $WALD_BIN repo add "github.com/user/dotfiles" --alias dots --alias dotfiles

    # Plant using alias
    $WALD_BIN plant dots "infrastructure/dotfiles" main

    assert_dir_exists "infrastructure/dotfiles/.baum"
    assert_worktree_exists "infrastructure/dotfiles/_main.wt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald plant fails if repo not in manifest"
    setup_wald_workspace

    _result=$($WALD_BIN plant "github.com/unknown/repo" "tools/repo" main 2>&1 || true)
    assert_contains "$_result" "not found"

    teardown_wald_workspace
end_test

begin_test "wald plant fails if bare repo missing"
    setup_wald_workspace

    # Add to manifest but don't create bare repo
    $WALD_BIN repo add "github.com/test/missing"

    _result=$($WALD_BIN plant "github.com/test/missing" "tools/missing" main 2>&1 || true)
    assert_contains "$_result" "not found"

    # Verify manifest exists but bare repo doesn't
    assert_file_exists ".wald/manifest.yaml"
    assert_dir_not_exists ".wald/repos/github.com/test/missing.git"

    teardown_wald_workspace
end_test

begin_test "wald plant fails if container already exists as file"
    setup_wald_workspace

    # Create file where baum would go
    mkdir -p tools
    touch tools/repo

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"

    _result=$($WALD_BIN plant "github.com/test/repo" "tools/repo" main 2>&1 || true)
    assert_contains "$_result" "not a directory"

    teardown_wald_workspace
end_test

begin_test "wald plant adds worktrees to existing baum"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Second plant should add to existing baum
    $WALD_BIN plant "github.com/test/repo" "tools/repo" dev

    # Verify both worktrees exist
    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "dev"

    # Verify bare repo registry has both
    assert_bare_worktree_count "github.com/test/repo" 2

    teardown_wald_workspace
end_test

begin_test "wald plant on existing baum fails if branch exists"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main

    # Second plant with same branch should fail
    _result=$($WALD_BIN plant "github.com/test/repo" "tools/repo" main 2>&1 || true)
    assert_contains "$_result" "already exists"

    # Verify original plant still intact
    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald plant warns for partial clone"
    setup_wald_workspace

    # Create bare repo and mark it as partial clone
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo"

    # Mark as partial clone
    _bare_path=$(get_bare_repo_path "github.com/test/repo")
    git -C "$_bare_path" config remote.origin.promisor true
    git -C "$_bare_path" config remote.origin.partialclonefilter "blob:none"

    # Plant should warn about partial clone
    _result=$($WALD_BIN plant "github.com/test/repo" "tools/repo" main 2>&1)
    assert_contains "$_result" "partial clone"
    assert_contains "$_result" "Network"

    # Plant should still succeed
    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
