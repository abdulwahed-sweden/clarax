#!/bin/bash
# Author: Abdulwahed Mansour
# Builds and publishes pyforge-core to PyPI.
set -e

BUILD_ONLY=""
if [ "$1" = "--build" ]; then BUILD_ONLY=1; fi

REPO="$(cd "$(dirname "$0")" && pwd)"
WHEELS="$REPO/target/wheels"
mkdir -p "$WHEELS"

VERSION=$(python3 -c "import tomllib; print(tomllib.load(open('$REPO/pyforge-core/pyproject.toml','rb'))['project']['version'])")

echo "PyForge Core v${VERSION} — Building wheel"
echo ""

# Step 1: Compile the Rust extension
echo "Step 1: Compiling native extension..."
cargo build -p pyforge-core --release
DYLIB=$(find "$REPO/target/release" -maxdepth 1 \( -name 'libpyforge_core.dylib' -o -name 'libpyforge_core.so' -o -name 'pyforge_core.dll' \) | head -1)
if [ -z "$DYLIB" ]; then echo "ERROR: compiled lib not found"; exit 1; fi
echo "Built: $DYLIB ($(du -h "$DYLIB" | cut -f1))"

# Step 2: Assemble the wheel
echo ""
echo "Step 2: Assembling wheel..."

PYTAG="cp312"
MACHINE=$(python3 -c "import platform; print(platform.machine().lower())")
SYSTEM=$(python3 -c "import platform; print(platform.system().lower())")
if [ "$SYSTEM" = "darwin" ]; then
    PLAT="macosx_11_0_${MACHINE}"
elif [ "$SYSTEM" = "linux" ]; then
    PLAT="manylinux_2_17_${MACHINE}"
else
    PLAT="win_${MACHINE}"
fi

WHEEL_NAME="pyforge_core-${VERSION}-${PYTAG}-${PYTAG}-${PLAT}.whl"
WHEEL_PATH="$WHEELS/$WHEEL_NAME"

STAGING=$(mktemp -d)
PKG="$STAGING/pyforge_core"
DIST="$STAGING/pyforge_core-${VERSION}.dist-info"
mkdir -p "$PKG" "$DIST"

# Copy Python sources
cp "$REPO/pyforge-core/pyforge_core/__init__.py" "$PKG/"
cp "$REPO/pyforge-core/pyforge_core/__init__.pyi" "$PKG/"
cp "$REPO/pyforge-core/pyforge_core/py.typed" "$PKG/"
cp "$REPO/pyforge-core/pyforge_core/auto_schema.py" "$PKG/"
cp "$REPO/pyforge-core/pyforge_core/constraints.py" "$PKG/"

# Copy the native extension
cp "$DYLIB" "$PKG/_native.so"

# Read README
README_CONTENT=$(cat "$REPO/pyforge-core/README.md")

# Write METADATA
cat > "$DIST/METADATA" << EOF
Metadata-Version: 2.1
Name: pyforge-core
Version: ${VERSION}
Summary: Rust-accelerated serialization and validation for Python — framework-agnostic
Author: Abdulwahed Mansour
License: MIT
Requires-Python: >=3.11
Classifier: Development Status :: 3 - Alpha
Classifier: Intended Audience :: Developers
Classifier: License :: OSI Approved :: MIT License
Classifier: Programming Language :: Python :: 3.11
Classifier: Programming Language :: Python :: 3.12
Classifier: Programming Language :: Python :: 3.13
Classifier: Programming Language :: Rust
Description-Content-Type: text/markdown
Project-URL: Homepage, https://github.com/abdulwahed-sweden/pyforge
Project-URL: Repository, https://github.com/abdulwahed-sweden/pyforge

${README_CONTENT}
EOF

# Write WHEEL
cat > "$DIST/WHEEL" << EOF
Wheel-Version: 1.0
Generator: pyforge-publish
Root-Is-Purelib: false
Tag: ${PYTAG}-${PYTAG}-${PLAT}
EOF

echo "pyforge_core" > "$DIST/top_level.txt"

# Create the zip
rm -f "$WHEEL_PATH"
cd "$STAGING"
python3 -c "
import zipfile, os, hashlib, base64, csv, io
whl = '$WHEEL_PATH'
with zipfile.ZipFile(whl, 'w', zipfile.ZIP_DEFLATED) as zf:
    records = []
    for root, dirs, files in os.walk('.'):
        for f in files:
            path = os.path.join(root, f)
            arc = os.path.relpath(path, '.')
            data = open(path, 'rb').read()
            zf.writestr(arc, data)
            h = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b'=').decode()
            records.append((arc, f'sha256={h}', str(len(data))))
    buf = io.StringIO()
    w = csv.writer(buf)
    for r in records: w.writerow(r)
    rec_path = 'pyforge_core-${VERSION}.dist-info/RECORD'
    w.writerow((rec_path, '', ''))
    zf.writestr(rec_path, buf.getvalue())
"

rm -rf "$STAGING"
cd "$REPO"

echo ""
echo "Wheel built: $WHEEL_PATH"
python3 -m zipfile -l "$WHEEL_PATH" | grep -E "\.so|\.py$|METADATA"
echo ""
ls -lh "$WHEEL_PATH"

if [ -z "$BUILD_ONLY" ]; then
    echo ""
    echo "Step 3: Uploading to PyPI..."
    twine upload "$WHEEL_PATH"
    echo "Published: pip install pyforge-core==${VERSION}"
fi
