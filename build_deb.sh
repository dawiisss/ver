#!/bin/bash
set -e

APP_NAME="ver"
VERSION="1.0.0"
ARCH="amd64"
PACKAGE_NAME="${APP_NAME}_${VERSION}_${ARCH}"

echo "Building Rust binary..."
cargo build --release

echo "Creating Debian package structure..."
mkdir -p $PACKAGE_NAME/usr/bin
mkdir -p $PACKAGE_NAME/usr/share/applications
mkdir -p $PACKAGE_NAME/usr/share/pixmaps
mkdir -p $PACKAGE_NAME/DEBIAN

echo "Copying files..."
cp target/release/beautiful-goodall $PACKAGE_NAME/usr/bin/ver
cp data/com.example.ver.desktop $PACKAGE_NAME/usr/share/applications/
cp data/com.example.ver.png $PACKAGE_NAME/usr/share/pixmaps/

echo "Creating control file..."
cat << 'CTRL_EOF' > $PACKAGE_NAME/DEBIAN/control
Package: ver
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: VER Team
Description: Very Easy Remote - A GTK4 Connection Manager
CTRL_EOF

echo "Building package..."
dpkg-deb --build $PACKAGE_NAME
rm -rf $PACKAGE_NAME
echo "Done! Generated ${PACKAGE_NAME}.deb"
