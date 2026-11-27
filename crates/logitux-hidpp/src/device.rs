use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::{HidppError, Result};
use crate::features::{
    battery::{BatteryInfo, ChargingStatus},
    buttons::{cid_name, ControlInfo},
    dpi::{DpiSpec, SensorDpiState},
    info::{DeviceFwInfo, DeviceKind, FirmwareEntity},
    onboard::{
        crc16_ccitt, parse_profile_directory, OnboardDescription, OnboardMode, OnboardProfile,
        ProfileDirectoryEntry,
    },
    perkey::{PerKeyInfo, ZoneColor},
    rate::{
        decode_8060_rate_list, decode_8061_rate_list, hz_to_8060_ms, hz_to_8061_index,
        index_to_8061_hz, ms_to_8060_hz, ReportRateInfo,
    },
    rgb::{
        cluster_location_name, effect_id_name, encode_8071_fn1, encode_8071_fn1_with_index,
        RgbClusterInfo, RgbColor, RgbDeviceInfo, RgbEffect, RgbEffectDescriptor,
    },
    scroll::{HiResWheelState, SmartShiftState, WheelMode},
    FeatureEntry, FeatureId,
};
use crate::packet::{HidppPacket, Transport, DEFAULT_SW_ID};

const DEFAULT_TIMEOUT_MS: u32 = 1000;
const MAX_SKIPPED_EVENTS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub name: String,
    pub kind: DeviceKind,
    pub protocol_version: (u8, u8),
    pub device_index: u8,
    pub fw_info: Option<DeviceFwInfo>,
    pub features: Vec<FeatureEntry>,
    pub dpi: Option<Vec<SensorDpiState>>,
    pub report_rate: Option<ReportRateInfo>,
    pub rgb: Option<RgbDeviceInfo>,
    pub perkey: Option<PerKeyInfo>,
    pub onboard: Option<OnboardDescription>,
    pub onboard_mode: Option<OnboardMode>,
    pub battery: Option<BatteryInfo>,
    pub smartshift: Option<SmartShiftState>,
    pub hires_wheel: Option<HiResWheelState>,
    pub angle_snapping: Option<bool>,
    pub startup_effect: Option<bool>,
    pub buttons: Option<Vec<ControlInfo>>,
}

pub struct HidppDevice<T: Transport> {
    transport: T,
    sw_id: u8,
    protocol_version: (u8, u8),
    features: BTreeMap<u16, FeatureEntry>,
    feature_list: Vec<FeatureEntry>,
    spy_queue: RefCell<VecDeque<u16>>,
    rgb_clusters_cache: RefCell<Option<Vec<RgbClusterInfo>>>,
}

impl<T: Transport> HidppDevice<T> {
    pub fn connect(transport: T) -> Result<Self> {
        let mut dev = Self {
            transport,
            sw_id: DEFAULT_SW_ID,
            protocol_version: (0, 0),
            features: BTreeMap::new(),
            feature_list: Vec::new(),
            spy_queue: RefCell::new(VecDeque::new()),
            rgb_clusters_cache: RefCell::new(None),
        };

        let ping_byte = 0x5A;
        let reply = dev.request_idx(0x00, 1, &[0x00, 0x00, ping_byte])?;
        let p = reply.params();
        if p.len() < 3 || p[2] != ping_byte {
            return Err(HidppError::UnexpectedResponse(format!(
                "Ping echo mismatch: {}",
                reply.to_hex()
            )));
        }
        dev.protocol_version = (p[0], p[1]);

        let root_entry = FeatureEntry {
            index: 0,
            id: FeatureId::IROOT,
            flags: 0,
            version: 0,
        };
        dev.features.insert(FeatureId::IROOT, root_entry.clone());
        dev.feature_list.push(root_entry);

        let fset_reply = dev.request_idx(0x00, 0, &FeatureId::IFEATURE_SET.to_be_bytes())?;
        let fp = fset_reply.params();
        let fset_idx = fp[0];
        if fset_idx != 0 {
            let count_reply = dev.request_idx(fset_idx, 0, &[])?;
            let count = count_reply.params()[0];
            for idx in 1..=count {
                if let Ok(r) = dev.request_idx(fset_idx, 1, &[idx]) {
                    let rp = r.params();
                    if rp.len() >= 3 {
                        let id = u16::from_be_bytes([rp[0], rp[1]]);
                        let flags = rp[2];
                        let version = rp.get(3).copied().unwrap_or(0);
                        let entry = FeatureEntry {
                            index: idx,
                            id,
                            flags,
                            version,
                        };
                        dev.features.insert(id, entry.clone());
                        dev.feature_list.push(entry);
                    }
                }
            }
        }

        Ok(dev)
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn protocol_version(&self) -> (u8, u8) {
        self.protocol_version
    }

    pub fn features(&self) -> &[FeatureEntry] {
        &self.feature_list
    }

    pub fn feature(&self, id: u16) -> Option<&FeatureEntry> {
        self.features.get(&id)
    }

    pub fn has_feature(&self, id: u16) -> bool {
        self.features.contains_key(&id)
    }

    pub fn request_idx(
        &self,
        feature_index: u8,
        function_id: u8,
        params: &[u8],
    ) -> Result<HidppPacket> {
        if let Some(entry) = self.feature_list.iter().find(|f| f.index == feature_index) {
            if entry.is_safety_locked() {
                return Err(HidppError::InvalidArgument(format!(
                    "Safety lock: feature 0x{:04X} ({}) is blocked to protect device firmware",
                    entry.id,
                    entry.name()
                )));
            }
        }

        let dev_idx = self.transport.device_index();
        let req = if params.len() <= 3
            && self.transport.supports_short()
            && !self.transport.supports_long()
        {
            HidppPacket::short(dev_idx, feature_index, function_id, self.sw_id, params)
        } else if self.transport.supports_long() {
            HidppPacket::long(dev_idx, feature_index, function_id, self.sw_id, params)
        } else {
            HidppPacket::short(dev_idx, feature_index, function_id, self.sw_id, params)
        };

        self.transport.write_packet(&req)?;

        for _ in 0..MAX_SKIPPED_EVENTS {
            match self.transport.read_packet(DEFAULT_TIMEOUT_MS)? {
                Some(pkt) => {
                    if pkt.matches_request(&req) {
                        pkt.check_error()?;
                        return Ok(pkt);
                    }
                    if let Some(spy_entry) = self.features.get(&FeatureId::MOUSE_BUTTON_SPY) {
                        if pkt.feature_index() == spy_entry.index && pkt.sw_id() == 0 {
                            let p = pkt.params();
                            if p.len() >= 2 {
                                self.spy_queue
                                    .borrow_mut()
                                    .push_back(u16::from_be_bytes([p[0], p[1]]));
                            }
                        }
                    }
                }
                None => return Err(HidppError::Timeout),
            }
        }
        Err(HidppError::Timeout)
    }

    pub fn request_feature(
        &self,
        feature_id: u16,
        function_id: u8,
        params: &[u8],
    ) -> Result<HidppPacket> {
        let entry = self
            .features
            .get(&feature_id)
            .ok_or(HidppError::FeatureUnsupported(feature_id))?;
        if entry.is_safety_locked() {
            return Err(HidppError::InvalidArgument(format!(
                "Safety lock: feature 0x{feature_id:04X} is blocked"
            )));
        }
        self.request_idx(entry.index, function_id, params)
    }

    pub fn device_name(&self) -> Result<String> {
        if !self.has_feature(FeatureId::DEVICE_NAME) {
            return Err(HidppError::FeatureUnsupported(FeatureId::DEVICE_NAME));
        }
        let len = self
            .request_feature(FeatureId::DEVICE_NAME, 0, &[])?
            .params()[0] as usize;
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let reply = self.request_feature(FeatureId::DEVICE_NAME, 1, &[out.len() as u8])?;
            let chunk = reply.params();
            let remain = len - out.len();
            let take = remain.min(chunk.len());
            out.extend_from_slice(&chunk[..take]);
            if take == 0 {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&out)
            .trim_end_matches('\0')
            .to_string())
    }

