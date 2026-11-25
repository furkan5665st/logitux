use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn scaled(&self, brightness_pct: u8) -> Self {
        let s = brightness_pct.min(100) as u16;
        Self {
            r: ((self.r as u16) * s / 100) as u8,
            g: ((self.g as u16) * s / 100) as u8,
            b: ((self.b as u16) * s / 100) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveDirection {
    Horizontal = 1,
    Vertical = 2,
    CenterOut = 3,
    ReverseHorizontal = 6,
}

impl WaveDirection {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Horizontal,
            2 => Self::Vertical,
            3 => Self::CenterOut,
            6 => Self::ReverseHorizontal,
            _ => Self::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RgbEffect {
    Off,
    Fixed {
        color: RgbColor,
        brightness: u8,
    },
    ColorCycle {
        period_ms: u16,
        brightness: u8,
    },
    Breathing {
        color: RgbColor,
        period_ms: u16,
        brightness: u8,
    },
    ColorWave {
        period_ms: u16,
        direction: WaveDirection,
        brightness: u8,
    },
    ColorBlend {
        period_ms: u16,
        brightness: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbEffectDescriptor {
    pub effect_index: u8,
    pub effect_id: u16,
    pub effect_name: String,
    pub capabilities: u16,
    pub default_period_ms: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbClusterInfo {
    pub cluster_index: u8,
    pub location: u16,
    pub location_name: String,
    pub effects: Vec<RgbEffectDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbDeviceInfo {
    pub feature_id: u16,
    pub cluster_count: u8,
    pub extended_flags: u8,
    pub led_bin_info: Option<String>,
    pub clusters: Vec<RgbClusterInfo>,
}

pub fn cluster_location_name(loc: u16) -> &'static str {
    match loc {
        0 => "Primary",
        1 => "Logo",
        2 => "SideStrip",
        3 => "DpiIndicator",
        4 => "ScrollWheel",
        5 => "FrontStrip",
        6 => "BackStrip",
        7 => "AllZones",
        _ => "Zone",
    }
}

pub fn effect_id_name(id: u16) -> &'static str {
    match id {
        0x0000 => "Off",
        0x0001 => "Fixed",
        0x0003 => "ColorCycle",
        0x0004 => "ColorWave",
        0x000A => "Breathing",
        0x000B => "Ripple",
        0x000C | 0x000D => "ColorBlend",
        0x000E => "PerKey",
        _ => "CustomEffect",
    }
}

impl RgbEffect {
    pub fn effect_id(self) -> u16 {
        match self {
            Self::Off => 0x0000,
            Self::Fixed { .. } => 0x0001,
            Self::ColorCycle { .. } => 0x0003,
            Self::ColorWave { .. } => 0x0004,
            Self::Breathing { .. } => 0x000A,
            Self::ColorBlend { .. } => 0x000D,
        }
    }

    pub fn default_8071_index(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::Fixed { .. } => 0x01,
            Self::ColorCycle { .. } => 0x02,
            Self::ColorWave { .. } => 0x03,
            Self::Breathing { .. } => 0x04,
            Self::ColorBlend { .. } => 0x05,
        }
    }
}

pub fn encode_11byte_effect(effect: RgbEffect) -> [u8; 11] {
    let mut b = [0u8; 11];
    match effect {
        RgbEffect::Off => {
            b[0] = 0x00;
        }
        RgbEffect::Fixed { color, brightness } => {
            let c = color.scaled(brightness);
            b[0] = 0x01;
            b[1] = c.r;
            b[2] = c.g;
            b[3] = c.b;
            b[4] = 0x02;
        }
        RgbEffect::ColorCycle {
            period_ms,
            brightness,
        } => {
            let [hi, lo] = period_ms.max(200).to_be_bytes();
            b[0] = 0x03;
            b[6] = hi;
            b[7] = lo;
            b[8] = brightness.clamp(1, 100);
        }
        RgbEffect::ColorWave {
            period_ms,
            direction,
            brightness,
        } => {
            let [hi, lo] = period_ms.max(200).to_be_bytes();
            b[0] = 0x04;
            b[7] = lo;
            b[8] = direction as u8;
            b[9] = brightness.clamp(1, 100);
            b[10] = hi;
        }
        RgbEffect::Breathing {
            color,
            period_ms,
            brightness,
        } => {
            let [hi, lo] = period_ms.max(200).to_be_bytes();
            b[0] = 0x0A;
            b[1] = color.r;
            b[2] = color.g;
            b[3] = color.b;
            b[4] = hi;
            b[5] = lo;
            b[6] = 0x00;
            b[7] = brightness.clamp(1, 100);
        }
        RgbEffect::ColorBlend {
            period_ms,
            brightness,
        } => {
            let [hi, lo] = period_ms.max(200).to_be_bytes();
            b[0] = 0x0C;
            b[6] = hi;
            b[7] = lo;
            b[8] = brightness.clamp(1, 100);
        }
    }
    b
}

pub fn decode_11byte_effect(b: &[u8]) -> RgbEffect {
    if b.len() < 11 {
        return RgbEffect::Off;
    }
    match b[0] {
        0x00 => RgbEffect::Off,
        0x01 => RgbEffect::Fixed {
            color: RgbColor::new(b[1], b[2], b[3]),
            brightness: 100,
        },
        0x03 => RgbEffect::ColorCycle {
            period_ms: u16::from_be_bytes([b[6], b[7]]).max(500),
            brightness: b[8].clamp(1, 100),
        },
        0x04 => RgbEffect::ColorWave {
            period_ms: u16::from_be_bytes([b[10], b[7]]).max(500),
            direction: WaveDirection::from_u8(b[8]),
            brightness: b[9].clamp(1, 100),
        },
        0x0A => RgbEffect::Breathing {
            color: RgbColor::new(b[1], b[2], b[3]),
            period_ms: u16::from_be_bytes([b[4], b[5]]).max(500),
            brightness: b[7].clamp(1, 100),
        },
        0x0C | 0x0D => RgbEffect::ColorBlend {
            period_ms: u16::from_be_bytes([b[6], b[7]]).max(500),
            brightness: b[8].clamp(1, 100),
        },
        _ => RgbEffect::Off,
    }
}

pub fn encode_8071_fn1_with_index(
    cluster_index: u8,
    effect_index: u8,
    effect: RgbEffect,
    persistent: bool,
) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = cluster_index;
    p[1] = effect_index;
    let eff = encode_11byte_effect(effect);
    p[2..12].copy_from_slice(&eff[1..11]);
    p[12] = if persistent { 0x01 } else { 0x00 };
    p
}

pub fn encode_8071_fn1(cluster_index: u8, effect: RgbEffect, persistent: bool) -> [u8; 16] {
    encode_8071_fn1_with_index(
        cluster_index,
        effect.default_8071_index(),
        effect,
        persistent,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_8071_effects_exact_bytes() {
        let fixed = RgbEffect::Fixed {
            color: RgbColor::new(0x00, 0xB8, 0xFC),
            brightness: 100,
        };
        assert_eq!(
            encode_11byte_effect(fixed),
            [0x01, 0x00, 0xB8, 0xFC, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(decode_11byte_effect(&encode_11byte_effect(fixed)), fixed);

        let cycle = RgbEffect::ColorCycle {
            period_ms: 8000,
            brightness: 100,
        };
        assert_eq!(
            encode_11byte_effect(cycle),
            [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x40, 0x64, 0x00, 0x00]
        );
        assert_eq!(decode_11byte_effect(&encode_11byte_effect(cycle)), cycle);

        let breath = RgbEffect::Breathing {
            color: RgbColor::new(0x00, 0xB8, 0xFC),
            period_ms: 3000,
            brightness: 100,
        };
        assert_eq!(
            encode_11byte_effect(breath),
            [0x0A, 0x00, 0xB8, 0xFC, 0x0B, 0xB8, 0x00, 0x64, 0x00, 0x00, 0x00]
        );
        assert_eq!(decode_11byte_effect(&encode_11byte_effect(breath)), breath);

        let wave = RgbEffect::ColorWave {
            period_ms: 5000,
            direction: WaveDirection::Horizontal,
            brightness: 100,
        };
        assert_eq!(
            encode_11byte_effect(wave),
            [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x01, 0x64, 0x13]
        );
        assert_eq!(decode_11byte_effect(&encode_11byte_effect(wave)), wave);

        assert_eq!(encode_8071_fn1(0, cycle, true)[1], 0x02);
        assert_eq!(encode_8071_fn1(0, wave, true)[1], 0x03);
        assert_eq!(encode_8071_fn1(0, breath, true)[1], 0x04);
    }
}
