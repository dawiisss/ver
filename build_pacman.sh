#!/bin/bash
set -e

echo "Creating PKGBUILD for Arch Linux..."

cat << 'EOF' > PKGBUILD
pkgname=ver
pkgver=1.0.0
pkgrel=1
pkgdesc="Very Easy Remote - A GTK4 Connection Manager"
arch=('x86_64')
url="https://github.com/dawiisss/ver"
license=('GPL3')
depends=('gtk4' 'libadwaita')
makedepends=('cargo')
source=()

build() {
  cd "$srcdir/.."
  cargo build --release
}

package() {
  cd "$srcdir/.."
  install -Dm755 target/release/beautiful-goodall "$pkgdir/usr/bin/ver"
  install -Dm644 data/com.example.ver.desktop "$pkgdir/usr/share/applications/com.example.ver.desktop"
  install -Dm644 data/com.example.ver.png "$pkgdir/usr/share/pixmaps/com.example.ver.png"
}
EOF

echo "Building pacman package using makepkg..."
makepkg -f

echo "Cleaning up PKGBUILD..."
rm PKGBUILD src -rf

echo "Done! Pacman package generated."