    pub fn device_kind(&self) -> Result<DeviceKind> {
        let r = self.request_feature(FeatureId::DEVICE_NAME, 2, &[])?;
        Ok(DeviceKind::from(r.params()[0]))
    }

    pub fn firmware_info(&self) -> Result<DeviceFwInfo> {
        let r = self.request_feature(FeatureId::DEVICE_FW_INFO, 0, &[])?;
        let p = r.params();
        let entity_count = p.first().copied().unwrap_or(0);
        let mut unit_id = [0u8; 4];
        if p.len() >= 5 {
            unit_id.copy_from_slice(&p[1..5]);
        }
        let transport_flags = if p.len() >= 7 {
            u16::from_be_bytes([p[5], p[6]])
        } else {
            0
        };
        let mut model_id = [0u8; 6];
        if p.len() >= 13 {
            model_id.copy_from_slice(&p[7..13]);
        }
        let extended_model_id = p.get(13).copied().unwrap_or(0);

        let mut entities = Vec::new();
        for i in 0..entity_count {
            if let Ok(er) = self.request_feature(FeatureId::DEVICE_FW_INFO, 1, &[i]) {
                let ep = er.params();
                if ep.len() >= 11 {
                    let prefix = String::from_utf8_lossy(&ep[1..4])
                        .trim_end_matches('\0')
                        .to_string();
                    let version = format!("{:02X}.{:02X}", ep[4], ep[5]);
                    let build = u16::from_be_bytes([ep[6], ep[7]]);
                    let active = (ep[8] & 0x01) != 0;
                    let transport_pid = u16::from_be_bytes([ep[9], ep[10]]);
                    entities.push(FirmwareEntity {
                        index: i,
                        entity_type: ep[0],
                        prefix,
                        version,
                        build,
                        active,
                        transport_pid,
                    });
                }
            }
        }

        Ok(DeviceFwInfo {
            entity_count,
            unit_id,
            transport_flags,
            model_id,
            extended_model_id,
            entities,
        })
    }

    pub fn get_dpi_states(&self) -> Result<Vec<SensorDpiState>> {
        if self.has_feature(FeatureId::ADJUSTABLE_DPI) {
            let count = self
                .request_feature(FeatureId::ADJUSTABLE_DPI, 0, &[])?
                .params()[0];
            let mut sensors = Vec::new();
            for s in 0..count {
                let list_r = self.request_feature(FeatureId::ADJUSTABLE_DPI, 1, &[s])?;
                let spec = DpiSpec::from_2201_params(list_r.params());
                let cur_r = self.request_feature(FeatureId::ADJUSTABLE_DPI, 2, &[s])?;
                let cp = cur_r.params();
                let current_dpi = if cp.len() >= 3 {
                    u16::from_be_bytes([cp[1], cp[2]])
                } else {
                    0
                };
                let default_dpi = if cp.len() >= 5 {
                    u16::from_be_bytes([cp[3], cp[4]])
                } else {
                    current_dpi
                };
                sensors.push(SensorDpiState {
                    sensor_index: s,
                    current_dpi,
                    current_dpi_y: None,
                    default_dpi,
                    spec,
                    lod: None,
                });
            }
            return Ok(sensors);
        }

        if self.has_feature(FeatureId::EXTENDED_ADJUSTABLE_DPI) {
            let count = self
                .request_feature(FeatureId::EXTENDED_ADJUSTABLE_DPI, 0, &[])?
                .params()[0];
            let mut sensors = Vec::new();
            for s in 0..count {
                let list_r = self.request_feature(FeatureId::EXTENDED_ADJUSTABLE_DPI, 2, &[s])?;
                let spec = DpiSpec::from_2201_params(list_r.params());
                let cur_r = self.request_feature(FeatureId::EXTENDED_ADJUSTABLE_DPI, 5, &[s])?;
                let cp = cur_r.params();
                let current_dpi = if cp.len() >= 3 {
                    u16::from_be_bytes([cp[1], cp[2]])
                } else {
                    800
                };
                let current_dpi_y = if cp.len() >= 5 {
                    let y = u16::from_be_bytes([cp[3], cp[4]]);
                    (y > 0).then_some(y)
                } else {
                    None
                };
                let lod = cp.get(5).copied();
                sensors.push(SensorDpiState {
                    sensor_index: s,
                    current_dpi,
                    current_dpi_y,
                    default_dpi: current_dpi,
                    spec,
                    lod,
                });
            }
            return Ok(sensors);
        }

        Err(HidppError::FeatureUnsupported(FeatureId::ADJUSTABLE_DPI))
    }

