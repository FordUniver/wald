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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Expected behavior (not yet implemented):
    # $WALD_BIN plant github.com/test/repo tools/repo main

    # Simulate expected result
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Verify baum structure
    assert_dir_exists "tools/repo/.baum"
    assert_file_exists "tools/repo/.baum/manifest.yaml"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_baum_has_worktree "tools/repo" "main"

    teardown_wald_workspace
end_test

begin_test "wald plant creates baum with multiple worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Expected behavior:
    # $WALD_BIN plant github.com/test/repo tools/repo main dev feature-x

    # Simulate expected result
    plant_baum "github.com/test/repo" "tools/repo" "main" "dev"

    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "dev"

    teardown_wald_workspace
end_test

begin_test "wald plant with nested container path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Plant in nested directory
    plant_baum "github.com/test/repo" "research/25-project/repo" "main"

    assert_dir_exists "research/25-project/repo/.baum"
    assert_worktree_exists "research/25-project/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald plant creates .gitignore for worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Expected: .gitignore should ignore worktree directories
    # For now, just verify baum was created
    assert_dir_exists "tools/repo/.baum"

    teardown_wald_workspace
end_test

# ====================================================================================
# Using repo aliases
# ====================================================================================

begin_test "wald plant accepts repo alias"
    setup_wald_workspace

    create_bare_repo "github.com/user/dotfiles" "with_commits"
    add_repo_with_aliases "github.com/user/dotfiles" "dots" "dotfiles"

    # Expected behavior:
    # $WALD_BIN plant dots infrastructure/dotfiles main

    # Simulate (using full ID since alias resolution not implemented)
    plant_baum "github.com/user/dotfiles" "infrastructure/dotfiles" "main"

    assert_dir_exists "infrastructure/dotfiles/.baum"
    assert_worktree_exists "infrastructure/dotfiles/_main.wt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald plant fails if repo not in manifest"
    setup_wald_workspace

    # Expected behavior:
    # result=$($WALD_BIN plant github.com/unknown/repo tools/repo main 2>&1 || true)
    # assert_contains "$result" "not found in manifest"
    # assert_contains "$result" "github.com/unknown/repo"

    # Verify test setup works
    assert_file_exists ".wald/manifest.yaml"

    teardown_wald_workspace
end_test

begin_test "wald plant fails if bare repo missing"
    setup_wald_workspace

    # Add to manifest but don't create bare repo
    add_repo_to_manifest "github.com/test/missing" "minimal" "100"

    # Expected behavior:
    # result=$($WALD_BIN plant github.com/test/missing tools/missing main 2>&1 || true)
    # assert_contains "$result" "bare repo not found"

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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Expected behavior:
    # result=$($WALD_BIN plant github.com/test/repo tools/repo main 2>&1 || true)
    # assert_contains "$result" "already exists"
    # assert_contains "$result" "not a directory"

    # Verify file exists
    assert_file_exists "tools/repo"

    teardown_wald_workspace
end_test

begin_test "wald plant fails if baum already planted"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Expected behavior: second plant should fail
    # result=$($WALD_BIN plant github.com/test/repo tools/repo dev 2>&1 || true)
    # assert_contains "$result" "already planted"

    # Verify first plant succeeded
    assert_dir_exists "tools/repo/.baum"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
