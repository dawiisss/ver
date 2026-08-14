#!/bin/bash
set -e

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2)}"
VERSION="${VERSION:-1.0.0}"
VERSION="${VERSION#v}"
APP_NAME="ver"
PKG_DIR="$(mktemp -d)"
PKG_TAR="${APP_NAME}-${VERSION}-1-x86_64.pkg.tar.zst"

echo "Compiling release binary with cargo..."
cargo build --release

echo "Building Arch Linux pacman package for version $VERSION..."

mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/pixmaps"

cp target/release/beautiful-goodall "$PKG_DIR/usr/bin/ver"
cp data/com.example.ver.desktop "$PKG_DIR/usr/share/applications/"
cp data/com.example.ver.png "$PKG_DIR/usr/share/pixmaps/"

SIZE=$(du -sb "$PKG_DIR/usr" | awk '{print $1}')
BUILD_DATE=$(date +%s)

cat << EOF > "$PKG_DIR/.PKGINFO"
pkgname = ver
pkgbase = ver
pkgver = ${VERSION}-1
pkgdesc = Very Easy Remote - A GTK4 Connection Manager
url = https://github.com/dawiisss/ver
builddate = $BUILD_DATE
packager = VER Team
size = $SIZE
arch = x86_64
license = GPL3
depend = gtk4
depend = libadwaita
EOF

cat << EOF > "$PKG_DIR/.BUILDINFO"
format = 2
pkgname = ver
pkgver = ${VERSION}-1
pkgarch = x86_64
pkgbuild_sha256sum = 0000000000000000000000000000000000000000000000000000000000000000
packager = VER Team
builddate = $BUILD_DATE
buildenv = color
EOF

(
  cd "$PKG_DIR"
  tar -c --zstd -f "$PKG_TAR" .PKGINFO .BUILDINFO usr
  mv "$PKG_TAR" "$OLDPWD/"
)

rm -rf "$PKG_DIR"
echo "Done! Generated $PKG_TAR"


