# Wald Testing Infrastructure

Comprehensive test suite for validating wald's design and implementation.

## Quick Start

```bash
# Run all tests
make test

# Run only unit tests (Rust)
make test-unit

# Run only integration tests (Shell)
make test-integration

# Run with verbose output
make test-verbose

# Run specific test file
cd test && ./run_tests.sh "" cases/test_plant.sh

# Run integration tests with built binary
cargo build --release
cd test && ./run_tests.sh ../target/release/wald
```

## Test Structure

```
test/
├── run_tests.sh              # Test orchestrator (TAP output)
├── lib/
│   ├── assertions.sh         # Test assertion library
│   ├── setup.sh              # Environment setup/teardown
│   └── helpers.sh            # Wald-specific helpers
├── fixtures/
│   ├── *.yaml                # Sample manifests
│   └── sample-repos/         # Test repo bundles
├── cases/                    # Integration test files
│   ├── test_repo_add.sh
│   ├── test_plant.sh
│   ├── test_move.sh
│   ├── test_sync_basic.sh
│   └── test_sync_moves.sh
└── integration/
    └── test_multi_machine.sh # E2E multi-machine tests
```

## Test Layers

### 1. Unit Tests (Rust)

Located in `src/` alongside implementation code using `#[test]` attributes.

**Purpose:** Test internal logic in isolation
- Manifest parsing and validation
- Repo ID generation and normalization
- State machine transitions
- Path manipulation utilities
- Configuration hierarchy

**Run:** `cargo test --lib`

### 2. Integration Tests (Shell)

Located in `test/cases/`, one file per command or feature area.

**Purpose:** Test CLI behavior and git interactions
- Command execution
- Git operations (bare repos, worktrees)
- Manifest manipulation
- Error handling
- State file updates

**Run:** `cd test && ./run_tests.sh`

### 3. End-to-End Tests (Shell)

Located in `test/integration/`, multi-machine simulation tests.

**Purpose:** Test cross-machine sync workflows
- Plant → sync → verify
- Move → sync → replay
- Conflict detection
- Reconciliation

**Run:** `cd test && ./run_tests.sh "" integration/test_multi_machine.sh`

## Writing Tests

### Basic Test Structure

```bash
#!/usr/bin/env bash
# Test file: test/cases/test_feature.sh

# Source libraries (handled by run_tests.sh)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

begin_test "feature does expected thing"
    setup_wald_workspace

    # Test operations
    create_bare_repo "github.com/test/repo" "with_commits"
    plant_baum "github.com/test/repo" "tools/repo" "main"

    # Assertions
    assert_dir_exists "tools/repo/.baum"
    assert_worktree_exists "tools/repo/_main.wt"

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
```

### Available Assertions

**File/Directory:**
- `assert_file_exists <path>`
- `assert_file_not_exists <path>`
- `assert_dir_exists <path>`
- `assert_dir_not_exists <path>`
- `assert_file_contains <path> <text>`

**Content:**
- `assert_eq <expected> <actual> [message]`
- `assert_contains <haystack> <needle> [message]`
- `assert_not_contains <haystack> <needle> [message]`
- `assert_matches <pattern> <actual> [message]`

**Command execution:**
- `assert_exit_code <code> <command> [args...]`
- `assert_stdout_contains <needle> <command> [args...]`
- `assert_stderr_contains <needle> <command> [args...]`

**Numeric:**
- `assert_gt <actual> <threshold> [message]`

**Structured data:**
- `assert_json_valid <output>`
- `assert_json_field <output> <jq_path> <expected>`
- `assert_yaml_valid <output>`
- `assert_yaml_field <output> <yq_path> <expected>`

**Wald-specific:**
- `assert_worktree_exists <path>`
- `assert_baum_has_worktree <baum_path> <branch>`

### Environment Setup Functions

**Single machine:**
```bash
setup_wald_workspace    # Creates isolated test workspace
teardown_wald_workspace # Cleans up
```

Creates:
- `/tmp/wald-test.XXXXXX/` with initialized `.wald/` structure
- Git repo with test user config
- Sets `$TEST_WS` variable

**Multi-machine:**
```bash
setup_multi_machine     # Creates two workspaces with shared remote
teardown_multi_machine  # Cleans up
```

Creates:
- `/tmp/wald-multi.XXXXXX/remotes/workspace.git` (bare repo)
- `/tmp/wald-multi.XXXXXX/machine-alpha/` (first workspace)
- `/tmp/wald-multi.XXXXXX/machine-beta/` (second workspace)
- Sets `$TEST_ALPHA`, `$TEST_BETA`, `$TEST_REMOTES` variables

### Helper Functions

**Bare repo creation:**
```bash
create_bare_repo "github.com/test/repo" "with_commits"
```

**Manifest manipulation:**
```bash
add_repo_to_manifest "github.com/test/repo" "minimal" "100"
add_repo_with_aliases "github.com/user/dotfiles" "dots" "dotfiles"
add_repo_with_upstream "git.zib.de/fork" "git.zib.de/upstream"
```

**Baum operations:**
```bash
plant_baum "github.com/test/repo" "tools/repo" "main" "dev"
```

**Move detection:**
```bash
moves=$(detect_moves "$TEST_ALPHA" "$before_commit" "$after_commit")
```