    pub fn set_dpi(&self, sensor_index: u8, dpi: u16) -> Result<u16> {
        if self.has_feature(FeatureId::ADJUSTABLE_DPI) {
            let [hi, lo] = dpi.to_be_bytes();
            self.request_feature(FeatureId::ADJUSTABLE_DPI, 3, &[sensor_index, hi, lo])?;
            return Ok(dpi);
        }
        if self.has_feature(FeatureId::EXTENDED_ADJUSTABLE_DPI) {
            let [hi, lo] = dpi.to_be_bytes();
            self.request_feature(
                FeatureId::EXTENDED_ADJUSTABLE_DPI,
                6,
                &[sensor_index, hi, lo, hi, lo, 0],
            )?;
            return Ok(dpi);
        }
        Err(HidppError::FeatureUnsupported(FeatureId::ADJUSTABLE_DPI))
    }

    pub fn get_report_rate(&self) -> Result<ReportRateInfo> {
        if self.has_feature(FeatureId::EXTENDED_REPORT_RATE) {
            let list_r = self.request_feature(FeatureId::EXTENDED_REPORT_RATE, 1, &[])?;
            let supported_hz = decode_8061_rate_list(list_r.params()[0]);
            let cur_r = self.request_feature(FeatureId::EXTENDED_REPORT_RATE, 2, &[])?;
            let current_hz = index_to_8061_hz(cur_r.params()[0]);
            return Ok(ReportRateInfo {
                current_hz,
                supported_hz,
                extended_feature: true,
            });
        }
        if self.has_feature(FeatureId::REPORT_RATE) {
            let list_r = self.request_feature(FeatureId::REPORT_RATE, 0, &[])?;
            let supported_hz = decode_8060_rate_list(list_r.params()[0]);
            let cur_r = self.request_feature(FeatureId::REPORT_RATE, 1, &[])?;
            let current_hz = ms_to_8060_hz(cur_r.params()[0]);
            return Ok(ReportRateInfo {
                current_hz,
                supported_hz,
                extended_feature: false,
            });
        }
        Err(HidppError::FeatureUnsupported(FeatureId::REPORT_RATE))
    }

    pub fn set_report_rate(&self, hz: u16) -> Result<()> {
        if self.has_feature(FeatureId::EXTENDED_REPORT_RATE) {
            if self.has_feature(FeatureId::ONBOARD_PROFILES) {
                let _ = self.set_onboard_mode(OnboardMode::Host);
            }
            let idx = hz_to_8061_index(hz).ok_or_else(|| {
                HidppError::InvalidArgument(format!("Unsupported extended polling rate: {hz} Hz"))
            })?;
            self.request_feature(FeatureId::EXTENDED_REPORT_RATE, 3, &[idx])?;
            return Ok(());
        }
        if self.has_feature(FeatureId::REPORT_RATE) {
            let ms = hz_to_8060_ms(hz).ok_or_else(|| {
                HidppError::InvalidArgument(format!(
                    "Unsupported polling rate: {hz} Hz (use 125, 250, 500, or 1000)"
                ))
            })?;
            if self.has_feature(FeatureId::ONBOARD_PROFILES) {
                let _ = self.set_onboard_mode(OnboardMode::Host);
            }
            self.request_feature(FeatureId::REPORT_RATE, 2, &[ms])?;
            return Ok(());
        }
        Err(HidppError::FeatureUnsupported(FeatureId::REPORT_RATE))
    }

