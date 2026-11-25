<p align="center">
  <img src="assets/logitux.png" width="160" alt="LogiTux">
</p>

# LogiTux

Logitech mouse configuration tool for Linux and Windows, written in Rust.

## Download

Packages are published on the [Releases](https://github.com/furkan5665st/logitux/releases/latest) page.

### Linux

- AppImage (any distribution)
  ```bash
  chmod +x LogiTux-0.3.3-x86_64.AppImage
  ./LogiTux-0.3.3-x86_64.AppImage --install-udev
  ./LogiTux-0.3.3-x86_64.AppImage
  ```
- Fedora / openSUSE
  ```bash
  sudo dnf install ./logitux-0.3.3-1.x86_64.rpm
  ```
- Ubuntu / Debian / Mint
  ```bash
  sudo apt install ./logitux_0.3.3_amd64.deb
  ```
- Arch and others
  ```bash
  tar -xzf LogiTux-0.3.3-linux-x86_64.tar.gz
  cd LogiTux-0.3.3-linux-x86_64 && ./install.sh
  ```

### Windows

Run `LogiTux-Portable-Windows-x64.exe`. No installation required.

## Features

- DPI stages, DPI shift and report rate
- LIGHTSYNC presets, per-zone colors and animations
- Button assignments: shortcuts, keys, media, macros, Discord and OBS actions
- Profiles and onboard memory
- Multi-language interface

## Build

```bash
cargo build --release --workspace
```

## License

MIT or Apache-2.0.
