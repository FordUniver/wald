#!/usr/bin/env bash
# Create test repository bundles
# These bundles can be unpacked in tests for realistic git scenarios

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR"

# ====================================================================================
# Simple Repository
# ====================================================================================

create_simple_repo() {
    local temp_dir
    temp_dir=$(mktemp -d /tmp/wald-fixture.XXXXXX)
    cd "$temp_dir" || exit 1

    git init --quiet
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initial commit
    echo "# Simple Test Repo" > README.md
    git add README.md
    git commit --quiet -m "Initial commit"

    # Add some content
    echo "This is a test repository" >> README.md
    git add README.md
    git commit --quiet -m "Add description"

    # Create a file
    echo "println!(\"Hello, world!\");" > main.rs
    git add main.rs
    git commit --quiet -m "Add main.rs"

    # Bundle
    git bundle create "$OUTPUT_DIR/simple-repo.bundle" --all

    cd - >/dev/null || exit 1
    rm -rf "$temp_dir"

    echo "Created: simple-repo.bundle"
}

# ====================================================================================
# Multi-Branch Repository
# ====================================================================================

create_multi_branch_repo() {
    local temp_dir
    temp_dir=$(mktemp -d /tmp/wald-fixture.XXXXXX)
    cd "$temp_dir" || exit 1

    git init --quiet
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Main branch commits
    echo "# Multi-Branch Repo" > README.md
    git add README.md
    git commit --quiet -m "Initial commit"

    echo "Main branch content" > main.txt
    git add main.txt
    git commit --quiet -m "Add main.txt"

    # Create dev branch
    git checkout -b dev --quiet
    echo "Development features" > dev.txt
    git add dev.txt
    git commit --quiet -m "Start dev branch"

    echo "More dev work" >> dev.txt
    git add dev.txt
    git commit --quiet -m "Continue dev work"

    # Create feature branch from dev
    git checkout -b feature-x --quiet
    echo "Feature X implementation" > feature-x.txt
    git add feature-x.txt
    git commit --quiet -m "Implement feature X"

    # Return to main
    git checkout main --quiet

    # Bundle all branches
    git bundle create "$OUTPUT_DIR/multi-branch-repo.bundle" --all

    cd - >/dev/null || exit 1
    rm -rf "$temp_dir"

    echo "Created: multi-branch-repo.bundle"
}

# ====================================================================================
# Repository with History
# ====================================================================================

create_deep_history_repo() {
    local temp_dir
    temp_dir=$(mktemp -d /tmp/wald-fixture.XXXXXX)
    cd "$temp_dir" || exit 1

    git init --quiet
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initial commit
    echo "# Deep History Repo" > README.md
    git add README.md
    git commit --quiet -m "Initial commit"

    # Create 100 commits
    for i in {1..100}; do
        echo "Commit $i" >> log.txt
        git add log.txt
        git commit --quiet -m "Commit $i"
    done

    # Bundle
    git bundle create "$OUTPUT_DIR/deep-history-repo.bundle" --all

    cd - >/dev/null || exit 1
    rm -rf "$temp_dir"

    echo "Created: deep-history-repo.bundle (100 commits)"
}

# ====================================================================================
# Repository with Large Files (simulated LFS)
# ====================================================================================

create_lfs_repo() {
    local temp_dir
    temp_dir=$(mktemp -d /tmp/wald-fixture.XXXXXX)
    cd "$temp_dir" || exit 1

    git init --quiet
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Initialize git-lfs (if available)
    if command -v git-lfs >/dev/null 2>&1; then
        git lfs install --local --quiet 2>/dev/null || true
        git lfs track "*.bin" >/dev/null 2>&1 || true
        git add .gitattributes 2>/dev/null || true
    fi

    # Initial commit
    echo "# LFS Test Repo" > README.md
    git add README.md
    git commit --quiet -m "Initial commit"

    # Add "large" file (actually small but marked as LFS)
    dd if=/dev/urandom of=data.bin bs=1024 count=10 2>/dev/null
    git add data.bin
    git commit --quiet -m "Add large file"

    # Add another large file
    dd if=/dev/urandom of=model.bin bs=1024 count=20 2>/dev/null
    git add model.bin
    git commit --quiet -m "Add model file"

    # Bundle
    git bundle create "$OUTPUT_DIR/lfs-repo.bundle" --all

    cd - >/dev/null || exit 1
    rm -rf "$temp_dir"

    echo "Created: lfs-repo.bundle"
}

# ====================================================================================
# Fork Scenario (origin + fork repos)
# ====================================================================================

create_fork_repos() {
    local temp_dir
    temp_dir=$(mktemp -d /tmp/wald-fixture.XXXXXX)
    cd "$temp_dir" || exit 1

    # Create origin repo
    mkdir origin
    cd origin || exit 1

    git init --quiet
    git config user.name "Original Author"
    git config user.email "author@example.com"

    echo "# Original Project" > README.md
    git add README.md
    git commit --quiet -m "Initial commit"

    echo "Original content" > file.txt
    git add file.txt
    git commit --quiet -m "Add file"

    # Bundle origin
    git bundle create "$OUTPUT_DIR/fork-origin.bundle" --all

    # Create fork (clone and modify)
    cd "$temp_dir" || exit 1
    git clone --quiet origin fork
    cd fork || exit 1

    git config user.name "Fork Maintainer"
    git config user.email "fork@example.com"

    # Add fork-specific changes
    echo "Fork improvements" >> file.txt
    git add file.txt
    git commit --quiet -m "Fork: improve file"

    echo "Fork feature" > fork-feature.txt
    git add fork-feature.txt
    git commit --quiet -m "Fork: add feature"

    # Bundle fork
    git bundle create "$OUTPUT_DIR/fork-fork.bundle" --all

    cd - >/dev/null || exit 1
    rm -rf "$temp_dir"

    echo "Created: fork-origin.bundle and fork-fork.bundle"
}

# ====================================================================================
# Main
# ====================================================================================

main() {
    echo "Creating test repository bundles..."
    echo ""

    create_simple_repo
    create_multi_branch_repo
    create_deep_history_repo
    create_lfs_repo
    create_fork_repos

    echo ""
    echo "All bundles created in: $OUTPUT_DIR"
    echo ""
    echo "To use in tests:"
    echo "  git clone <bundle-file> <target-dir>"
    echo "  or"
    echo "  git clone --bare <bundle-file> <target-dir>.git"
}

# Only run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
