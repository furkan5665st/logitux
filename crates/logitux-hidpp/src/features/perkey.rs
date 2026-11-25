use crate::features::rgb::RgbColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerKeyInfo {
    pub raw: Vec<u8>,
    pub flags: u8,
    pub led_count: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ZoneColor {
    pub zone_id: u8,
    pub color: RgbColor,
}
