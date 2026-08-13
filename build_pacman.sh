#!/bin/bash
set -e

echo "Building release binary..."
cargo build --release

BUILD_TMP="$(mktemp -d)"
ROOT_DIR="$(pwd)"

cat << EOF > "$BUILD_TMP/PKGBUILD"
pkgname=ver
pkgver=1.0.0
pkgrel=1
pkgdesc="Very Easy Remote - A GTK4 Connection Manager"
arch=('x86_64')
url="https://github.com/dawiisss/ver"
license=('GPL3')
depends=('gtk4' 'libadwaita')
source=()

package() {
  install -Dm755 "$ROOT_DIR/target/release/beautiful-goodall" "\$pkgdir/usr/bin/ver"
  install -Dm644 "$ROOT_DIR/data/com.example.ver.desktop" "\$pkgdir/usr/share/applications/com.example.ver.desktop"
  install -Dm644 "$ROOT_DIR/data/com.example.ver.png" "\$pkgdir/usr/share/pixmaps/com.example.ver.png"
}
EOF

echo "Building pacman package using makepkg in isolated directory..."
(
  cd "$BUILD_TMP"
  makepkg -f -d --nodeps
  cp *.pkg.tar.* "$ROOT_DIR/" 2>/dev/null || true
)

rm -rf "$BUILD_TMP"
echo "Done! Pacman package generated safely."

