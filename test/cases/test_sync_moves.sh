#!/usr/bin/env bash
# Tests for 'wald sync' move detection and replay
# Core feature: detecting and replaying baum moves across machines
#
# NOTE: Tests use CLI where possible. Some helpers remain for setup:
# - create_bare_repo: Creates bare repos (simulates clone from remote)
# - workspace_commit: Git operations on workspace repo
# - detect_moves: Uses git diff -M to verify move detection works

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic move detection
# ====================================================================================

begin_test "wald sync detects baum move via git diff -M"
    setup_multi_machine

    # Alpha: plant and commit
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    _before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Alpha: move baum using CLI
    $WALD_BIN move tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo to admin"

    _after_commit=$(get_commit_hash "$TEST_ALPHA")

    # Verify git detects the move
    _moves=$(detect_moves "$TEST_ALPHA" "$_before_commit" "$_after_commit")

    assert_contains "$_moves" ".baum/manifest.yaml"
    assert_contains "$_moves" "tools/repo" "Should show old path"
    assert_contains "$_moves" "admin/repo" "Should show new path"

    teardown_multi_machine
end_test

begin_test "wald sync replays single baum move"
    setup_multi_machine

    # Alpha: plant baum
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync to get initial plant
    cd "$TEST_BETA" || exit 1
    # Create bare repo before sync (simulates having cloned it)
    create_bare_repo "github.com/test/repo" "with_commits"
    # Sync pulls manifest (with repo entry) and baum directory
    $WALD_BIN sync
    # Materialize baum (creates worktrees from manifest entries)
    materialize_baum "tools/repo"

    assert_worktree_exists "tools/repo/_main.wt"

    # Alpha: move baum using CLI
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move to admin"

    # Beta: sync should replay move
    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify move replayed
    assert_dir_not_exists "tools/repo"
    assert_dir_exists "admin/repo/.baum"
    assert_worktree_exists "admin/repo/_main.wt"

    teardown_multi_machine
end_test

begin_test "wald sync replays move with multiple worktrees"
    setup_multi_machine

    # Alpha: plant with multiple branches
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main dev
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync and materialize
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN sync
    materialize_baum "tools/repo"

    # Alpha: move baum using CLI
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/repo research/project/repo
    workspace_commit "$TEST_ALPHA" "Move to research"

    # Beta: sync replays the move
    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify all worktrees moved (sync moves the entire baum directory)
    assert_dir_not_exists "tools/repo"
    assert_worktree_exists "research/project/repo/_main.wt"
    assert_worktree_exists "research/project/repo/_dev.wt"

    teardown_multi_machine
end_test

# ====================================================================================
# Multiple moves in single commit
# ====================================================================================

begin_test "wald sync replays multiple baum moves in one commit"
    setup_multi_machine

    # Alpha: plant two baums
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo-a" "with_commits"
    create_bare_repo "github.com/test/repo-b" "with_commits"
    $WALD_BIN repo add "github.com/test/repo-a"
    $WALD_BIN repo add "github.com/test/repo-b"
    $WALD_BIN plant "github.com/test/repo-a" "tools/repo-a" main
    $WALD_BIN plant "github.com/test/repo-b" "tools/repo-b" main
    workspace_commit "$TEST_ALPHA" "Plant repos"

    # Beta: sync and materialize
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/repo-a" "with_commits"
    create_bare_repo "github.com/test/repo-b" "with_commits"
    $WALD_BIN sync
    materialize_baum "tools/repo-a"
    materialize_baum "tools/repo-b"

    # Alpha: move both baums (move commits together, push once)
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/repo-a admin/repo-a
    $WALD_BIN move tools/repo-b admin/repo-b
    workspace_commit "$TEST_ALPHA" "Move both repos to admin"

    # Beta: sync replays both moves
    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify both moves replayed
    assert_dir_not_exists "tools/repo-a"
    assert_dir_not_exists "tools/repo-b"
    assert_worktree_exists "admin/repo-a/_main.wt"
    assert_worktree_exists "admin/repo-b/_main.wt"

    teardown_multi_machine
