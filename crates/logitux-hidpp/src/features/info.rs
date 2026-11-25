use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Keyboard,
    RemoteControl,
    Numpad,
    Mouse,
    Touchpad,
    Trackball,
    Presenter,
    Receiver,
    Headset,
    Unknown(u8),
}

impl From<u8> for DeviceKind {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Keyboard,
            1 => Self::RemoteControl,
            2 => Self::Numpad,
            3 => Self::Mouse,
            4 => Self::Touchpad,
            5 => Self::Trackball,
            6 => Self::Presenter,
            7 => Self::Receiver,
            8 => Self::Headset,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareEntity {
    pub index: u8,
    pub entity_type: u8,
    pub prefix: String,
    pub version: String,
    pub build: u16,
    pub active: bool,
    pub transport_pid: u16,
}

impl FirmwareEntity {
    pub fn type_name(&self) -> &'static str {
        match self.entity_type & 0x0F {
            0 => "MainApplication",
            1 => "Bootloader",
            2 => "Hardware",
            3 => "Touchpad",
            4 => "OpticalSensor",
            5 => "SoftDevice",
            6 => "RfCompanion",
            7 => "FactoryApplication",
            _ => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFwInfo {
    pub entity_count: u8,
    pub unit_id: [u8; 4],
    pub transport_flags: u16,
    pub model_id: [u8; 6],
    pub extended_model_id: u8,
    pub entities: Vec<FirmwareEntity>,
}

impl DeviceFwInfo {
    pub fn unit_id_hex(&self) -> String {
        format!(
            "{:02X}{:02X}{:02X}{:02X}",
            self.unit_id[0], self.unit_id[1], self.unit_id[2], self.unit_id[3]
        )
    }

    pub fn model_id_hex(&self) -> String {
        self.model_id
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    }

    pub fn ghub_firmware_string(&self) -> String {
        if let Some(main_fw) = self
            .entities
            .iter()
            .find(|e| (e.entity_type & 0x0F) == 0)
            .or_else(|| self.entities.first())
        {
            let prefix_digits: String = main_fw
                .prefix
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            let mut parts = main_fw.version.split('.');
            let maj_raw = parts.next().unwrap_or("0").trim_start_matches('0');
            let maj = if maj_raw.is_empty() { "0" } else { maj_raw };
            let min_raw = parts.next().unwrap_or("0").trim_start_matches('0');
            let min = if min_raw.is_empty() { "0" } else { min_raw };
            let bld_hex = format!("{:X}", main_fw.build);
            return format!("{prefix_digits}{maj}.{min}.{bld_hex}");
        }
        "1.0.0".to_string()
    }
}
