#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

sudo install -Dm755 "$SCRIPT_DIR/logitux-gui" /usr/local/bin/logitux-gui
sudo install -Dm755 "$SCRIPT_DIR/logitux" /usr/local/bin/logitux
sudo rm -f /etc/udev/rules.d/99-logitux.rules
sudo install -Dm644 "$SCRIPT_DIR/60-logitux.rules" /etc/udev/rules.d/60-logitux.rules
sudo install -Dm644 "$SCRIPT_DIR/logitux.desktop" /usr/share/applications/logitux.desktop
sudo install -Dm644 "$SCRIPT_DIR/logitux.png" /usr/share/icons/hicolor/256x256/apps/logitux.png

if command -v udevadm >/dev/null 2>&1; then
  sudo udevadm control --reload-rules
  sudo udevadm trigger --subsystem-match=hidraw --action=change || true
fi
