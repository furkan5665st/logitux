use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WheelMode {
    FreeSpin,
    Ratchet,
    Unknown(u8),
}

impl From<u8> for WheelMode {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::FreeSpin,
            2 => Self::Ratchet,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartShiftState {
    pub wheel_mode: WheelMode,
    pub auto_disengage: u8,
    pub auto_disengage_default: u8,
    pub torque: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiResWheelState {
    pub multiplier: u8,
    pub has_invert: bool,
    pub has_switch: bool,
    pub hires_enabled: bool,
    pub inverted: bool,
    pub target_hidpp: bool,
}