    pub fn get_rgb_info(&self) -> Result<RgbDeviceInfo> {
        if self.has_feature(FeatureId::RGB_EFFECTS) {
            let fid = FeatureId::RGB_EFFECTS;
            let info_r = self.request_feature(fid, 0, &[0xFF, 0xFF, 0x00])?;
            let ip = info_r.params();
            let extended_flags = ip.get(1).copied().unwrap_or(0);
            let cluster_count = ip.get(2).copied().unwrap_or(0);

            let mut clusters = Vec::new();
            for c in 0..cluster_count {
                let cr = self.request_feature(fid, 0, &[c, 0xFF, 0x00])?;
                let cp = cr.params();
                let location = if cp.len() >= 4 {
                    u16::from_be_bytes([cp[2], cp[3]])
                } else {
                    0
                };
                let effect_count = cp.get(4).copied().unwrap_or(0);
                let effect_persistency = cp.get(6).copied().unwrap_or(0);
                let mut effects = Vec::new();
                for e in 0..effect_count {
                    if let Ok(er) = self.request_feature(fid, 0, &[c, e, 0x00]) {
                        let ep = er.params();
                        if ep.len() >= 8 {
                            let effect_id = u16::from_be_bytes([ep[2], ep[3]]);
                            let capabilities = u16::from_be_bytes([ep[4], ep[5]]);
                            let default_period_ms = u16::from_be_bytes([ep[6], ep[7]]);
                            effects.push(RgbEffectDescriptor {
                                effect_index: e,
                                effect_id,
                                effect_name: effect_id_name(effect_id).to_string(),
                                capabilities,
                                default_period_ms,
                            });
                        }
                    }
                }
                clusters.push(RgbClusterInfo {
                    cluster_index: c,
                    location,
                    location_name: cluster_location_name(location).to_string(),
                    effect_persistency,
                    effects,
                });
            }

            *self.rgb_clusters_cache.borrow_mut() = Some(clusters.clone());

            let led_bin_info = self.request_feature(fid, 4, &[0x00]).ok().and_then(|r| {
                let p = r.params();
                if p.len() >= 6 {
                    let s = String::from_utf8_lossy(&p[3..])
                        .trim_end_matches('\0')
                        .to_string();
                    (!s.is_empty()).then_some(s)
                } else {
                    None
                }
            });

            return Ok(RgbDeviceInfo {
                feature_id: fid,
                cluster_count,
                extended_flags,
                led_bin_info,
                clusters,
            });
        }

        if self.has_feature(FeatureId::COLOR_LED_EFFECTS) {
            let fid = FeatureId::COLOR_LED_EFFECTS;
            let info_r = self.request_feature(fid, 0, &[])?;
            let ip = info_r.params();
            let cluster_count = ip.first().copied().unwrap_or(0);
            let extended_flags = ip.get(1).copied().unwrap_or(0);
            let mut clusters = Vec::new();
            for c in 0..cluster_count {
                let cr = self.request_feature(fid, 1, &[c])?;
                let cp = cr.params();
                let location = if cp.len() >= 3 {
                    u16::from_be_bytes([cp[1], cp[2]])
                } else {
                    0
                };
                let effect_count = cp.get(3).copied().unwrap_or(0);
                let mut effects = Vec::new();
                for e in 0..effect_count {
                    if let Ok(er) = self.request_feature(fid, 2, &[c, e]) {
                        let ep = er.params();
                        if ep.len() >= 8 {
                            let effect_id = u16::from_be_bytes([ep[2], ep[3]]);
                            let capabilities = u16::from_be_bytes([ep[4], ep[5]]);
                            let default_period_ms = u16::from_be_bytes([ep[6], ep[7]]);
                            effects.push(RgbEffectDescriptor {
                                effect_index: e,
                                effect_id,
                                effect_name: effect_id_name(effect_id).to_string(),
                                capabilities,
                                default_period_ms,
                            });
                        }
                    }
                }
                clusters.push(RgbClusterInfo {
                    cluster_index: c,
                    location,
                    location_name: cluster_location_name(location).to_string(),
                    effect_persistency: cp.get(4).copied().unwrap_or(1),
                    effects,
                });
            }
            return Ok(RgbDeviceInfo {
                feature_id: fid,
                cluster_count,
                extended_flags,
                led_bin_info: None,
                clusters,
            });
        }

        Err(HidppError::FeatureUnsupported(FeatureId::RGB_EFFECTS))
    }

    fn resolve_8071_effect_index(
        &self,
        cluster_index: u8,
        target_effect_id: u16,
        fallback_index: u8,
    ) -> (u8, bool, usize) {
        if self.rgb_clusters_cache.borrow().is_none() {
            let _ = self.get_rgb_info();
        }
        if let Some(clusters) = self.rgb_clusters_cache.borrow().as_ref() {
            let count = clusters.len();
            if let Some(cl) = clusters.iter().find(|c| c.cluster_index == cluster_index) {
                let persist_ok = (cl.effect_persistency & 0x01) != 0;
                if let Some(desc) = cl.effects.iter().find(|d| {
                    d.effect_id == target_effect_id
                        || (target_effect_id == 0x0003 && d.effect_id == 0x0015)
                        || (target_effect_id == 0x0004 && d.effect_id == 0x0016)
                        || (target_effect_id == 0x000E && d.effect_id == 0x000C)
                }) {
                    return (desc.effect_index, persist_ok, count);
                }
                return (fallback_index, persist_ok, count);
            }
            return (fallback_index, false, count);
        }
        (fallback_index, false, 1)
    }

    pub fn supports_rgb_effect(&self, effect_id: u16) -> bool {
        if self.rgb_clusters_cache.borrow().is_none() {
            let _ = self.get_rgb_info();
        }
        if let Some(clusters) = self.rgb_clusters_cache.borrow().as_ref() {
            return clusters
                .iter()
                .any(|cl| cl.effects.iter().any(|e| e.effect_id == effect_id));
        }
        false
    }

    pub fn set_rgb_sw_control(&self, software_controlled: bool) -> Result<()> {
        if self.has_feature(FeatureId::RGB_EFFECTS) {
            if software_controlled {
                let _ = self.request_feature(FeatureId::RGB_EFFECTS, 5, &[0x01, 0x03, 0x04]);
                self.ensure_rgb_powered();
            } else {
                let _ = self.request_feature(FeatureId::RGB_EFFECTS, 5, &[0x01, 0x00, 0x00]);
            }
            return Ok(());
        }
        if self.has_feature(FeatureId::COLOR_LED_EFFECTS) {
            return Ok(());
        }
        Err(HidppError::FeatureUnsupported(FeatureId::RGB_EFFECTS))
    }

    pub fn ensure_rgb_powered(&self) {
        if !self.has_feature(FeatureId::RGB_EFFECTS) {
            return;
        }
        let on = self
            .request_feature(FeatureId::RGB_EFFECTS, 8, &[0x00])
            .map(|r| r.params().get(1).copied() == Some(0x01))
            .unwrap_or(false);
        if !on {
            let _ = self.request_feature(FeatureId::RGB_EFFECTS, 8, &[0x01, 0x01]);
        }
    }

