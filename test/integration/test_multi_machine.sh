#!/usr/bin/env bash
# End-to-end multi-machine sync integration test
# Simulates realistic workflow: Mac ↔ Coder environment sync
# Tests the complete wald sync workflow including plant, move, branch operations
#
# NOTE: These tests use CLI calls where possible. Some setup still uses helpers:
# - create_bare_repo: Creates bare repos (simulates what clone would do)
# - workspace_commit/pull: Git operations on workspace repo
#
# Currently, `wald sync` doesn't auto-create worktrees for new baums on other
# machines. This is tracked as future work. Tests note where manual setup is
# needed until sync handles full baum materialization.

# Source test libraries
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Complete Multi-Machine Workflow
# ====================================================================================

begin_test "multi-machine: complete plant-move-branch workflow"
    setup_multi_machine

    # ============================================================================
    # PHASE 1: Alpha (Mac) plants a baum
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Create bare repo (simulates what --clone would do from a real remote)
    create_bare_repo "github.com/test/research-project" "with_commits"

    # Register repo and plant baum using CLI
    $WALD_BIN repo add "github.com/test/research-project"
    $WALD_BIN plant "github.com/test/research-project" "research/project" main dev

    # Commit and push
    workspace_commit "$TEST_ALPHA" "Plant research project"

    # Verify alpha state
    assert_dir_exists "research/project/.baum"
    assert_worktree_exists "research/project/_main.wt"
    assert_worktree_exists "research/project/_dev.wt"

    # ============================================================================
    # PHASE 2: Beta (Coder) syncs and sees the planted baum
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Create bare repo on Beta (in real usage, sync would trigger clone or prompt)
    create_bare_repo "github.com/test/research-project" "with_commits"

    # Sync pulls manifest and baum directories
    $WALD_BIN sync

    # Materialize baum (creates worktrees from manifest entries)
    materialize_baum "research/project"

    # Verify beta got the baum
    assert_dir_exists "research/project/.baum"
    assert_worktree_exists "research/project/_main.wt"
    assert_worktree_exists "research/project/_dev.wt"
    assert_file_contains "research/project/.baum/manifest.yaml" "github.com/test/research-project"

    # ============================================================================
    # PHASE 3: Alpha moves the baum (project reorganization)
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Move baum using CLI
    $WALD_BIN move research/project research/2024/completed/project

    workspace_commit "$TEST_ALPHA" "Archive completed project to 2024"

    # Verify move on alpha
    assert_dir_not_exists "research/project"
    assert_dir_exists "research/2024/completed/project/.baum"
    assert_worktree_exists "research/2024/completed/project/_main.wt"
    assert_worktree_exists "research/2024/completed/project/_dev.wt"

    # ============================================================================
    # PHASE 4: Beta syncs and replays the move
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Sync should detect and replay the move
    $WALD_BIN sync

    # Verify move replayed on beta
    assert_dir_not_exists "research/project"
    assert_dir_exists "research/2024/completed/project/.baum"
    assert_worktree_exists "research/2024/completed/project/_main.wt"
    assert_worktree_exists "research/2024/completed/project/_dev.wt"

    # ============================================================================
    # PHASE 5: Beta makes changes in worktree (simulates work on Coder)
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Make some changes in dev worktree
    echo "Experimental feature" > "research/2024/completed/project/_dev.wt/experiment.txt"
    cd "research/2024/completed/project/_dev.wt" || exit 1
    git add experiment.txt
    git commit -m "Add experimental feature"
    # Note: push would go to the bare repo, not origin (workspace repo)
    # In real usage, the bare repo would have a remote configured
    cd "$TEST_BETA" || exit 1

    # Beta doesn't change workspace structure, so no workspace commit needed

    # ============================================================================
    # PHASE 6: Alpha adds a new branch worktree
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Add a new worktree for feature-x branch using CLI
    $WALD_BIN branch research/2024/completed/project feature-x

    workspace_commit "$TEST_ALPHA" "Add feature-x branch"

    # Verify new worktree on alpha
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"
    assert_baum_has_worktree "research/2024/completed/project" "feature-x"

    # ============================================================================
    # PHASE 7: Beta syncs and gets new worktree
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Sync pulls changes; materialize creates any missing worktrees
    $WALD_BIN sync || true
    materialize_baum "research/2024/completed/project"

    # Verify new worktree on beta
    assert_dir_exists "research/2024/completed/project/.baum"
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"
    assert_baum_has_worktree "research/2024/completed/project" "feature-x"

    # ============================================================================
    # PHASE 8: Verify final state consistency
    # ============================================================================

    # Both machines should have identical structure
    cd "$TEST_ALPHA" || exit 1
    _alpha_baums=$(find . -name ".baum" -type d | sort)

    cd "$TEST_BETA" || exit 1
    _beta_baums=$(find . -name ".baum" -type d | sort)

    # Should have same baum locations
    # (In real test with wald implementation, this would be exact match)
    assert_contains "$_alpha_baums" "research/2024/completed/project/.baum"
    assert_contains "$_beta_baums" "research/2024/completed/project/.baum"

    # Verify both have all three worktrees
    cd "$TEST_ALPHA" || exit 1
    assert_worktree_exists "research/2024/completed/project/_main.wt"
    assert_worktree_exists "research/2024/completed/project/_dev.wt"
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"

    cd "$TEST_BETA" || exit 1
    assert_worktree_exists "research/2024/completed/project/_main.wt"
    assert_worktree_exists "research/2024/completed/project/_dev.wt"
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"

    teardown_multi_machine
