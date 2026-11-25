use std::fs;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use logitux_hidpp::{
    HidppError,
    features::{
        onboard::{OnboardMode, OnboardProfile},
        perkey::ZoneColor,
        rgb::{RgbColor, RgbEffect},
    },
    DeviceSummary, FeatureId,
};
use logitux_transport::{connect_all_devices, enumerate_logitech_devices, ConnectedDevice};

use crate::store::{MouseCmd, ProfileConfig, RichAssignment};

#[derive(Debug, Clone)]
pub struct DeviceUiSnapshot {
    pub summary: DeviceSummary,
    pub profile: Option<OnboardProfile>,
    pub startup_effect: bool,
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
    },
    FlashSlot1 {
        profile: ProfileConfig,
        effect: RgbEffect,
    },
    Shutdown,
}

pub enum HwEvent {
    DevicesUpdated(Vec<DeviceUiSnapshot>),
    ButtonMask(u16),
    OnboardModeChanged(OnboardMode),
    StartupEffectChanged(usize, bool),
    Toast { msg: String, is_err: bool },
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
    pub fn spawn(
        ctx: egui::Context,
        initial_map: [u8; 16],
        initial_effect: Option<RgbEffect>,
    ) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<HwCommand>();
        let (evt_tx, evt_rx) = mpsc::channel::<HwEvent>();

