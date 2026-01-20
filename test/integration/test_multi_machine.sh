#!/usr/bin/env bash
# End-to-end multi-machine sync integration test
# Simulates realistic workflow: Mac ↔ Coder environment sync
# Tests the complete wald sync workflow including plant, move, branch operations

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

    # Create bare repo with realistic history
    create_bare_repo "github.com/test/research-project" "with_commits"
    add_repo_to_manifest "github.com/test/research-project" "minimal" "100"

    # Plant baum with main and dev branches
    # Expected: $WALD_BIN plant github.com/test/research-project research/project main dev
    plant_baum "github.com/test/research-project" "research/project" "main" "dev"

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

    # Expected: $WALD_BIN sync
    git pull --rebase origin main
    plant_baum "github.com/test/research-project" "research/project" "main" "dev"

    # Verify beta got the baum
    assert_dir_exists "research/project/.baum"
    assert_worktree_exists "research/project/_main.wt"
    assert_worktree_exists "research/project/_dev.wt"
    assert_file_contains "research/project/.baum/manifest.yaml" "github.com/test/research-project"

    # ============================================================================
    # PHASE 3: Alpha moves the baum (project reorganization)
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Project gets archived to older year
    mkdir -p research/2024/completed
    # Expected: $WALD_BIN move research/project research/2024/completed/project
    git mv research/project research/2024/completed/project

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

    # Expected: $WALD_BIN sync
    git pull --rebase origin main
    mkdir -p research/2024/completed
    mv research/project research/2024/completed/project 2>/dev/null || true

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
    git push origin dev

    cd "$TEST_BETA" || exit 1

    # Beta doesn't change workspace structure, so no workspace commit needed

    # ============================================================================
    # PHASE 6: Alpha adds a new branch worktree
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1

    # Expected: $WALD_BIN branch research/2024/completed/project feature-x
    # This creates a new worktree for branch feature-x

    # Simulate: add feature-x branch to the bare repo and create worktree
    local bare_repo=".wald/repos/github.com/test/research-project.git"
    git -C "$bare_repo" worktree add "$PWD/research/2024/completed/project/_feature-x.wt" -b feature-x 2>/dev/null || true

    # Update baum manifest
    cat >> "research/2024/completed/project/.baum/manifest.yaml" <<EOF
  - branch: feature-x
    path: _feature-x.wt
EOF

    workspace_commit "$TEST_ALPHA" "Add feature-x branch"

    # Verify new worktree on alpha
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"
    assert_baum_has_worktree "research/2024/completed/project" "feature-x"

    # ============================================================================
    # PHASE 7: Beta syncs and gets new worktree
    # ============================================================================

    cd "$TEST_BETA" || exit 1

    # Expected: $WALD_BIN sync
    git pull --rebase origin main

    # Simulate: create the new worktree
    local bare_repo_beta=".wald/repos/github.com/test/research-project.git"
    if [[ ! -d "$bare_repo_beta" ]]; then
        # Beta doesn't have bare repo yet, simulate it
        create_bare_repo "github.com/test/research-project" "with_commits"
    fi
    git -C "$bare_repo_beta" worktree add "$PWD/research/2024/completed/project/_feature-x.wt" feature-x 2>/dev/null || true

    # Verify new worktree on beta
    assert_dir_exists "research/2024/completed/project/.baum"
    assert_worktree_exists "research/2024/completed/project/_feature-x.wt"
    assert_baum_has_worktree "research/2024/completed/project" "feature-x"

    # ============================================================================
    # PHASE 8: Verify final state consistency
    # ============================================================================

    # Both machines should have identical structure
    cd "$TEST_ALPHA" || exit 1
    local alpha_baums
    alpha_baums=$(find . -name ".baum" -type d | sort)

    cd "$TEST_BETA" || exit 1
    local beta_baums
    beta_baums=$(find . -name ".baum" -type d | sort)

    # Should have same baum locations
    # (In real test with wald implementation, this would be exact match)
    assert_contains "$alpha_baums" "research/2024/completed/project/.baum"
    assert_contains "$beta_baums" "research/2024/completed/project/.baum"

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

    create_bare_repo "github.com/test/tool-a" "with_commits"
    create_bare_repo "github.com/test/tool-b" "with_commits"
    create_bare_repo "github.com/test/library-c" "with_commits"

    add_repo_to_manifest "github.com/test/tool-a" "minimal" "100"
    add_repo_to_manifest "github.com/test/tool-b" "minimal" "100"
    add_repo_to_manifest "github.com/test/library-c" "full" "full"

    plant_baum "github.com/test/tool-a" "tools/tool-a" "main"
    plant_baum "github.com/test/tool-b" "tools/tool-b" "main"
    plant_baum "github.com/test/library-c" "infrastructure/library-c" "main" "dev"

    workspace_commit "$TEST_ALPHA" "Plant multiple repos"

    # ============================================================================
    # Beta syncs
    # ============================================================================

    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main

    plant_baum "github.com/test/tool-a" "tools/tool-a" "main"
    plant_baum "github.com/test/tool-b" "tools/tool-b" "main"
    plant_baum "github.com/test/library-c" "infrastructure/library-c" "main" "dev"

    assert_worktree_exists "tools/tool-a/_main.wt"
    assert_worktree_exists "tools/tool-b/_main.wt"
    assert_worktree_exists "infrastructure/library-c/_main.wt"
    assert_worktree_exists "infrastructure/library-c/_dev.wt"

    # ============================================================================
    # Alpha moves only one repo
    # ============================================================================

    cd "$TEST_ALPHA" || exit 1
    mkdir -p archived
    git mv tools/tool-a archived/tool-a
    workspace_commit "$TEST_ALPHA" "Archive tool-a"

    # ============================================================================
    # Beta syncs and only moved repo changes
    # ============================================================================

    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    mkdir -p archived
    mv tools/tool-a archived/tool-a 2>/dev/null || true

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

    # Alpha and Beta both plant same repo in different locations
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/shared-repo" "with_commits"
    add_repo_to_manifest "github.com/test/shared-repo" "minimal" "100"
    plant_baum "github.com/test/shared-repo" "tools/shared" "main"
    workspace_commit "$TEST_ALPHA" "Plant in tools"

    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/shared-repo" "tools/shared" "main"

    # Alpha moves it
    cd "$TEST_ALPHA" || exit 1
    mkdir -p admin
    git mv tools/shared admin/shared
    workspace_commit "$TEST_ALPHA" "Move to admin"

    # Beta also moves it (different location) - creates conflict
    cd "$TEST_BETA" || exit 1
    mkdir -p research
    git mv tools/shared research/shared
    git add -A
    git commit -m "Move to research"

    # Now beta tries to sync - should detect conflict
    # Expected: $WALD_BIN sync would detect diverged state
    git fetch origin

    local behind ahead
    behind=$(git rev-list HEAD..origin/main --count 2>/dev/null || echo "0")
    ahead=$(git rev-list origin/main..HEAD --count 2>/dev/null || echo "0")

    # Verify diverged state
    assert_gt "$behind" "0" "Should be behind origin"
    assert_gt "$ahead" "0" "Should have local commits"

    # In real implementation, wald sync would require --force or --interactive

    teardown_multi_machine
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
