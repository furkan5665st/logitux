use crate::error::{HidppError, Result};
use crate::features::rgb::{decode_11byte_effect, encode_11byte_effect, RgbEffect};
use serde::{Deserialize, Serialize};

pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardMode {
    Onboard,
    Host,
    Unknown(u8),
}

impl From<u8> for OnboardMode {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Onboard,
            2 => Self::Host,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardDescription {
    pub memory_model_id: u8,
    pub profile_format_id: u8,
    pub macro_format_id: u8,
    pub profile_count: u8,
    pub profile_count_oob: u8,
    pub button_count: u8,
    pub sector_count: u8,
    pub sector_size: u16,
    pub mechanical_layout: u8,
    pub various_info: u8,
}

impl OnboardDescription {
    pub fn from_params(p: &[u8]) -> Self {
        Self {
            memory_model_id: p.first().copied().unwrap_or(0),
            profile_format_id: p.get(1).copied().unwrap_or(0),
            macro_format_id: p.get(2).copied().unwrap_or(0),
            profile_count: p.get(3).copied().unwrap_or(0),
            profile_count_oob: p.get(4).copied().unwrap_or(0),
            button_count: p.get(5).copied().unwrap_or(0),
            sector_count: p.get(6).copied().unwrap_or(0),
            sector_size: if p.len() >= 9 {
                u16::from_be_bytes([p[7], p[8]])
            } else {
                256
            },
            mechanical_layout: p.get(9).copied().unwrap_or(0),
            various_info: p.get(10).copied().unwrap_or(0),
        }
    }

    pub fn has_gshift(&self) -> bool {
        (self.mechanical_layout & 0x03) == 0x02
    }

