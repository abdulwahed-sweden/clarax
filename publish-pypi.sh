#!/bin/bash
# Author: Abdulwahed Mansour
# Builds and publishes pyforge-django to PyPI.
set -e

BUILD_ONLY=""
if [ "$1" = "--build" ]; then BUILD_ONLY=1; fi

REPO="$(cd "$(dirname "$0")" && pwd)"
WHEELS="$REPO/target/wheels"
mkdir -p "$WHEELS"

VERSION=$(python3 -c "import tomllib; print(tomllib.load(open('$REPO/pyforge-django/pyproject.toml','rb'))['project']['version'])")

echo "PyForge Django v${VERSION} — Building wheel"
echo ""

# Step 1: Compile the Rust extension
echo "Step 1: Compiling native extension..."
cargo build -p pyforge-django --release
DYLIB=$(find "$REPO/target/release" -maxdepth 1 \( -name 'libpyforge_django.dylib' -o -name 'libpyforge_django.so' -o -name 'pyforge_django.dll' \) | head -1)
if [ -z "$DYLIB" ]; then echo "ERROR: compiled lib not found"; exit 1; fi
echo "Built: $DYLIB ($(du -h "$DYLIB" | cut -f1))"

# Step 2: Assemble the wheel
echo ""
echo "Step 2: Assembling wheel..."

# Detect platform
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

WHEEL_NAME="pyforge_django-${VERSION}-${PYTAG}-${PYTAG}-${PLAT}.whl"
WHEEL_PATH="$WHEELS/$WHEEL_NAME"

STAGING=$(mktemp -d)
PKG="$STAGING/django_pyforge"
DIST="$STAGING/pyforge_django-${VERSION}.dist-info"
mkdir -p "$PKG" "$DIST"

# Copy Python sources
cp "$REPO/pyforge-django/django_pyforge/__init__.py" "$PKG/"
cp "$REPO/pyforge-django/django_pyforge/__init__.pyi" "$PKG/" 2>/dev/null || true
cp "$REPO/pyforge-django/django_pyforge/apps.py" "$PKG/"
cp "$REPO/pyforge-django/django_pyforge/serializers.py" "$PKG/"
cp "$REPO/pyforge-django/django_pyforge/serializers.pyi" "$PKG/" 2>/dev/null || true
cp "$REPO/pyforge-django/django_pyforge/validators.py" "$PKG/"
cp "$REPO/pyforge-django/django_pyforge/validators.pyi" "$PKG/" 2>/dev/null || true
cp "$REPO/pyforge-django/django_pyforge/py.typed" "$PKG/" 2>/dev/null || true

# Copy the native extension at the root level (matches Rust lib name pyforge_django)
cp "$DYLIB" "$STAGING/pyforge_django.so"

# Read README for description body
README_CONTENT=$(cat "$REPO/pyforge-django/README.md")

# Write METADATA with full description
cat > "$DIST/METADATA" << EOF
Metadata-Version: 2.1
Name: pyforge-django
Version: ${VERSION}
Summary: Rust-accelerated Django serialization, validation, and field mapping
Author: Abdulwahed Mansour
License: MIT
Requires-Python: >=3.11
Requires-Dist: django>=4.2
Classifier: Development Status :: 3 - Alpha
Classifier: Framework :: Django
Classifier: Framework :: Django :: 4.2
Classifier: Framework :: Django :: 5.0
Classifier: Framework :: Django :: 5.1
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

# Write top_level.txt
echo "django_pyforge" > "$DIST/top_level.txt"
echo "pyforge_django" >> "$DIST/top_level.txt"

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
    # RECORD
    buf = io.StringIO()
    w = csv.writer(buf)
    for r in records: w.writerow(r)
    rec_path = 'pyforge_django-${VERSION}.dist-info/RECORD'
    w.writerow((rec_path, '', ''))
    zf.writestr(rec_path, buf.getvalue())
"

rm -rf "$STAGING"
cd "$REPO"

echo ""
echo "Wheel built: $WHEEL_PATH"
python3 -m zipfile -l "$WHEEL_PATH" | grep -E "\.so|\.pyd|\.py$|METADATA"
echo ""
ls -lh "$WHEEL_PATH"

if [ -z "$BUILD_ONLY" ]; then
    echo ""
    echo "Step 3: Uploading to PyPI..."
    twine upload "$WHEEL_PATH"
    echo "Published: pip install pyforge-django==${VERSION}"
fi
