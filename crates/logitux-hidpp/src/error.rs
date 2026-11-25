use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Hidpp20ErrorCode {
    NoError = 0x00,
    Unknown = 0x01,
    InvalidArgument = 0x02,
    OutOfRange = 0x03,
    HwError = 0x04,
    LogitechInternal = 0x05,
    InvalidFeatureIndex = 0x06,
    InvalidFunctionId = 0x07,
    Busy = 0x08,
    Unsupported = 0x09,
    UnknownCode(u8),
}

impl From<u8> for Hidpp20ErrorCode {
    fn from(v: u8) -> Self {
        match v {
            0x00 => Self::NoError,
            0x01 => Self::Unknown,
            0x02 => Self::InvalidArgument,
            0x03 => Self::OutOfRange,
            0x04 => Self::HwError,
            0x05 => Self::LogitechInternal,
            0x06 => Self::InvalidFeatureIndex,
            0x07 => Self::InvalidFunctionId,
            0x08 => Self::Busy,
            0x09 => Self::Unsupported,
            other => Self::UnknownCode(other),
        }
    }
}

impl std::fmt::Display for Hidpp20ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoError => write!(f, "NoError (0x00)"),
            Self::Unknown => write!(f, "Unknown (0x01)"),
            Self::InvalidArgument => write!(f, "InvalidArgument (0x02)"),
            Self::OutOfRange => write!(f, "OutOfRange (0x03)"),
            Self::HwError => write!(f, "HwError (0x04)"),
            Self::LogitechInternal => write!(f, "LogitechInternal (0x05)"),
            Self::InvalidFeatureIndex => write!(f, "InvalidFeatureIndex (0x06)"),
            Self::InvalidFunctionId => write!(f, "InvalidFunctionId (0x07)"),
            Self::Busy => write!(f, "Busy (0x08)"),
            Self::Unsupported => write!(f, "Unsupported (0x09)"),
            Self::UnknownCode(c) => write!(f, "UnknownCode (0x{c:02X})"),
        }
    }
}

#[derive(Debug, Error)]
pub enum HidppError {
    #[error("Transport I/O error: {0}")]
    Transport(String),

    #[error("Timeout waiting for HID++ response")]
    Timeout,

    #[error(
        "HID++ 2.0 protocol error on feature index 0x{feature_index:02X}, fn {function_id}: {code}"
    )]
    Protocol20 {
        feature_index: u8,
        function_id: u8,
        code: Hidpp20ErrorCode,
    },

    #[error(
        "HID++ 1.0 protocol error: sub_id=0x{sub_id:02X}, addr=0x{address:02X}, code=0x{code:02X}"
    )]
    Protocol10 { sub_id: u8, address: u8, code: u8 },

    #[error("Feature 0x{0:04X} is not supported by this device")]
    FeatureUnsupported(u16),

    #[error("Device responded with unexpected packet: {0}")]
    UnexpectedResponse(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("CRC-16 mismatch: expected 0x{expected:04X}, computed 0x{computed:04X}")]
    CrcMismatch { expected: u16, computed: u16 },
}

pub type Result<T> = std::result::Result<T, HidppError>;
