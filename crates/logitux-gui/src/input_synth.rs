use eframe::egui;

pub fn egui_key_to_hid(key: egui::Key) -> Option<u8> {
    use egui::Key::*;
    Some(match key {
        A => 0x04,
        B => 0x05,
        C => 0x06,
        D => 0x07,
        E => 0x08,
        F => 0x09,
        G => 0x0A,
        H => 0x0B,
        I => 0x0C,
        J => 0x0D,
        K => 0x0E,
        L => 0x0F,
        M => 0x10,
        N => 0x11,
        O => 0x12,
        P => 0x13,
        Q => 0x14,
        R => 0x15,
        S => 0x16,
        T => 0x17,
        U => 0x18,
        V => 0x19,
        W => 0x1A,
        X => 0x1B,
        Y => 0x1C,
        Z => 0x1D,
        Num1 => 0x1E,
        Num2 => 0x1F,
        Num3 => 0x20,
        Num4 => 0x21,
        Num5 => 0x22,
        Num6 => 0x23,
        Num7 => 0x24,
        Num8 => 0x25,
        Num9 => 0x26,
        Num0 => 0x27,
        Enter => 0x28,
        Escape => 0x29,
        Backspace => 0x2A,
        Tab => 0x2B,
        Space => 0x2C,
        Minus => 0x2D,
        Equals | Plus => 0x2E,
        Comma => 0x36,
        Period => 0x37,
        F1 => 0x3A,
        F2 => 0x3B,
        F3 => 0x3C,
        F4 => 0x3D,
        F5 => 0x3E,
        F6 => 0x3F,
        F7 => 0x40,
        F8 => 0x41,
        F9 => 0x42,
        F10 => 0x43,
        F11 => 0x44,
        F12 => 0x45,
        Insert => 0x49,
        Home => 0x4A,
        PageUp => 0x4B,
        Delete => 0x4C,
        End => 0x4D,
        PageDown => 0x4E,
        ArrowRight => 0x4F,
        ArrowLeft => 0x50,
        ArrowDown => 0x51,
        ArrowUp => 0x52,
        _ => return None,
    })
}

pub fn egui_modifiers_mask(m: egui::Modifiers) -> u8 {
    let mut out = 0u8;
    if m.ctrl {
        out |= 0x01;
    }
    if m.shift {
        out |= 0x02;
    }
    if m.alt {
        out |= 0x04;
    }
    if m.mac_cmd {
        out |= 0x08;
    }
    out
}

#[cfg(target_os = "windows")]
mod imp {
    use std::mem::size_of;
    use std::process::Command;

    use crate::store::MediaCmd;

