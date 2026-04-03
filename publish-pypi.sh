#!/bin/bash
# Author: Abdulwahed Mansour
# Builds and publishes pyforge-django to PyPI.
#
# Prerequisites:
#   1. python3 -m venv .venv && source .venv/bin/activate
#   2. pip install maturin twine
#   3. A PyPI API token (set TWINE_PASSWORD or use keyring)
#
# Usage:
#   ./publish-pypi.sh          # build and publish
#   ./publish-pypi.sh --build  # build only (no publish)

set -e

BUILD_ONLY=""
if [ "$1" = "--build" ]; then
    BUILD_ONLY=1
    echo "=== BUILD ONLY — no publishing ==="
fi

echo "PyForge Django v0.1.0 — Building wheel"
echo ""

cd pyforge-django

# Build the wheel with maturin
echo "Building release wheel..."
maturin build --release --strip

echo ""
echo "Built wheels:"
ls -la ../target/wheels/pyforge_django-*.whl 2>/dev/null || echo "No wheels found in target/wheels/"

if [ -z "$BUILD_ONLY" ]; then
    echo ""
    echo "Publishing to PyPI..."
    twine upload ../target/wheels/pyforge_django-*.whl
    echo ""
    echo "Published to PyPI: pip install pyforge-django"
fi
