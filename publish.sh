#!/bin/bash
# Author: Abdulwahed Mansour
# Publishes all PyForge crates to crates.io in dependency order.
#
# Usage:
#   ./publish.sh              # publish all crates
#   ./publish.sh --dry        # verify metadata locally
#   ./publish.sh --from NAME  # resume from a specific crate

set -e

DRY=""
START_FROM=""
if [ "$1" = "--dry" ]; then
    DRY="1"
elif [ "$1" = "--from" ] && [ -n "$2" ]; then
    START_FROM="$2"
fi

CRATE_NAMES=("pyforge-build-config" "pyforge-ffi" "pyforge-macros-backend" "pyforge-macros" "pyforge" "pyforge-django")
CRATE_DIRS=("pyo3-build-config" "pyo3-ffi" "pyo3-macros-backend" "pyo3-macros" "." "pyforge-django")

publish_crate() {
    local name=$1
    local attempt=1
    local max_attempts=3

    while [ $attempt -le $max_attempts ]; do
        echo ""
        echo "=== Publishing: $name (attempt $attempt/$max_attempts) ==="
        if cargo publish -p "$name" 2>&1; then
            echo "$name published successfully."
            return 0
        fi
        echo ""
        echo "Attempt $attempt failed. Waiting 60s for index to propagate..."
        sleep 60
        attempt=$((attempt + 1))
    done

    echo "ERROR: $name failed after $max_attempts attempts."
    echo "Run: ./publish.sh --from $name"
    exit 1
}

echo "PyForge v0.1.0 — crates.io publish"
echo ""

if [ -n "$DRY" ]; then
    echo "=== DRY RUN ==="
    cargo check --workspace
    echo ""
    cargo package -p pyforge-build-config
    echo ""
    echo "Metadata check:"
    for i in "${!CRATE_NAMES[@]}"; do
        name="${CRATE_NAMES[$i]}"
        dir="${CRATE_DIRS[$i]}"
        toml="$dir/Cargo.toml"
        printf "  %-28s" "$name"
        ok=true
        for field in name version license description repository authors; do
            grep -q "^$field" "$toml" 2>/dev/null || ok=false
        done
        if $ok; then echo "OK"; else echo "INCOMPLETE"; fi
    done
    echo ""
    echo "Dry run complete."
    exit 0
fi

cargo check --workspace
echo "Build OK."

skip=true
if [ -z "$START_FROM" ]; then
    skip=false
fi

for i in "${!CRATE_NAMES[@]}"; do
    name="${CRATE_NAMES[$i]}"

    if $skip; then
        if [ "$name" = "$START_FROM" ]; then
            skip=false
        else
            echo "Skipping $name (already published)"
            continue
        fi
    fi

    # Check if already published
    if cargo search "$name" 2>/dev/null | grep -q "^$name = \"0.1.0\""; then
        echo "$name v0.1.0 already on crates.io — skipping."
        continue
    fi

    publish_crate "$name"

    # Wait for index after publish (except last crate)
    if [ $i -lt $(( ${#CRATE_NAMES[@]} - 1 )) ]; then
        echo "Waiting for crates.io index..."
        local_attempt=0
        next_name="${CRATE_NAMES[$((i+1))]}"
        while [ $local_attempt -lt 12 ]; do
            sleep 15
            local_attempt=$((local_attempt + 1))
            if cargo search "$name" 2>/dev/null | grep -q "^$name = \"0.1.0\""; then
                echo "$name indexed after $((local_attempt * 15))s."
                break
            fi
            echo "  ...still waiting ($((local_attempt * 15))s)"
        done
    fi
done

echo ""
echo "All crates published:"
for name in "${CRATE_NAMES[@]}"; do
    echo "  https://crates.io/crates/$name"
done
