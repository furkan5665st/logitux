use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use logitux_hidpp::{
    features::{
        onboard::{
            build_macro_sector_image, encode_onboard_macro_bytes, ButtonAction, OnboardMode,
            OnboardProfile,
        },
        perkey::ZoneColor,
        rgb::{RgbColor, RgbEffect},
    },
    DeviceSummary, FeatureId, HidppError,
};
use logitux_transport::{
    connect_all_devices_diagnosed, enumerate_logitech_devices, grant_linux_hidraw_access,
    BlockedHidDevice, ConnectedDevice,
};

use crate::store::{
    compile_assignment_to_onboard_macro, DiscordSettings, MouseCmd, ProfileConfig, RichAssignment,
    UserMacro,
};

#[derive(Debug, Clone)]
pub struct DeviceUiSnapshot {
    pub summary: DeviceSummary,
    pub profile: Option<OnboardProfile>,
    pub startup_effect: bool,
}

pub fn compact_dpi_stages(
    stages: [u16; 5],
    active_idx: usize,
    shift_idx: usize,
) -> ([u16; 5], usize, usize) {
    let mut out = [0u16; 5];
    let mut new_active = 0usize;
    let mut new_shift = 0usize;
    let mut write_pos = 0usize;
    for (old_idx, &dpi) in stages.iter().enumerate() {
        if dpi > 0 {
            if old_idx == active_idx {
                new_active = write_pos;
            }
            if old_idx == shift_idx {
                new_shift = write_pos;
            }
            out[write_pos] = dpi;
            write_pos += 1;
        }
    }
    if write_pos == 0 {
        out[0] = 800;
        return (out, 0, 0);
    }
    (out, new_active.min(write_pos - 1), new_shift.min(write_pos - 1))
}

pub fn compute_8110_button_map(prof: &ProfileConfig, gshift_active: bool) -> [u8; 16] {
    let mut map = [0u8; 16];
    let layer = if gshift_active {
        &prof.gshift_buttons
    } else {
        &prof.buttons
    };
    for btn in 0..6usize {
        if matches!(prof.buttons[btn], RichAssignment::Mouse(MouseCmd::GShift)) {
            map[btn] = 0x00;
            continue;
        }
        map[btn] = match &layer[btn] {
            RichAssignment::Mouse(MouseCmd::PrimaryClick) => 0x01,
            RichAssignment::Mouse(MouseCmd::SecondaryClick) => 0x02,
            RichAssignment::Mouse(MouseCmd::MiddleClick) => 0x03,
            RichAssignment::Mouse(MouseCmd::Back) => 0x04,
            RichAssignment::Mouse(MouseCmd::Forward) => 0x05,
            _ => 0x00,
        };
    }
    if !map[0..6].contains(&0x01) {
        map[0] = 0x01;
    }
    map
}

pub enum HwCommand {
    ScanDevices,
    GrantLinuxAccess {
        persistent_rule: bool,
        allow_terminal_fallback: bool,
    },
    SelectDevice(usize),
    SetDpi(u16),
    SetReportRate(u16),
    SetRgbPreset {
        effect: RgbEffect,
        persist: bool,
    },
    SetPerKey {
        colors: [RgbColor; 3],
        persist: bool,
    },
    SetStartupEffect(bool),
    SyncButtonMap([u8; 16]),
    SetOnboardMode {
        mode: OnboardMode,
        profile: ProfileConfig,
        effect: RgbEffect,
        macros: Vec<UserMacro>,
        discord: DiscordSettings,
    },
    CommitOnboard {
        profile: ProfileConfig,
        effect: RgbEffect,
        macros: Vec<UserMacro>,
        discord: DiscordSettings,
        activate_onboard: bool,
        notify_toast: bool,
    },
    Shutdown(Option<Sender<()>>),
}

pub enum HwEvent {
    DevicesUpdated {
        devices: Vec<DeviceUiSnapshot>,
        blocked_linux: Vec<BlockedHidDevice>,
    },
    LinuxAccessFinished {
        ok: bool,
        msg: String,
    },
    ButtonMask(u16),
    OnboardModeChanged(OnboardMode),
    ProfileUpdated {
        idx: usize,
        profile: Box<OnboardProfile>,
        summary: Box<DeviceSummary>,
    },
    StartupEffectChanged(usize, bool),
    Toast {
        msg: String,
        is_err: bool,
    },
}

