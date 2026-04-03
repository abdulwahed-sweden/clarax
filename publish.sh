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
#   ./publish.sh --dry    # verify packaging locally

set -e

DRY=""
if [ "$1" = "--dry" ]; then
    DRY="1"
    echo "=== DRY RUN ==="
    echo ""
fi

DELAY=45

# Crate name → directory mapping
declare -a CRATE_NAMES=("pyforge-build-config" "pyforge-ffi" "pyforge-macros-backend" "pyforge-macros" "pyforge" "pyforge-django")
declare -a CRATE_DIRS=("pyo3-build-config" "pyo3-ffi" "pyo3-macros-backend" "pyo3-macros" "." "pyforge-django")

echo "PyForge v0.1.0 — crates.io publish"
echo ""
echo "Checking workspace..."
cargo check --workspace
echo "OK"
echo ""

if [ -n "$DRY" ]; then
    echo "Verifying first crate packages correctly:"
    cargo package -p pyforge-build-config
    echo ""
    echo "Checking all crate metadata:"
    echo ""
    for i in "${!CRATE_NAMES[@]}"; do
        name="${CRATE_NAMES[$i]}"
        dir="${CRATE_DIRS[$i]}"
        toml="$dir/Cargo.toml"
        printf "  %-28s" "$name"
        has_name=$(grep -c "^name = \"$name\"" "$toml" 2>/dev/null || echo 0)
        has_ver=$(grep -c '^version = "0.1.0"' "$toml" 2>/dev/null || echo 0)
        has_lic=$(grep -c '^license = "MIT"' "$toml" 2>/dev/null || echo 0)
        has_desc=$(grep -c '^description' "$toml" 2>/dev/null || echo 0)
        has_repo=$(grep -c '^repository' "$toml" 2>/dev/null || echo 0)
        has_auth=$(grep -c '^authors' "$toml" 2>/dev/null || echo 0)
        if [ "$has_name" -gt 0 ] && [ "$has_ver" -gt 0 ] && [ "$has_lic" -gt 0 ] && [ "$has_desc" -gt 0 ] && [ "$has_repo" -gt 0 ] && [ "$has_auth" -gt 0 ]; then
            echo "OK"
        else
            echo "INCOMPLETE (name=$has_name ver=$has_ver lic=$has_lic desc=$has_desc repo=$has_repo auth=$has_auth)"
        fi
    done
    echo ""
    echo "Dry run complete. Run ./publish.sh to publish for real."
    exit 0
fi

for i in "${!CRATE_NAMES[@]}"; do
    name="${CRATE_NAMES[$i]}"
    echo ""
    echo "=== Publishing: $name ==="
    cargo publish -p "$name"
    if [ $i -lt $(( ${#CRATE_NAMES[@]} - 1 )) ]; then
        echo "Waiting ${DELAY}s for crates.io index..."
        sleep $DELAY
    fi
done

echo ""
echo "All 6 crates published."
echo ""
for name in "${CRATE_NAMES[@]}"; do
    echo "  https://crates.io/crates/$name"
done
