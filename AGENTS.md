# AGENTS.md

LogiTux is a Logitech mouse configuration tool (DPI, polling rate, RGB, button
mapping, macros, onboard profiles) for Linux and Windows, written in Rust. It
speaks the HID++ 2.0 protocol directly over raw HID; there is no vendor SDK.

## Commands

```bash
cargo test --workspace                 # all unit tests
cargo build --release --workspace      # build CLI + GUI
cargo run -p logitux-gui               # launch the GUI
cargo run -p logitux-cli -- list       # CLI examples (see below)
```

CLI subcommands (`-p logitux-cli`): `list`, `probe`, `dpi [value] [--sensor N]`,
`rate [hz]`, `rgb <mode> [--color HEX] [--cluster N] [--persist]`,
`perkey <hex..> [--start-id N]`, `onboard [info|dump|host|onboard]`,
`raw <feature_hex> <fn_id> [hex..]`. A global `--json` flag switches to
structured output. Example: `cargo run -p logitux-cli -- --json probe`.

Linux builds require system headers before `cargo build`:
`libudev-dev libxkbcommon-dev libwayland-dev libx11-dev libxi-dev
libxcursor-dev libxrandr-dev libgl1-mesa-dev pkg-config` (exact list in
`.github/workflows/ci.yml`). Windows builds need no extras beyond the MSVC
toolchain.

## Crate architecture (strict dependency direction)

```
logitux-hidpp      protocol only: packets, feature codecs, no I/O deps
logitux-transport  hidapi I/O: enumerate devices, HidapiTransport, connect_all_devices
logitux-devices    static PID -> model metadata table (KNOWN_MICE)
logitux-cli        clap CLI over hidpp + transport + devices
logitux-gui        eframe/egui app over all of the above
```

`logitux-hidpp` deliberately has **no** `hidapi` dependency; all raw I/O goes
through the `Transport` trait implemented by `logitux-transport`. Keep protocol
code (encoders/decoders) pure and in the `features/*` modules so it stays
unit-testable without hardware.

### HID++ protocol layer (`crates/logitux-hidpp`)

- `features/mod.rs` holds `FeatureId` constants and `feature_name()`. When
  adding a feature, add the constant, the `feature_name` arm, and a codec in a
  new/existing `features/<name>.rs` module re-exported from `features/mod.rs`.
- **Safety lock**: `FeatureEntry::is_safety_locked()` blocks firmware/Dfu and
  hidden-debug features (`0x00C1 C2 C3 D0 D1 1E00 1E22 1EB0`). `request_idx`
  and `request_feature` refuse these before sending. Do not remove this check.
- Multi-byte protocol fields are **big-endian** (`from_be_bytes`/`to_be_bytes`).
- Framing: short = 7 bytes (`0x10`), long = 20 (`0x11`), very-long = 64
  (`0x12`); header is `[report_id, device_index, feature_index,
  (func<<4)|sw_id, params..]`. Default software id is `0x0A`, wired device
  index is `0xFF`, receiver slots are `1..=6`.
- `HidppDevice::request_*` writes, then reads until a packet
  `matches_request` (skipping unrelated packets, up to 32). Button-spy events
  (`FeatureId::MOUSE_BUTTON_SPY` 0x8110, sw_id 0) are queued in `spy_queue`
  while draining, not discarded.
- Errors: HID++ 2.0 errors arrive with feature index `0xFF` (see
  `check_error`); 1.0 errors use `0x8F`.
- DPI/rate accessors try multiple feature IDs in priority order (e.g.
  `EXTENDED_ADJUSTABLE_DPI` then `ADJUSTABLE_DPI`; `EXTENDED_REPORT_RATE`
  then `REPORT_RATE`). Follow that fallback pattern for new settings.
- Onboard profile sectors use CRC16-CCITT; `OnboardProfile::parse` validates
  it (test vector asserts `crc16_ccitt(b"123456789") == 0x29B1`).

### Transport (`crates/logitux-transport`)

- `enumerate_logitech_devices()` filters vendor `0x046D`, groups HID
  collections by `(vid, serial, interface)`, and records short/long/very-long
  open paths plus a fallback. On Linux interfaces with number >= 1 are treated
  as HID++.
- `HidapiTransport::open` may open up to three handles (short/long/very-long;
  duplicates collapse to one). `read_packet` reads the primary handle then a
  short secondary read to catch stragglers.
- `connect_all_devices()` iterates receiver slots `1..=6` for receiver PIDs and
  uses the wired index otherwise. Failures to open a slot are silently skipped.

### Devices (`crates/logitux-devices`)

