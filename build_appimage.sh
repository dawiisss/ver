#!/bin/bash
set -e

echo "Building Rust binary..."
cargo build --release

echo "Preparing AppDir..."
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/pixmaps

cp target/release/beautiful-goodall AppDir/usr/bin/ver
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
    wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage -O appimagetool
    chmod +x appimagetool
fi

echo "Building AppImage..."
./appimagetool AppDir VER-x86_64.AppImage

rm -rf AppDir
echo "Done! Generated VER-x86_64.AppImage"
