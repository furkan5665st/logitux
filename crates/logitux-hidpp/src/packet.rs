use crate::error::{Hidpp20ErrorCode, HidppError, Result};

pub const REPORT_SHORT: u8 = 0x10;
pub const REPORT_LONG: u8 = 0x11;
pub const REPORT_VERY_LONG: u8 = 0x12;

pub const SHORT_LEN: usize = 7;
pub const LONG_LEN: usize = 20;
pub const VERY_LONG_LEN: usize = 64;

pub const DEFAULT_SW_ID: u8 = 0x0A;

pub const DEVICE_INDEX_WIRED: u8 = 0xFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidppPacket {
    bytes: Vec<u8>,
}

impl HidppPacket {
    pub fn short(
        device_index: u8,
        feature_index: u8,
        function_id: u8,
        sw_id: u8,
        params: &[u8],
    ) -> Self {
        let mut buf = vec![0u8; SHORT_LEN];
        buf[0] = REPORT_SHORT;
        buf[1] = device_index;
        buf[2] = feature_index;
        buf[3] = ((function_id & 0x0F) << 4) | (sw_id & 0x0F);
        let n = params.len().min(SHORT_LEN - 4);
        buf[4..4 + n].copy_from_slice(&params[..n]);
        Self { bytes: buf }
    }

    pub fn long(
        device_index: u8,
        feature_index: u8,
        function_id: u8,
        sw_id: u8,
        params: &[u8],
    ) -> Self {
        let mut buf = vec![0u8; LONG_LEN];
        buf[0] = REPORT_LONG;
        buf[1] = device_index;
        buf[2] = feature_index;
        buf[3] = ((function_id & 0x0F) << 4) | (sw_id & 0x0F);
        let n = params.len().min(LONG_LEN - 4);
        buf[4..4 + n].copy_from_slice(&params[..n]);
        Self { bytes: buf }
    }

    pub fn very_long(
        device_index: u8,
        feature_index: u8,
        function_id: u8,
        sw_id: u8,
        params: &[u8],
    ) -> Self {
        let mut buf = vec![0u8; VERY_LONG_LEN];
        buf[0] = REPORT_VERY_LONG;
        buf[1] = device_index;
        buf[2] = feature_index;
        buf[3] = ((function_id & 0x0F) << 4) | (sw_id & 0x0F);
        let n = params.len().min(VERY_LONG_LEN - 4);
        buf[4..4 + n].copy_from_slice(&params[..n]);
        Self { bytes: buf }
    }

    pub fn from_raw(raw: &[u8]) -> Result<Self> {
        if raw.len() < SHORT_LEN {
            return Err(HidppError::UnexpectedResponse(format!(
                "Packet too short: {} bytes",
                raw.len()
            )));
        }
        let expected = match raw[0] {
            REPORT_SHORT => SHORT_LEN,
            REPORT_LONG => LONG_LEN,
            REPORT_VERY_LONG => VERY_LONG_LEN,
            other => {
                return Err(HidppError::UnexpectedResponse(format!(
                    "Unknown HID++ report ID 0x{other:02X}"
                )))
            }
        };
        let mut buf = vec![0u8; expected];
        let copy_len = raw.len().min(expected);
        buf[..copy_len].copy_from_slice(&raw[..copy_len]);
        Ok(Self { bytes: buf })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn report_id(&self) -> u8 {
        self.bytes[0]
    }

    pub fn device_index(&self) -> u8 {
        self.bytes[1]
    }

    pub fn feature_index(&self) -> u8 {
        self.bytes[2]
    }

    pub fn function_id(&self) -> u8 {
        (self.bytes[3] >> 4) & 0x0F
    }

    pub fn sw_id(&self) -> u8 {
        self.bytes[3] & 0x0F
    }

    pub fn params(&self) -> &[u8] {
        &self.bytes[4..]
    }

    pub fn is_hidpp20_error(&self) -> bool {
        self.bytes.len() >= 6 && self.bytes[2] == 0xFF
    }

    pub fn is_hidpp10_error(&self) -> bool {
        self.bytes.len() >= 6 && self.bytes[2] == 0x8F
    }

    pub fn check_error(&self) -> Result<()> {
        if self.is_hidpp20_error() {
            return Err(HidppError::Protocol20 {
                feature_index: self.bytes[3],
                function_id: (self.bytes[4] >> 4) & 0x0F,
                code: Hidpp20ErrorCode::from(self.bytes[5]),
            });
        }
        if self.is_hidpp10_error() {
            return Err(HidppError::Protocol10 {
                sub_id: self.bytes[3],
                address: self.bytes[4],
                code: self.bytes[5],
            });
        }
        Ok(())
    }

    pub fn matches_request(&self, req: &HidppPacket) -> bool {
        let dev_ok = self.device_index() == req.device_index()
            || (req.device_index() == DEVICE_INDEX_WIRED && self.device_index() == 0xFF);
        if !dev_ok {
            return false;
        }
        if self.is_hidpp20_error() {
            return self.bytes[3] == req.feature_index() && self.bytes[4] == req.bytes[3];
        }
        if self.is_hidpp10_error() {
            return self.bytes[3] == req.feature_index() && self.bytes[4] == req.bytes[3];
        }
        self.feature_index() == req.feature_index()
            && self.function_id() == req.function_id()
            && self.sw_id() == req.sw_id()
    }

    pub fn to_hex(&self) -> String {
        self.bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub trait Transport {
    fn write_packet(&self, packet: &HidppPacket) -> Result<()>;
    fn read_packet(&self, timeout_ms: u32) -> Result<Option<HidppPacket>>;
    fn supports_short(&self) -> bool {
        true
    }
    fn supports_long(&self) -> bool {
        true
    }
    fn device_index(&self) -> u8 {
        DEVICE_INDEX_WIRED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_and_long_packet_layout() {
        let p = HidppPacket::short(0xFF, 0x00, 0x01, 0x0A, &[0x00, 0x00, 0x5A]);
        assert_eq!(p.as_bytes(), &[0x10, 0xFF, 0x00, 0x1A, 0x00, 0x00, 0x5A]);
        assert_eq!(p.function_id(), 0x01);
        assert_eq!(p.sw_id(), 0x0A);

        let l = HidppPacket::long(0xFF, 0x05, 0x02, 0x0A, &[0x12, 0x34]);
        assert_eq!(l.as_bytes().len(), 20);
        assert_eq!(l.report_id(), 0x11);
        assert_eq!(l.params()[0], 0x12);
        assert_eq!(l.params()[1], 0x34);
    }

    #[test]
    fn test_error_detection() {
        let req = HidppPacket::short(0xFF, 0x04, 0x03, 0x0A, &[]);
        let err = HidppPacket::from_raw(&[0x10, 0xFF, 0xFF, 0x04, 0x3A, 0x02, 0x00]).unwrap();
        assert!(err.is_hidpp20_error());
        assert!(err.matches_request(&req));
        assert!(matches!(
            err.check_error(),
            Err(HidppError::Protocol20 {
                feature_index: 0x04,
                function_id: 0x03,
                code: Hidpp20ErrorCode::InvalidArgument
            })
        ));
    }
}
