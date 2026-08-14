#!/bin/bash
set -e

VERSION="${1:-$(grep '^version' Cargo.toml | head -1 | cut -d '"' -f 2)}"
VERSION="${VERSION:-1.1.0}"
VERSION="${VERSION#v}"
APP_NAME="ver"
APPIMAGE_NAME="${APP_NAME}-${VERSION}-x86_64.AppImage"

echo "Compiling release binary with cargo..."
cargo build --release

echo "Preparing AppDir for $APPIMAGE_NAME..."
rm -rf AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/pixmaps

cp target/release/ver AppDir/usr/bin/ver
cp data/com.example.ver.desktop AppDir/usr/share/applications/
cp data/com.example.ver.png AppDir/usr/share/pixmaps/

# AppRun script
cat << 'EOF' > AppDir/AppRun
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/ver" "$@"
EOF
chmod +x AppDir/AppRun

# Top-level desktop file and icon required by AppImage
cp data/com.example.ver.desktop AppDir/
cp data/com.example.ver.png AppDir/

echo "Downloading appimagetool if missing..."
if [ ! -f "appimagetool" ]; then
    wget -q https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage -O appimagetool || \
    wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -O appimagetool
    chmod +x appimagetool
fi

echo "Building AppImage..."
ARCH=x86_64 ./appimagetool --appimage-extract-and-run AppDir "$APPIMAGE_NAME"

rm -rf AppDir
echo "Done! Generated $APPIMAGE_NAME"

