use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlInfo {
    pub index: u8,
    pub cid: u16,
    pub task_id: u16,
    pub flags: u16,
    pub position: u8,
    pub group: u8,
    pub group_mask: u8,
    pub name: String,
    pub diverted: bool,
    pub remapped_cid: u16,
}

impl ControlInfo {
    pub fn is_mouse_button(&self) -> bool {
        (self.flags & 0x0001) != 0
    }
    pub fn is_reprogrammable(&self) -> bool {
        (self.flags & 0x0010) != 0
    }
    pub fn is_divertable(&self) -> bool {
        (self.flags & 0x0020) != 0
    }
    pub fn is_persistently_divertable(&self) -> bool {
        (self.flags & 0x0040) != 0
    }
}

pub fn cid_name(cid: u16) -> &'static str {
    match cid {
        0x0050 => "Left Click",
        0x0051 => "Right Click",
        0x0052 => "Middle Click",
        0x0053 => "Back (Mouse 4)",
        0x0056 => "Forward (Mouse 5)",
        0x005B => "Scroll Left",
        0x005D => "Scroll Right",
        0x00C3 => "Gesture Button",
        0x00C4 => "SmartShift Mode",
        0x00D7 => "Switch Receiver / Easy-Switch",
        0x00FD => "DPI Switch / Cycle",
        0x00FE => "DPI Down",
        0x00FF => "DPI Up",
        0x0100 => "DPI Shift (Sniper)",
        0x0101 => "Profile Cycle",
        0x0102 => "G-Shift",
        _ => "Button",
    }
}
