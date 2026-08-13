#!/bin/bash
set -e

echo "Building Debian package first..."
bash build_deb.sh

APP_NAME="ver"
VERSION="1.0.0"
ARCH="amd64"
DEB_PACKAGE="${APP_NAME}_${VERSION}_${ARCH}.deb"

echo "Converting to RPM using alien..."
if ! command -v alien &> /dev/null; then
    echo "Error: alien is not installed. Please install alien (sudo apt install alien) to build RPMs."
    exit 1
fi

sudo alien -r -c $DEB_PACKAGE
echo "Done! RPM package generated."
