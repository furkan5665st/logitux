<p align="center">
  <img src="assets/logitux.png" width="160" alt="LogiTux">
</p>

# LogiTux

Logitech mouse configuration tool for Linux and Windows, written in Rust. Settings are saved directly to mouse onboard memory so you do not need to keep the app open.

## Download

Packages are published on the [Releases](https://github.com/furkan5665st/logitux/releases/latest) page.

### Linux

- AppImage (any distribution, works without `libfuse2`, automatic access grant inside the app):
  ```bash
  chmod +x LogiTux-0.4.1-x86_64.AppImage
  ./LogiTux-0.4.1-x86_64.AppImage
  ```
- Fedora / openSUSE:
  ```bash
  sudo dnf install ./logitux-0.4.1-1.x86_64.rpm
  ```
- Ubuntu / Debian / Mint:
  ```bash
  sudo apt install ./logitux_0.4.1_amd64.deb
  ```
- Arch and others:
  ```bash
  tar -xzf LogiTux-0.4.1-linux-x86_64.tar.gz
  cd LogiTux-0.4.1-linux-x86_64 && ./install.sh
  ```

### Windows

Run `LogiTux-Portable-Windows-x64.exe`. No installation required.

## Features

- Onboard-first hardware sync: DPI stages, DPI shift, polling rate, LIGHTSYNC presets, button mappings, shortcuts, and macros are written directly to mouse onboard flash (`0x8100`)
- Direct Linux `/dev/hidraw` transport with automatic access setup (Wayland & X11)
- Multi-language interface

## Build

```bash
cargo build --release --workspace
```

## License

MIT or Apache-2.0.