    pub fn first_macro_sector(&self) -> u16 {
        (self.profile_count as u16).max(1) + 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDirectoryEntry {
    pub sector_id: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardMacroOp {
    Noop,
    WaitForRelease,
    RepeatWhilePressed,
    RepeatUntilCanceled,
    DelayMs(u16),
    ButtonDown(u16),
    ButtonUp(u16),
    KeyPress { modifiers: u8, key: u8 },
    KeyRelease { modifiers: u8, key: u8 },
    ConsumerDown(u16),
    ConsumerUp(u16),
    End,
}

impl OnboardMacroOp {
    pub fn encode_into(self, out: &mut Vec<u8>) {
        match self {
            Self::Noop => out.push(0x00),
            Self::WaitForRelease => out.push(0x01),
            Self::RepeatWhilePressed => out.push(0x02),
            Self::RepeatUntilCanceled => out.push(0x03),
            Self::DelayMs(ms) => {
                let [hi, lo] = ms.max(1).to_be_bytes();
                out.extend_from_slice(&[0x40, hi, lo]);
            }
            Self::ButtonDown(mask) => {
                let [hi, lo] = mask.to_be_bytes();
                out.extend_from_slice(&[0x41, hi, lo]);
            }
            Self::ButtonUp(mask) => {
                let [hi, lo] = mask.to_be_bytes();
                out.extend_from_slice(&[0x42, hi, lo]);
            }
            Self::KeyPress { modifiers, key } => {
                out.extend_from_slice(&[0x43, modifiers, key]);
            }
            Self::KeyRelease { modifiers, key } => {
                out.extend_from_slice(&[0x44, modifiers, key]);
            }
            Self::ConsumerDown(usage) => {
                let [hi, lo] = usage.to_be_bytes();
                out.extend_from_slice(&[0x45, hi, lo]);
            }
            Self::ConsumerUp(usage) => {
                let [hi, lo] = usage.to_be_bytes();
                out.extend_from_slice(&[0x46, hi, lo]);
            }
            Self::End => out.push(0xFF),
        }
    }
}

pub fn encode_onboard_macro_bytes(ops: &[OnboardMacroOp], max_len: usize) -> Vec<u8> {
    let limit = max_len.max(1);
    let mut out = Vec::with_capacity(limit.min(256));
    let mut tmp = Vec::with_capacity(4);
    for &op in ops {
        if matches!(op, OnboardMacroOp::End) {
            break;
        }
        tmp.clear();
        op.encode_into(&mut tmp);
        if out.len() + tmp.len() + 1 > limit {
            break;
        }
        out.extend_from_slice(&tmp);
    }
    out.push(0xFF);
    out
}

pub fn build_macro_sector_image(stream: &[u8], sector_size: usize) -> Vec<u8> {
    let len = sector_size.max(16);
    let payload_len = len - 2;
    let mut buf = vec![0xFFu8; len];
    let n = stream.len().min(payload_len);
    buf[..n].copy_from_slice(&stream[..n]);
    if n < payload_len {
        buf[n] = 0xFF;
    } else if let Some(last) = buf[..payload_len].last_mut() {
        *last = 0xFF;
    }
    let crc = crc16_ccitt(&buf[..payload_len]);
    let [hi, lo] = crc.to_be_bytes();
    buf[len - 2] = hi;
    buf[len - 1] = lo;
    buf
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonAction {
    MouseButton(u16),
    Keyboard { modifiers: u8, key: u8 },
    ConsumerKey(u16),
    Special(u8),
    Macro { page: u8, offset: u8 },
    Disabled,
    Inherit,
    Raw([u8; 4]),
}

impl ButtonAction {
    pub fn decode(b: [u8; 4]) -> Self {
        match b {
            [0x00, 0x00, 0x00, 0x00] | [0x80, 0x00, _, _] => Self::Disabled,
            [0xFF, 0xFF, 0xFF, 0xFF] => Self::Inherit,
            [0x00, page, 0x00, offset] => Self::Macro { page, offset },
            [0x80, 0x01, hi, lo] => Self::MouseButton(u16::from_be_bytes([hi, lo])),
            [0x80, 0x02, modifiers, key] => Self::Keyboard { modifiers, key },
            [0x80, 0x03, hi, lo] => Self::ConsumerKey(u16::from_be_bytes([hi, lo])),
            [0x90, code, _, _] => Self::Special(code),
            raw => Self::Raw(raw),
        }
    }

    pub fn encode(self) -> [u8; 4] {
        match self {
            Self::Disabled => [0x80, 0x00, 0x00, 0x00],
            Self::Inherit => [0xFF, 0xFF, 0xFF, 0xFF],
            Self::Macro { page, offset } => [0x00, page, 0x00, offset],
            Self::MouseButton(mask) => {
                let [hi, lo] = mask.to_be_bytes();
                [0x80, 0x01, hi, lo]
            }
            Self::Keyboard { modifiers, key } => [0x80, 0x02, modifiers, key],
            Self::ConsumerKey(usage) => {
                let [hi, lo] = usage.to_be_bytes();
                [0x80, 0x03, hi, lo]
            }
            Self::Special(code) => [0x90, code, 0xFF, 0xFF],
            Self::Raw(r) => r,
        }
    }

    pub fn label(&self) -> String {
        match *self {
            Self::Disabled => "Disabled".into(),
            Self::Inherit => "Default / Inherit".into(),
            Self::Macro { page, offset } => format!("Macro (sec {page} +{offset})"),
            Self::MouseButton(0x0001) => "Left Click".into(),
            Self::MouseButton(0x0002) => "Right Click".into(),
            Self::MouseButton(0x0004) => "Middle Click".into(),
            Self::MouseButton(0x0008) => "Back (Mouse 4)".into(),
            Self::MouseButton(0x0010) => "Forward (Mouse 5)".into(),
            Self::MouseButton(m) => format!("Mouse Button 0x{m:04X}"),
            Self::ConsumerKey(0x00E9) => "Volume Up".into(),
            Self::ConsumerKey(0x00EA) => "Volume Down".into(),
            Self::ConsumerKey(0x00E2) => "Mute".into(),
            Self::ConsumerKey(0x00CD) => "Play / Pause".into(),
            Self::ConsumerKey(0x00B5) => "Next Track".into(),
            Self::ConsumerKey(0x00B6) => "Previous Track".into(),
            Self::ConsumerKey(u) => format!("Media 0x{u:04X}"),
            Self::Special(0x01) => "Scroll Left".into(),
            Self::Special(0x02) => "Scroll Right".into(),
            Self::Special(0x03) => "DPI Up".into(),
            Self::Special(0x04) => "DPI Down".into(),
            Self::Special(0x05) => "DPI Cycle".into(),
            Self::Special(0x06) => "DPI Default".into(),
            Self::Special(0x07) => "DPI Shift (Sniper)".into(),
            Self::Special(0x08) => "Next Profile".into(),
            Self::Special(0x09) => "Previous Profile".into(),
            Self::Special(0x0A) => "Profile Cycle".into(),
            Self::Special(0x0B) => "G-Shift".into(),
            Self::Special(0x0C) => "Battery Check".into(),
            Self::Special(0x10) => "Scroll Down".into(),
            Self::Special(0x11) => "Scroll Up".into(),
            Self::Special(c) => format!("Special 0x{c:02X}"),
            Self::Keyboard { modifiers, key } => format_hid_key(modifiers, key),
            Self::Raw(r) => format!("{:02X} {:02X} {:02X} {:02X}", r[0], r[1], r[2], r[3]),
        }
    }

    pub fn presets() -> Vec<(&'static str, ButtonAction)> {
        vec![
            ("Left Click", Self::MouseButton(0x0001)),
            ("Right Click", Self::MouseButton(0x0002)),
            ("Middle Click", Self::MouseButton(0x0004)),
            ("Back (Mouse 4)", Self::MouseButton(0x0008)),
            ("Forward (Mouse 5)", Self::MouseButton(0x0010)),
            ("DPI Cycle", Self::Special(0x05)),
            ("DPI Up", Self::Special(0x03)),
            ("DPI Down", Self::Special(0x04)),
            ("DPI Shift (Sniper)", Self::Special(0x07)),
            ("Profile Cycle", Self::Special(0x0A)),
            ("G-Shift", Self::Special(0x0B)),
            ("Scroll Up", Self::Special(0x11)),
            ("Scroll Down", Self::Special(0x10)),
            ("Play / Pause", Self::ConsumerKey(0x00CD)),
            ("Volume Up", Self::ConsumerKey(0x00E9)),
            ("Volume Down", Self::ConsumerKey(0x00EA)),
            ("Mute", Self::ConsumerKey(0x00E2)),
            ("Next Track", Self::ConsumerKey(0x00B5)),
            ("Previous Track", Self::ConsumerKey(0x00B6)),
            (
                "Copy (Ctrl+C)",
                Self::Keyboard {
                    modifiers: 0x01,
                    key: 0x06,
                },
            ),
            (
                "Paste (Ctrl+V)",
                Self::Keyboard {
                    modifiers: 0x01,
                    key: 0x19,
                },
            ),
            (
                "Undo (Ctrl+Z)",
                Self::Keyboard {
                    modifiers: 0x01,
                    key: 0x1D,
                },
            ),
            (
                "Alt+Tab",
                Self::Keyboard {
                    modifiers: 0x04,
                    key: 0x2B,
                },
            ),
            (
                "Space",
                Self::Keyboard {
                    modifiers: 0x00,
                    key: 0x2C,
                },
            ),
            (
                "Enter",
                Self::Keyboard {
                    modifiers: 0x00,
                    key: 0x28,
                },
            ),
            (
                "Escape",
                Self::Keyboard {
                    modifiers: 0x00,
                    key: 0x29,
                },
            ),
            ("Disabled", Self::Disabled),
            ("Default / Inherit", Self::Inherit),
        ]
    }
}

fn format_hid_key(modifiers: u8, key: u8) -> String {
    let mut parts = Vec::new();
    if (modifiers & 0x01) != 0 {
        parts.push("Ctrl");
    }
    if (modifiers & 0x02) != 0 {
        parts.push("Shift");
    }
    if (modifiers & 0x04) != 0 {
        parts.push("Alt");
    }
    if (modifiers & 0x08) != 0 {
        parts.push("Super");
    }
    let key_name = match key {
        0x04..=0x1D => {
            let ch = (b'A' + (key - 0x04)) as char;
            return if parts.is_empty() {
                format!("Key {ch}")
            } else {
                format!("{}+{ch}", parts.join("+"))
            };
        }
        0x1E..=0x26 => {
            let ch = (b'1' + (key - 0x1E)) as char;
            return if parts.is_empty() {
                format!("Key {ch}")
            } else {
                format!("{}+{ch}", parts.join("+"))
            };
        }
        0x27 => "0",
        0x28 => "Enter",
        0x29 => "Esc",
        0x2A => "Backspace",
        0x2B => "Tab",
        0x2C => "Space",
        0x3A => "F1",
        0x3B => "F2",
        0x3C => "F3",
        0x3D => "F4",
        0x3E => "F5",
        0x3F => "F6",
        0x40 => "F7",
        0x41 => "F8",
        0x42 => "F9",
        0x43 => "F10",
        0x44 => "F11",
        0x45 => "F12",
        _ => "",
    };
    if key_name.is_empty() {
        format!("HID Key mod=0x{modifiers:02X} code=0x{key:02X}")
    } else if parts.is_empty() {
        key_name.to_string()
    } else {
        format!("{}+{key_name}", parts.join("+"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProfile {
    pub sector_id: u16,
    pub report_rate_ms: u8,
    pub default_dpi_index: u8,
    pub shift_dpi_index: u8,
    pub dpi_stages: [u16; 5],
    pub buttons: Vec<ButtonAction>,
    pub gshift_buttons: Vec<ButtonAction>,
    pub name: String,
    pub active_effect_0: RgbEffect,
    pub active_effect_1: RgbEffect,
    pub powersave_effect_0: RgbEffect,
    pub powersave_effect_1: RgbEffect,
    pub crc_valid: bool,
    pub raw_sector: Vec<u8>,
}

impl OnboardProfile {
    pub fn parse(sector_id: u16, data: &[u8], button_count: usize) -> Result<Self> {
        if data.len() < 255 {
            return Err(HidppError::InvalidArgument(format!(
                "Sector too small: {} bytes (expected >= 255)",
                data.len()
            )));
        }
        let stored_crc = u16::from_be_bytes([data[data.len() - 2], data[data.len() - 1]]);
        let computed_crc = crc16_ccitt(&data[..data.len() - 2]);
        let crc_valid = stored_crc == computed_crc;

        let report_rate_ms = data[0];
        let default_dpi_index = data[1];
        let shift_dpi_index = data[2];
        let mut dpi_stages = [0u16; 5];
        for (i, slot) in dpi_stages.iter_mut().enumerate() {
            let off = 3 + i * 2;
            *slot = u16::from_le_bytes([data[off], data[off + 1]]);
        }

        let nbtn = button_count.clamp(1, 16);
        let mut buttons = Vec::with_capacity(nbtn);
        let mut gshift_buttons = Vec::with_capacity(nbtn);
        for i in 0..nbtn {
            let boff = 0x20 + i * 4;
            let goff = 0x60 + i * 4;
            buttons.push(ButtonAction::decode([
                data[boff],
                data[boff + 1],
                data[boff + 2],
                data[boff + 3],
            ]));
            gshift_buttons.push(ButtonAction::decode([
                data[goff],
                data[goff + 1],
                data[goff + 2],
                data[goff + 3],
            ]));
        }

        let units: Vec<u16> = data[0xA0..0xD0]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0 && u != 0xFFFF)
            .collect();
        let name = String::from_utf16_lossy(&units);

        let active_effect_0 = decode_11byte_effect(&data[0xD0..0xDB]);
        let active_effect_1 = decode_11byte_effect(&data[0xDB..0xE6]);
        let powersave_effect_0 = decode_11byte_effect(&data[0xE6..0xF1]);
        let powersave_effect_1 = decode_11byte_effect(&data[0xF1..0xFC]);

        Ok(Self {
            sector_id,
            report_rate_ms,
            default_dpi_index,
            shift_dpi_index,
            dpi_stages,
            buttons,
            gshift_buttons,
            name,
            active_effect_0,
            active_effect_1,
            powersave_effect_0,
            powersave_effect_1,
            crc_valid,
            raw_sector: data.to_vec(),
        })
    }

    pub fn encode_updated(&self) -> Vec<u8> {
        let mut buf = self.raw_sector.clone();
        buf[0] = self.report_rate_ms;
        buf[1] = self.default_dpi_index;
        buf[2] = self.shift_dpi_index;
        for (i, &dpi) in self.dpi_stages.iter().enumerate() {
            let off = 3 + i * 2;
            let [lo, hi] = dpi.to_le_bytes();
            buf[off] = lo;
            buf[off + 1] = hi;
        }
        for (i, btn) in self.buttons.iter().take(16).enumerate() {
            let off = 0x20 + i * 4;
            buf[off..off + 4].copy_from_slice(&btn.encode());
        }
        for (i, btn) in self.gshift_buttons.iter().take(16).enumerate() {
            let off = 0x60 + i * 4;
            buf[off..off + 4].copy_from_slice(&btn.encode());
        }
        buf[0xA0..0xD0].fill(0x00);
        for (i, u) in self.name.encode_utf16().take(24).enumerate() {
            let off = 0xA0 + i * 2;
            let [lo, hi] = u.to_le_bytes();
            buf[off] = lo;
            buf[off + 1] = hi;
        }
        buf[0xD0..0xDB].copy_from_slice(&encode_11byte_effect(self.active_effect_0));
        buf[0xDB..0xE6].copy_from_slice(&encode_11byte_effect(self.active_effect_1));
        buf[0xE6..0xF1].copy_from_slice(&encode_11byte_effect(self.powersave_effect_0));
        buf[0xF1..0xFC].copy_from_slice(&encode_11byte_effect(self.powersave_effect_1));

        let len = buf.len();
        let crc = crc16_ccitt(&buf[..len - 2]);
        let [hi, lo] = crc.to_be_bytes();
        buf[len - 2] = hi;
        buf[len - 1] = lo;
        buf
    }
}

pub fn parse_profile_directory(sector0: &[u8]) -> Vec<ProfileDirectoryEntry> {
    let mut entries = Vec::new();
    for chunk in sector0.chunks_exact(4) {
        let sector_id = u16::from_be_bytes([chunk[0], chunk[1]]);
        if sector_id == 0xFFFF || sector_id == 0x0000 {
            break;
        }
        entries.push(ProfileDirectoryEntry {
            sector_id,
            enabled: chunk[2] != 0,
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_ccitt_standard_and_g102_sector0() {
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
        let mut sec0 = vec![0xFFu8; 253];
        sec0[0..4].copy_from_slice(&[0x00, 0x01, 0x01, 0x00]);
        assert_eq!(crc16_ccitt(&sec0), 0x991B);
    }

    #[test]
    fn test_button_action_roundtrip() {
        let dpi_cycle = ButtonAction::decode([0x90, 0x05, 0xFF, 0xFF]);
        assert_eq!(dpi_cycle, ButtonAction::Special(0x05));
        assert_eq!(dpi_cycle.encode(), [0x90, 0x05, 0xFF, 0xFF]);
        assert_eq!(dpi_cycle.label(), "DPI Cycle");

        let gshift = ButtonAction::Special(0x0B);
        assert_eq!(gshift.label(), "G-Shift");
        assert_eq!(ButtonAction::decode(gshift.encode()), gshift);

        let mac = ButtonAction::Macro { page: 2, offset: 0 };
        assert_eq!(mac.encode(), [0x00, 0x02, 0x00, 0x00]);
        assert_eq!(ButtonAction::decode(mac.encode()), mac);

        let dis = ButtonAction::Disabled;
        assert_eq!(dis.encode(), [0x80, 0x00, 0x00, 0x00]);
        assert_eq!(ButtonAction::decode(dis.encode()), ButtonAction::Disabled);
    }

    #[test]
    fn test_onboard_macro_encoding() {
        let ops = [
            OnboardMacroOp::ButtonDown(0x0001),
            OnboardMacroOp::DelayMs(25),
            OnboardMacroOp::ButtonUp(0x0001),
            OnboardMacroOp::KeyPress {
                modifiers: 0x01,
                key: 0x00,
            },
            OnboardMacroOp::KeyPress {
                modifiers: 0x00,
                key: 0x06,
            },
            OnboardMacroOp::KeyRelease {
                modifiers: 0x00,
                key: 0x06,
            },
            OnboardMacroOp::KeyRelease {
                modifiers: 0x01,
                key: 0x00,
            },
        ];
        let stream = encode_onboard_macro_bytes(&ops, 253);
        assert_eq!(
            stream,
            vec![
                0x41, 0x00, 0x01, 0x40, 0x00, 0x19, 0x42, 0x00, 0x01, 0x43, 0x01, 0x00, 0x43, 0x00,
                0x06, 0x44, 0x00, 0x06, 0x44, 0x01, 0x00, 0xFF
            ]
        );
        let img = build_macro_sector_image(&stream, 255);
        assert_eq!(img.len(), 255);
        assert_eq!(img[stream.len() - 1], 0xFF);
        let stored_crc = u16::from_be_bytes([img[253], img[254]]);
        assert_eq!(stored_crc, crc16_ccitt(&img[..253]));
    }
}
