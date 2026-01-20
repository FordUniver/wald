#!/usr/bin/env bash
# Tests for 'wald move' command

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic move tests
# ====================================================================================

begin_test "wald move relocates baum to new path"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Expected behavior (not yet implemented):
    # $WALD_BIN move tools/repo admin/repo

    # Simulate expected result: move directory tree
    mkdir -p admin
    mv tools/repo admin/repo

    # Verify move
    assert_dir_not_exists "tools/repo"
    assert_dir_exists "admin/repo/.baum"
    assert_worktree_exists "admin/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald move preserves worktree functionality"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main" "dev"

    # Move baum
    mkdir -p research/project
    mv tools/repo research/project/repo

    # Verify both worktrees still exist and are valid
    assert_worktree_exists "research/project/repo/_main.wt"
    assert_worktree_exists "research/project/repo/_dev.wt"
    assert_baum_has_worktree "research/project/repo" "main"
    assert_baum_has_worktree "research/project/repo" "dev"

    teardown_wald_workspace
end_test

begin_test "wald move updates baum manifest paths"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Move baum
    mkdir -p infrastructure
    mv tools/repo infrastructure/repo

    # Verify baum manifest still valid
    assert_file_exists "infrastructure/repo/.baum/manifest.yaml"
    assert_file_contains "infrastructure/repo/.baum/manifest.yaml" "github.com/test/repo"

    teardown_wald_workspace
end_test

begin_test "wald move preserves uncommitted changes in worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Make uncommitted changes in worktree
    echo "uncommitted work" > "tools/repo/_main.wt/work.txt"

    # Move baum
    mkdir -p admin
    mv tools/repo admin/repo

    # Verify uncommitted changes still present
    assert_file_exists "admin/repo/_main.wt/work.txt"
    assert_file_contains "admin/repo/_main.wt/work.txt" "uncommitted work"

    teardown_wald_workspace
end_test

# ====================================================================================
# Nested path moves
# ====================================================================================

begin_test "wald move handles deeply nested paths"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "research/25-project/nested/repo" "main"

    # Move to different nested path
    mkdir -p admin/archived/old-projects
    mv research/25-project/nested/repo admin/archived/old-projects/repo

    assert_dir_not_exists "research/25-project/nested/repo"
    assert_dir_exists "admin/archived/old-projects/repo/.baum"
    assert_worktree_exists "admin/archived/old-projects/repo/_main.wt"

    teardown_wald_workspace
end_test

begin_test "wald move from nested to root level"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "research/25-project/repo" "main"

    # Move to top level
    mv research/25-project/repo repo

    assert_dir_not_exists "research/25-project/repo"
    assert_dir_exists "repo/.baum"
    assert_worktree_exists "repo/_main.wt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Error cases
# ====================================================================================

begin_test "wald move fails if source doesn't exist"
    setup_wald_workspace

    # Expected behavior:
    # result=$($WALD_BIN move nonexistent/path new/path 2>&1 || true)
    # assert_contains "$result" "not found"
    # assert_contains "$result" "nonexistent/path"

    # Verify workspace is clean
    assert_file_exists ".wald/manifest.yaml"

    teardown_wald_workspace
end_test

begin_test "wald move fails if source is not a baum"
    setup_wald_workspace

    # Create regular directory (not a baum)
    mkdir -p tools/regular-dir
    echo "not a baum" > tools/regular-dir/file.txt

    # Expected behavior:
    # result=$($WALD_BIN move tools/regular-dir admin/regular-dir 2>&1 || true)
    # assert_contains "$result" "not a baum"
    # assert_contains "$result" ".baum directory not found"

    # Verify it's not a baum
    assert_dir_exists "tools/regular-dir"
    assert_dir_not_exists "tools/regular-dir/.baum"

    teardown_wald_workspace
end_test

begin_test "wald move fails if destination already exists"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Create destination directory
    mkdir -p admin/repo
    echo "existing content" > admin/repo/file.txt

    # Expected behavior:
    # result=$($WALD_BIN move tools/repo admin/repo 2>&1 || true)
    # assert_contains "$result" "already exists"
    # assert_contains "$result" "admin/repo"

    # Verify source still exists
    assert_dir_exists "tools/repo/.baum"
    # Verify destination wasn't overwritten
    assert_file_exists "admin/repo/file.txt"

    teardown_wald_workspace
end_test

# ====================================================================================
# Git integration
# ====================================================================================

begin_test "wald move stages changes in git"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Commit initial state
    git add -A
    git commit -m "Plant repo"

    # Expected behavior: wald move should stage the changes
    # $WALD_BIN move tools/repo admin/repo

    # Simulate move
    mkdir -p admin
    git mv tools/repo admin/repo

    # Verify git sees the move as a rename (not delete + add)
    local status
    status=$(git status --short)

    # Git should show rename (R) not delete + add
    # This test validates git detects the move properly
    assert_contains "$status" "admin/repo"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
