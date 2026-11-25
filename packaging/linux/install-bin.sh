#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Installing LogiTux G Studio (Universal Linux Installer)..."
sudo install -Dm755 "$SCRIPT_DIR/logitux-gui" /usr/local/bin/logitux-gui
sudo install -Dm755 "$SCRIPT_DIR/logitux" /usr/local/bin/logitux
sudo install -Dm644 "$SCRIPT_DIR/99-logitux.rules" /etc/udev/rules.d/99-logitux.rules
sudo install -Dm644 "$SCRIPT_DIR/logitux.desktop" /usr/share/applications/logitux.desktop
sudo install -Dm644 "$SCRIPT_DIR/logitux.png" /usr/share/icons/hicolor/256x256/apps/logitux.png

if command -v udevadm >/dev/null 2>&1; then
  sudo udevadm control --reload-rules
  sudo udevadm trigger --subsystem-match=hidraw || true
fi

echo "==> LogiTux installed! Launch from your application menu or run: logitux-gui"
