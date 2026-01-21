#!/usr/bin/env bash
# Bump version in VERSION and Cargo.toml, build, test, and commit
#
# Usage:
#   ./bump-version.sh 0.2.0           # Bump to specific version
#   ./bump-version.sh --dry-run 0.2.0 # Show what would change without modifying
#
# This script:
# 1. Updates VERSION file and Cargo.toml
# 2. Runs full build (cargo build --release)
# 3. Runs test suite (cargo test)
# 4. If any step fails: restores all files, exits 1
# 5. If all pass: creates git commit with version bump

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DRY_RUN=false

# Detect sed variant (gsed on macOS via Homebrew, sed on Linux)
if command -v gsed &>/dev/null; then
  SED=gsed
elif sed --version 2>&1 | grep -q GNU; then
  SED=sed
else
  echo "Error: GNU sed required (install via 'brew install gnu-sed' on macOS)" >&2
  exit 1
fi

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run|-n)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      head -15 "$0" | tail -10
      exit 0
      ;;
    *)
      NEW_VERSION="$1"
      shift
      ;;
  esac
done

if [[ -z "${NEW_VERSION:-}" ]]; then
  echo "Usage: $0 [--dry-run] <version>" >&2
  echo "Example: $0 0.2.0" >&2
  exit 1
fi

# Validate version format (semver)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.]+)?$ ]]; then
  echo "Error: Invalid version format: $NEW_VERSION" >&2
  echo "Expected semver format: X.Y.Z or X.Y.Z-suffix" >&2
  exit 1
fi

OLD_VERSION=$(cat "$REPO_ROOT/VERSION" | tr -d '\n')

echo "Bumping version: $OLD_VERSION -> $NEW_VERSION"
echo ""

# Track modified files for rollback
MODIFIED_FILES=()

rollback() {
  echo ""
  echo "Rolling back changes..."
  for file in "${MODIFIED_FILES[@]}"; do
    git -C "$REPO_ROOT" checkout -- "$file" 2>/dev/null || true
    echo "  Restored: $file"
  done
  echo ""
  echo "Version bump failed. All changes have been rolled back."
  exit 1
}

trap 'rollback' ERR

# ============================================================================
# Update version in all files
# ============================================================================

echo "Updating version files..."

if [[ "$DRY_RUN" == "true" ]]; then
  echo "  Would update: VERSION"
  echo "  Would update: Cargo.toml"
  echo ""
  echo "Dry run complete. No files were modified."
  exit 0
fi

# Update VERSION file
echo "$NEW_VERSION" > "$REPO_ROOT/VERSION"
MODIFIED_FILES+=("VERSION")
echo "  Updated: VERSION"

# Update Cargo.toml
$SED -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" "$REPO_ROOT/Cargo.toml"
MODIFIED_FILES+=("Cargo.toml")
echo "  Updated: Cargo.toml"

echo ""

# ============================================================================
# Build
# ============================================================================

echo "Building..."
echo ""

if ! (cd "$REPO_ROOT" && cargo build --release); then
  echo ""
  echo "Build failed!"
  rollback
fi

echo ""

# ============================================================================
# Run tests
# ============================================================================

echo "Running tests..."
echo ""

if ! (cd "$REPO_ROOT" && cargo test); then
  echo ""
  echo "Tests failed!"
  rollback
fi

echo ""

# ============================================================================
# Commit changes
# ============================================================================

echo "All checks passed. Creating commit..."

# Stage modified files (including Cargo.lock which cargo updates)
git -C "$REPO_ROOT" add VERSION Cargo.toml Cargo.lock

# Create commit
git -C "$REPO_ROOT" commit -m "$(cat <<EOF
Bump version to $NEW_VERSION

Updated version in:
  - VERSION
  - Cargo.toml
EOF
)"

echo ""
echo "Version bumped to $NEW_VERSION successfully!"
echo ""
echo "Next steps:"
echo "  1. Review the commit: git log -1"
echo "  2. Push when ready: git push"
echo "  3. Create release: ./release.sh $NEW_VERSION"