    const INPUT_MOUSE: u32 = 0;
    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_EXTENDEDKEY: u32 = 0x0001;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const KEYEVENTF_UNICODE: u32 = 0x0004;
    const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
    const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
    const MOUSEEVENTF_MIDDLEDOWN: u32 = 0x0020;
    const MOUSEEVENTF_MIDDLEUP: u32 = 0x0040;
    const MOUSEEVENTF_XDOWN: u32 = 0x0080;
    const MOUSEEVENTF_XUP: u32 = 0x0100;
    const MOUSEEVENTF_WHEEL: u32 = 0x0800;
    const MOUSEEVENTF_HWHEEL: u32 = 0x01000;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MouseInput {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        dw_flags: u32,
        time: u32,
        dw_extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct KeybdInput {
        w_vk: u16,
        w_scan: u16,
        dw_flags: u32,
        time: u32,
        dw_extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    union InputUnion {
        mi: MouseInput,
        ki: KeybdInput,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Input {
        r#type: u32,
        u: InputUnion,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SendInput(c_inputs: u32, p_inputs: *const Input, cb_size: i32) -> u32;
        fn MapVirtualKeyW(u_code: u32, u_map_type: u32) -> u32;
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(h_wnd: isize, lpdw_process_id: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
        fn QueryFullProcessImageNameW(
            h_process: isize,
            dw_flags: u32,
            lp_exe_name: *mut u16,
            lpdw_size: *mut u32,
        ) -> i32;
        fn CloseHandle(h_object: isize) -> i32;
    }

    fn is_extended_vk(vk: u16) -> bool {
        matches!(
            vk,
            0x21..=0x28 | 0x2C..=0x2E | 0x5B | 0x5C | 0x6F | 0xA3 | 0xA5 | 0xAD..=0xB4
        )
    }

    fn send_vk(vk: u16, up: bool) {
        let scan = unsafe { MapVirtualKeyW(vk as u32, 0) as u16 };
        let mut flags = if up { KEYEVENTF_KEYUP } else { 0 };
        if is_extended_vk(vk) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        let inp = Input {
            r#type: INPUT_KEYBOARD,
            u: InputUnion {
                ki: KeybdInput {
                    w_vk: vk,
                    w_scan: scan,
                    dw_flags: flags,
                    time: 0,
                    dw_extra_info: 0,
                },
            },
        };
        unsafe {
            let _ = SendInput(1, &inp, size_of::<Input>() as i32);
        }
    }

    fn hid_to_vk(hid: u8) -> Option<u16> {
        Some(match hid {
            0x04..=0x1D => (b'A' + (hid - 0x04)) as u16,
            0x1E..=0x26 => (b'1' + (hid - 0x1E)) as u16,
            0x27 => b'0' as u16,
            0x28 => 0x0D,
            0x29 => 0x1B,
            0x2A => 0x08,
            0x2B => 0x09,
            0x2C => 0x20,
            0x2D => 0xBD,
            0x2E => 0xBB,
            0x36 => 0xBC,
            0x37 => 0xBE,
            0x39 => 0x14,
            0x3A..=0x45 => 0x70 + (hid - 0x3A) as u16,
            0x68..=0x73 => 0x7C + (hid - 0x68) as u16,
            0x46 => 0x2C,
            0x47 => 0x91,
            0x48 => 0x13,
            0x49 => 0x2D,
            0x4A => 0x24,
            0x4B => 0x21,
            0x4C => 0x2E,
            0x4D => 0x23,
            0x4E => 0x22,
            0x4F => 0x27,
            0x50 => 0x25,
            0x51 => 0x28,
            0x52 => 0x26,
            0x54 => 0x6F,
            0x55 => 0x6A,
            0x56 => 0x6D,
            0x57 => 0x6B,
            0x58 => 0x0D,
            0x59..=0x61 => 0x61 + (hid - 0x59) as u16,
            0x62 => 0x60,
            _ => return None,
        })
    }

    pub fn key_down(modifiers: u8, hid_key: u8) {
        if (modifiers & 0x01) != 0 {
            send_vk(0xA2, false);
        }
        if (modifiers & 0x02) != 0 {
            send_vk(0xA0, false);
        }
        if (modifiers & 0x04) != 0 {
            send_vk(0xA4, false);
        }
        if (modifiers & 0x08) != 0 {
            send_vk(0x5B, false);
        }
        if let Some(vk) = hid_to_vk(hid_key) {
            send_vk(vk, false);
        }
    }

    pub fn key_up(modifiers: u8, hid_key: u8) {
        if let Some(vk) = hid_to_vk(hid_key) {
            send_vk(vk, true);
        }
        if (modifiers & 0x08) != 0 {
            send_vk(0x5B, true);
        }
        if (modifiers & 0x04) != 0 {
            send_vk(0xA4, true);
        }
        if (modifiers & 0x02) != 0 {
            send_vk(0xA0, true);
        }
        if (modifiers & 0x01) != 0 {
            send_vk(0xA2, true);
        }
    }

    pub fn tap_shortcut(modifiers: u8, hid_key: u8) {
        if (modifiers & 0x01) != 0 {
            send_vk(0xA2, false);
        }
        if (modifiers & 0x02) != 0 {
            send_vk(0xA0, false);
        }
        if (modifiers & 0x04) != 0 {
            send_vk(0xA4, false);
        }
        if (modifiers & 0x08) != 0 {
            send_vk(0x5B, false);
        }
        if modifiers != 0 {
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        if let Some(vk) = hid_to_vk(hid_key) {
            send_vk(vk, false);
            std::thread::sleep(std::time::Duration::from_millis(32));
            send_vk(vk, true);
        } else {
            std::thread::sleep(std::time::Duration::from_millis(24));
        }
        if modifiers != 0 {
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        if (modifiers & 0x08) != 0 {
            send_vk(0x5B, true);
        }
        if (modifiers & 0x04) != 0 {
            send_vk(0xA4, true);
        }
        if (modifiers & 0x02) != 0 {
            send_vk(0xA0, true);
        }
        if (modifiers & 0x01) != 0 {
            send_vk(0xA2, true);
        }
    }

    pub fn type_text(text: &str) {
        let mut inputs = Vec::new();
        for unit in text.encode_utf16() {
            inputs.push(Input {
                r#type: INPUT_KEYBOARD,
                u: InputUnion {
                    ki: KeybdInput {
                        w_vk: 0,
                        w_scan: unit,
                        dw_flags: KEYEVENTF_UNICODE,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            });
            inputs.push(Input {
                r#type: INPUT_KEYBOARD,
                u: InputUnion {
                    ki: KeybdInput {
                        w_vk: 0,
                        w_scan: unit,
                        dw_flags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            });
        }
        if !inputs.is_empty() {
            unsafe {
                let _ = SendInput(
                    inputs.len() as u32,
                    inputs.as_ptr(),
                    size_of::<Input>() as i32,
                );
            }
        }
    }

    pub fn mouse_click(button: u8) {
        let (down, up, data) = match button {
            1 => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
            2 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
            3 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
            4 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 1),
            5 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 2),
            _ => return,
        };
        let events = [
            Input {
                r#type: INPUT_MOUSE,
                u: InputUnion {
                    mi: MouseInput {
                        dx: 0,
                        dy: 0,
                        mouse_data: data,
                        dw_flags: down,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            },
            Input {
                r#type: INPUT_MOUSE,
                u: InputUnion {
                    mi: MouseInput {
                        dx: 0,
                        dy: 0,
                        mouse_data: data,
                        dw_flags: up,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            },
        ];
        unsafe {
            let _ = SendInput(2, events.as_ptr(), size_of::<Input>() as i32);
        }
    }

    pub fn mouse_scroll(dx: i32, dy: i32) {
        if dy != 0 {
            let inp = Input {
                r#type: INPUT_MOUSE,
                u: InputUnion {
                    mi: MouseInput {
                        dx: 0,
                        dy: 0,
                        mouse_data: (dy * 120) as u32,
                        dw_flags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            };
            unsafe {
                let _ = SendInput(1, &inp, size_of::<Input>() as i32);
            }
        }
        if dx != 0 {
            let inp = Input {
                r#type: INPUT_MOUSE,
                u: InputUnion {
                    mi: MouseInput {
                        dx: 0,
                        dy: 0,
                        mouse_data: (dx * 120) as u32,
                        dw_flags: MOUSEEVENTF_HWHEEL,
                        time: 0,
                        dw_extra_info: 0,
                    },
                },
            };
            unsafe {
                let _ = SendInput(1, &inp, size_of::<Input>() as i32);
            }
        }
    }

    pub fn trigger_media(cmd: MediaCmd) {
        let vk = match cmd {
            MediaCmd::PlayPause => 0xB3,
            MediaCmd::Stop => 0xB2,
            MediaCmd::NextTrack => 0xB0,
            MediaCmd::PrevTrack => 0xB1,
            MediaCmd::VolumeUp => 0xAF,
            MediaCmd::VolumeDown => 0xAE,
            MediaCmd::Mute => 0xAD,
            MediaCmd::BrowserHome => 0xAC,
            MediaCmd::Mail => 0xB4,
            MediaCmd::Search => 0xAA,
            MediaCmd::Screenshot => 0x2C,
            MediaCmd::Calculator => {
                let _ = Command::new("calc.exe").spawn();
                return;
            }
        };
        send_vk(vk, false);
        send_vk(vk, true);
    }

    pub fn active_foreground_exe() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd == 0 {
                return None;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 {
                return None;
            }
            let handle = OpenProcess(0x1000, 0, pid);
            if handle == 0 {
                return None;
            }
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(handle);
            if ok == 0 || len == 0 {
                return None;
            }
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            std::path::Path::new(&full)
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};

    use crate::store::MediaCmd;

    const EV_SYN: u16 = 0x00;
    const EV_KEY: u16 = 0x01;
    const EV_REL: u16 = 0x02;
    const SYN_REPORT: u16 = 0;
    const REL_WHEEL: u16 = 0x08;
    const REL_HWHEEL: u16 = 0x06;

    const UI_SET_EVBIT: u32 = 0x40045564;
    const UI_SET_KEYBIT: u32 = 0x40045565;
    const UI_SET_RELBIT: u32 = 0x40045566;
    const UI_DEV_CREATE: u32 = 0x5501;

    #[repr(C)]
    struct UinputUserDev {
        name: [u8; 80],
        id_bustype: u16,
        id_vendor: u16,
        id_product: u16,
        id_version: u16,
        ff_effects_max: u32,
        absmax: [i32; 64],
        absmin: [i32; 64],
        absfuzz: [i32; 64],
        absflat: [i32; 64],
    }

    #[repr(C)]
    struct InputEvent {
        tv_sec: libc::time_t,
        tv_usec: libc::suseconds_t,
        r#type: u16,
        code: u16,
        value: i32,
    }

    struct UinputSlot {
        file: Option<File>,
        last_attempt: Option<std::time::Instant>,
    }

    static UINPUT_DEV: OnceLock<Mutex<UinputSlot>> = OnceLock::new();

    fn init_uinput() -> Option<File> {
        let mut f = OpenOptions::new().write(true).open("/dev/uinput").ok()?;
        let fd = f.as_raw_fd();
        unsafe {
            libc::ioctl(fd, UI_SET_EVBIT as _, EV_KEY as libc::c_int);
            libc::ioctl(fd, UI_SET_EVBIT as _, EV_REL as libc::c_int);
            libc::ioctl(fd, UI_SET_RELBIT as _, REL_WHEEL as libc::c_int);
            libc::ioctl(fd, UI_SET_RELBIT as _, REL_HWHEEL as libc::c_int);
            for code in 1..=248 {
                libc::ioctl(fd, UI_SET_KEYBIT as _, code as libc::c_int);
            }
            for btn in 0x110..=0x116 {
                libc::ioctl(fd, UI_SET_KEYBIT as _, btn as libc::c_int);
            }
        }
        let mut udev = UinputUserDev {
            name: [0u8; 80],
            id_bustype: 0x03,
            id_vendor: 0x046D,
            id_product: 0xC092,
            id_version: 1,
            ff_effects_max: 0,
            absmax: [0; 64],
            absmin: [0; 64],
            absfuzz: [0; 64],
            absflat: [0; 64],
        };
        let label = b"LogiTux Virtual Input";
        udev.name[..label.len()].copy_from_slice(label);
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &udev as *const UinputUserDev as *const u8,
                std::mem::size_of::<UinputUserDev>(),
            )
        };
        f.write_all(bytes).ok()?;
        unsafe {
            if libc::ioctl(fd, UI_DEV_CREATE as _) < 0 {
                return None;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
        Some(f)
    }

    fn emit_uinput(ev_type: u16, code: u16, value: i32) -> bool {
        let mtx = UINPUT_DEV.get_or_init(|| {
            Mutex::new(UinputSlot {
                file: None,
                last_attempt: None,
            })
        });
        let Ok(mut guard) = mtx.lock() else {
            return false;
        };
        if guard.file.is_none() {
            let should_try = guard
                .last_attempt
                .is_none_or(|t| t.elapsed() >= std::time::Duration::from_secs(3));
            if should_try {
                guard.last_attempt = Some(std::time::Instant::now());
                guard.file = init_uinput();
            }
        }
        let Some(f) = guard.file.as_mut() else {
            return false;
        };
        let events = [
            InputEvent {
                tv_sec: 0,
                tv_usec: 0,
                r#type: ev_type,
                code,
                value,
            },
            InputEvent {
                tv_sec: 0,
                tv_usec: 0,
                r#type: EV_SYN,
                code: SYN_REPORT,
                value: 0,
            },
        ];
        let raw = unsafe {
            std::slice::from_raw_parts(events.as_ptr() as *const u8, std::mem::size_of_val(&events))
        };
        if f.write_all(raw).is_err() {
            guard.file = None;
            return false;
        }
        true
    }

    fn hid_to_evdev(hid: u8) -> Option<u16> {
        Some(match hid {
            0x04 => 30,
            0x05 => 48,
            0x06 => 46,
            0x07 => 32,
            0x08 => 18,
            0x09 => 33,
            0x0A => 34,
            0x0B => 35,
            0x0C => 23,
            0x0D => 36,
            0x0E => 37,
            0x0F => 38,
            0x10 => 50,
            0x11 => 49,
            0x12 => 24,
            0x13 => 25,
            0x14 => 16,
            0x15 => 19,
            0x16 => 31,
            0x17 => 20,
            0x18 => 22,
            0x19 => 47,
            0x1A => 17,
            0x1B => 45,
            0x1C => 21,
            0x1D => 44,
            0x1E..=0x26 => 2 + (hid - 0x1E) as u16,
            0x27 => 11,
            0x28 => 28,
            0x29 => 1,
            0x2A => 14,
            0x2B => 15,
            0x2C => 57,
            0x2D => 12,
            0x2E => 13,
            0x2F => 26,
            0x30 => 27,
            0x31 => 43,
            0x33 => 39,
            0x34 => 40,
            0x35 => 41,
            0x36 => 51,
            0x37 => 52,
            0x38 => 53,
            0x39 => 58,
            0x3A..=0x43 => 59 + (hid - 0x3A) as u16,
            0x44 => 87,
            0x45 => 88,
            0x46 => 99,
            0x47 => 70,
            0x48 => 119,
            0x49 => 110,
            0x4A => 102,
            0x4B => 104,
            0x4C => 111,
            0x4D => 107,
            0x4E => 109,
            0x4F => 106,
            0x50 => 105,
            0x51 => 108,
            0x52 => 103,
            0x54 => 98,
            0x55 => 55,
            0x56 => 74,
            0x57 => 78,
            0x58 => 96,
            0x59 => 79,
            0x5A => 80,
            0x5B => 81,
            0x5C => 75,
            0x5D => 76,
            0x5E => 77,
            0x5F => 71,
            0x60 => 72,
            0x61 => 73,
            0x62 => 82,
            0x68..=0x73 => 183 + (hid - 0x68) as u16,
            _ => return None,
        })
    }

    pub fn key_down(modifiers: u8, hid_key: u8) {
        if (modifiers & 0x01) != 0 {
            let _ = emit_uinput(EV_KEY, 29, 1);
        }
        if (modifiers & 0x02) != 0 {
            let _ = emit_uinput(EV_KEY, 42, 1);
        }
        if (modifiers & 0x04) != 0 {
            let _ = emit_uinput(EV_KEY, 56, 1);
        }
        if (modifiers & 0x08) != 0 {
            let _ = emit_uinput(EV_KEY, 125, 1);
        }
        if let Some(ev) = hid_to_evdev(hid_key) {
            let _ = emit_uinput(EV_KEY, ev, 1);
        }
    }

    pub fn key_up(modifiers: u8, hid_key: u8) {
        if let Some(ev) = hid_to_evdev(hid_key) {
            let _ = emit_uinput(EV_KEY, ev, 0);
        }
        if (modifiers & 0x08) != 0 {
            let _ = emit_uinput(EV_KEY, 125, 0);
        }
        if (modifiers & 0x04) != 0 {
            let _ = emit_uinput(EV_KEY, 56, 0);
        }
        if (modifiers & 0x02) != 0 {
            let _ = emit_uinput(EV_KEY, 42, 0);
        }
        if (modifiers & 0x01) != 0 {
            let _ = emit_uinput(EV_KEY, 29, 0);
        }
    }

    pub fn tap_shortcut(modifiers: u8, hid_key: u8) {
        if (modifiers & 0x01) != 0 {
            let _ = emit_uinput(EV_KEY, 29, 1);
        }
        if (modifiers & 0x02) != 0 {
            let _ = emit_uinput(EV_KEY, 42, 1);
        }
        if (modifiers & 0x04) != 0 {
            let _ = emit_uinput(EV_KEY, 56, 1);
        }
        if (modifiers & 0x08) != 0 {
            let _ = emit_uinput(EV_KEY, 125, 1);
        }
        if modifiers != 0 {
            std::thread::sleep(std::time::Duration::from_millis(12));
        }
        if let Some(ev) = hid_to_evdev(hid_key) {
            let _ = emit_uinput(EV_KEY, ev, 1);
            std::thread::sleep(std::time::Duration::from_millis(32));
            let _ = emit_uinput(EV_KEY, ev, 0);
        } else {
            std::thread::sleep(std::time::Duration::from_millis(24));
        }
        if modifiers != 0 {
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        if (modifiers & 0x08) != 0 {
            let _ = emit_uinput(EV_KEY, 125, 0);
        }
        if (modifiers & 0x04) != 0 {
            let _ = emit_uinput(EV_KEY, 56, 0);
        }
        if (modifiers & 0x02) != 0 {
            let _ = emit_uinput(EV_KEY, 42, 0);
        }
        if (modifiers & 0x01) != 0 {
            let _ = emit_uinput(EV_KEY, 29, 0);
        }
    }

    pub fn type_text(text: &str) {
        if Command::new("wtype")
            .arg(text)
            .status()
            .is_ok_and(|s| s.success())
        {
            return;
        }
        if Command::new("xdotool")
            .args(["type", "--delay", "6", text])
            .status()
            .is_ok_and(|s| s.success())
        {
            return;
        }
        for ch in text.chars() {
            if let Some((mods, hid)) = match ch {
                'a'..='z' => Some((0x00, 0x04 + (ch as u8 - b'a'))),
                'A'..='Z' => Some((0x02, 0x04 + (ch as u8 - b'A'))),
                '1'..='9' => Some((0x00, 0x1E + (ch as u8 - b'1'))),
                '0' => Some((0x00, 0x27)),
                '\n' => Some((0x00, 0x28)),
                '\t' => Some((0x00, 0x2B)),
                ' ' => Some((0x00, 0x2C)),
                '-' => Some((0x00, 0x2D)),
                '_' => Some((0x02, 0x2D)),
                '=' => Some((0x00, 0x2E)),
                '+' => Some((0x02, 0x2E)),
                '[' => Some((0x00, 0x2F)),
                '{' => Some((0x02, 0x2F)),
                ']' => Some((0x00, 0x30)),
                '}' => Some((0x02, 0x30)),
                '\\' => Some((0x00, 0x31)),
                '|' => Some((0x02, 0x31)),
                ';' => Some((0x00, 0x33)),
                ':' => Some((0x02, 0x33)),
                '\'' => Some((0x00, 0x34)),
                '"' => Some((0x02, 0x34)),
                ',' => Some((0x00, 0x36)),
                '.' => Some((0x00, 0x37)),
                '/' => Some((0x00, 0x38)),
                '!' => Some((0x02, 0x1E)),
                '@' => Some((0x02, 0x1F)),
                '#' => Some((0x02, 0x20)),
                '?' => Some((0x02, 0x38)),
                _ => None,
            } {
                tap_shortcut(mods, hid);
            }
        }
    }

    pub fn mouse_click(button: u8) {
        let code = match button {
            1 => 0x110,
            2 => 0x111,
            3 => 0x112,
            4 => 0x113,
            5 => 0x114,
            _ => return,
        };
        let _ = emit_uinput(EV_KEY, code, 1);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = emit_uinput(EV_KEY, code, 0);
    }

    pub fn mouse_scroll(dx: i32, dy: i32) {
        if dy != 0 {
            let _ = emit_uinput(EV_REL, REL_WHEEL, dy);
        }
        if dx != 0 {
            let _ = emit_uinput(EV_REL, REL_HWHEEL, dx);
        }
    }

    pub fn trigger_media(cmd: MediaCmd) {
        let key_code = match cmd {
            MediaCmd::PlayPause => 164,
            MediaCmd::Stop => 166,
            MediaCmd::NextTrack => 163,
            MediaCmd::PrevTrack => 165,
            MediaCmd::VolumeUp => 115,
            MediaCmd::VolumeDown => 114,
            MediaCmd::Mute => 113,
            MediaCmd::Calculator => 140,
            MediaCmd::BrowserHome => 172,
            MediaCmd::Mail => 155,
            MediaCmd::Search => 217,
            MediaCmd::Screenshot => 99,
        };
        let _ = emit_uinput(EV_KEY, key_code, 1);
        let _ = emit_uinput(EV_KEY, key_code, 0);
    }

    #[derive(Clone, Copy)]
    enum FgBackend {
        Hyprland,
        Sway,
        Xdotool,
        None,
    }

    static FG_BACKEND: OnceLock<FgBackend> = OnceLock::new();

    fn detect_fg_backend() -> FgBackend {
        let has = |cmd: &str| {
            Command::new("sh")
                .args(["-c", &format!("command -v {cmd} >/dev/null 2>&1")])
                .status()
                .is_ok_and(|s| s.success())
        };
        if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() && has("hyprctl") {
            return FgBackend::Hyprland;
        }
        if std::env::var_os("SWAYSOCK").is_some() && has("swaymsg") {
            return FgBackend::Sway;
        }
        if std::env::var_os("DISPLAY").is_some() && has("xdotool") {
            return FgBackend::Xdotool;
        }
        FgBackend::None
    }

    pub fn active_foreground_exe() -> Option<String> {
        match *FG_BACKEND.get_or_init(detect_fg_backend) {
            FgBackend::Hyprland => {
                let out = Command::new("hyprctl")
                    .args(["activewindow", "-j"])
                    .output()
                    .ok()?;
                if !out.status.success() {
                    return None;
                }
                let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
                v.get("class")
                    .and_then(|c| c.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_lowercase())
            }
            FgBackend::Sway => {
                let out = Command::new("swaymsg")
                    .args(["-t", "get_tree"])
                    .output()
                    .ok()?;
                if !out.status.success() {
                    return None;
                }
                let txt = String::from_utf8_lossy(&out.stdout);
                let idx = txt.find("\"focused\": true")?;
                let slice = &txt[idx.saturating_sub(260)..(idx + 260).min(txt.len())];
                for key in ["\"app_id\": \"", "\"class\": \""] {
                    if let Some(pos) = slice.find(key) {
                        let rest = &slice[pos + key.len()..];
                        if let Some(end) = rest.find('"') {
                            let name = &rest[..end];
                            if !name.is_empty() {
                                return Some(name.to_lowercase());
                            }
                        }
                    }
                }
                None
            }
            FgBackend::Xdotool => {
                let out = Command::new("xdotool")
                    .args(["getactivewindow", "getwindowpid"])
                    .output()
                    .ok()?;
                if out.status.success() {
                    let pid = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                        return Some(comm.trim().to_lowercase());
                    }
                }
                None
            }
            FgBackend::None => None,
        }
    }
}

pub use imp::{
    active_foreground_exe, key_down, key_up, mouse_click, mouse_scroll, tap_shortcut,
    trigger_media, type_text,
};