    pub fn get_startup_effect(&self) -> Result<bool> {
        if !self.has_feature(FeatureId::RGB_EFFECTS) {
            return Err(HidppError::FeatureUnsupported(FeatureId::RGB_EFFECTS));
        }
        let r = self.request_feature(FeatureId::RGB_EFFECTS, 3, &[0x00, 0x00, 0x01])?;
        let p = r.params();
        if p.len() >= 4 {
            Ok(p[3] == 0x01)
        } else {
            Ok(false)
        }
    }

    pub fn set_startup_effect(&self, enabled: bool) -> Result<()> {
        if !self.has_feature(FeatureId::RGB_EFFECTS) {
            return Err(HidppError::FeatureUnsupported(FeatureId::RGB_EFFECTS));
        }
        let state_byte = if enabled { 0x01 } else { 0x02 };
        for cfg_lo in [0x01u8, 0x02u8] {
            if let Ok(cur) = self.request_feature(FeatureId::RGB_EFFECTS, 3, &[0x00, 0x00, cfg_lo])
            {
                let cp = cur.params();
                let mut payload = [0u8; 10];
                payload[0] = 0x01;
                payload[1] = 0x00;
                payload[2] = cfg_lo;
                payload[3] = state_byte;
                for i in 4..10 {
                    payload[i] = cp.get(i).copied().unwrap_or(0);
                }
                if cfg_lo == 0x01 {
                    self.request_feature(FeatureId::RGB_EFFECTS, 3, &payload)?;
                } else {
                    let _ = self.request_feature(FeatureId::RGB_EFFECTS, 3, &payload);
                }
            }
        }
        Ok(())
    }

    pub fn set_rgb_effect(
        &self,
        cluster_index: u8,
        effect: RgbEffect,
        persistent: bool,
    ) -> Result<()> {
        if self.has_feature(FeatureId::RGB_EFFECTS) {
            if self.has_feature(FeatureId::ONBOARD_PROFILES) {
                let _ = self.set_onboard_mode(OnboardMode::Host);
            }
            let _ = self.set_rgb_sw_control(true);
            let (eff_idx, persist_ok, cluster_count) = self.resolve_8071_effect_index(
                cluster_index,
                effect.effect_id(),
                effect.default_8071_index(),
            );
            let params = encode_8071_fn1_with_index(
                cluster_index,
                eff_idx,
                effect,
                persistent && persist_ok,
            );
            self.request_feature(FeatureId::RGB_EFFECTS, 1, &params)?;
            if cluster_index == 0 && cluster_count > 1 {
                let (eff_idx1, persist_ok1, _) = self.resolve_8071_effect_index(
                    1,
                    effect.effect_id(),
                    effect.default_8071_index(),
                );
                let params1 = encode_8071_fn1_with_index(
                    1,
                    eff_idx1,
                    effect,
                    persistent && persist_ok1,
                );
                let _ = self.request_feature(FeatureId::RGB_EFFECTS, 1, &params1);
            }
            return Ok(());
        }
        if self.has_feature(FeatureId::COLOR_LED_EFFECTS) {
            let params = encode_8071_fn1(cluster_index, effect, persistent);
            self.request_feature(FeatureId::COLOR_LED_EFFECTS, 3, &params)?;
            return Ok(());
        }
        Err(HidppError::FeatureUnsupported(FeatureId::RGB_EFFECTS))
    }

    pub fn get_perkey_info(&self) -> Result<PerKeyInfo> {
        let r = self.request_feature(FeatureId::PER_KEY_LIGHTING, 0, &[])?;
        let p = r.params();
        Ok(PerKeyInfo {
            raw: p.to_vec(),
            flags: p.first().copied().unwrap_or(0),
            led_count: p.get(1).copied().unwrap_or(0),
        })
    }

    pub fn set_perkey_zones(&self, zones: &[ZoneColor], persist: bool) -> Result<()> {
        if !self.has_feature(FeatureId::PER_KEY_LIGHTING) {
            return Err(HidppError::FeatureUnsupported(FeatureId::PER_KEY_LIGHTING));
        }
        if self.has_feature(FeatureId::ONBOARD_PROFILES) {
            let _ = self.set_onboard_mode(OnboardMode::Host);
        }
        let _ = self.set_rgb_sw_control(true);
        for chunk in zones.chunks(4) {
            let mut params = [0u8; 16];
            for (i, z) in chunk.iter().enumerate() {
                let off = i * 4;
                params[off] = z.zone_id;
                params[off + 1] = z.color.r;
                params[off + 2] = z.color.g;
                params[off + 3] = z.color.b;
            }
            if chunk.len() < 4 {
                params[chunk.len() * 4] = 0xFF;
            }
            self.request_feature(FeatureId::PER_KEY_LIGHTING, 1, &params)?;
        }
        let persist_byte = if persist { 0x01 } else { 0x00 };
        self.request_feature(
            FeatureId::PER_KEY_LIGHTING,
            7,
            &[persist_byte, 0x00, 0x00, 0x00, 0x00],
        )?;
        Ok(())
    }

    pub fn set_perkey_single_color(
        &self,
        zone_ids: &[u8],
        color: RgbColor,
        persist: bool,
    ) -> Result<()> {
        let zones: Vec<ZoneColor> = zone_ids
            .iter()
            .map(|&zone_id| ZoneColor { zone_id, color })
            .collect();
        self.set_perkey_zones(&zones, persist)
    }

    pub fn get_onboard_description(&self) -> Result<OnboardDescription> {
        let r = self.request_feature(FeatureId::ONBOARD_PROFILES, 0, &[])?;
        Ok(OnboardDescription::from_params(r.params()))
    }

