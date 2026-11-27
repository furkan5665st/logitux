#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

VERSION="0.4.1"

echo "==> [1/6] Tests"
cargo test --workspace

echo "==> [2/6] Build"
cargo build --release --workspace
strip target/release/logitux-gui target/release/logitux
ldd target/release/logitux-gui || true

rm -rf dist AppDir deb_pkg rpmbuild "LogiTux-${VERSION}-linux-x86_64"
mkdir -p dist

echo "==> [3/6] AppImage (static type2-runtime, no libfuse2 required)"
mkdir -p \
  AppDir/usr/bin \
  AppDir/usr/lib/udev/rules.d \
  AppDir/usr/share/applications \
  AppDir/usr/share/icons/hicolor/256x256/apps

cp target/release/logitux-gui AppDir/usr/bin/logitux-gui
cp target/release/logitux AppDir/usr/bin/logitux
cp udev/60-logitux.rules AppDir/usr/lib/udev/rules.d/60-logitux.rules
cp assets/logitux.png AppDir/logitux.png
cp assets/logitux.png AppDir/usr/share/icons/hicolor/256x256/apps/logitux.png

cat > AppDir/logitux.desktop << 'EOF'
[Desktop Entry]
Name=LogiTux
Comment=Logitech mouse configuration
Exec=logitux-gui
Icon=logitux
Terminal=false
Type=Application
Categories=Utility;Settings;HardwareSettings;
StartupNotify=true
EOF
cp AppDir/logitux.desktop AppDir/usr/share/applications/logitux.desktop

cat > AppDir/AppRun << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
if [ "${1:-}" = "--install-udev" ]; then
  CMD="rm -f /etc/udev/rules.d/99-logitux.rules && cp '${HERE}/usr/lib/udev/rules.d/60-logitux.rules' /etc/udev/rules.d/60-logitux.rules && udevadm control --reload-rules && udevadm trigger --subsystem-match=hidraw --action=change && udevadm trigger --subsystem-match=misc --sysname-match=uinput --action=change"
  if command -v pkexec >/dev/null 2>&1; then
    pkexec sh -c "$CMD"
  else
    sudo sh -c "$CMD"
  fi
  exit 0
fi
if [ "${1:-}" = "--cli" ]; then
  shift
  exec "${HERE}/usr/bin/logitux" "$@"
fi
if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -e /proc/driver/nvidia/version ] && [ -z "${__NV_DISABLE_EXPLICIT_SYNC:-}" ]; then
  export __NV_DISABLE_EXPLICIT_SYNC=1
fi
"${HERE}/usr/bin/logitux-gui" "$@"
RC=$?
if [ "$RC" -ne 0 ] && [ -n "${WAYLAND_DISPLAY:-}" ] && [ -n "${DISPLAY:-}" ]; then
  WAYLAND_DISPLAY="" exec "${HERE}/usr/bin/logitux-gui" "$@"
fi
exit "$RC"
EOF
chmod +x AppDir/AppRun

if [ ! -f appimagetool-x86_64.AppImage ]; then
  wget -q https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x appimagetool-x86_64.AppImage
fi
if [ ! -f runtime-x86_64 ]; then
  wget -q https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64
fi
ARCH=x86_64 ./appimagetool-x86_64.AppImage --appimage-extract-and-run --comp zstd --runtime-file ./runtime-x86_64 AppDir "dist/LogiTux-${VERSION}-x86_64.AppImage"
chmod +x "dist/LogiTux-${VERSION}-x86_64.AppImage"

echo "==> [4/6] deb"
DEB_DIR="deb_pkg/logitux_${VERSION}_amd64"
mkdir -p \
  "$DEB_DIR/DEBIAN" \
  "$DEB_DIR/usr/bin" \
  "$DEB_DIR/lib/udev/rules.d" \
  "$DEB_DIR/usr/share/applications" \
  "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"

cp target/release/logitux-gui "$DEB_DIR/usr/bin/logitux-gui"
cp target/release/logitux "$DEB_DIR/usr/bin/logitux"
cp udev/60-logitux.rules "$DEB_DIR/lib/udev/rules.d/60-logitux.rules"
cp AppDir/logitux.desktop "$DEB_DIR/usr/share/applications/logitux.desktop"
cp assets/logitux.png "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/logitux.png"

cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: logitux
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: amd64
Depends: libc6
Recommends: libgl1, libxkbcommon0, libwayland-client0, libx11-6
Maintainer: LogiTux Contributors
Description: Logitech mouse configuration tool
 Configure DPI stages, polling rate, LIGHTSYNC RGB, macros, and hardware
 onboard profiles for Logitech mice on Linux.
EOF

cat > "$DEB_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/sh
set -e
rm -f /etc/udev/rules.d/99-logitux.rules /lib/udev/rules.d/99-logitux.rules /usr/lib/udev/rules.d/99-logitux.rules || true
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=hidraw --action=change || true
  udevadm trigger --subsystem-match=misc --sysname-match=uinput --action=change || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi
EOF
chmod 755 "$DEB_DIR/DEBIAN/postinst"
dpkg-deb --build --root-owner-group "$DEB_DIR" "dist/logitux_${VERSION}_amd64.deb"

echo "==> [5/6] rpm"
RPM_TOP="$ROOT/rpmbuild"
mkdir -p "$RPM_TOP/BUILD" "$RPM_TOP/RPMS" "$RPM_TOP/SOURCES" "$RPM_TOP/SPECS" "$RPM_TOP/SRPMS"
cat > "$RPM_TOP/SPECS/logitux.spec" << EOF
Name:           logitux
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Logitech mouse configuration tool
License:        MIT OR Apache-2.0
URL:            https://github.com/furkan5665st/logitux
AutoReqProv:    no

%description
Logitech mouse configuration tool for Linux.

%install
mkdir -p %{buildroot}/usr/bin %{buildroot}/usr/lib/udev/rules.d %{buildroot}/usr/share/applications %{buildroot}/usr/share/icons/hicolor/256x256/apps
install -m 755 $ROOT/target/release/logitux-gui %{buildroot}/usr/bin/logitux-gui
install -m 755 $ROOT/target/release/logitux %{buildroot}/usr/bin/logitux
install -m 644 $ROOT/udev/60-logitux.rules %{buildroot}/usr/lib/udev/rules.d/60-logitux.rules
install -m 644 $ROOT/AppDir/logitux.desktop %{buildroot}/usr/share/applications/logitux.desktop
install -m 644 $ROOT/assets/logitux.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/logitux.png

%post
rm -f /etc/udev/rules.d/99-logitux.rules /usr/lib/udev/rules.d/99-logitux.rules || true
if command -v udevadm >/dev/null 2>&1; then
  udevadm control --reload-rules || true
  udevadm trigger --subsystem-match=hidraw --action=change || true
  udevadm trigger --subsystem-match=misc --sysname-match=uinput --action=change || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi

%files
/usr/bin/logitux-gui
/usr/bin/logitux
/usr/lib/udev/rules.d/60-logitux.rules
/usr/share/applications/logitux.desktop
/usr/share/icons/hicolor/256x256/apps/logitux.png
EOF

rpmbuild --define "_topdir $RPM_TOP" -bb "$RPM_TOP/SPECS/logitux.spec"
cp "$RPM_TOP"/RPMS/x86_64/*.rpm "dist/logitux-${VERSION}-1.x86_64.rpm"

echo "==> [6/6] tarball"
BUNDLE="LogiTux-${VERSION}-linux-x86_64"
mkdir -p "$BUNDLE"
cp target/release/logitux-gui "$BUNDLE/logitux-gui"
cp target/release/logitux "$BUNDLE/logitux"
cp udev/60-logitux.rules "$BUNDLE/60-logitux.rules"
cp AppDir/logitux.desktop "$BUNDLE/logitux.desktop"
cp assets/logitux.png "$BUNDLE/logitux.png"
cp packaging/linux/install-bin.sh "$BUNDLE/install.sh"
cp packaging/linux/PKGBUILD "$BUNDLE/PKGBUILD"
chmod +x "$BUNDLE/install.sh" "$BUNDLE/logitux-gui" "$BUNDLE/logitux"
tar -czf "dist/LogiTux-${VERSION}-linux-x86_64.tar.gz" "$BUNDLE"

ls -lh dist/
