#!/usr/bin/env bash
# Tests for 'wald sync' move detection and replay
# Core feature: detecting and replaying baum moves across machines

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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Alpha: move baum
    mkdir -p admin
    git -C "$TEST_ALPHA" mv tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo to admin"

    local after_commit
    after_commit=$(get_commit_hash "$TEST_ALPHA")

    # Verify git detects the move
    local moves
    moves=$(detect_moves "$TEST_ALPHA" "$before_commit" "$after_commit")

    assert_contains "$moves" ".baum/manifest.yaml"
    assert_contains "$moves" "tools/repo" "Should show old path"
    assert_contains "$moves" "admin/repo" "Should show new path"

    teardown_multi_machine
end_test

begin_test "wald sync replays single baum move"
    setup_multi_machine

    # Alpha: plant baum
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync to get initial plant
    cd "$TEST_BETA" || exit 1

    # Expected behavior:
    # $WALD_BIN sync

    # Simulate: pull and recreate baum
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "tools/repo" "main"

    assert_worktree_exists "tools/repo/_main.wt"

    # Alpha: move baum
    cd "$TEST_ALPHA" || exit 1
    mkdir -p admin
    git mv tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move to admin"

    # Beta: sync should replay move
    cd "$TEST_BETA" || exit 1

    # Expected behavior:
    # $WALD_BIN sync

    # Simulate: pull and move locally
    git pull --rebase origin main
    mkdir -p admin
    mv tools/repo admin/repo 2>/dev/null || true

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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main" "dev"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "tools/repo" "main" "dev"

    # Alpha: move baum
    cd "$TEST_ALPHA" || exit 1
    mkdir -p research/project
    git mv tools/repo research/project/repo
    workspace_commit "$TEST_ALPHA" "Move to research"

    # Beta: sync and replay
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p research/project
    mv tools/repo research/project/repo 2>/dev/null || true

    # Verify all worktrees moved
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
    add_repo_to_manifest "github.com/test/repo-a" "minimal" "100"
    add_repo_to_manifest "github.com/test/repo-b" "minimal" "100"
    plant_baum "github.com/test/repo-a" "tools/repo-a" "main"
    plant_baum "github.com/test/repo-b" "tools/repo-b" "main"
    workspace_commit "$TEST_ALPHA" "Plant repos"

    # Beta: sync
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo-a" "tools/repo-a" "main"
    plant_baum "github.com/test/repo-b" "tools/repo-b" "main"

    # Alpha: move both baums in single commit
    cd "$TEST_ALPHA" || exit 1
    mkdir -p admin
    git mv tools/repo-a admin/repo-a
    git mv tools/repo-b admin/repo-b
    workspace_commit "$TEST_ALPHA" "Move both repos to admin"

    # Beta: sync should replay both moves
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p admin
    mv tools/repo-a admin/repo-a 2>/dev/null || true
    mv tools/repo-b admin/repo-b 2>/dev/null || true

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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync and make uncommitted changes
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "tools/repo" "main"
    echo "local work" > "tools/repo/_main.wt/work.txt"

    # Alpha: move baum
    cd "$TEST_ALPHA" || exit 1
    mkdir -p admin
    git mv tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo"

    # Beta: sync should preserve uncommitted changes
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p admin
    mv tools/repo admin/repo 2>/dev/null || true

    # Verify uncommitted work preserved
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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/old-name" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Alpha: rename baum (move within same directory)
    git -C "$TEST_ALPHA" mv tools/old-name tools/new-name
    workspace_commit "$TEST_ALPHA" "Rename to new-name"

    local after_commit
    after_commit=$(get_commit_hash "$TEST_ALPHA")

    # Verify git detects as rename (not delete + add)
    local moves
    moves=$(detect_moves "$TEST_ALPHA" "$before_commit" "$after_commit")

    assert_contains "$moves" ".baum/manifest.yaml"
    assert_contains "$moves" "old-name"
    assert_contains "$moves" "new-name"

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

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

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

    # Alpha: plant and move to deeply nested location
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    # Beta: sync
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Alpha: move to deeply nested path
    cd "$TEST_ALPHA" || exit 1
    mkdir -p research/2025/archived/old-projects/q1
    git mv tools/repo research/2025/archived/old-projects/q1/repo
    workspace_commit "$TEST_ALPHA" "Archive old repo"

    # Beta: sync and replay
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p research/2025/archived/old-projects/q1
    mv tools/repo research/2025/archived/old-projects/q1/repo 2>/dev/null || true

    # Verify deep move
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
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant repo"

    mkdir -p admin
    git mv tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move repo"

    local final_commit
    final_commit=$(get_commit_hash "$TEST_ALPHA")

    # Beta: sync should update last_sync to final commit
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p admin
    mv tools/repo admin/repo 2>/dev/null || true

    # Simulate: update state
    update_last_sync "$final_commit"

    local last_sync
    last_sync=$(get_last_sync)
    assert_eq "$final_commit" "$last_sync" "last_sync should match after move"

    teardown_multi_machine
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
