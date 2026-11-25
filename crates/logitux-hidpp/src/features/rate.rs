use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRateInfo {
    pub current_hz: u16,
    pub supported_hz: Vec<u16>,
    pub extended_feature: bool,
}

pub fn decode_8060_rate_list(mask: u8) -> Vec<u16> {
    let mut rates = Vec::new();
    for bit in 0..8u8 {
        if (mask & (1 << bit)) != 0 {
            let ms = (bit + 1) as u16;
            rates.push(1000 / ms);
        }
    }
    rates.sort_unstable();
    rates
}

pub fn hz_to_8060_ms(hz: u16) -> Option<u8> {
    match hz {
        1000 => Some(1),
        500 => Some(2),
        333 => Some(3),
        250 => Some(4),
        200 => Some(5),
        166 => Some(6),
        142 => Some(7),
        125 => Some(8),
        _ => None,
    }
}

pub fn ms_to_8060_hz(ms: u8) -> u16 {
    if ms == 0 {
        0
    } else {
        1000 / (ms as u16)
    }
}

pub fn decode_8061_rate_list(mask: u8) -> Vec<u16> {
    const TABLE: [u16; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];
    TABLE
        .iter()
        .enumerate()
        .filter_map(|(i, &hz)| ((mask & (1 << i)) != 0).then_some(hz))
        .collect()
}

pub fn hz_to_8061_index(hz: u16) -> Option<u8> {
    match hz {
        125 => Some(0),
        250 => Some(1),
        500 => Some(2),
        1000 => Some(3),
        2000 => Some(4),
        4000 => Some(5),
        8000 => Some(6),
        _ => None,
    }
}

pub fn index_to_8061_hz(idx: u8) -> u16 {
    match idx {
        0 => 125,
        1 => 250,
        2 => 500,
        3 => 1000,
        4 => 2000,
        5 => 4000,
        6 => 8000,
        _ => 0,
    }
}
