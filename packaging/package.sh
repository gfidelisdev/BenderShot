#!/usr/bin/env bash
# ==============================================================================
# BenderShot package.sh
# Builds .deb, .rpm, .AppImage, and portable tar.gz without external package mgr
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${ROOT_DIR}/target/release"
DIST_DIR="${SCRIPT_DIR}/dist"
ASSETS_DIR="${SCRIPT_DIR}/assets"

VERSION="0.1.0"
BIN_NAME="bendershot"

echo "==> Building release binary with Cargo..."
cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"

BINARY="${TARGET_DIR}/${BIN_NAME}"
if [ ! -f "${BINARY}" ]; then
    echo "Error: Binary ${BINARY} not found!" >&2
    exit 1
fi

mkdir -p "${DIST_DIR}"

# ------------------------------------------------------------------------------
# 1. Build .deb package
# ------------------------------------------------------------------------------
echo "==> Building Debian (.deb) package..."
DEB_ROOT="${SCRIPT_DIR}/deb/bendershot_${VERSION}_amd64"
rm -rf "${DEB_ROOT}"
mkdir -p "${DEB_ROOT}/DEBIAN"
mkdir -p "${DEB_ROOT}/usr/bin"
mkdir -p "${DEB_ROOT}/usr/share/applications"
mkdir -p "${DEB_ROOT}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${DEB_ROOT}/usr/lib/systemd/user"

cp "${BINARY}" "${DEB_ROOT}/usr/bin/${BIN_NAME}"
chmod 755 "${DEB_ROOT}/usr/bin/${BIN_NAME}"
ln -sf "${BIN_NAME}" "${DEB_ROOT}/usr/bin/enhanced-shot"

cp "${ASSETS_DIR}/bendershot.desktop" "${DEB_ROOT}/usr/share/applications/"
cp "${ASSETS_DIR}/bendershot.svg" "${DEB_ROOT}/usr/share/icons/hicolor/scalable/apps/"
cp "${ASSETS_DIR}/bendershot.service" "${DEB_ROOT}/usr/lib/systemd/user/"

cat << 'EOF' > "${DEB_ROOT}/DEBIAN/control"
Package: bendershot
Version: 0.1.0
Section: graphics
Priority: optional
Architecture: amd64
Provides: enhanced-shot
Maintainer: BenderShot Maintainers <contact@bendershot.org>
Description: Ferramenta moderna e nativa de captura, gravação e anotação para Wayland
 BenderShot oferece captura e gravação de tela, anotação com camadas vetoriais
 não-destrutivas, ferramentas de geometria (Bézier, Polígonos), suavização de
 traço livre, transições orgânicas write-on e fade-out, ofuscação (desfoque, mosaico, smudge),
 seletor de cores, e janela flutuante fixável (pin to screen), no estilo Adwaita/GNOME.
EOF

cat << 'EOF' > "${DEB_ROOT}/DEBIAN/postinst"
#!/bin/sh
set -e
if [ -x /usr/bin/update-desktop-database ]; then
    /usr/bin/update-desktop-database /usr/share/applications || true
fi
if [ -x /usr/bin/gtk-update-icon-cache ]; then
    /usr/bin/gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "${DEB_ROOT}/DEBIAN/postinst"

cat << 'EOF' > "${DEB_ROOT}/DEBIAN/postrm"
#!/bin/sh
set -e
if [ -x /usr/bin/update-desktop-database ]; then
    /usr/bin/update-desktop-database /usr/share/applications || true
fi
if [ -x /usr/bin/gtk-update-icon-cache ]; then
    /usr/bin/gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 "${DEB_ROOT}/DEBIAN/postrm"

dpkg-deb --build --root-owner-group "${DEB_ROOT}" "${DIST_DIR}/bendershot_${VERSION}_amd64.deb"
rm -rf "${DEB_ROOT}"
echo "    Created ${DIST_DIR}/bendershot_${VERSION}_amd64.deb"

# ------------------------------------------------------------------------------
# 2. Build .rpm package
# ------------------------------------------------------------------------------
echo "==> Building Red Hat / Fedora (.rpm) package..."
RPM_BUILD_DIR="${SCRIPT_DIR}/rpm/build"
rm -rf "${RPM_BUILD_DIR}"
mkdir -p "${RPM_BUILD_DIR}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cp "${BINARY}" "${RPM_BUILD_DIR}/SOURCES/${BIN_NAME}"
cp "${ASSETS_DIR}/bendershot.desktop" "${RPM_BUILD_DIR}/SOURCES/"
cp "${ASSETS_DIR}/bendershot.svg" "${RPM_BUILD_DIR}/SOURCES/"
cp "${ASSETS_DIR}/bendershot.service" "${RPM_BUILD_DIR}/SOURCES/"
cp "${SCRIPT_DIR}/rpm/bendershot.spec" "${RPM_BUILD_DIR}/SPECS/"

