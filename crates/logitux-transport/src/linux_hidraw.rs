use logitux_hidpp::{HidppPacket, REPORT_LONG, REPORT_SHORT, REPORT_VERY_LONG};
use serde::{Deserialize, Serialize};

pub const LOGITECH_VID: u16 = 0x046D;

pub const UDEV_RULES_FILENAME: &str = "60-logitux.rules";
pub const UDEV_RULES_CONTENT: &str = concat!(
    "ACTION==\"add|change\", SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"046d\", MODE=\"0666\", TAG+=\"uaccess\"\n",
    "ACTION==\"add|change\", SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"046D\", MODE=\"0666\", TAG+=\"uaccess\"\n",
    "ACTION==\"add|change\", SUBSYSTEM==\"hidraw\", KERNELS==\"*:046D:*\", MODE=\"0666\", TAG+=\"uaccess\"\n",
    "ACTION==\"add|change\", SUBSYSTEM==\"hidraw\", KERNELS==\"*:046d:*\", MODE=\"0666\", TAG+=\"uaccess\"\n",
    "ACTION==\"add|change\", SUBSYSTEM==\"hidraw\", DRIVERS==\"logitech-hidpp-device|logitech-djdevice|logitech-djreceiver\", MODE=\"0666\", TAG+=\"uaccess\"\n",
    "KERNEL==\"uinput\", SUBSYSTEM==\"misc\", OPTIONS+=\"static_node=uinput\", MODE=\"0666\", TAG+=\"uaccess\"\n",
);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HidrawUevent {
    pub bus: u16,
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: String,
    pub uniq: String,
    pub phys: String,
    pub driver: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HidppReportCaps {
    pub has_vendor_ff00: bool,
    pub has_short: bool,
    pub has_long: bool,
    pub has_very_long: bool,
}

impl HidppReportCaps {
    pub fn looks_hidpp(self) -> bool {
        self.has_vendor_ff00 || self.has_short || self.has_long || self.has_very_long
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedHidDevice {
    pub path: String,
    pub product_id: u16,
    pub name: String,
}

pub fn parse_hidraw_uevent(content: &str) -> Option<HidrawUevent> {
    let mut out = HidrawUevent::default();
    let mut found_id = false;
    for line in content.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "HID_ID" => {
                let mut parts = v.split(':');
                let bus = parts
                    .next()
                    .and_then(|s| u16::from_str_radix(s, 16).ok())
                    .unwrap_or(0);
                let vid = parts
                    .next()
                    .and_then(|s| u32::from_str_radix(s, 16).ok())
                    .unwrap_or(0) as u16;
                let pid = parts
                    .next()
                    .and_then(|s| u32::from_str_radix(s, 16).ok())
                    .unwrap_or(0) as u16;
                out.bus = bus;
                out.vendor_id = vid;
                out.product_id = pid;
                found_id = true;
            }
            "HID_NAME" => out.name = v.to_string(),
            "HID_UNIQ" => out.uniq = v.to_string(),
            "HID_PHYS" => out.phys = v.to_string(),
            "DRIVER" => out.driver = v.to_string(),
            _ => {}
        }
    }
    found_id.then_some(out)
}

pub fn inspect_report_descriptor(desc: &[u8]) -> HidppReportCaps {
    let mut caps = HidppReportCaps::default();
    let mut in_vendor_page = false;
    let mut i = 0usize;
    while i < desc.len() {
        let b0 = desc[i];
        if b0 == 0xFE {
            if i + 2 >= desc.len() {
                break;
            }
            let data_len = desc[i + 1] as usize;
            i = i.saturating_add(3 + data_len);
            continue;
        }
        let size_code = b0 & 0x03;
        let data_len = if size_code == 3 { 4 } else { size_code as usize };
        if i + 1 + data_len > desc.len() {
            break;
        }
        let tag_type = b0 & 0xFC;
        let data = &desc[i + 1..i + 1 + data_len];
        if tag_type == 0x04 {
            let page = match data.len() {
                1 => data[0] as u16,
                2 => u16::from_le_bytes([data[0], data[1]]),
                4 => u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as u16,
                _ => 0,
            };
            in_vendor_page = page >= 0xFF00;
            if page == 0xFF00 {
                caps.has_vendor_ff00 = true;
            }
        } else if tag_type == 0x84 && data.len() == 1 {
            match data[0] {
                REPORT_SHORT if in_vendor_page => caps.has_short = true,
                REPORT_LONG if in_vendor_page => caps.has_long = true,
                REPORT_VERY_LONG if in_vendor_page => caps.has_very_long = true,
                REPORT_SHORT => caps.has_short = true,
                REPORT_LONG => caps.has_long = true,
                REPORT_VERY_LONG => caps.has_very_long = true,
                _ => {}
            }
        }
        i += 1 + data_len;
    }
    caps
}

pub fn is_hidpp_report_id(id: u8) -> bool {
    matches!(id, REPORT_SHORT | REPORT_LONG | REPORT_VERY_LONG)
}

pub fn try_parse_hidpp_frame(raw: &[u8]) -> Option<HidppPacket> {
    if raw.len() < 7 || !is_hidpp_report_id(raw[0]) {
        return None;
    }
    HidppPacket::from_raw(raw).ok()
}

#[cfg(target_os = "linux")]
pub mod imp {
    use std::collections::HashSet;
    use std::fs::{self, File, OpenOptions};
    use std::io::{ErrorKind, Write};
    use std::os::unix::io::AsRawFd;
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use logitux_hidpp::{HidppDevice, HidppError, HidppPacket, Result, Transport, DEVICE_INDEX_WIRED};

    use crate::{is_receiver_pid, ConnectedDevice, ConnectionReport, DiscoveredHidppDevice};

    use super::{
        inspect_report_descriptor, parse_hidraw_uevent, try_parse_hidpp_frame, BlockedHidDevice,
        HidppReportCaps, LOGITECH_VID, UDEV_RULES_CONTENT, UDEV_RULES_FILENAME,
    };

    pub struct LinuxHidrawTransport {
        file: File,
        device_index: u8,
        supports_short: bool,
        supports_long: bool,
    }

    impl LinuxHidrawTransport {
        pub fn open(
            path: &str,
            device_index: u8,
            caps: HidppReportCaps,
        ) -> std::result::Result<Self, std::io::Error> {
            let file = OpenOptions::new().read(true).write(true).open(path)?;
            let supports_short = caps.has_short || !caps.has_long;
            let supports_long = caps.has_long || !caps.has_short;
            Ok(Self {
                file,
                device_index,
                supports_short,
                supports_long,
            })
        }
    }

    impl Transport for LinuxHidrawTransport {
        fn write_packet(&self, packet: &HidppPacket) -> Result<()> {
            let mut f = &self.file;
            f.write_all(packet.as_bytes())
                .map_err(|e| HidppError::Transport(format!("hidraw write failed: {e}")))?;
            Ok(())
        }

        fn read_packet(&self, timeout_ms: u32) -> Result<Option<HidppPacket>> {
            let fd = self.file.as_raw_fd();
            let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
            let mut buf = [0u8; 64];

            loop {
                let now = Instant::now();
                let rem_ms = if now >= deadline {
                    0
                } else {
                    deadline
                        .saturating_duration_since(now)
                        .as_millis()
                        .min(i32::MAX as u128) as i32
                };

                let mut pfd = libc::pollfd {
                    fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pfd, 1, rem_ms) };
                if ret < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(HidppError::Transport(format!("hidraw poll failed: {err}")));
                }
                if ret == 0 {
                    return Ok(None);
                }
                if (pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                    return Err(HidppError::Transport("HID device disconnected".into()));
                }
                if (pfd.revents & libc::POLLIN) != 0 {
                    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
                    if n < 0 {
                        let err = std::io::Error::last_os_error();
                        if err.kind() == ErrorKind::Interrupted || err.kind() == ErrorKind::WouldBlock
                        {
                            continue;
                        }
                        return Err(HidppError::Transport(format!("hidraw read failed: {err}")));
                    }
                    if n == 0 {
                        return Err(HidppError::Transport("HID device closed".into()));
                    }
                    if let Some(pkt) = try_parse_hidpp_frame(&buf[..n as usize]) {
                        return Ok(Some(pkt));
                    }
                }
                if Instant::now() >= deadline {
                    return Ok(None);
                }
            }
        }

        fn supports_short(&self) -> bool {
            self.supports_short
        }

        fn supports_long(&self) -> bool {
            self.supports_long
        }

        fn device_index(&self) -> u8 {
            self.device_index
        }
    }

    struct RawCandidate {
        desc: DiscoveredHidppDevice,
        path: String,
        caps: HidppReportCaps,
        phys: String,
    }

    fn read_interface_number(sys_dev: &Path) -> i32 {
        let mut cur = Some(sys_dev);
        for _ in 0..4 {
            let Some(p) = cur else { break };
            let iface_file = p.join("bInterfaceNumber");
            if let Ok(txt) = fs::read_to_string(&iface_file) {
                if let Ok(v) = i32::from_str_radix(txt.trim(), 16) {
                    return v;
                }
            }
            cur = p.parent();
        }
        -1
    }

    fn scan_candidates() -> Vec<RawCandidate> {
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir("/sys/class/hidraw") else {
            return out;
        };

        for entry in entries.flatten() {
            let node_name = entry.file_name().to_string_lossy().to_string();
            if !node_name.starts_with("hidraw") {
                continue;
            }
            let sys_dev = entry.path().join("device");
            let Ok(uevent_txt) = fs::read_to_string(sys_dev.join("uevent")) else {
                continue;
            };
            let Some(uev) = parse_hidraw_uevent(&uevent_txt) else {
                continue;
            };
            if uev.vendor_id != LOGITECH_VID {
                continue;
            }

            let caps = fs::read(sys_dev.join("report_descriptor"))
                .map(|b| inspect_report_descriptor(&b))
                .unwrap_or_default();
            let iface = read_interface_number(&sys_dev);
            let dev_path = format!("/dev/{node_name}");
            let name = if uev.name.is_empty() {
                "Logitech Device".to_string()
            } else {
                uev.name.clone()
            };

            let desc = DiscoveredHidppDevice {
                vendor_id: LOGITECH_VID,
                product_id: uev.product_id,
                product_name: name,
                manufacturer: "Logitech".to_string(),
                serial_number: if uev.uniq.is_empty() {
                    uev.phys.clone()
                } else {
                    uev.uniq.clone()
                },
                interface_number: iface,
                is_receiver: is_receiver_pid(uev.product_id),
                short_path: caps.has_short.then(|| dev_path.clone()),
                long_path: caps.has_long.then(|| dev_path.clone()),
                very_long_path: caps.has_very_long.then(|| dev_path.clone()),
                fallback_path: Some(dev_path.clone()),
            };

            out.push(RawCandidate {
                desc,
                path: dev_path,
                caps,
                phys: uev.phys,
            });
        }

        out.sort_by_key(|c| {
            (
                !c.caps.has_vendor_ff00,
                !(c.caps.has_long || c.caps.has_short),
                c.path.clone(),
            )
        });
        out
    }

    pub fn enumerate_logitech_devices() -> Result<Vec<DiscoveredHidppDevice>> {
        let cands = scan_candidates();
        let has_explicit_hidpp = cands.iter().any(|c| c.caps.looks_hidpp());
        let mut out = Vec::new();
        for c in cands {
            if !has_explicit_hidpp || c.caps.looks_hidpp() || c.desc.interface_number >= 1 {
                out.push(c.desc);
            }
        }
        Ok(out)
    }

    pub fn connect_all_devices_diagnosed() -> Result<ConnectionReport> {
        let candidates = scan_candidates();
        let any_explicit_hidpp = candidates.iter().any(|c| c.caps.looks_hidpp());
        let mut connected = Vec::new();
        let mut blocked = Vec::new();
        let mut seen_devices: HashSet<(u16, String, u8)> = HashSet::new();

        for cand in candidates {
            let prefer_hidpp =
                !any_explicit_hidpp || cand.caps.looks_hidpp() || cand.desc.interface_number != 0;
            if !prefer_hidpp {
                continue;
            }

            let indices: Vec<u8> = if cand.desc.is_receiver {
                vec![1, 2, 3, 4, 5, 6, DEVICE_INDEX_WIRED]
            } else {
                vec![DEVICE_INDEX_WIRED, 1]
            };

            let mut perm_denied = false;
            for dev_idx in indices {
                match LinuxHidrawTransport::open(&cand.path, dev_idx, cand.caps) {
                    Ok(transport) => {
                        if let Ok(device) = HidppDevice::connect(transport) {
                            let uniq_key = if !cand.desc.serial_number.is_empty() {
                                cand.desc.serial_number.clone()
                            } else if !cand.phys.is_empty() {
                                cand.phys.clone()
                            } else {
                                cand.path.clone()
                            };
                            if seen_devices.insert((cand.desc.product_id, uniq_key, dev_idx)) {
                                connected.push(ConnectedDevice {
                                    descriptor: cand.desc.clone(),
                                    device,
                                });
                                if !cand.desc.is_receiver {
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                        perm_denied = true;
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            if perm_denied {
                blocked.push(BlockedHidDevice {
                    path: cand.path,
                    product_id: cand.desc.product_id,
                    name: cand.desc.product_name,
                });
            }
        }

        Ok(ConnectionReport {
            devices: connected,
            permission_denied: blocked,
        })
    }

    pub fn grant_linux_hidraw_access(install_persistent_rule: bool) -> std::result::Result<String, String> {
        let uid = unsafe { libc::geteuid() };
        let grant_live = format!(
            "for d in /sys/class/hidraw/hidraw*; do \
               [ -r \"$d/device/uevent\" ] || continue; \
               if grep -qiE 'HID_ID=[0-9a-f]+:0*046d:' \"$d/device/uevent\"; then \
                 dev=\"/dev/$(basename \"$d\")\"; \
                 [ -e \"$dev\" ] || continue; \
                 if command -v setfacl >/dev/null 2>&1; then \
                   setfacl -m u:{uid}:rw \"$dev\" 2>/dev/null || chmod 0666 \"$dev\" 2>/dev/null || true; \
                 else \
                   chmod 0666 \"$dev\" 2>/dev/null || true; \
                 fi; \
               fi; \
             done"
        );

        let script = if install_persistent_rule {
            format!(
                "{grant_live}; \
                 rm -f /etc/udev/rules.d/99-logitux.rules 2>/dev/null || true; \
                 RULE_DIR=/etc/udev/rules.d; \
                 if ! mkdir -p \"$RULE_DIR\" 2>/dev/null || [ ! -w \"$RULE_DIR\" ]; then \
                   RULE_DIR=/run/udev/rules.d; \
                   mkdir -p \"$RULE_DIR\" 2>/dev/null || true; \
                 fi; \
                 if [ -w \"$RULE_DIR\" ]; then \
                   cat <<'LOGITUX_EOF' > \"$RULE_DIR/{UDEV_RULES_FILENAME}\"\n\
{UDEV_RULES_CONTENT}\
LOGITUX_EOF\n\
                   chmod 0644 \"$RULE_DIR/{UDEV_RULES_FILENAME}\" 2>/dev/null || true; \
                   if command -v udevadm >/dev/null 2>&1; then \
                     udevadm control --reload-rules 2>/dev/null || true; \
                     udevadm trigger --subsystem-match=hidraw --action=change 2>/dev/null || true; \
                   fi; \
                 fi; \
                 {grant_live}"
            )
        } else {
            grant_live
        };

        let has_gui_session = std::env::var_os("DISPLAY").is_some()
            || std::env::var_os("WAYLAND_DISPLAY").is_some();

        let status = if uid == 0 {
            Command::new("sh").arg("-c").arg(&script).status()
        } else if has_gui_session && command_exists("pkexec") {
            Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()
        } else if has_gui_session && command_exists("kdesu") {
            Command::new("kdesu")
                .args(["-c", &format!("sh -c '{}'", script.replace('\'', "'\"'\"'"))])
                .status()
        } else if command_exists("sudo") {
            Command::new("sudo").args(["sh", "-c", &script]).status()
        } else if command_exists("pkexec") {
            Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()
        } else {
            return Err("pkexec / sudo bulunamadı".into());
        };

        match status {
            Ok(s) if s.success() => Ok("Linux HID erişimi etkinleştirildi.".into()),
            Ok(s) => Err(format!(
                "Yetkilendirme iptal edildi veya başarısız oldu (kod: {:?}).",
                s.code()
            )),
            Err(e) => Err(format!("Yetkilendirme başlatılamadı: {e}")),
        }
    }

    fn command_exists(name: &str) -> bool {
        Command::new("sh")
            .args(["-c", &format!("command -v {name} >/dev/null 2>&1")])
            .status()
            .is_ok_and(|s| s.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hidraw_uevent_usb_and_bt() {
        let g102 = "\
DRIVER=hid-generic
HID_ID=0003:0000046D:0000C092
HID_NAME=Logitech G102 LIGHTSYNC Gaming Mouse
HID_PHYS=usb-0000:00:14.0-2/input1
HID_UNIQ=206B3766424D
";
        let u = parse_hidraw_uevent(g102).unwrap();
        assert_eq!(u.bus, 0x0003);
        assert_eq!(u.vendor_id, 0x046D);
        assert_eq!(u.product_id, 0xC092);
        assert_eq!(u.name, "Logitech G102 LIGHTSYNC Gaming Mouse");
        assert_eq!(u.uniq, "206B3766424D");

        let bt = "HID_ID=0005:0000046D:0000B034\nHID_NAME=MX Master 3S\n";
        let ub = parse_hidraw_uevent(bt).unwrap();
        assert_eq!(ub.bus, 0x0005);
        assert_eq!(ub.vendor_id, 0x046D);
        assert_eq!(ub.product_id, 0xB034);
    }

    #[test]
    fn test_inspect_report_descriptor_detects_hidpp() {
        let g102_mi01 = [
            0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x85, 0x01, 0xC0,
            0x06, 0x00, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x10, 0xC0,
            0x06, 0x00, 0xFF, 0x09, 0x02, 0xA1, 0x01, 0x85, 0x11, 0xC0,
        ];
        let caps = inspect_report_descriptor(&g102_mi01);
        assert!(caps.has_vendor_ff00);
        assert!(caps.has_short);
        assert!(caps.has_long);
        assert!(!caps.has_very_long);
        assert!(caps.looks_hidpp());

        let mouse_only = [0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0xC0];
        let mc = inspect_report_descriptor(&mouse_only);
        assert!(!mc.looks_hidpp());
    }

    #[test]
    fn test_try_parse_hidpp_frame_skips_non_hidpp() {
        assert!(try_parse_hidpp_frame(&[0x01, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]).is_none());
        assert!(try_parse_hidpp_frame(&[0x10, 0xFF, 0x00, 0x1A, 0x04, 0x02, 0x5A]).is_some());
    }
}