end_test

# ====================================================================================
# Move with uncommitted changes in worktrees
# ====================================================================================

begin_test "wald sync move preserves uncommitted changes"
    setup_multi_machine

    # Alpha: plant and push
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync and make uncommitted changes
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN sync
    materialize_baum "tools/repo"
    echo "local work" > "tools/repo/_main.wt/work.txt"

    # Alpha: move baum
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo"

    # Beta: sync replays move, preserving local worktree contents
    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify uncommitted work preserved (sync moves the whole baum directory)
    assert_file_exists "admin/repo/_main.wt/work.txt"
    assert_file_contains "admin/repo/_main.wt/work.txt" "local work"

    teardown_multi_machine
end_test

# ====================================================================================
# Rename detection (move with similarity)
# ====================================================================================

begin_test "wald sync detects baum rename as move"
    setup_multi_machine

    # Alpha: plant baum
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/old-name" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    _before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Alpha: rename baum (move within same directory)
    $WALD_BIN move tools/old-name tools/new-name
    workspace_commit "$TEST_ALPHA" "Rename to new-name"

    _after_commit=$(get_commit_hash "$TEST_ALPHA")

    # Verify git detects as rename (not delete + add)
    _moves=$(detect_moves "$TEST_ALPHA" "$_before_commit" "$_after_commit")

    assert_contains "$_moves" ".baum/manifest.yaml"
    assert_contains "$_moves" "old-name"
    assert_contains "$_moves" "new-name"

    teardown_multi_machine
end_test

# ====================================================================================
# Edge cases
# ====================================================================================

begin_test "wald sync ignores non-baum directory moves"
    setup_multi_machine

    # Alpha: create regular directory (not a baum) and move it
    cd "$TEST_ALPHA" || exit 1
    mkdir -p tools/regular-dir
    echo "not a baum" > tools/regular-dir/file.txt
    git add tools/regular-dir/file.txt
    git commit -m "Add regular directory"

    _before_commit=$(get_commit_hash "$TEST_ALPHA")

    mkdir -p admin
    git mv tools/regular-dir admin/regular-dir
    workspace_commit "$TEST_ALPHA" "Move regular directory"

    # Beta: sync should pull changes but not treat as baum move
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main

    # Verify regular directory moved via git, not as baum operation
    assert_dir_not_exists "tools/regular-dir"
    assert_dir_exists "admin/regular-dir"
    assert_file_exists "admin/regular-dir/file.txt"
    # Should NOT have .baum directory
    assert_dir_not_exists "admin/regular-dir/.baum"

    teardown_multi_machine
end_test

begin_test "wald sync handles move to deeply nested path"
    setup_multi_machine

    # Alpha: plant
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync and materialize
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN sync
    materialize_baum "tools/repo"

    # Alpha: move to deeply nested path
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/repo research/2025/archived/old-projects/q1/repo
    workspace_commit "$TEST_ALPHA" "Archive old repo"

    # Beta: sync replays the move
    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify deep move (sync moves the whole baum with worktrees)
    assert_dir_not_exists "tools/repo"
    assert_worktree_exists "research/2025/archived/old-projects/q1/repo/_main.wt"

    teardown_multi_machine
end_test

# ====================================================================================
# State tracking
# ====================================================================================

begin_test "wald sync updates last_sync after processing moves"
    setup_multi_machine

    # Alpha: plant and move
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN repo add "github.com/test/repo"
    $WALD_BIN plant "github.com/test/repo" "tools/repo" main
    workspace_commit "$TEST_ALPHA" "Plant repo"

    $WALD_BIN move tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo"

    _final_commit=$(get_commit_hash "$TEST_ALPHA")

    # Beta: sync should update last_sync to final commit
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    $WALD_BIN sync

    # Verify state was updated (sync writes last_sync)
    _last_sync=$(get_last_sync)
    assert_eq "$_final_commit" "$_last_sync" "last_sync should match after move"

    teardown_multi_machine
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
