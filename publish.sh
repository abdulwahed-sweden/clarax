#!/bin/bash
# Author: Abdulwahed Mansour
# Publishes all PyForge crates to crates.io in dependency order.
#
# Prerequisites:
#   1. cargo login (with a valid crates.io API token)
#   2. All tests passing: cargo test --workspace
#
# Usage:
#   ./publish.sh          # publish all crates
#   ./publish.sh --dry    # dry-run (no actual publish)

set -e

DRY=""
if [ "$1" = "--dry" ]; then
    DRY="--dry-run"
    echo "=== DRY RUN — no actual publishing ==="
fi

DELAY=30  # seconds between publishes (crates.io index update lag)

publish_crate() {
    local crate=$1
    echo ""
    echo "============================================"
    echo "Publishing: $crate"
    echo "============================================"
    cargo publish -p "$crate" $DRY
    if [ -z "$DRY" ]; then
        echo "Waiting ${DELAY}s for crates.io index to update..."
        sleep $DELAY
    fi
}

echo "PyForge v0.1.0 — Publishing to crates.io"
echo ""
echo "Verifying workspace builds..."
cargo check --workspace
echo "Build OK."
echo ""

# Publish in strict dependency order
publish_crate "pyforge-build-config"
publish_crate "pyforge-ffi"
publish_crate "pyforge-macros-backend"
publish_crate "pyforge-macros"
publish_crate "pyforge"
publish_crate "pyforge-django"

echo ""
echo "============================================"
echo "All crates published successfully."
echo "============================================"