Static `KNOWN_MICE: &[MouseProfileMeta]` keyed by product ID; look up with
`lookup_by_pid(pid)`. Add a new mouse by appending an entry (model name, series,
button/RGB-zone counts, DPI range, onboard/battery flags). The GUI falls back to
generic metadata when a PID is absent.

## GUI (`crates/logitux-gui`)

The UI must **never touch hidapi directly**. All hardware work runs on a
background thread created by `HardwareWorkerHandle::spawn`, communicating over
two `mpsc` channels: `HwCommand` (UI -> worker) and `HwEvent` (worker -> UI,
wakes egui via `ctx.request_repaint`).

- Send hardware changes with `self.hw.send(HwCommand::...)`; drain events with
  `while let Ok(ev) = self.hw.rx.try_recv()` at the top of `App::update`.
- The worker coalesces rapid commands (DPI, non-persistent RGB/perkey, button
  map) so only the latest wins. Preserve this when adding commands.
- Live button events come from the 0x8110 button-spy mask polled by the worker;
  `app.rs` diffs previous/next masks to fire press/release bindings and macros.
- `AppStore` (`store.rs`) is the single persisted state object. Call
  `store.save()` after mutations. Config path: Windows
  `%APPDATA%/LogiTux/settings.json`, else `$XDG_CONFIG_HOME/logitux` or
  `~/.config/logitux/settings.json`.
- `app.rs` is a large monolith holding navigation state, hardware sync, and
  binding execution. Page rendering is split into `views_home.rs`,
  `macro_ui.rs`, and shared drawing helpers in `canvas.rs` / `theme.rs`.
- `input_synth.rs` emits real input: Windows `SendInput`, Linux `/dev/uinput`.
  `integrations.rs` drives Discord (IPC pipe/socket) and OBS (TCP/WebSocket).
- `i18n.rs`: 20 locales matching Logitech G HUB. `Lang::detect()` reads the
  system locale and falls back to English. Call sites stay bilingual via
  `lang.pick("Türkçe", "English")`; `pick` returns the Turkish literal for
  `Tr`, the English literal for `En`, and looks the English string up in
  `i18n_data.json` (generated from G HUB's own locale YAML) for every other
  language, falling back to English. Add new UI strings as `pick(tr, en)`;
  untranslated strings simply show English.

## Conventions

- Edition 2021, `thiserror` for errors, `serde` derive on every public data
  struct that crosses the crate/persistence/CLI-JSON boundary. New settings
  usually need `Serialize + Deserialize` and a default.
- Errors: protocol errors live in `hidpp::error::HidppError`; transport wraps
  them. Prefer specific variants over `String`.
- Unit tests are colocated `#[cfg(test)] mod tests` inside `features/*` and
  `packet.rs` (pure codec/CRC/layout checks). There are no hardware or GUI
  integration tests, so test encoders/decoders directly.

## Platform / packaging

- `udev/99-logitux.rules` grants `uaccess` on Logitech hidraw, Bluetooth
  hidraw, and `/dev/uinput`. Without it non-root users cannot open devices or
  synthesize macros on Linux. Install for local dev:
  `sudo cp udev/99-logitux.rules /etc/udev/rules.d/ && sudo udevadm control
  --reload-rules && sudo udevadm trigger`.
- `crates/logitux-gui/build.rs` embeds `assets/logitux.ico` on Windows;
  `main.rs` sets `windows_subsystem = "windows"` in release.
- `scripts/gen_icons.py` regenerates `assets/logitux.png` / `.ico` from
  `assets/logitux-source.png` (needs Pillow).
- `packaging/linux/build-packages.sh` runs tests + release build, then emits
  AppImage/DEB/RPM/tarball into `dist/`. Its `AppRun` handles
  `--install-udev` and `--cli`.

## Gotchas

- The version appears in several places that must be bumped together: the
  workspace `[workspace.package] version`, `packaging/linux/build-packages.sh`
  (`VERSION=`), README download snippets, and the stale
  `packaging/linux/PKGBUILD` (`pkgver`). They drift if only one is updated.
- Do not add `hidapi` to `logitux-hidpp`; it breaks the layering and the
  no-hardware test guarantee.
- `request_*` calls are synchronous and can block for up to ~1s; anything on
  the UI thread that talks to a device belongs in the hardware worker.
- Feature indices are per-device and resolved at connect time; never hardcode a
  feature index, always go through `has_feature`/`request_feature` or the
  `feature_map`.
- Receiver (`is_receiver`) devices expose multiple sub-devices via slots; the
  same physical mouse can appear more than once, so always address by
  `device_index`.