rpmbuild --define "_topdir ${RPM_BUILD_DIR}" -bb "${RPM_BUILD_DIR}/SPECS/bendershot.spec"
find "${RPM_BUILD_DIR}/RPMS" -name "*.rpm" -exec cp {} "${DIST_DIR}/" \;
rm -rf "${RPM_BUILD_DIR}"
echo "    Created RPM in ${DIST_DIR}/"

# ------------------------------------------------------------------------------
# 3. Build AppImage
# ------------------------------------------------------------------------------
echo "==> Building AppImage package..."
APPDIR="${SCRIPT_DIR}/appimage/BenderShot.AppDir"
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor/scalable/apps"

cp "${BINARY}" "${APPDIR}/usr/bin/${BIN_NAME}"
chmod 755 "${APPDIR}/usr/bin/${BIN_NAME}"
ln -sf "${BIN_NAME}" "${APPDIR}/usr/bin/enhanced-shot"

cp "${ASSETS_DIR}/bendershot.desktop" "${APPDIR}/bendershot.desktop"
cp "${ASSETS_DIR}/bendershot.desktop" "${APPDIR}/usr/share/applications/"
cp "${ASSETS_DIR}/bendershot.svg" "${APPDIR}/bendershot.svg"
cp "${ASSETS_DIR}/bendershot.svg" "${APPDIR}/usr/share/icons/hicolor/scalable/apps/"
cp "${ASSETS_DIR}/AppRun" "${APPDIR}/AppRun"
chmod 755 "${APPDIR}/AppRun"

APPIMAGETOOL="${SCRIPT_DIR}/appimage/appimagetool-x86_64.AppImage"
if [ ! -f "${APPIMAGETOOL}" ]; then
    echo "    Downloading appimagetool..."
    curl -sSL -o "${APPIMAGETOOL}" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" || true
    chmod +x "${APPIMAGETOOL}" 2>/dev/null || true
fi

APPIMAGE_OUTPUT="${DIST_DIR}/bendershot-${VERSION}-x86_64.AppImage"
if [ -x "${APPIMAGETOOL}" ]; then
    ARCH=x86_64 "${APPIMAGETOOL}" --appimage-extract-and-run "${APPDIR}" "${APPIMAGE_OUTPUT}" || \
    ARCH=x86_64 "${APPIMAGETOOL}" "${APPDIR}" "${APPIMAGE_OUTPUT}" || true
fi

if [ ! -f "${APPIMAGE_OUTPUT}" ]; then
    echo "    Note: AppImage generation skipped/failed; creating standalone portable AppDir bundle..."
    tar -czf "${DIST_DIR}/bendershot-${VERSION}-AppDir.tar.gz" -C "${SCRIPT_DIR}/appimage" "BenderShot.AppDir"
fi
rm -rf "${APPDIR}"

# ------------------------------------------------------------------------------
# 4. Build Standalone Multi-Distro Tarball with installer
# ------------------------------------------------------------------------------
echo "==> Building universal standalone tarball (multi-distro)..."
TARBALL_DIR="${SCRIPT_DIR}/bendershot-${VERSION}-linux-x86_64"
rm -rf "${TARBALL_DIR}"
mkdir -p "${TARBALL_DIR}/assets"

cp "${BINARY}" "${TARBALL_DIR}/${BIN_NAME}"
cp "${ASSETS_DIR}/bendershot.desktop" "${TARBALL_DIR}/assets/"
cp "${ASSETS_DIR}/bendershot.svg" "${TARBALL_DIR}/assets/"
cp "${ASSETS_DIR}/bendershot.service" "${TARBALL_DIR}/assets/"
cp "${ROOT_DIR}/install.sh" "${TARBALL_DIR}/install.sh"
chmod 755 "${TARBALL_DIR}/install.sh"

tar -czf "${DIST_DIR}/bendershot-${VERSION}-linux-x86_64.tar.gz" -C "${SCRIPT_DIR}" "bendershot-${VERSION}-linux-x86_64"
rm -rf "${TARBALL_DIR}"
echo "    Created ${DIST_DIR}/bendershot-${VERSION}-linux-x86_64.tar.gz"

echo "==> Packaging complete! Artifacts available in ${DIST_DIR}:"
ls -lh "${DIST_DIR}"
