#!/bin/bash
set -e

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2)}"
VERSION="${VERSION:-1.1.0}"
VERSION="${VERSION#v}"
APP_NAME="ver"
ARCH="amd64"
PACKAGE_NAME="${APP_NAME}_${VERSION}_${ARCH}"

echo "Compiling release binary with cargo..."
cargo build --release

echo "Building Debian package for version $VERSION..."
mkdir -p "$PACKAGE_NAME/usr/bin"
mkdir -p "$PACKAGE_NAME/usr/share/applications"
mkdir -p "$PACKAGE_NAME/usr/share/pixmaps"
mkdir -p "$PACKAGE_NAME/DEBIAN"

cp target/release/ver "$PACKAGE_NAME/usr/bin/ver"
cp data/com.example.ver.desktop "$PACKAGE_NAME/usr/share/applications/"
cp data/com.example.ver.png "$PACKAGE_NAME/usr/share/pixmaps/"

cat << CTRL_EOF > "$PACKAGE_NAME/DEBIAN/control"
Package: ver
Version: $VERSION
Section: utils
Priority: optional
Architecture: amd64
Maintainer: VER Team
Depends: libgtk-4-1, libadwaita-1-0
Description: Very Easy Remote - A GTK4 Connection Manager
CTRL_EOF

dpkg-deb --build "$PACKAGE_NAME" "${PACKAGE_NAME}.deb"
rm -rf "$PACKAGE_NAME"
echo "Done! Generated ${PACKAGE_NAME}.deb"

