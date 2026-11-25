use std::collections::BTreeMap;
use std::ffi::CString;

use hidapi::{HidApi, HidDevice};
use logitux_hidpp::{
    HidppDevice, HidppError, HidppPacket, Result, Transport, DEVICE_INDEX_WIRED, REPORT_LONG,
    REPORT_SHORT, REPORT_VERY_LONG,
};
use serde::{Deserialize, Serialize};

pub const LOGITECH_VID: u16 = 0x046D;
pub const HIDPP_USAGE_PAGE: u16 = 0xFF00;
pub const HIDPP_USAGE_SHORT: u16 = 0x0001;
pub const HIDPP_USAGE_LONG: u16 = 0x0002;
pub const HIDPP_USAGE_VERY_LONG: u16 = 0x0004;

pub fn is_receiver_pid(pid: u16) -> bool {
    matches!(
        pid,
        0xC52B
            | 0xC52F
            | 0xC532
            | 0xC534
            | 0xC537
            | 0xC539
            | 0xC53A
            | 0xC53D
            | 0xC53F
            | 0xC541
            | 0xC545
            | 0xC547
            | 0xC548
            | 0xC54D
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHidppDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_name: String,
    pub manufacturer: String,
    pub serial_number: String,
    pub interface_number: i32,
    pub is_receiver: bool,
    pub short_path: Option<String>,
    pub long_path: Option<String>,
    pub very_long_path: Option<String>,
    pub fallback_path: Option<String>,
}

pub struct HidapiTransport {
    short_dev: Option<HidDevice>,
    long_dev: Option<HidDevice>,
    very_long_dev: Option<HidDevice>,
    device_index: u8,
    last_used_long: std::cell::Cell<bool>,
}

impl HidapiTransport {
    pub fn open(api: &HidApi, desc: &DiscoveredHidppDevice, device_index: u8) -> Result<Self> {
        let open_path = |p: &Option<String>| -> Result<Option<HidDevice>> {
            match p {
                Some(path_str) => {
                    let cstr = CString::new(path_str.as_str())
                        .map_err(|e| HidppError::Transport(format!("Invalid HID path: {e}")))?;
                    let dev = api.open_path(&cstr).map_err(|e| {
                        HidppError::Transport(format!("Failed to open {path_str}: {e}"))
                    })?;
                    Ok(Some(dev))
                }
                None => Ok(None),
            }
        };

        let short_dev = open_path(&desc.short_path)?;
        let long_dev = if desc.long_path == desc.short_path {
            None
        } else {
            open_path(&desc.long_path)?
        };
        let very_long_dev =
            if desc.very_long_path == desc.short_path || desc.very_long_path == desc.long_path {
                None
            } else {
                open_path(&desc.very_long_path)?
            };

        let (short_dev, long_dev) = if short_dev.is_none() && long_dev.is_none() {
            (open_path(&desc.fallback_path)?, None)
        } else {
            (short_dev, long_dev)
        };

        if short_dev.is_none() && long_dev.is_none() {
            return Err(HidppError::Transport(
                "No usable HID++ endpoint found for device".into(),
            ));
        }

        Ok(Self {
            short_dev,
            long_dev,
            very_long_dev,
            device_index,
            last_used_long: std::cell::Cell::new(true),
        })
    }

    fn dev_for_report(&self, report_id: u8) -> Option<&HidDevice> {
        match report_id {
            REPORT_SHORT => self.short_dev.as_ref().or(self.long_dev.as_ref()),
            REPORT_LONG => self.long_dev.as_ref().or(self.short_dev.as_ref()),
            REPORT_VERY_LONG => self
                .very_long_dev
                .as_ref()
                .or(self.long_dev.as_ref())
                .or(self.short_dev.as_ref()),
            _ => self.long_dev.as_ref().or(self.short_dev.as_ref()),
        }
    }
}

impl Transport for HidapiTransport {
    fn write_packet(&self, packet: &HidppPacket) -> Result<()> {
        let dev = self
            .dev_for_report(packet.report_id())
            .ok_or_else(|| HidppError::Transport("No HID handle available".into()))?;
        self.last_used_long.set(packet.report_id() != REPORT_SHORT);
        dev.write(packet.as_bytes())
            .map_err(|e| HidppError::Transport(format!("HID write failed: {e}")))?;
        Ok(())
    }

    fn read_packet(&self, timeout_ms: u32) -> Result<Option<HidppPacket>> {
        let primary = if self.last_used_long.get() {
            self.long_dev.as_ref().or(self.short_dev.as_ref())
        } else {
            self.short_dev.as_ref().or(self.long_dev.as_ref())
        };
        let secondary = if self.last_used_long.get() {
            self.short_dev.as_ref()
        } else {
            self.long_dev.as_ref()
        };

        let mut buf = [0u8; 64];
        if let Some(dev) = primary {
            let n = dev
                .read_timeout(&mut buf, timeout_ms as i32)
                .map_err(|e| HidppError::Transport(format!("HID read failed: {e}")))?;
            if n >= 7 {
                return Ok(Some(HidppPacket::from_raw(&buf[..n])?));
            }
        }
        if let Some(dev) = secondary {
            let sec_timeout = if timeout_ms <= 10 {
                0
            } else {
                (timeout_ms / 4).min(40) as i32
            };
            let n = dev
                .read_timeout(&mut buf, sec_timeout)
                .map_err(|e| HidppError::Transport(format!("HID secondary read failed: {e}")))?;
            if n >= 7 {
                return Ok(Some(HidppPacket::from_raw(&buf[..n])?));
            }
        }
        Ok(None)
    }

    fn supports_short(&self) -> bool {
        self.short_dev.is_some()
    }

    fn supports_long(&self) -> bool {
        self.long_dev.is_some() || self.short_dev.is_some()
    }

    fn device_index(&self) -> u8 {
        self.device_index
    }
}

pub fn enumerate_logitech_devices() -> Result<Vec<DiscoveredHidppDevice>> {
    let api = HidApi::new()
        .map_err(|e| HidppError::Transport(format!("Failed to initialize hidapi: {e}")))?;

    let mut grouped: BTreeMap<(u16, String, i32), DiscoveredHidppDevice> = BTreeMap::new();

    for info in api.device_list() {
        if info.vendor_id() != LOGITECH_VID {
            continue;
        }
        let usage_page = info.usage_page();
        let usage = info.usage();

        let is_hidpp_collection = usage_page == HIDPP_USAGE_PAGE
            || (usage_page >= 0xFF00 && (usage == 1 || usage == 2 || usage == 4))
            || (cfg!(target_os = "linux") && info.interface_number() >= 1);

        if !is_hidpp_collection {
            continue;
        }

        let pid = info.product_id();
        let serial = info.serial_number().unwrap_or_default().to_string();
        let iface = info.interface_number();
        let path_str = info.path().to_string_lossy().to_string();
        let key = (pid, serial.clone(), iface);

        let entry = grouped.entry(key).or_insert_with(|| DiscoveredHidppDevice {
            vendor_id: LOGITECH_VID,
            product_id: pid,
            product_name: info
                .product_string()
                .unwrap_or("Logitech Device")
                .to_string(),
            manufacturer: info.manufacturer_string().unwrap_or("Logitech").to_string(),
            serial_number: serial,
            interface_number: iface,
            is_receiver: is_receiver_pid(pid),
            short_path: None,
            long_path: None,
            very_long_path: None,
            fallback_path: None,
        });

        if usage_page == HIDPP_USAGE_PAGE {
            match usage {
                HIDPP_USAGE_SHORT => entry.short_path = Some(path_str),
                HIDPP_USAGE_LONG => entry.long_path = Some(path_str),
                HIDPP_USAGE_VERY_LONG => entry.very_long_path = Some(path_str),
                _ => {
                    if entry.fallback_path.is_none() {
                        entry.fallback_path = Some(path_str);
                    }
                }
            }
        } else if entry.fallback_path.is_none() {
            entry.fallback_path = Some(path_str);
        }
    }

    Ok(grouped.into_values().collect())
}

pub struct ConnectedDevice {
    pub descriptor: DiscoveredHidppDevice,
    pub device: HidppDevice<HidapiTransport>,
}

pub fn connect_all_devices() -> Result<Vec<ConnectedDevice>> {
    let api = HidApi::new()
        .map_err(|e| HidppError::Transport(format!("Failed to initialize hidapi: {e}")))?;
    let discovered = enumerate_logitech_devices()?;
    let mut connected = Vec::new();

    for desc in discovered {
        if desc.is_receiver {
            for slot in 1..=6u8 {
                if let Ok(transport) = HidapiTransport::open(&api, &desc, slot) {
                    if let Ok(device) = HidppDevice::connect(transport) {
                        connected.push(ConnectedDevice {
                            descriptor: desc.clone(),
                            device,
                        });
                    }
                }
            }
        } else {
            if let Ok(transport) = HidapiTransport::open(&api, &desc, DEVICE_INDEX_WIRED) {
                if let Ok(device) = HidppDevice::connect(transport) {
                    connected.push(ConnectedDevice {
                        descriptor: desc,
                        device,
                    });
                }
            }
        }
    }

    Ok(connected)
}