    pub fn get_onboard_mode(&self) -> Result<OnboardMode> {
        let r = self.request_feature(FeatureId::ONBOARD_PROFILES, 2, &[])?;
        Ok(OnboardMode::from(r.params()[0]))
    }

    pub fn set_onboard_mode(&self, mode: OnboardMode) -> Result<()> {
        let val = match mode {
            OnboardMode::Onboard => 1,
            OnboardMode::Host => 2,
            OnboardMode::Unknown(v) => v,
        };
        self.request_feature(FeatureId::ONBOARD_PROFILES, 1, &[val])?;
        Ok(())
    }

    pub fn get_active_onboard_profile(&self) -> Result<u16> {
        let r = self.request_feature(FeatureId::ONBOARD_PROFILES, 4, &[])?;
        let p = r.params();
        Ok(u16::from_be_bytes([p[0], p[1]]))
    }

    pub fn set_active_onboard_profile(&self, sector_id: u16) -> Result<()> {
        let [hi, lo] = sector_id.to_be_bytes();
        self.request_feature(FeatureId::ONBOARD_PROFILES, 3, &[hi, lo])?;
        Ok(())
    }

    pub fn get_active_onboard_dpi_index(&self) -> Result<u8> {
        let r = self.request_feature(FeatureId::ONBOARD_PROFILES, 11, &[])?;
        Ok(r.params()[0])
    }

    pub fn set_active_onboard_dpi_index(&self, dpi_index: u8) -> Result<()> {
        self.request_feature(FeatureId::ONBOARD_PROFILES, 12, &[dpi_index])?;
        Ok(())
    }

