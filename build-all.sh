#!/usr/bin/env bash
# Build wald for all platforms
#
# Usage:
#   ./build-all.sh                    # Current platform only
#   ./build-all.sh --platform all     # All 4 platforms
#   ./build-all.sh --platform native  # Current platform only (explicit)
#   ./build-all.sh --checksums        # Generate checksums after build
#
# Platforms: native (default), darwin-arm64, darwin-amd64, linux-amd64, linux-arm64, all
#
# Output structure:
#   build/wald-<platform>-<version>
#   build/checksums-<version>.txt

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION=""  # Will default to VERSION file if not overridden

# Detect current platform
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)  PLATFORM="darwin-arm64" ;;
  Darwin-x86_64) PLATFORM="darwin-amd64" ;;
  Linux-x86_64)  PLATFORM="linux-amd64" ;;
  Linux-aarch64) PLATFORM="linux-arm64" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

# Defaults
PLATFORMS="native"
GENERATE_CHECKSUMS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --platform|--platforms)
      [[ $# -lt 2 || "$2" == --* ]] && { echo "Error: --platform requires a value" >&2; exit 1; }
      PLATFORMS="$2"
      shift 2
      ;;
    --version)
      [[ $# -lt 2 || "$2" == --* ]] && { echo "Error: --version requires a value" >&2; exit 1; }
      VERSION="$2"
      shift 2
      ;;
    --checksums)
      GENERATE_CHECKSUMS=true
      shift
      ;;
    -h|--help)
      head -14 "$0" | tail -11
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Default version from VERSION file if not specified
if [[ -z "$VERSION" ]]; then
  VERSION=$(cat "$REPO_ROOT/VERSION" | tr -d '\n')
fi

# ============================================================================
# Platform-specific build functions
# ============================================================================

build_darwin_arm64() {
  if [[ "$PLATFORM" == "darwin-arm64" ]]; then
    echo "Building for darwin-arm64 (native)..."
    mkdir -p "$REPO_ROOT/build"
    (cd "$REPO_ROOT" && cargo build --release --quiet)
    cp "$REPO_ROOT/target/release/wald" "$REPO_ROOT/build/wald-darwin-arm64-$VERSION"
    echo "  -> wald-darwin-arm64-$VERSION"
  else
    echo "Warning: darwin-arm64 native build only available on ARM Mac" >&2
  fi
}

build_darwin_amd64() {
  echo "Cross-compiling for darwin-amd64..."
  rustup target add x86_64-apple-darwin 2>/dev/null || true
  mkdir -p "$REPO_ROOT/build"
  (cd "$REPO_ROOT" && cargo build --release --quiet --target x86_64-apple-darwin)
  cp "$REPO_ROOT/target/x86_64-apple-darwin/release/wald" "$REPO_ROOT/build/wald-darwin-amd64-$VERSION"
  echo "  -> wald-darwin-amd64-$VERSION"
}

build_linux_amd64() {
  echo "Building for linux-amd64 via Apple Containers..."

  if ! command -v container &>/dev/null; then
    echo "Error: 'container' CLI not found. Install Apple Containers." >&2
    return 1
  fi

  if ! container image list | grep -q "wald-builder.*amd64"; then
    echo "Error: wald-builder:amd64 image not found. Run ./build-container.sh first." >&2
    return 1
  fi

  container run --rm \
    --arch amd64 \
    --mount type=bind,source="$REPO_ROOT",target=/src \
    wald-builder:amd64 \
    /src/build-linux.sh linux-amd64 "$VERSION"
}

build_linux_arm64() {
  echo "Building for linux-arm64 via Apple Containers..."

  if ! command -v container &>/dev/null; then
    echo "Error: 'container' CLI not found. Install Apple Containers." >&2
    return 1
  fi

  if ! container image list | grep -q "wald-builder.*arm64"; then
    echo "Error: wald-builder:arm64 image not found. Run ./build-container.sh first." >&2
    return 1
  fi

  container run --rm \
    --arch arm64 \
    --mount type=bind,source="$REPO_ROOT",target=/src \
    wald-builder:arm64 \
    /src/build-linux.sh linux-arm64 "$VERSION"
}

# ============================================================================
# Resolve platforms to build
# ============================================================================

resolve_platforms() {
  case "$PLATFORMS" in
    native)
      echo "$PLATFORM"
      ;;
    all)
      echo "darwin-arm64 darwin-amd64 linux-amd64 linux-arm64"
      ;;
    *)
      echo "$PLATFORMS" | tr ',' ' '
      ;;
  esac
}

PLATFORM_LIST=$(resolve_platforms)

echo "Building wald v$VERSION"
echo "Platforms: $PLATFORM_LIST"
echo ""

# ============================================================================
# Build for each platform
# ============================================================================

for plat in $PLATFORM_LIST; do
  case "$plat" in
    darwin-arm64)
      build_darwin_arm64
      ;;
    darwin-amd64)
      build_darwin_amd64
      ;;
    linux-amd64)
      build_linux_amd64
      ;;
    linux-arm64)
      build_linux_arm64
      ;;
    *)
      echo "Unknown platform: $plat" >&2
      ;;
  esac
  echo ""
done

# ============================================================================
# Generate checksums
# ============================================================================

if [[ "$GENERATE_CHECKSUMS" == "true" ]]; then
  echo "Generating checksums..."
  CHECKSUM_FILE="$REPO_ROOT/build/checksums-$VERSION.txt"

  # Find all built artifacts for this version and generate checksums
  (cd "$REPO_ROOT/build" && find . -type f -name "wald-*-$VERSION" | sort | while read -r f; do
    shasum -a 256 "$f"
  done) > "$CHECKSUM_FILE"

  echo "  Written to: $CHECKSUM_FILE"
  echo ""
fi

# ============================================================================
# Summary
# ============================================================================

echo "Build complete. Artifacts:"
echo ""

# List all built files with sizes (for this version)
find "$REPO_ROOT/build" -type f -name "*-$VERSION*" -exec ls -lh {} \; 2>/dev/null | while read -r line; do
  echo "  $line"
done
