#!/bin/bash
set -e

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2)}"
VERSION="${VERSION:-1.3.0}"
VERSION="${VERSION#v}"
APP_NAME="ver"
ARCH="amd64"
DEB_PACKAGE="${APP_NAME}_${VERSION}_${ARCH}.deb"

if [ ! -f "$DEB_PACKAGE" ]; then
    bash build_deb.sh "$VERSION"
fi

echo "Converting to RPM using alien for version $VERSION..."
if ! command -v alien &> /dev/null; then
    echo "Error: alien is not installed. Please install alien (sudo apt install alien) to build RPMs."
    exit 1
fi

alien -r -c -v "$DEB_PACKAGE"
echo "Done! RPM package generated."

