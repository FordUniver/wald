#!/usr/bin/env bash
# Clean build artifacts
#
# Usage:
#   ./clean.sh        # Remove build/ directory
#   ./clean.sh --all  # Also run cargo clean

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Cleaning build artifacts..."

# Remove build directory
if [[ -d "$REPO_ROOT/build" ]]; then
  rm -rf "$REPO_ROOT/build"
  echo "  Removed build/"
else
  echo "  build/ not found (already clean)"
fi

if [[ "${1:-}" == "--all" ]]; then
  echo "Running cargo clean..."
  (cd "$REPO_ROOT" && cargo clean)
  echo "  Removed target/"
fi

echo "Done."