**State manipulation:**
```bash
update_last_sync "$commit_hash"
last_sync=$(get_last_sync)
```

**Git operations:**
```bash
workspace_commit "$TEST_ALPHA" "Commit message"
workspace_pull "$TEST_BETA"
commit_hash=$(get_commit_hash "$TEST_ALPHA")
```

## Test Patterns

See [PATTERNS.md](PATTERNS.md) for common test patterns and examples.

## Multi-Machine Testing

Wald's key feature is cross-machine sync. We test this using **local bare repos** as fake remotes:

```
/tmp/wald-multi-$$/
├── remotes/
│   └── workspace.git/      # Bare repo = shared "origin"
├── machine-alpha/           # Workspace 1 (simulates Mac)
└── machine-beta/            # Workspace 2 (simulates Coder)
```

Both workspaces push/pull from `remotes/workspace.git`. Git is the synchronization mechanism - no Docker or network required.

**Example workflow:**
```bash
# Alpha makes change
cd "$TEST_ALPHA"
plant_baum "github.com/test/repo" "tools/repo" "main"
workspace_commit "$TEST_ALPHA" "Plant repo"

# Beta syncs
cd "$TEST_BETA"
git pull --rebase origin main
# Wald sync would recreate the baum here
```

The multi-machine setup simulates workspace sync between machines using separate directories.

## Fixtures

### Manifest Fixtures

Located in `test/fixtures/`:
- `manifest-valid.yaml` - Example valid manifest
- `manifest-invalid.yaml` - Malformed YAML
- `manifest-missing-repos.yaml` - Missing required keys
- `manifest-empty.yaml` - Minimal valid manifest

### Repository Bundles

Generated via `test/fixtures/sample-repos/create.sh`:
- `simple-repo.bundle` - Basic repo with a few commits
- `multi-branch-repo.bundle` - Repo with main, dev, feature-x branches
- `deep-history-repo.bundle` - 100+ commits
- `lfs-repo.bundle` - Repo with git-lfs tracked files
- `fork-origin.bundle` / `fork-fork.bundle` - Fork scenario

**Usage in tests:**
```bash
# Load bundle into multi-machine test
load_test_bundle "$SCRIPT_DIR/fixtures/sample-repos/simple-repo.bundle" "test-repo"
```

## TAP Output

Tests use TAP (Test Anything Protocol) format for machine-readable results:

```
ok 1 - wald plant creates baum with single worktree
ok 2 - wald plant creates baum with multiple worktrees
not ok 3 - wald plant fails if repo not in manifest
  # Assertion failed: directory should exist
  #   path: tools/repo/.baum
ok 4 - wald move relocates baum to new path

1..4
```

TAP format is compatible with many test runners and CI systems.

## Debugging Failed Tests

### Run specific test with debug output:
```bash
cd test
DEBUG=1 ./run_tests.sh "" cases/test_plant.sh
```

### Inspect test workspace:
```bash
# Disable cleanup to inspect state
begin_test "my test"
    setup_wald_workspace

    # Your test operations...

    # Debug helper
    debug_workspace

    # Comment out teardown to inspect manually
    # teardown_wald_workspace
end_test
```

### Common issues:

**Tests skip with "not implemented"**
- Normal during development - tests define expected behavior before implementation

**Permission denied errors**
- Check file permissions: `chmod +x test/run_tests.sh`
- Worktrees may have read-only files: cleanup uses `chmod -R u+w`

**Git user.name/user.email not set**
- Test setup configures this automatically
- If running git commands manually, set in test environment

**Bare repo not found**
- Ensure `create_bare_repo` called before `plant_baum`
- Check path: `.wald/repos/<host>/<owner>/<name>.git`

## CI/CD Integration

Tests run automatically on every push via GitLab CI:

```yaml
test:rust:
  script:
    - cargo test --lib
    - cd test && bash run_tests.sh
```

See [.gitlab-ci.yml](../.gitlab-ci.yml) for full configuration.

## Development Workflow

1. **Design phase (current):**
   - Write tests that define expected behavior
   - Tests skip with "not implemented"
   - Validates test infrastructure works

2. **Implementation phase:**
   - Implement wald commands
   - Tests turn green as features complete
   - Regression protection

3. **Maintenance phase:**
   - Add tests for bug fixes
   - Extend coverage for edge cases
   - Keep tests passing

## Performance

Integration tests are slower than unit tests (seconds vs milliseconds):
- Each test creates temp directories
- Git operations have overhead
- Multi-machine tests do multiple clones

**Optimization strategies:**
- Parallelize independent tests (future)
- Cache test repo bundles
- Use shallow clones where appropriate
- Keep unit tests fast, integration tests thorough

## Requirements

- Bash ≥3.2 (stock macOS compatible)
- Git ≥2.25 (worktree features)
- Git-LFS (optional, for LFS tests)
- yq (optional, for YAML manipulation - CI installs)
- Rust/Cargo (for building wald)

## Contributing

When adding features:
1. Write tests first (TDD)
2. Run `make test` before committing
3. Ensure CI passes on push
4. Update this README for new patterns

See [PATTERNS.md](PATTERNS.md) for examples of well-structured tests.
