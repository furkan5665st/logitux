#!/usr/bin/env bash
set -euo pipefail

sudo dnf install -y \
  libxkbcommon-devel \
  wayland-devel \
  libX11-devel \
  libXi-devel \
  libXcursor-devel \
  libXrandr-devel \
  mesa-libGL-devel \
  pkgconf-pkg-config

sudo rm -f /etc/udev/rules.d/99-logitux.rules
sudo install -m 644 udev/60-logitux.rules /etc/udev/rules.d/60-logitux.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw --action=change
sudo udevadm trigger --subsystem-match=misc --sysname-match=uinput --action=change || true

cargo build --release --workspace

mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications"
cp target/release/logitux "$HOME/.local/bin/logitux"
cp target/release/logitux-gui "$HOME/.local/bin/logitux-gui"
cp assets/logitux.desktop "$HOME/.local/share/applications/logitux.desktop"