    pub fn read_onboard_sector(&self, sector_id: u16, sector_size: u16) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(sector_size as usize);
        let [s_hi, s_lo] = sector_id.to_be_bytes();
        let mut offset: u16 = 0;
        while offset < sector_size {
            let remaining = sector_size - offset;
            if remaining >= 16 {
                let [o_hi, o_lo] = offset.to_be_bytes();
                let r = self.request_feature(
                    FeatureId::ONBOARD_PROFILES,
                    5,
                    &[s_hi, s_lo, o_hi, o_lo],
                )?;
                let chunk = r.params();
                let take = 16.min(chunk.len());
                data.extend_from_slice(&chunk[..take]);
                offset += take as u16;
                if take == 0 {
                    break;
                }
            } else {
                let aligned_off = sector_size.saturating_sub(16);
                let skip = (offset - aligned_off) as usize;
                let [o_hi, o_lo] = aligned_off.to_be_bytes();
                let r = self.request_feature(
                    FeatureId::ONBOARD_PROFILES,
                    5,
                    &[s_hi, s_lo, o_hi, o_lo],
                )?;
                let chunk = r.params();
                if skip < chunk.len() {
                    let take = (remaining as usize).min(chunk.len() - skip);
                    data.extend_from_slice(&chunk[skip..skip + take]);
                }
                break;
            }
        }
        Ok(data)
    }

    pub fn write_onboard_sector(&self, sector_id: u16, data: &[u8]) -> Result<()> {
        if sector_id == 0 {
            return Err(HidppError::InvalidArgument(
                "Refusing to overwrite sector 0 (profile directory) directly".into(),
            ));
        }
        if data.len() < 16 {
            return Err(HidppError::InvalidArgument(format!(
                "Sector data too short: {} bytes",
                data.len()
            )));
        }
        let stored_crc = u16::from_be_bytes([data[data.len() - 2], data[data.len() - 1]]);
        let computed_crc = crc16_ccitt(&data[..data.len() - 2]);
        if stored_crc != computed_crc {
            return Err(HidppError::CrcMismatch {
                expected: stored_crc,
                computed: computed_crc,
            });
        }

        let [s_hi, s_lo] = sector_id.to_be_bytes();
        let [c_hi, c_lo] = (data.len() as u16).to_be_bytes();
        self.request_feature(
            FeatureId::ONBOARD_PROFILES,
            6,
            &[s_hi, s_lo, 0x00, 0x00, c_hi, c_lo],
        )?;
        for chunk in data.chunks(16) {
            if chunk.len() == 16 {
                self.request_feature(FeatureId::ONBOARD_PROFILES, 7, chunk)?;
            } else {
                let mut padded = [0xFFu8; 16];
                padded[..chunk.len()].copy_from_slice(chunk);
                self.request_feature(FeatureId::ONBOARD_PROFILES, 7, &padded)?;
            }
        }
        self.request_feature(FeatureId::ONBOARD_PROFILES, 8, &[])?;
        Ok(())
    }

    pub fn write_onboard_sector_verified(&self, sector_id: u16, data: &[u8]) -> Result<Vec<u8>> {
        let sector_size = data.len() as u16;
        for attempt in 0..2 {
            self.write_onboard_sector(sector_id, data)?;
            let readback = self.read_onboard_sector(sector_id, sector_size)?;
            if readback == data {
                return Ok(readback);
            }
            if attempt == 0 {
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
        Err(HidppError::InvalidArgument(format!(
            "Onboard sector 0x{sector_id:04X} readback mismatch after write"
        )))
    }

    pub fn write_onboard_raw_sector(&self, sector_id: u16, data: &[u8]) -> Result<()> {
        if sector_id < 2 {
            return Err(HidppError::InvalidArgument(
                "Refusing to write raw macro data into directory or primary profile sector".into(),
            ));
        }
        self.write_onboard_sector(sector_id, data)
    }

    pub fn read_onboard_directory(&self) -> Result<Vec<ProfileDirectoryEntry>> {
        let desc = self.get_onboard_description()?;
        let sec0 = self.read_onboard_sector(0, desc.sector_size)?;
        Ok(parse_profile_directory(&sec0))
    }

    pub fn read_onboard_profile(&self, sector_id: u16) -> Result<OnboardProfile> {
        let desc = self.get_onboard_description()?;
        let raw = self.read_onboard_sector(sector_id, desc.sector_size)?;
        OnboardProfile::parse(sector_id, &raw, desc.button_count as usize)
    }

    pub fn get_button_spy_count(&self) -> Result<u8> {
        let r = self.request_feature(FeatureId::MOUSE_BUTTON_SPY, 0, &[])?;
        Ok(r.params().first().copied().unwrap_or(0))
    }

    pub fn start_button_spy(&self) -> Result<()> {
        self.request_feature(FeatureId::MOUSE_BUTTON_SPY, 1, &[])?;
        Ok(())
    }

    pub fn stop_button_spy(&self) -> Result<()> {
        self.request_feature(FeatureId::MOUSE_BUTTON_SPY, 2, &[])?;
        Ok(())
    }

    pub fn get_button_spy_remapping(&self) -> Result<[u8; 16]> {
        let r = self.request_feature(FeatureId::MOUSE_BUTTON_SPY, 3, &[])?;
        let mut out = [0u8; 16];
        let p = r.params();
        let n = p.len().min(16);
        out[..n].copy_from_slice(&p[..n]);
        Ok(out)
    }

    pub fn set_button_spy_remapping(&self, map: &[u8; 16]) -> Result<()> {
        self.request_feature(FeatureId::MOUSE_BUTTON_SPY, 4, map)?;
        Ok(())
    }

    pub fn reset_button_spy_remapping(&self) -> Result<()> {
        let mut map = [0u8; 16];
        map[0] = 0x01;
        map[1] = 0x02;
        map[2] = 0x03;
        map[3] = 0x04;
        map[4] = 0x05;
        self.set_button_spy_remapping(&map)
    }

    pub fn poll_button_spy_event(&self, timeout_ms: u32) -> Result<Option<u16>> {
        if let Some(queued) = self.spy_queue.borrow_mut().pop_front() {
            return Ok(Some(queued));
        }
        let spy_idx = match self.feature(FeatureId::MOUSE_BUTTON_SPY) {
            Some(e) => e.index,
            None => return Ok(None),
        };
        if let Some(pkt) = self.transport.read_packet(timeout_ms)? {
            if pkt.feature_index() == spy_idx && pkt.sw_id() == 0 {
                let p = pkt.params();
                if p.len() >= 2 {
                    return Ok(Some(u16::from_be_bytes([p[0], p[1]])));
                }
            }
        }
        Ok(None)
    }

    pub fn get_battery(&self) -> Result<BatteryInfo> {
        if self.has_feature(FeatureId::UNIFIED_BATTERY) {
            let r = self.request_feature(FeatureId::UNIFIED_BATTERY, 1, &[])?;
            let p = r.params();
            let status = match p.get(2).copied().unwrap_or(0) {
                0 => ChargingStatus::Discharging,
                1 => ChargingStatus::Recharging,
                2 => ChargingStatus::SlowRecharge,
                3 => ChargingStatus::Full,
                other => ChargingStatus::Unknown(other),
            };
            return Ok(BatteryInfo {
                percentage: Some(p[0]),
                voltage_mv: None,
                status,
                feature_id: FeatureId::UNIFIED_BATTERY,
            });
        }
        if self.has_feature(FeatureId::BATTERY_STATUS) {
            let r = self.request_feature(FeatureId::BATTERY_STATUS, 0, &[])?;
            let p = r.params();
            let status = match p.get(2).copied().unwrap_or(0) {
                0 => ChargingStatus::Discharging,
                1 => ChargingStatus::Recharging,
                2 => ChargingStatus::AlmostFull,
                3 => ChargingStatus::Full,
                4 => ChargingStatus::SlowRecharge,
                5 => ChargingStatus::InvalidBattery,
                6 => ChargingStatus::ThermalError,
                other => ChargingStatus::Unknown(other),
            };
            return Ok(BatteryInfo {
                percentage: Some(p[0]),
                voltage_mv: None,
                status,
                feature_id: FeatureId::BATTERY_STATUS,
            });
        }
        if self.has_feature(FeatureId::BATTERY_VOLTAGE) {
            let r = self.request_feature(FeatureId::BATTERY_VOLTAGE, 0, &[])?;
            let p = r.params();
            let mv = u16::from_be_bytes([p[0], p[1]]);
            let flags = p.get(2).copied().unwrap_or(0);
            let status = if (flags & 0x80) != 0 {
                ChargingStatus::Recharging
            } else {
                ChargingStatus::Discharging
            };
            return Ok(BatteryInfo {
                percentage: None,
                voltage_mv: Some(mv),
                status,
                feature_id: FeatureId::BATTERY_VOLTAGE,
            });
        }
        Err(HidppError::FeatureUnsupported(FeatureId::UNIFIED_BATTERY))
    }

    pub fn get_buttons(&self) -> Result<Vec<ControlInfo>> {
        if !self.has_feature(FeatureId::SPECIAL_KEYS_MSE_BUTTONS) {
            return Err(HidppError::FeatureUnsupported(
                FeatureId::SPECIAL_KEYS_MSE_BUTTONS,
            ));
        }
        let count = self
            .request_feature(FeatureId::SPECIAL_KEYS_MSE_BUTTONS, 0, &[])?
            .params()[0];
        let mut out = Vec::new();
        for i in 0..count {
            let r = self.request_feature(FeatureId::SPECIAL_KEYS_MSE_BUTTONS, 1, &[i])?;
            let p = r.params();
            if p.len() >= 8 {
                let cid = u16::from_be_bytes([p[0], p[1]]);
                let task_id = u16::from_be_bytes([p[2], p[3]]);
                let flags = (p[4] as u16) | ((p.get(8).copied().unwrap_or(0) as u16) << 8);
                let position = p[5];
                let group = p[6];
                let group_mask = p[7];

                let mut diverted = false;
                let mut remapped_cid = 0;
                if let Ok(rep) =
                    self.request_feature(FeatureId::SPECIAL_KEYS_MSE_BUTTONS, 2, &cid.to_be_bytes())
                {
                    let rp = rep.params();
                    if rp.len() >= 5 {
                        diverted = (rp[2] & 0x01) != 0;
                        remapped_cid = u16::from_be_bytes([rp[3], rp[4]]);
                    }
                }

                out.push(ControlInfo {
                    index: i,
                    cid,
                    task_id,
                    flags,
                    position,
                    group,
                    group_mask,
                    name: cid_name(cid).to_string(),
                    diverted,
                    remapped_cid,
                });
            }
        }
        Ok(out)
    }

    pub fn remap_button(&self, cid: u16, target_cid: u16) -> Result<()> {
        let [c_hi, c_lo] = cid.to_be_bytes();
        let [t_hi, t_lo] = target_cid.to_be_bytes();
        self.request_feature(
            FeatureId::SPECIAL_KEYS_MSE_BUTTONS,
            3,
            &[c_hi, c_lo, 0x00, t_hi, t_lo],
        )?;
        Ok(())
    }

    pub fn get_smartshift(&self) -> Result<SmartShiftState> {
        if self.has_feature(FeatureId::SMARTSHIFT_ENHANCED) {
            let r = self.request_feature(FeatureId::SMARTSHIFT_ENHANCED, 1, &[])?;
            let p = r.params();
            return Ok(SmartShiftState {
                wheel_mode: WheelMode::from(p[0]),
                auto_disengage: p.get(1).copied().unwrap_or(0),
                auto_disengage_default: p.get(2).copied().unwrap_or(0),
                torque: p.get(3).copied(),
            });
        }
        if self.has_feature(FeatureId::SMARTSHIFT) {
            let r = self.request_feature(FeatureId::SMARTSHIFT, 0, &[])?;
            let p = r.params();
            return Ok(SmartShiftState {
                wheel_mode: WheelMode::from(p[0]),
                auto_disengage: p.get(1).copied().unwrap_or(0),
                auto_disengage_default: p.get(2).copied().unwrap_or(0),
                torque: None,
            });
        }
        Err(HidppError::FeatureUnsupported(FeatureId::SMARTSHIFT))
    }

    pub fn set_smartshift(&self, mode: WheelMode, auto_disengage: u8) -> Result<()> {
        let m = match mode {
            WheelMode::FreeSpin => 1,
            WheelMode::Ratchet => 2,
            WheelMode::Unknown(v) => v,
        };
        if self.has_feature(FeatureId::SMARTSHIFT_ENHANCED) {
            self.request_feature(FeatureId::SMARTSHIFT_ENHANCED, 2, &[m, auto_disengage, 0])?;
            return Ok(());
        }
        if self.has_feature(FeatureId::SMARTSHIFT) {
            self.request_feature(FeatureId::SMARTSHIFT, 1, &[m, auto_disengage, 0])?;
            return Ok(());
        }
        Err(HidppError::FeatureUnsupported(FeatureId::SMARTSHIFT))
    }

    pub fn get_hires_wheel(&self) -> Result<HiResWheelState> {
        let cap = self.request_feature(FeatureId::HIRES_WHEEL_V2, 0, &[])?;
        let cp = cap.params();
        let mode = self.request_feature(FeatureId::HIRES_WHEEL_V2, 1, &[])?;
        let mp = mode.params();
        Ok(HiResWheelState {
            multiplier: cp[0],
            has_invert: (cp[1] & 0x08) != 0,
            has_switch: (cp[1] & 0x04) != 0,
            target_hidpp: (mp[0] & 0x01) != 0,
            hires_enabled: (mp[0] & 0x02) != 0,
            inverted: (mp[0] & 0x04) != 0,
        })
    }

    pub fn get_angle_snapping(&self) -> Result<bool> {
        let r = self.request_feature(FeatureId::ANGLE_SNAPPING, 0, &[])?;
        Ok(r.params()[0] != 0)
    }

    pub fn set_angle_snapping(&self, enabled: bool) -> Result<()> {
        self.request_feature(FeatureId::ANGLE_SNAPPING, 1, &[u8::from(enabled)])?;
        Ok(())
    }

    pub fn probe_summary(&self) -> DeviceSummary {
        let name = self
            .device_name()
            .unwrap_or_else(|_| "Logitech HID++ Device".to_string());
        let kind = self.device_kind().unwrap_or(DeviceKind::Mouse);
        let fw_info = self.firmware_info().ok();
        let dpi = self.get_dpi_states().ok();
        let report_rate = self.get_report_rate().ok();
        let rgb = self.get_rgb_info().ok();
        let perkey = self.get_perkey_info().ok();
        let onboard = self.get_onboard_description().ok();
        let onboard_mode = self.get_onboard_mode().ok();
        let battery = self.get_battery().ok();
        let smartshift = self.get_smartshift().ok();
        let hires_wheel = self.get_hires_wheel().ok();
        let angle_snapping = self.get_angle_snapping().ok();
        let startup_effect = self.get_startup_effect().ok();
        let buttons = self.get_buttons().ok();

        DeviceSummary {
            name,
            kind,
            protocol_version: self.protocol_version,
            device_index: self.transport.device_index(),
            fw_info,
            features: self.feature_list.clone(),
            dpi,
            report_rate,
            rgb,
            perkey,
            onboard,
            onboard_mode,
            battery,
            smartshift,
            hires_wheel,
            angle_snapping,
            startup_effect,
            buttons,
        }
    }
}
