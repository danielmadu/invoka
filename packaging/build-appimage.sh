#!/usr/bin/env bash
# Build an invoka AppImage with linuxdeploy + the Qt plugin.
#
# Requirements (Debian/Ubuntu names): qt6-base-dev qt6-svg-dev, plus the
# `linuxdeploy` and `linuxdeploy-plugin-qt` AppImages (downloaded on demand
# below), and the `appimagetool`-compatible runtime they embed.
#
# Usage:
#   ./packaging/build-appimage.sh            # uses target/release/invoka
#   APPIMAGE_ARCH=aarch64 ./build-appimage.sh
#
# Result: Invoka-<version>-<arch>.AppImage in the repo root.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${APPIMAGE_ARCH:-x86_64}"
BUILD_DIR="${REPO_ROOT}/target/appimage"
VERSION="$(cd "$REPO_ROOT" && cargo read-manifest | sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' | head -n1)"

TOOLS_DIR="${BUILD_DIR}/tools"
mkdir -p "$TOOLS_DIR"
cd "$TOOLS_DIR"

case "$ARCH" in
    x86_64)  triple="x86_64" ;;
    aarch64) triple="aarch64" ;;
    *) echo "unsupported APPIMAGE_ARCH: $ARCH" >&2; exit 1 ;;
esac

fetch() {
    local url="$1" out="$2"
    if [ ! -f "$out" ]; then
        echo "downloading $url"
        curl -fsSL --retry 3 -o "$out" "$url"
        chmod +x "$out"
    fi
}

fetch "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-${triple}.AppImage" "linuxdeploy-${triple}.AppImage"
fetch "https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-${triple}.AppImage" "linuxdeploy-plugin-qt-${triple}.AppImage"

# A release build is expected; build if missing.
if [ ! -x "${REPO_ROOT}/target/release/invoka" ]; then
    echo "release binary not found, building..."
    (cd "$REPO_ROOT" && QMAKE="$(command -v qmake6 || command -v qmake)" cargo build --release)
fi

STAGE="${BUILD_DIR}/AppDir"
rm -rf "$STAGE"
mkdir -p "${STAGE}/usr/bin" "${STAGE}/usr/share/applications" "${STAGE}/usr/share/icons/hicolor/scalable/apps"

cp "${REPO_ROOT}/target/release/invoka" "${STAGE}/usr/bin/"
cp "${REPO_ROOT}/packaging/invoka.desktop" "${STAGE}/usr/share/applications/invoka.desktop"
cp "${REPO_ROOT}/packaging/invoka-daemon.desktop" "${STAGE}/usr/share/applications/invoka-daemon.desktop"

# Placeholder icon (AppImage needs one); the launcher itself resolves icons
# through the system icon theme at runtime.
if command -v convert >/dev/null 2>&1; then
    convert -size 128x128 xc:"#1e1e2e" -fill "#89b4fa" -gravity center \
        -pointsize 64 -annotate 0 "i" "${STAGE}/usr/share/icons/hicolor/scalable/apps/invoka.png"
    cp "${STAGE}/usr/share/icons/hicolor/scalable/apps/invoka.png" "${STAGE}/invoka.png"
else
    cp "${REPO_ROOT}/packaging/invoka.png" "${STAGE}/invoka.png" 2>/dev/null || true
fi

export QMAKE="$(command -v qmake6 || command -v qmake)"
export OUTPUT="Invoka-${VERSION}-${triple}.AppImage"

cd "$BUILD_DIR"
"${TOOLS_DIR}/linuxdeploy-${triple}.AppImage" \
    --appdir "$STAGE" \
    --plugin qt \
    --output appimage

echo "done: ${BUILD_DIR}/${OUTPUT}"