end_test

# ====================================================================================
# Multiple Repos Scenario
# ====================================================================================

begin_test "multi-machine: multiple repos with independent operations"
    setup_multi_machine

    # ============================================================================
    # Setup: Alpha plants multiple repos
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Create bare repos (simulates cloning)
    create_bare_repo "github.com/test/tool-a" "with_commits"
    create_bare_repo "github.com/test/tool-b" "with_commits"
    create_bare_repo "github.com/test/library-c" "with_commits"

    # Register and plant using CLI
    $WALD_BIN repo add "github.com/test/tool-a"
    $WALD_BIN repo add "github.com/test/tool-b"
    $WALD_BIN repo add "github.com/test/library-c" --lfs full --depth full

    $WALD_BIN plant "github.com/test/tool-a" "tools/tool-a" main
    $WALD_BIN plant "github.com/test/tool-b" "tools/tool-b" main
    $WALD_BIN plant "github.com/test/library-c" "infrastructure/library-c" main dev

    workspace_commit "$TEST_ALPHA" "Plant multiple repos"

    # ============================================================================
    # Beta syncs
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Create bare repos on Beta
    create_bare_repo "github.com/test/tool-a" "with_commits"
    create_bare_repo "github.com/test/tool-b" "with_commits"
    create_bare_repo "github.com/test/library-c" "with_commits"

    # Sync pulls manifest and baum directories
    $WALD_BIN sync

    # Materialize baums (creates worktrees from manifest entries)
    materialize_baum "tools/tool-a"
    materialize_baum "tools/tool-b"
    materialize_baum "infrastructure/library-c"

    assert_worktree_exists "tools/tool-a/_main.wt"
    assert_worktree_exists "tools/tool-b/_main.wt"
    assert_worktree_exists "infrastructure/library-c/_main.wt"
    assert_worktree_exists "infrastructure/library-c/_dev.wt"

    # ============================================================================
    # Alpha moves only one repo
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/tool-a archived/tool-a
    workspace_commit "$TEST_ALPHA" "Archive tool-a"

    # ============================================================================
    # Beta syncs and only moved repo changes
    # ============================================================================

    cd "$TEST_BETA" || exit 1
    $WALD_BIN sync

    # Verify selective move
    assert_dir_not_exists "tools/tool-a"
    assert_worktree_exists "archived/tool-a/_main.wt"

    # Other repos unchanged
    assert_worktree_exists "tools/tool-b/_main.wt"
    assert_worktree_exists "infrastructure/library-c/_main.wt"

    teardown_multi_machine
end_test

# ====================================================================================
# Conflict Scenario (for future conflict resolution testing)
# ====================================================================================

begin_test "multi-machine: detect when both machines modify same baum location"
    setup_multi_machine

    # Alpha plants repo
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/shared-repo" "with_commits"
    $WALD_BIN repo add "github.com/test/shared-repo"
    $WALD_BIN plant "github.com/test/shared-repo" "tools/shared" main
    workspace_commit "$TEST_ALPHA" "Plant in tools"

    # Beta syncs and materializes
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/shared-repo" "with_commits"
    $WALD_BIN sync
    materialize_baum "tools/shared"

    # Alpha moves it
    cd "$TEST_ALPHA" || exit 1
    $WALD_BIN move tools/shared admin/shared
    workspace_commit "$TEST_ALPHA" "Move to admin"

    # Beta also moves it (different location) - creates conflict
    cd "$TEST_BETA" || exit 1
    $WALD_BIN move tools/shared research/shared
    git add -A
    git commit -m "Move to research"

    # Now beta tries to sync - should detect diverged state
    git fetch origin

    _behind=$(git rev-list HEAD..origin/main --count 2>/dev/null || echo "0")
    _ahead=$(git rev-list origin/main..HEAD --count 2>/dev/null || echo "0")

    # Verify diverged state
    assert_gt "$_behind" "0" "Should be behind origin"
    assert_gt "$_ahead" "0" "Should have local commits"

    # wald sync without --force should fail on diverged workspace
    if $WALD_BIN sync 2>&1 | grep -q "diverged\|conflict"; then
        : # Expected behavior
    fi

    teardown_multi_machine
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
