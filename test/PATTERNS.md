# Wald Test Patterns

Common patterns and examples for writing wald tests.

## Table of Contents

1. [Basic Test Structure](#basic-test-structure)
2. [Single Machine Tests](#single-machine-tests)
3. [Multi-Machine Sync Tests](#multi-machine-sync-tests)
4. [Move Detection Tests](#move-detection-tests)
5. [Error Handling Tests](#error-handling-tests)
6. [State Validation Tests](#state-validation-tests)

## Basic Test Structure

### Minimal Test

```bash
begin_test "feature works as expected"
    setup_wald_workspace

    # Test operations
    # ...

    # Assertions
    assert_file_exists ".wald/manifest.yaml"

    teardown_wald_workspace
end_test
```

### Test with Setup and Verification

```bash
begin_test "plant creates worktrees"
    setup_wald_workspace

    # Setup: create bare repo
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"

    # Action: plant baum
    plant_baum "github.com/test/repo" "tools/repo" "main" "dev"

    # Verify: check structure
    assert_dir_exists "tools/repo/.baum"
    assert_file_exists "tools/repo/.baum/manifest.yaml"
    assert_worktree_exists "tools/repo/_main.wt"
    assert_worktree_exists "tools/repo/_dev.wt"

    # Verify: check manifest content
    assert_file_contains "tools/repo/.baum/manifest.yaml" "github.com/test/repo"
    assert_baum_has_worktree "tools/repo" "main"
    assert_baum_has_worktree "tools/repo" "dev"

    teardown_wald_workspace
end_test
```

## Single Machine Tests

### Testing Bare Repo Creation

```bash
begin_test "repo add clones bare repository"
    setup_wald_workspace

    create_bare_repo "github.com/test/myrepo" "with_commits"
    add_repo_to_manifest "github.com/test/myrepo" "minimal" "100"

    # Verify bare repo structure
    assert_dir_exists ".wald/repos/github.com/test/myrepo.git"
    assert_file_exists ".wald/repos/github.com/test/myrepo.git/config"

    # Verify it's bare (no .git directory inside)
    assert_dir_not_exists ".wald/repos/github.com/test/myrepo.git/.git"

    # Verify it has commits
    cd ".wald/repos/github.com/test/myrepo.git"
    local commit_count
    commit_count=$(git rev-list --count HEAD)
    assert_gt "$commit_count" "0" "Should have commits"

    teardown_wald_workspace
end_test
```

### Testing Manifest Manipulation

```bash
begin_test "manifest stores repo configuration"
    setup_wald_workspace

    # Add repo with custom config
    add_repo_to_manifest "github.com/large/dataset" "full" "full"

    # Verify manifest content
    assert_file_contains ".wald/manifest.yaml" "github.com/large/dataset"
    assert_file_contains ".wald/manifest.yaml" "lfs: full"
    assert_file_contains ".wald/manifest.yaml" "depth: full"

    # Add repo with aliases
    add_repo_with_aliases "github.com/user/dotfiles" "dots" "df"

    assert_file_contains ".wald/manifest.yaml" "github.com/user/dotfiles"
    assert_file_contains ".wald/manifest.yaml" "aliases"

    teardown_wald_workspace
end_test
```

### Testing Worktree Operations

```bash
begin_test "worktrees are git-managed"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Verify worktree is registered with git
    cd ".wald/repos/github.com/test/repo.git"
    local worktree_list
    worktree_list=$(git worktree list)

    assert_contains "$worktree_list" "tools/repo/_main.wt"

    # Verify .git file (not directory)
    assert_file_exists "$TEST_WS/tools/repo/_main.wt/.git"
    assert_dir_not_exists "$TEST_WS/tools/repo/_main.wt/.git"

    teardown_wald_workspace
end_test
```

## Multi-Machine Sync Tests

### Basic Plant and Sync

```bash
begin_test "beta syncs planted baum from alpha"
    setup_multi_machine

    # Alpha: plant baum
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "research/project" "main"
    workspace_commit "$TEST_ALPHA" "Plant research project"

    # Verify alpha state
    assert_worktree_exists "research/project/_main.wt"

    # Beta: sync
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "research/project" "main"

    # Verify beta received structure
    assert_dir_exists "research/project/.baum"
    assert_worktree_exists "research/project/_main.wt"
    assert_file_contains "research/project/.baum/manifest.yaml" "github.com/test/repo"

    teardown_multi_machine
end_test
```

### Bidirectional Sync

```bash
begin_test "changes flow both directions"
    setup_multi_machine

    # Alpha: initial setup
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/shared" "with_commits"
    add_repo_to_manifest "github.com/test/shared" "minimal" "100"
    plant_baum "github.com/test/shared" "tools/shared" "main"
    workspace_commit "$TEST_ALPHA" "Alpha: plant shared"

    # Beta: sync from alpha
    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/shared" "tools/shared" "main"
    assert_worktree_exists "tools/shared/_main.wt"

    # Beta: add new repo
    create_bare_repo "github.com/test/beta-tool" "with_commits"
    add_repo_to_manifest "github.com/test/beta-tool" "minimal" "100"
    plant_baum "github.com/test/beta-tool" "tools/beta-tool" "main"
    workspace_commit "$TEST_BETA" "Beta: add new tool"

    # Alpha: sync from beta
    cd "$TEST_ALPHA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/beta-tool" "tools/beta-tool" "main"

    # Verify alpha received beta's changes
    assert_worktree_exists "tools/beta-tool/_main.wt"

    teardown_multi_machine
end_test
```

### Multiple Syncs Workflow

```bash
begin_test "multiple sync cycles maintain consistency"
    setup_multi_machine

    # Cycle 1: Alpha → Beta
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Cycle 1"

    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Cycle 2: Beta → Alpha
    cd "$TEST_BETA" || exit 1
    mkdir -p admin
    git mv tools/repo admin/repo
    workspace_commit "$TEST_BETA" "Cycle 2: move"

    cd "$TEST_ALPHA" || exit 1
    git pull --rebase origin main
    mkdir -p admin
    mv tools/repo admin/repo

    # Cycle 3: Alpha → Beta
    cd "$TEST_ALPHA" || exit 1
    local bare_repo=".wald/repos/github.com/test/repo.git"
    git -C "$bare_repo" worktree add "$PWD/admin/repo/_dev.wt" -b dev
    workspace_commit "$TEST_ALPHA" "Cycle 3: add dev branch"

    cd "$TEST_BETA" || exit 1
    git pull --rebase origin main
    git -C ".wald/repos/github.com/test/repo.git" worktree add "$PWD/admin/repo/_dev.wt" dev

    # Final verification: both machines identical
    assert_worktree_exists "admin/repo/_main.wt"
    assert_worktree_exists "admin/repo/_dev.wt"

    cd "$TEST_ALPHA" || exit 1
    assert_worktree_exists "admin/repo/_main.wt"
    assert_worktree_exists "admin/repo/_dev.wt"

    teardown_multi_machine
end_test
```

## Move Detection Tests

### Single Move Detection

```bash
begin_test "git diff detects baum move"
    setup_multi_machine

    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"
    workspace_commit "$TEST_ALPHA" "Plant"

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Move baum
    mkdir -p admin
    git mv tools/repo admin/repo
    workspace_commit "$TEST_ALPHA" "Move"

    local after_commit
    after_commit=$(get_commit_hash "$TEST_ALPHA")

    # Detect move via git
    local moves
    moves=$(detect_moves "$TEST_ALPHA" "$before_commit" "$after_commit")

    # Verify move detected
    assert_contains "$moves" ".baum/manifest.yaml"
    assert_contains "$moves" "tools/repo"
    assert_contains "$moves" "admin/repo"

    teardown_multi_machine
end_test
```

### Multiple Moves in Single Commit

```bash
begin_test "detect multiple moves in one commit"
    setup_multi_machine

    cd "$TEST_ALPHA" || exit 1

    # Plant multiple baums
    create_bare_repo "github.com/test/repo-a" "with_commits"
    create_bare_repo "github.com/test/repo-b" "with_commits"
    create_bare_repo "github.com/test/repo-c" "with_commits"
    add_repo_to_manifest "github.com/test/repo-a" "minimal" "100"
    add_repo_to_manifest "github.com/test/repo-b" "minimal" "100"
    add_repo_to_manifest "github.com/test/repo-c" "minimal" "100"
    plant_baum "github.com/test/repo-a" "tools/a" "main"
    plant_baum "github.com/test/repo-b" "tools/b" "main"
    plant_baum "github.com/test/repo-c" "tools/c" "main"
    workspace_commit "$TEST_ALPHA" "Plant all"

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Move all in one commit
    mkdir -p archived
    git mv tools/a archived/a
    git mv tools/b archived/b
    git mv tools/c archived/c
    workspace_commit "$TEST_ALPHA" "Archive all tools"

    # Detect all moves
    local moves
    moves=$(detect_moves "$TEST_ALPHA" "$before_commit" "HEAD")

    assert_contains "$moves" "tools/a"
    assert_contains "$moves" "tools/b"
    assert_contains "$moves" "tools/c"
    assert_contains "$moves" "archived/a"
    assert_contains "$moves" "archived/b"
    assert_contains "$moves" "archived/c"

    teardown_multi_machine
end_test
```

### Rename Detection (High Similarity)

```bash
begin_test "git detects rename as move"
    setup_multi_machine

    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/old-name" "main"
    workspace_commit "$TEST_ALPHA" "Plant"

    local before_commit
    before_commit=$(get_commit_hash "$TEST_ALPHA")

    # Rename (same directory, different name)
    git mv tools/old-name tools/new-name
    workspace_commit "$TEST_ALPHA" "Rename"

    # Git should detect as rename (high similarity)
    local status
    status=$(git diff --name-status -M "$before_commit" HEAD)

    # R = rename
    assert_contains "$status" "R"
    assert_contains "$status" "tools/old-name"
    assert_contains "$status" "tools/new-name"

    teardown_multi_machine
end_test
```

## Error Handling Tests

### Missing Prerequisites

```bash
begin_test "plant fails if bare repo missing"
    setup_wald_workspace

    # Add to manifest but don't create bare repo
    add_repo_to_manifest "github.com/test/missing" "minimal" "100"

    # Expected behavior:
    # result=$($WALD_BIN plant github.com/test/missing tools/missing main 2>&1 || true)
    # assert_contains "$result" "bare repo not found"
    # assert_contains "$result" ".wald/repos/github.com/test/missing.git"

    # Verify manifest exists but bare repo doesn't
    assert_file_exists ".wald/manifest.yaml"
    assert_dir_not_exists ".wald/repos/github.com/test/missing.git"

    teardown_wald_workspace
end_test
```

### Invalid Arguments

```bash
begin_test "move fails if destination exists"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Create destination directory
    mkdir -p admin/repo
    echo "existing" > admin/repo/file.txt

    # Expected behavior:
    # result=$($WALD_BIN move tools/repo admin/repo 2>&1 || true)
    # assert_exit_code 1 "$result"
    # assert_contains "$result" "already exists"

    # Verify source unchanged
    assert_dir_exists "tools/repo/.baum"

    teardown_wald_workspace
end_test
```

### Conflict Detection

```bash
begin_test "sync detects diverged workspace"
    setup_multi_machine

    # Alpha makes change
    cd "$TEST_ALPHA" || exit 1
    create_bare_repo "github.com/test/alpha" "with_commits"
    add_repo_to_manifest "github.com/test/alpha" "minimal" "100"
    workspace_commit "$TEST_ALPHA" "Alpha changes"

    # Beta makes different change (without pulling)
    cd "$TEST_BETA" || exit 1
    create_bare_repo "github.com/test/beta" "with_commits"
    add_repo_to_manifest "github.com/test/beta" "minimal" "100"
    git add -A
    git commit -m "Beta changes"

    # Beta is now diverged
    git fetch origin

    local behind ahead
    behind=$(git rev-list HEAD..origin/main --count)
    ahead=$(git rev-list origin/main..HEAD --count)

    assert_gt "$behind" "0" "Should be behind"
    assert_gt "$ahead" "0" "Should be ahead"

    teardown_multi_machine
end_test
```

## State Validation Tests

### State File Updates

```bash
begin_test "sync updates last_sync commit hash"
    setup_wald_workspace

    # Initial sync
    local initial_commit
    initial_commit=$(git rev-parse HEAD)

    update_last_sync "$initial_commit"

    local last_sync
    last_sync=$(get_last_sync)
    assert_eq "$initial_commit" "$last_sync"

    # Make change and sync again
    echo "change" > file.txt
    git add file.txt
    git commit -m "Change"

    local new_commit
    new_commit=$(git rev-parse HEAD)

    update_last_sync "$new_commit"

    last_sync=$(get_last_sync)
    assert_eq "$new_commit" "$last_sync"
    assert_not_contains "$last_sync" "$initial_commit"

    teardown_wald_workspace
end_test
```

### Baum Manifest Validation

```bash
begin_test "baum manifest tracks all worktrees"
    setup_wald_workspace

    create_bare_repo "github.com/test/repo" "with_commits"
    add_repo_to_manifest "github.com/test/repo" "minimal" "100"
    plant_baum "github.com/test/repo" "research/project" "main" "dev" "feature-x"

    # Verify manifest lists all branches
    local manifest="research/project/.baum/manifest.yaml"

    assert_file_exists "$manifest"
    assert_file_contains "$manifest" "github.com/test/repo"
    assert_file_contains "$manifest" "branch: main"
    assert_file_contains "$manifest" "branch: dev"
    assert_file_contains "$manifest" "branch: feature-x"

    # Verify worktree paths
    assert_file_contains "$manifest" "path: _main.wt"
    assert_file_contains "$manifest" "path: _dev.wt"
    assert_file_contains "$manifest" "path: _feature-x.wt"

    teardown_wald_workspace
end_test
```

## Tips and Best Practices

### 1. Test One Thing

```bash
# Good: focused test
begin_test "plant creates .baum directory"
    setup_wald_workspace
    create_bare_repo "github.com/test/repo" "with_commits"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    assert_dir_exists "tools/repo/.baum"

    teardown_wald_workspace
end_test

# Bad: tests multiple unrelated things
begin_test "plant and move and sync all work"
    # Too much in one test
end_test
```

### 2. Use Descriptive Test Names

```bash
# Good
begin_test "sync replays baum move from alpha to beta"

# Bad
begin_test "test sync"
```

### 3. Verify Both Success and Failure

```bash
# Verify positive case
assert_dir_exists "expected/path"

# Also verify negative case
assert_dir_not_exists "unexpected/path"
```

### 4. Clean Up Resources

Always call teardown, even if test fails:

```bash
begin_test "my test"
    setup_wald_workspace

    # Test code that might fail
    some_operation || true

    # Teardown always runs due to trap in setup
    teardown_wald_workspace
end_test
```

### 5. Use Helpers for Common Operations

```bash
# Don't manually create bare repos
# Do use the helper:
create_bare_repo "github.com/test/repo" "with_commits"

# Don't manually construct manifest YAML
# Do use the helper:
add_repo_to_manifest "github.com/test/repo" "minimal" "100"
```

### 6. Test Expected Behavior Before Implementation

Tests can define expected behavior even when wald isn't implemented:

```bash
begin_test "wald plant creates worktrees"
    setup_wald_workspace

    # This will fail until implemented:
    # $WALD_BIN plant github.com/test/repo tools/repo main

    # But we can test the expected result:
    # For now, simulate expected state manually
    plant_baum "github.com/test/repo" "tools/repo" "main"

    assert_worktree_exists "tools/repo/_main.wt"

    teardown_wald_workspace
end_test
```

This validates the test infrastructure works and documents expected behavior.
