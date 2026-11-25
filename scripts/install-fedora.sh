#!/usr/bin/env bash
set -euo pipefail

echo "==> Installing LogiTux build dependencies on Fedora Linux..."
sudo dnf install -y \
  systemd-devel \
  libudev-devel \
  libxkbcommon-devel \
  wayland-devel \
  libX11-devel \
  libXi-devel \
  libXcursor-devel \
  libXrandr-devel \
  mesa-libGL-devel \
  pkgconf-pkg-config

echo "==> Installing udev rules for non-root Logitech HID++ access..."
sudo install -m 644 udev/99-logitux.rules /etc/udev/rules.d/99-logitux.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw

echo "==> Building LogiTux (CLI + GUI) in release mode..."
cargo build --release --workspace

echo "==> Installing binaries and desktop launcher..."
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
cp target/release/logitux "$HOME/.local/bin/logitux"
if [ -f target/release/logitux-gui ]; then
  cp target/release/logitux-gui "$HOME/.local/bin/logitux-gui"
fi
cp assets/logitux.desktop "$HOME/.local/share/applications/logitux.desktop"

echo "==> Done! Try running: logitux probe"
