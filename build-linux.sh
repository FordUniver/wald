#!/usr/bin/env bash
# Build wald binary inside Linux container
# Called by build-all.sh via: container run ... /src/build-linux.sh <platform>

set -euo pipefail

VERSION=$(cat /src/VERSION)
PLATFORM="${1:-linux-amd64}"

echo "=== Building wald for $PLATFORM ==="
echo ""

echo "Building Rust..."
mkdir -p /src/build
(cd /src && cargo build --release --quiet)
cp /src/target/release/wald "/src/build/wald-$PLATFORM-$VERSION"
echo "  -> wald-$PLATFORM-$VERSION"

echo ""
echo "=== Linux build complete for $PLATFORM ==="