struct WorkerMouse {
    connected: ConnectedDevice,
    summary: DeviceSummary,
    profile: Option<OnboardProfile>,
    active_sector: u16,
    startup_effect: bool,
}

pub struct HardwareWorkerHandle {
    pub tx: Sender<HwCommand>,
    pub rx: Receiver<HwEvent>,
}

impl HardwareWorkerHandle {
    pub fn spawn(ctx: egui::Context, initial_map: [u8; 16]) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<HwCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<HwEvent>();

        thread::spawn(move || {
            run_worker_loop(ctx, cmd_rx, evt_tx, initial_map);
        });

        Self {
            tx: cmd_tx,
            rx: evt_rx,
        }
    }

    pub fn send(&self, cmd: HwCommand) {
        let _ = self.tx.send(cmd);
    }
}

fn run_worker_loop(
    ctx: egui::Context,
    cmd_rx: Receiver<HwCommand>,
    evt_tx: Sender<HwEvent>,
    initial_map: [u8; 16],
) {
    let mut devices: Vec<WorkerMouse> = Vec::new();
    let mut selected_idx: usize = 0;
    let mut onboard_mode = OnboardMode::Onboard;
    let mut spy_enabled = false;
    let mut last_mask: u16 = 0;
    let mut current_button_map = initial_map;

    scan_into(
        &mut devices,
        &mut selected_idx,
        &mut onboard_mode,
        &mut spy_enabled,
        &current_button_map,
        true,
        &evt_tx,
        &ctx,
    );

    let mut last_presence = presence_signature();
    let mut last_presence_check = Instant::now();
    let mut rescan_requested = false;

    loop {
        if last_presence_check.elapsed() >= Duration::from_millis(900) {
            last_presence_check = Instant::now();
            let signature = presence_signature();
            if signature != last_presence {
                last_presence = signature;
                if cfg!(target_os = "linux") {
                    thread::sleep(Duration::from_millis(250));
                }
                rescan_requested = true;
            }
        }

        let mut pending_cmds = Vec::new();
        while let Ok(c) = cmd_rx.try_recv() {
            pending_cmds.push(c);
        }

        if pending_cmds.is_empty() && !spy_enabled {
            match cmd_rx.recv_timeout(Duration::from_millis(250)) {
                Ok(c) => pending_cmds.push(c),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    cleanup_all_devices(&devices);
                    break;
                }
            }
            while let Ok(c) = cmd_rx.try_recv() {
                pending_cmds.push(c);
            }
        }

        let mut coalesced: Vec<HwCommand> = Vec::with_capacity(pending_cmds.len());
        for cmd in pending_cmds {
            match &cmd {
                HwCommand::SetDpi(_) => {
                    coalesced.retain(|prev| !matches!(prev, HwCommand::SetDpi(_)));
                }
                HwCommand::SetRgbPreset { persist: false, .. } => {
                    coalesced.retain(|prev| {
                        !matches!(prev, HwCommand::SetRgbPreset { persist: false, .. })
                    });
                }
                HwCommand::SetPerKey { persist: false, .. } => {
                    coalesced.retain(|prev| {
                        !matches!(prev, HwCommand::SetPerKey { persist: false, .. })
                    });
                }
                HwCommand::SyncButtonMap(_) => {
                    coalesced.retain(|prev| !matches!(prev, HwCommand::SyncButtonMap(_)));
                }
                HwCommand::CommitOnboard { .. } => {
                    coalesced.retain(|prev| !matches!(prev, HwCommand::CommitOnboard { .. }));
                }
                _ => {}
            }
            coalesced.push(cmd);
        }

        let mut should_exit = false;
        let mut shutdown_ack: Option<Sender<()>> = None;
        for cmd in coalesced {
            match cmd {
                HwCommand::Shutdown(ack) => {
                    cleanup_all_devices(&devices);
                    shutdown_ack = ack;
                    should_exit = true;
                    break;
                }
                HwCommand::ScanDevices => {
                    scan_into(
                        &mut devices,
                        &mut selected_idx,
                        &mut onboard_mode,
                        &mut spy_enabled,
                        &current_button_map,
                        false,
                        &evt_tx,
                        &ctx,
                    );
                    last_presence = presence_signature();
                }
                HwCommand::GrantLinuxAccess {
                    persistent_rule,
                    allow_terminal_fallback,
                } => {
                    let res = grant_linux_hidraw_access(persistent_rule, allow_terminal_fallback);
                    match res {
                        Ok(msg) => {
                            let _ = evt_tx.send(HwEvent::LinuxAccessFinished { ok: true, msg });
                            thread::sleep(Duration::from_millis(300));
                            scan_into(
                                &mut devices,
                                &mut selected_idx,
                                &mut onboard_mode,
                                &mut spy_enabled,
                                &current_button_map,
                                true,
                                &evt_tx,
                                &ctx,
                            );
                            last_presence = presence_signature();
                        }
                        Err(msg) => {
                            let _ = evt_tx.send(HwEvent::LinuxAccessFinished { ok: false, msg });
                        }
                    }
                    ctx.request_repaint();
                }
                HwCommand::SelectDevice(idx) => {
                    if idx < devices.len() {
                        if spy_enabled {
                            if let Some(prev_m) = devices.get(selected_idx) {
                                let _ = prev_m.connected.device.reset_button_spy_remapping();
                                let _ = prev_m.connected.device.stop_button_spy();
                            }
                            spy_enabled = false;
                        }
                        selected_idx = idx;
                        onboard_mode = devices[idx]
                            .summary
                            .onboard_mode
                            .unwrap_or(OnboardMode::Onboard);
                        sync_spy(
                            &devices,
                            selected_idx,
                            onboard_mode,
                            &current_button_map,
                            &mut spy_enabled,
                        );
                    }
                }
                HwCommand::SyncButtonMap(map) => {
                    current_button_map = map;
                    if onboard_mode != OnboardMode::Onboard {
                        if let Some(m) = devices.get(selected_idx) {
                            if m.connected.device.has_feature(FeatureId::MOUSE_BUTTON_SPY) {
                                let _ = m
                                    .connected
                                    .device
                                    .set_button_spy_remapping(&current_button_map);
                            }
                        }
                    }
                }
                HwCommand::SetDpi(dpi) => {
                    if let Some(m) = devices.get(selected_idx) {
                        if let Err(e) = m.connected.device.set_dpi(0, dpi) {
                            if is_connection_error(&e) {
                                rescan_requested = true;
                                let _ = evt_tx.send(HwEvent::Toast {
                                    msg: format!("DPI hatası: {e}"),
                                    is_err: true,
                                });
                                ctx.request_repaint();
                            } else if onboard_mode != OnboardMode::Onboard {
                                let _ = evt_tx.send(HwEvent::Toast {
                                    msg: format!("DPI hatası: {e}"),
                                    is_err: true,
                                });
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                HwCommand::SetReportRate(hz) => {
                    if let Some(m) = devices.get_mut(selected_idx) {
                        match m.connected.device.set_report_rate(hz) {
                            Ok(_) => {
                                ctx.request_repaint();
                            }
                            Err(e) => {
                                if is_connection_error(&e) {
                                    rescan_requested = true;
                                }
                                let _ = evt_tx.send(HwEvent::Toast {
                                    msg: format!("Bildirme hızı hatası: {e}"),
                                    is_err: true,
                                });
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                HwCommand::SetRgbPreset { effect, persist } => {
                    if let Some(m) = devices.get_mut(selected_idx) {
                        match m.connected.device.set_rgb_effect(0, effect, persist) {
                            Ok(_) => {
                                if m.connected.device.has_feature(FeatureId::ONBOARD_PROFILES) {
                                    onboard_mode = OnboardMode::Host;
                                    m.summary.onboard_mode = Some(OnboardMode::Host);
                                }
                                if let Some(p) = &mut m.profile {
                                    p.active_effect_0 = effect;
                                    p.active_effect_1 = effect;
                                }
                            }
                            Err(e) => {
                                if is_connection_error(&e) {
                                    rescan_requested = true;
                                }
                                let _ = evt_tx.send(HwEvent::Toast {
                                    msg: format!("LIGHTSYNC hatası: {e}"),
                                    is_err: true,
                                });
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                HwCommand::SetPerKey { colors, persist } => {
                    let zones = [
                        ZoneColor {
                            zone_id: 1,
                            color: colors[0],
                        },
                        ZoneColor {
                            zone_id: 2,
                            color: colors[1],
                        },
                        ZoneColor {
                            zone_id: 3,
                            color: colors[2],
                        },
                    ];
                    if let Some(m) = devices.get_mut(selected_idx) {
                        match m.connected.device.set_perkey_zones(&zones, persist) {
                            Ok(()) => {
                                onboard_mode = OnboardMode::Host;
                                m.summary.onboard_mode = Some(OnboardMode::Host);
                                let _ = evt_tx.send(HwEvent::OnboardModeChanged(OnboardMode::Host));
                                sync_spy(
                                    &devices,
                                    selected_idx,
                                    onboard_mode,
                                    &current_button_map,
                                    &mut spy_enabled,
                                );
                            }
                            Err(e) if is_connection_error(&e) => {
                                rescan_requested = true;
                            }
                            Err(_) => {}
                        }
                    }
                }
                HwCommand::SetStartupEffect(en) => {
                    if let Some(m) = devices.get_mut(selected_idx) {
                        match m.connected.device.set_startup_effect(en) {
                            Ok(_) => {
                                m.startup_effect = en;
                                let _ =
                                    evt_tx.send(HwEvent::StartupEffectChanged(selected_idx, en));
                                ctx.request_repaint();
                            }
                            Err(e) => {
                                if is_connection_error(&e) {
                                    rescan_requested = true;
                                }
                                let _ = evt_tx.send(HwEvent::Toast {
                                    msg: format!("Başlatma efekti hatası: {e}"),
                                    is_err: true,
                                });
                                ctx.request_repaint();
                            }
                        }
                    }
                }
                HwCommand::CommitOnboard {
                    profile,
                    effect,
                    macros,
                    discord,
                    activate_onboard,
                    notify_toast,
                } => {
                    let ok = commit_onboard_impl(
                        &mut devices,
                        selected_idx,
                        &profile,
                        effect,
                        &macros,
                        &discord,
                        activate_onboard,
                        notify_toast,
                        &mut onboard_mode,
                        &mut rescan_requested,
                        &evt_tx,
                        &ctx,
                    );
                    if ok {
                        sync_spy(
                            &devices,
                            selected_idx,
                            onboard_mode,
                            &current_button_map,
                            &mut spy_enabled,
                        );
                    }
                }
                HwCommand::SetOnboardMode {
                    mode,
                    profile,
                    effect,
                    macros,
                    discord,
                } => {
                    if mode == OnboardMode::Onboard {
                        let ok = commit_onboard_impl(
                            &mut devices,
                            selected_idx,
                            &profile,
                            effect,
                            &macros,
                            &discord,
                            true,
                            true,
                            &mut onboard_mode,
                            &mut rescan_requested,
                            &evt_tx,
                            &ctx,
                        );
                        if ok {
                            sync_spy(
                                &devices,
                                selected_idx,
                                onboard_mode,
                                &current_button_map,
                                &mut spy_enabled,
                            );
                        }
                    } else if let Some(m) = devices.get_mut(selected_idx) {
                        if m.connected.device.set_onboard_mode(mode).is_ok() {
                            let _ = m.connected.device.set_rgb_effect(0, effect, false);
                            onboard_mode = mode;
                            m.summary.onboard_mode = Some(mode);
                            let _ = evt_tx.send(HwEvent::OnboardModeChanged(mode));
                            sync_spy(
                                &devices,
                                selected_idx,
                                onboard_mode,
                                &current_button_map,
                                &mut spy_enabled,
                            );
                            ctx.request_repaint();
                        }
                    }
                }
            }
        }

        if should_exit {
            if let Some(ack) = shutdown_ack {
                let _ = ack.send(());
            }
            break;
        }

        if rescan_requested {
            rescan_requested = false;
            scan_into(
                &mut devices,
                &mut selected_idx,
                &mut onboard_mode,
                &mut spy_enabled,
                &current_button_map,
                true,
                &evt_tx,
                &ctx,
            );
            last_mask = 0;
            last_presence = presence_signature();
            last_presence_check = Instant::now();
        }

        if spy_enabled {
            if let Some(m) = devices.get(selected_idx) {
                match m.connected.device.poll_button_spy_event(6) {
                    Ok(Some(mask)) => {
                        if mask != last_mask {
                            last_mask = mask;
                            let _ = evt_tx.send(HwEvent::ButtonMask(mask));
                            ctx.request_repaint();
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        if is_connection_error(&e) {
                            rescan_requested = true;
                        }
                        thread::sleep(Duration::from_millis(8));
                    }
                }
            }
        }
    }
}

fn is_connection_error(err: &HidppError) -> bool {
    matches!(err, HidppError::Timeout | HidppError::Transport(_))
}

fn presence_signature() -> Vec<(u16, u16, String, i32)> {
    let mut signature: Vec<(u16, u16, String, i32)> = enumerate_logitech_devices()
        .map(|list| {
            list.into_iter()
                .map(|d| {
                    (
                        d.vendor_id,
                        d.product_id,
                        d.serial_number,
                        d.interface_number,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    signature.sort();
    signature
}

fn cleanup_all_devices(devices: &[WorkerMouse]) {
    for m in devices {
        if m.connected.device.has_feature(FeatureId::MOUSE_BUTTON_SPY) {
            let _ = m.connected.device.reset_button_spy_remapping();
            let _ = m.connected.device.stop_button_spy();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_into(
    devices: &mut Vec<WorkerMouse>,
    selected_idx: &mut usize,
    onboard_mode: &mut OnboardMode,
    spy_enabled: &mut bool,
    current_button_map: &[u8; 16],
    retry_on_udev_race: bool,
    evt_tx: &Sender<HwEvent>,
    ctx: &egui::Context,
) {
    devices.clear();
    *spy_enabled = false;
    let mut report = connect_all_devices_diagnosed().ok();
    if retry_on_udev_race && cfg!(target_os = "linux") {
        if let Some(r) = &report {
            if r.devices.is_empty() && !r.permission_denied.is_empty() {
                thread::sleep(Duration::from_millis(450));
                if let Ok(retried) = connect_all_devices_diagnosed() {
                    report = Some(retried);
                }
            }
        }
    }

    let mut blocked_linux = Vec::new();
    if let Some(rep) = report {
        blocked_linux = rep.permission_denied;
        for connected in rep.devices {
            let summary = connected.device.probe_summary();
            let active_sector = connected
                .device
                .get_active_onboard_profile()
                .unwrap_or(1)
                .max(1);
            let profile = connected.device.read_onboard_profile(active_sector).ok();
            let startup_effect = summary.startup_effect.unwrap_or(false);
            devices.push(WorkerMouse {
                connected,
                summary,
                profile,
                active_sector,
                startup_effect,
            });
        }
    }

    if !devices.is_empty() {
        *selected_idx = (*selected_idx).min(devices.len() - 1);
        *onboard_mode = devices[*selected_idx]
            .summary
            .onboard_mode
            .unwrap_or(OnboardMode::Onboard);
        sync_spy(
            devices,
            *selected_idx,
            *onboard_mode,
            current_button_map,
            spy_enabled,
        );
    } else {
        *selected_idx = 0;
    }

    let snaps: Vec<DeviceUiSnapshot> = devices
        .iter()
        .map(|d| DeviceUiSnapshot {
            summary: d.summary.clone(),
            profile: d.profile.clone(),
            startup_effect: d.startup_effect,
        })
        .collect();
    let _ = evt_tx.send(HwEvent::DevicesUpdated {
        devices: snaps,
        blocked_linux,
    });
    ctx.request_repaint();
}

fn sync_spy(
    devices: &[WorkerMouse],
    selected_idx: usize,
    onboard_mode: OnboardMode,
    current_button_map: &[u8; 16],
    spy_enabled: &mut bool,
) {
    let Some(m) = devices.get(selected_idx) else {
        *spy_enabled = false;
        return;
    };
    if !m.connected.device.has_feature(FeatureId::MOUSE_BUTTON_SPY) {
        *spy_enabled = false;
        return;
    }
    let want = onboard_mode != OnboardMode::Onboard;
    if want {
        if !*spy_enabled && m.connected.device.start_button_spy().is_ok() {
            *spy_enabled = true;
        }
        let _ = m
            .connected
            .device
            .set_button_spy_remapping(current_button_map);
    } else if *spy_enabled {
        let _ = m.connected.device.reset_button_spy_remapping();
        let _ = m.connected.device.stop_button_spy();
        *spy_enabled = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_onboard_impl(
    devices: &mut [WorkerMouse],
    selected_idx: usize,
    active_prof: &ProfileConfig,
    eff: RgbEffect,
    macros: &[UserMacro],
    discord: &DiscordSettings,
    activate_onboard: bool,
    notify_toast: bool,
    onboard_mode: &mut OnboardMode,
    rescan_requested: &mut bool,
    evt_tx: &Sender<HwEvent>,
    ctx: &egui::Context,
) -> bool {
    let Some(m) = devices.get_mut(selected_idx) else {
        return false;
    };

    let (compacted_dpis, active_idx, shift_idx) = compact_dpi_stages(
        active_prof.dpi_stages,
        active_prof.active_dpi_idx,
        active_prof.shift_dpi_idx,
    );

    if !m.connected.device.has_feature(FeatureId::ONBOARD_PROFILES) {
        let target_dpi = compacted_dpis[active_idx].max(400);
        let _ = m.connected.device.set_dpi(0, target_dpi);
        if active_prof.report_rate_hz > 0 {
            let _ = m.connected.device.set_report_rate(active_prof.report_rate_hz);
        }
        let _ = m.connected.device.set_rgb_effect(0, eff, true);
        ctx.request_repaint();
        return true;
    }

    if m.profile.is_none() {
        let sec = m.active_sector;
        m.profile = m.connected.device.read_onboard_profile(sec).ok();
    }
    let Some(hw_prof) = &mut m.profile else {
        let _ = evt_tx.send(HwEvent::Toast {
            msg: "Yerleşik profil sektörü okunamadı.".into(),
            is_err: true,
        });
        ctx.request_repaint();
        return false;
    };

    let sector_count = m
        .summary
        .onboard
        .as_ref()
        .map(|d| d.sector_count as u16)
        .unwrap_or(16);
    let sector_size = m
        .summary
        .onboard
        .as_ref()
        .map(|d| d.sector_size as usize)
        .unwrap_or(255)
        .max(16);
    let mut next_macro_sector = m
        .summary
        .onboard
        .as_ref()
        .map(|d| d.first_macro_sector())
        .unwrap_or(2)
        .max(2);

    let mut macro_writes: Vec<(u16, Vec<u8>)> = Vec::new();
    let prev_buttons = hw_prof.buttons.clone();
    let prev_gshift = hw_prof.gshift_buttons.clone();

    let mut resolve_btn = |item: &RichAssignment, prev: Option<ButtonAction>| -> ButtonAction {
        if let Some(ops) = compile_assignment_to_onboard_macro(item, macros, discord) {
            if next_macro_sector < sector_count && next_macro_sector <= 0xFF {
                let page = next_macro_sector as u8;
                let stream = encode_onboard_macro_bytes(&ops, sector_size.saturating_sub(2));
                let sector_img = build_macro_sector_image(&stream, sector_size);
                macro_writes.push((next_macro_sector, sector_img));
                next_macro_sector += 1;
                return ButtonAction::Macro { page, offset: 0 };
            }
        }
        if let RichAssignment::Macro { macro_id, .. } = item {
            if macro_id.starts_with("hw-macro-") {
                if let Some(old @ ButtonAction::Macro { .. }) = prev {
                    return old;
                }
            }
        }
        item.to_onboard_with_discord(discord)
    };

    hw_prof.dpi_stages = compacted_dpis;
    hw_prof.default_dpi_index = active_idx as u8;
    hw_prof.shift_dpi_index = shift_idx as u8;
    if active_prof.report_rate_hz > 0 && active_prof.report_rate_hz <= 1000 {
        hw_prof.report_rate_ms = (1000 / active_prof.report_rate_hz).max(1) as u8;
    }
    hw_prof.name = active_prof.name.clone();
    hw_prof.active_effect_0 = eff;
    hw_prof.active_effect_1 = eff;
    hw_prof.powersave_effect_0 = eff;
    hw_prof.powersave_effect_1 = eff;

    for (i, b) in active_prof.buttons.iter().enumerate() {
        if i < hw_prof.buttons.len() {
            hw_prof.buttons[i] = resolve_btn(b, prev_buttons.get(i).copied());
        }
    }
    for (i, b) in active_prof.gshift_buttons.iter().enumerate() {
        if i < hw_prof.gshift_buttons.len() {
            hw_prof.gshift_buttons[i] = resolve_btn(b, prev_gshift.get(i).copied());
        }
    }

    let encoded = hw_prof.encode_updated();
    if let Some(parent) = crate::store::AppStore::config_path().parent() {
        let backup_dir = parent.join("backups");
        let _ = fs::create_dir_all(&backup_dir);
        if let Ok(j) = serde_json::to_string_pretty(&hw_prof) {
            let _ = fs::write(
                backup_dir.join(format!(
                    "sector_{:04X}_pid_{:04X}.json",
                    m.active_sector, m.connected.descriptor.product_id
                )),
                j,
            );
        }
    }

    let prev_mode = *onboard_mode;
    let _ = m.connected.device.set_onboard_mode(OnboardMode::Host);

    for (sec_id, data) in macro_writes {
        if let Err(e) = m
            .connected
            .device
            .write_onboard_sector_verified(sec_id, &data)
        {
            if is_connection_error(&e) {
                *rescan_requested = true;
            }
            if prev_mode == OnboardMode::Onboard {
                let _ = m.connected.device.set_onboard_mode(OnboardMode::Onboard);
            }
            let _ = evt_tx.send(HwEvent::Toast {
                msg: format!("Makro bellek yazma hatası (sektör {sec_id}): {e}"),
                is_err: true,
            });
            ctx.request_repaint();
            return false;
        }
    }

    let sec = m.active_sector;
    let active_dpi_idx = active_idx as u8;
    let btn_count = m
        .summary
        .onboard
        .as_ref()
        .map(|d| d.button_count as usize)
        .unwrap_or(6);
    let write_res = m
        .connected
        .device
        .write_onboard_sector_verified(sec, &encoded);

    match write_res {
        Ok(verified_raw) => {
            m.connected.device.ensure_rgb_powered();
            let target_mode = if activate_onboard || prev_mode == OnboardMode::Onboard {
                OnboardMode::Onboard
            } else {
                prev_mode
            };
            if target_mode == OnboardMode::Onboard {
                if m.connected.device.set_onboard_mode(OnboardMode::Onboard).is_ok() {
                    *onboard_mode = OnboardMode::Onboard;
                    m.summary.onboard_mode = Some(OnboardMode::Onboard);
                    let _ = evt_tx.send(HwEvent::OnboardModeChanged(OnboardMode::Onboard));
                }
                let _ = m.connected.device.set_active_onboard_profile(sec);
                let _ = m.connected.device.set_active_onboard_dpi_index(active_dpi_idx);
            } else {
                *onboard_mode = target_mode;
                m.summary.onboard_mode = Some(target_mode);
            }
            if let Ok(reloaded) = OnboardProfile::parse(sec, &verified_raw, btn_count) {
                if let Some(dpis) = &mut m.summary.dpi {
                    if let Some(s0) = dpis.first_mut() {
                        let d = reloaded.dpi_stages[active_idx.min(4)];
                        if d > 0 {
                            s0.current_dpi = d;
                        }
                    }
                }
                if let Some(rr) = &mut m.summary.report_rate {
                    if reloaded.report_rate_ms > 0 {
                        rr.current_hz = 1000 / (reloaded.report_rate_ms as u16);
                    }
                }
                m.profile = Some(reloaded.clone());
                let _ = evt_tx.send(HwEvent::ProfileUpdated {
                    idx: selected_idx,
                    profile: Box::new(reloaded),
                    summary: Box::new(m.summary.clone()),
                });
            }
            if notify_toast {
                let _ = evt_tx.send(HwEvent::Toast {
                    msg: "Ayarlar farenin dahili belleğine kaydedildi.".into(),
                    is_err: false,
                });
            }
            ctx.request_repaint();
            true
        }
        Err(e) => {
            if is_connection_error(&e) {
                *rescan_requested = true;
            }
            if prev_mode == OnboardMode::Onboard {
                let _ = m.connected.device.set_onboard_mode(OnboardMode::Onboard);
            }
            let _ = evt_tx.send(HwEvent::Toast {
                msg: format!("Bellek yazma hatası: {e}"),
                is_err: true,
            });
            ctx.request_repaint();
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_dpi_stages_removes_zero_gaps() {
        let (stages, act, sh) = compact_dpi_stages([400, 0, 1600, 3200, 0], 2, 3);
        assert_eq!(stages, [400, 1600, 3200, 0, 0]);
        assert_eq!(act, 1);
        assert_eq!(sh, 2);

        let (empty, a2, s2) = compact_dpi_stages([0, 0, 0, 0, 0], 0, 0);
        assert_eq!(empty, [800, 0, 0, 0, 0]);
        assert_eq!((a2, s2), (0, 0));
    }
}