        thread::spawn(move || {
            run_worker_loop(ctx, cmd_rx, evt_tx, initial_map, initial_effect);
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
    initial_effect: Option<RgbEffect>,
) {
    let mut devices: Vec<WorkerMouse> = Vec::new();
    let mut selected_idx: usize = 0;
    let mut onboard_mode = OnboardMode::Host;
    let mut spy_enabled = false;
    let mut last_mask: u16 = 0;
    let mut current_button_map = initial_map;

    scan_into(
        &mut devices,
        &mut selected_idx,
        &mut onboard_mode,
        &mut spy_enabled,
        &current_button_map,
        initial_effect,
        &evt_tx,
        &ctx,
    );

    let mut last_presence = presence_signature();
    let mut last_presence_check = Instant::now();

    loop {
        let mut rescan_requested = false;
        if last_presence_check.elapsed() >= Duration::from_millis(900) {
            last_presence_check = Instant::now();
            let signature = presence_signature();
            if signature != last_presence {
                last_presence = signature;
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
                _ => {}
            }
            coalesced.push(cmd);
        }

        let mut should_exit = false;
        for cmd in coalesced {
            match cmd {
                HwCommand::Shutdown => {
                    cleanup_all_devices(&devices);
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
                        None,
                        &evt_tx,
                        &ctx,
                    );
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
                            .unwrap_or(OnboardMode::Host);
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
                            }
                            let _ = evt_tx.send(HwEvent::Toast {
                                msg: format!("DPI hatası: {e}"),
                                is_err: true,
                            });
                            ctx.request_repaint();
                        }
                    }
                }
                HwCommand::SetReportRate(hz) => {
                    if let Some(m) = devices.get_mut(selected_idx) {
                        match m.connected.device.set_report_rate(hz) {
                            Ok(_) => {
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
                                onboard_mode = OnboardMode::Host;
                                if let Some(p) = &mut m.profile {
                                    p.active_effect_0 = effect;
                                    p.active_effect_1 = effect;
                                }
                                let _ = evt_tx.send(HwEvent::OnboardModeChanged(OnboardMode::Host));
                                sync_spy(
                                    &devices,
                                    selected_idx,
                                    onboard_mode,
                                    &current_button_map,
                                    &mut spy_enabled,
                                );
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
                                let _ =
                                    evt_tx.send(HwEvent::OnboardModeChanged(OnboardMode::Host));
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
                HwCommand::FlashSlot1 { profile, effect } => {
                    flash_slot1_impl(
                        &mut devices,
                        selected_idx,
                        &profile,
                        effect,
                        onboard_mode,
                        &evt_tx,
                        &ctx,
                    );
                }
                HwCommand::SetOnboardMode {
                    mode,
                    profile,
                    effect,
                } => {
                    if mode == OnboardMode::Onboard {
                        flash_slot1_impl(
                            &mut devices,
                            selected_idx,
                            &profile,
                            effect,
                            mode,
                            &evt_tx,
                            &ctx,
                        );
                    }
                    if let Some(m) = devices.get_mut(selected_idx) {
                        if m.connected.device.set_onboard_mode(mode).is_ok() {
                            if mode == OnboardMode::Onboard {
                                let _ = m
                                    .connected
                                    .device
                                    .set_active_onboard_profile(m.active_sector);
                            } else {
                                let _ = m.connected.device.set_rgb_effect(0, effect, true);
                            }
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
            break;
        }

        if rescan_requested {
            scan_into(
                &mut devices,
                &mut selected_idx,
                &mut onboard_mode,
                &mut spy_enabled,
                &current_button_map,
                None,
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
                    Err(_) => {
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
    initial_effect: Option<RgbEffect>,
    evt_tx: &Sender<HwEvent>,
    ctx: &egui::Context,
) {
    devices.clear();
    *spy_enabled = false;
    if let Ok(list) = connect_all_devices() {
        for connected in list {
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
            .unwrap_or(OnboardMode::Host);
        if *onboard_mode != OnboardMode::Onboard {
            if let Some(eff) = initial_effect {
                let _ = devices[*selected_idx]
                    .connected
                    .device
                    .set_rgb_effect(0, eff, false);
            }
        }
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
    let _ = evt_tx.send(HwEvent::DevicesUpdated(snaps));
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

fn flash_slot1_impl(
    devices: &mut [WorkerMouse],
    selected_idx: usize,
    active_prof: &ProfileConfig,
    eff: RgbEffect,
    onboard_mode: OnboardMode,
    evt_tx: &Sender<HwEvent>,
    ctx: &egui::Context,
) {
    let Some(m) = devices.get_mut(selected_idx) else {
        return;
    };
    let Some(hw_prof) = &mut m.profile else {
        return;
    };

    hw_prof.dpi_stages = active_prof.dpi_stages;
    hw_prof.default_dpi_index = active_prof.active_dpi_idx.min(4) as u8;
    hw_prof.shift_dpi_index = active_prof.shift_dpi_idx.min(4) as u8;
    if active_prof.report_rate_hz > 0 && active_prof.report_rate_hz <= 1000 {
        hw_prof.report_rate_ms = (1000 / active_prof.report_rate_hz) as u8;
    }
    hw_prof.name = active_prof.name.clone();
    hw_prof.active_effect_0 = eff;
    hw_prof.active_effect_1 = eff;
    hw_prof.powersave_effect_0 = eff;
    hw_prof.powersave_effect_1 = eff;

    for (i, b) in active_prof.buttons.iter().enumerate() {
        if i < hw_prof.buttons.len() {
            hw_prof.buttons[i] = b.to_onboard();
        }
    }
    for (i, b) in active_prof.gshift_buttons.iter().enumerate() {
        if i < hw_prof.gshift_buttons.len() {
            hw_prof.gshift_buttons[i] = b.to_onboard();
        }
    }

    let encoded = hw_prof.encode_updated();
    let _ = fs::create_dir_all("backups");
    if let Ok(j) = serde_json::to_string_pretty(&hw_prof) {
        let _ = fs::write(
            format!(
                "backups/sector_{:04X}_pid_{:04X}.json",
                m.active_sector, m.connected.descriptor.product_id
            ),
            j,
        );
    }

    let sec = m.active_sector;
    match m.connected.device.write_onboard_sector(sec, &encoded) {
        Ok(_) => {
            if onboard_mode == OnboardMode::Onboard {
                let _ = m.connected.device.set_active_onboard_profile(sec);
            }
            if let Ok(reloaded) = m.connected.device.read_onboard_profile(sec) {
                m.profile = Some(reloaded);
            }
            let _ = evt_tx.send(HwEvent::Toast {
                msg: "Profil 1 yerleşik belleğe (CRC-16 doğrulanmış) kaydedildi.".into(),
                is_err: false,
            });
            ctx.request_repaint();
        }
        Err(e) => {
            let _ = evt_tx.send(HwEvent::Toast {
                msg: format!("Bellek yazma hatası: {e}"),
                is_err: true,
            });
            ctx.request_repaint();
        }
    }
}
