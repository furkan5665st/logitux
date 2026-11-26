use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use logitux_hidpp::{
    features::{
        onboard::OnboardMode,
        rgb::{RgbColor, RgbEffect, WaveDirection},
    },
    FeatureId,
};
use logitux_transport::BlockedHidDevice;

use crate::{
    canvas::{self, FullDpiEvent},
    hw_worker::{
        compute_8110_button_map, DeviceUiSnapshot, HardwareWorkerHandle, HwCommand, HwEvent,
    },
    i18n::Lang,
    icons::{self, GhubIcon},
    input_synth, integrations,
    macro_engine::MacroRuntime,
    macro_ui::{MacroWizardOutcome, MacroWizardState},
    store::{
        all_hid_keys, builtin_ghub_commands, format_hid_combo, AppStore, CustomAction,
        CustomActionKind, DiscordAction, IntegrationAction, MediaCmd, MouseCmd, ObsAction,
        ProfileConfig, RichAssignment, SlobsAction,
    },
    theme,
    views_home::{self, DeviceSettingsEvent, ProfileFilter},
};

fn clean_device_display_name(raw: &str) -> String {
    let up = raw.trim().to_uppercase();
    for suffix in [" GAMING MOUSE", " OYUN MOUSE'U", " MOUSE"] {
        if let Some(stripped) = up.strip_suffix(suffix) {
            let s = stripped.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    up
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootScreen {
    HomeDevices,
    HomeProfiles,
    DeviceStudio,
    DeviceSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceTab {
    Sensitivity,
    Assignments,
    Lightsync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignSubTab {
    Commands,
    Keys,
    Actions,
    Macros,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightsyncSubTab {
    Presets,
    Freestyle,
    Animations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresetKind {
    Off,
    Fixed,
    ColorCycle,
    Breathing,
    ColorWave,
    ColorBlend,
}

pub struct LogiTuxApp {
    store: AppStore,
    hw: HardwareWorkerHandle,
    devices: Vec<DeviceUiSnapshot>,
    selected_idx: usize,

    screen: RootScreen,
    dev_tab: DeviceTab,
    assign_tab: AssignSubTab,
    rgb_tab: LightsyncSubTab,

    profile_popup_open: bool,
    global_settings_open: bool,
    about_open: bool,

    prof_filter: ProfileFilter,
    prof_search: String,
    adding_app_modal: bool,
    new_app_name: String,
    new_app_exe: String,

    live_dpi: u16,
    dpi_min: u16,
    dpi_max: u16,
    dpi_step: u16,
    editing_dpi_stage: Option<usize>,
    dpi_edit_buf: String,
    supported_rates: Vec<u16>,

    selected_button: usize,
    gshift_layer: bool,
    gshift_held_live: bool,
    dpi_shift_held_live: bool,
    assign_search: String,
    dragging_item: Option<RichAssignment>,
    custom_key_ctrl: bool,
    custom_key_shift: bool,
    custom_key_alt: bool,
    custom_key_super: bool,

    discord_detected: Arc<AtomicBool>,
    obs_detected: Arc<AtomicBool>,
    fg_exe: Arc<Mutex<Option<String>>>,
    discord_config_open: bool,
    recording_discord_shortcut: Option<u8>,
    obs_config_open: bool,
    custom_action_modal: bool,
    new_act_name: String,
    new_act_target: String,
    new_act_kind_idx: usize,
    discord_join_chan_buf: String,

    macro_wizard: MacroWizardState,
    macro_runtime: MacroRuntime,

    pressed_mask: u16,

    lightsync_enabled: bool,
    preset_kind: PresetKind,
    preset_color: [u8; 3],
    preset_period_ms: u16,
    preset_brightness: u8,
    wave_dir: WaveDirection,
    perkey_colors: [RgbColor; 3],
    selected_zone: usize,
    selected_anim_idx: usize,
    last_anim_step: Instant,
    anim_frame_cursor: usize,

    onboard_mode: OnboardMode,
    pending_onboard_commit: Option<Instant>,
    onboard_saving: bool,
    onboard_saved_ok: bool,

    blocked_linux_devices: Vec<BlockedHidDevice>,
    linux_perm_auto_tried: bool,
    linux_perm_busy: bool,
    linux_install_persistent_rule: bool,

    toast_msg: String,
    toast_err: bool,
}

impl LogiTuxApp {
    pub fn new(ctx: egui::Context) -> Self {
        let store = AppStore::load();
        let initial_map = compute_8110_button_map(store.active_profile(), false);
        let hw = HardwareWorkerHandle::spawn(ctx, initial_map);

        let discord_detected = Arc::new(AtomicBool::new(false));
        let obs_detected = Arc::new(AtomicBool::new(false));
        let fg_exe = Arc::new(Mutex::new(None));

        {
            let disc_flag = discord_detected.clone();
            let obs_flag = obs_detected.clone();
            let fg_slot = fg_exe.clone();
            let obs_cfg = store.obs.clone();
            std::thread::spawn(move || loop {
                disc_flag.store(integrations::is_discord_detected(), Ordering::Relaxed);
                obs_flag.store(integrations::is_obs_detected(&obs_cfg), Ordering::Relaxed);
                if let Ok(mut g) = fg_slot.lock() {
                    *g = input_synth::active_foreground_exe();
                }
                std::thread::sleep(Duration::from_millis(2500));
            });
        }

        Self {
            store,
            hw,
            devices: Vec::new(),
            selected_idx: 0,
            screen: RootScreen::HomeDevices,
            dev_tab: DeviceTab::Sensitivity,
            assign_tab: AssignSubTab::Commands,
            rgb_tab: LightsyncSubTab::Presets,
            profile_popup_open: false,
            global_settings_open: false,
            about_open: false,
            prof_filter: ProfileFilter::All,
            prof_search: String::new(),
            adding_app_modal: false,
            new_app_name: String::new(),
            new_app_exe: String::new(),
            live_dpi: 800,
            dpi_min: 50,
            dpi_max: 8000,
            dpi_step: 50,
            editing_dpi_stage: None,
            dpi_edit_buf: String::new(),
            supported_rates: vec![1000, 500, 250, 125],
            selected_button: 5,
            gshift_layer: false,
            gshift_held_live: false,
            dpi_shift_held_live: false,
            assign_search: String::new(),
            dragging_item: None,
            custom_key_ctrl: false,
            custom_key_shift: false,
            custom_key_alt: false,
            custom_key_super: false,
            discord_detected,
            obs_detected,
            fg_exe,
            discord_config_open: false,
            recording_discord_shortcut: None,
            obs_config_open: false,
            custom_action_modal: false,
            new_act_name: String::new(),
            new_act_target: String::new(),
            new_act_kind_idx: 0,
            discord_join_chan_buf: String::new(),
            macro_wizard: MacroWizardState::default(),
            macro_runtime: MacroRuntime::new(),
            pressed_mask: 0,
            lightsync_enabled: true,
            preset_kind: PresetKind::ColorWave,
            preset_color: [0, 149, 255],
            preset_period_ms: 5000,
            preset_brightness: 100,
            wave_dir: WaveDirection::Horizontal,
            perkey_colors: [
                RgbColor::new(255, 0, 128),
                RgbColor::new(0, 184, 252),
                RgbColor::new(148, 56, 255),
            ],
            selected_zone: 0,
            selected_anim_idx: 0,
            last_anim_step: Instant::now(),
            anim_frame_cursor: 0,
            onboard_mode: OnboardMode::Onboard,
            pending_onboard_commit: None,
            onboard_saving: false,
            onboard_saved_ok: false,
            blocked_linux_devices: Vec::new(),
            linux_perm_auto_tried: false,
            linux_perm_busy: false,
            linux_install_persistent_rule: true,
            toast_msg: String::new(),
            toast_err: false,
        }
    }

    fn notify(&mut self, msg: impl Into<String>, is_err: bool) {
        self.toast_msg = msg.into();
        self.toast_err = is_err;
    }

    fn scan_devices(&mut self) {
        self.pressed_mask = 0;
        self.hw.send(HwCommand::ScanDevices);
    }

    fn sync_from_hardware(&mut self) {
        let Some(m) = self.devices.get(self.selected_idx).cloned() else {
            return;
        };
        if let Some(dpis) = &m.summary.dpi {
            if let Some(s0) = dpis.first() {
                self.live_dpi = s0.current_dpi;
                self.dpi_min = s0.spec.min_dpi().max(50);
                self.dpi_max = s0.spec.max_dpi().max(800);
                self.dpi_step = s0.spec.step_dpi().max(1);
            }
        }
        if let Some(rr) = &m.summary.report_rate {
            if rr.current_hz > 0 {
                self.store.active_profile_mut().report_rate_hz = rr.current_hz;
            }
            if !rr.supported_hz.is_empty() {
                let mut r = rr.supported_hz.clone();
                r.sort_unstable_by(|a, b| b.cmp(a));
                self.supported_rates = r;
            }
        }
        if let Some(mode) = m.summary.onboard_mode {
            self.onboard_mode = mode;
            if mode == OnboardMode::Onboard {
                self.onboard_saved_ok = true;
            }
        }
        if let Some(prof) = &m.profile {
            if prof.crc_valid {
                let discord_cfg = self.store.discord.clone();
                let active = self.store.active_profile_mut();
                if prof.dpi_stages.iter().any(|&d| d > 0) {
                    active.dpi_stages = prof.dpi_stages;
                    let def_idx = (prof.default_dpi_index as usize).min(4);
                    if active.dpi_stages[def_idx] > 0 {
                        active.active_dpi_idx = def_idx;
                        self.live_dpi = active.dpi_stages[def_idx];
                    }
                    let sh_idx = (prof.shift_dpi_index as usize).min(4);
                    if active.dpi_stages[sh_idx] > 0 {
                        active.shift_dpi_idx = sh_idx;
                    }
                }
                if prof.report_rate_ms > 0 {
                    let hz = 1000u16 / (prof.report_rate_ms as u16);
                    if matches!(hz, 125 | 250 | 500 | 1000) {
                        active.report_rate_hz = hz;
                    }
                }
                for (i, &hw_act) in prof.buttons.iter().take(6).enumerate() {
                    if active.buttons[i].to_onboard_with_discord(&discord_cfg) != hw_act
                        && !matches!(hw_act, logitux_hidpp::features::onboard::ButtonAction::Inherit)
                    {
                        if !matches!(
                            (&active.buttons[i], hw_act),
                            (
                                RichAssignment::Macro { .. }
                                    | RichAssignment::Mouse(MouseCmd::DoubleClick),
                                logitux_hidpp::features::onboard::ButtonAction::Macro { .. }
                            )
                        ) {
                            active.buttons[i] = RichAssignment::from_onboard(hw_act);
                        }
                    }
                }
                for (i, &hw_act) in prof.gshift_buttons.iter().take(6).enumerate() {
                    if active.gshift_buttons[i].to_onboard_with_discord(&discord_cfg) != hw_act
                        && !matches!(hw_act, logitux_hidpp::features::onboard::ButtonAction::Inherit)
                    {
                        if !matches!(
                            (&active.gshift_buttons[i], hw_act),
                            (
                                RichAssignment::Macro { .. }
                                    | RichAssignment::Mouse(MouseCmd::DoubleClick),
                                logitux_hidpp::features::onboard::ButtonAction::Macro { .. }
                            )
                        ) {
                            active.gshift_buttons[i] = RichAssignment::from_onboard(hw_act);
                        }
                    }
                }
            }
            match prof.active_effect_0 {
                RgbEffect::Off => {
                    self.lightsync_enabled = false;
                    self.preset_kind = PresetKind::Off;
                }
                RgbEffect::Fixed { color, brightness } => {
                    self.lightsync_enabled = true;
                    self.preset_kind = PresetKind::Fixed;
                    self.preset_color = [color.r, color.g, color.b];
                    self.preset_brightness = brightness.clamp(1, 100);
                }
                RgbEffect::Breathing {
                    color,
                    period_ms,
                    brightness,
                } => {
                    self.lightsync_enabled = true;
                    self.preset_kind = PresetKind::Breathing;
                    self.preset_color = [color.r, color.g, color.b];
                    self.preset_period_ms = period_ms.clamp(500, 20000);
                    self.preset_brightness = brightness.clamp(1, 100);
                }
                RgbEffect::ColorCycle {
                    period_ms,
                    brightness,
                } => {
                    self.lightsync_enabled = true;
                    self.preset_kind = PresetKind::ColorCycle;
                    self.preset_period_ms = period_ms.clamp(500, 20000);
                    self.preset_brightness = brightness.clamp(1, 100);
                }
                RgbEffect::ColorWave {
                    period_ms,
                    direction,
                    brightness,
                } => {
                    self.lightsync_enabled = true;
                    self.preset_kind = PresetKind::ColorWave;
                    self.preset_period_ms = period_ms.clamp(500, 20000);
                    self.wave_dir = direction;
                    self.preset_brightness = brightness.clamp(1, 100);
                }
                RgbEffect::ColorBlend {
                    period_ms,
                    brightness,
                } => {
                    self.lightsync_enabled = true;
                    self.preset_kind = PresetKind::ColorBlend;
                    self.preset_period_ms = period_ms.clamp(500, 20000);
                    self.preset_brightness = brightness.clamp(1, 100);
                }
            }
        }
    }

    fn sync_button_map_to_hw(&self) {
        let map = compute_8110_button_map(
            self.store.active_profile(),
            self.gshift_held_live || self.gshift_layer,
        );
        self.hw.send(HwCommand::SyncButtonMap(map));
    }

    fn commit_onboard_now(&mut self, notify_toast: bool) {
        self.pending_onboard_commit = None;
        if self.devices.is_empty() {
            return;
        }
        self.onboard_saving = true;
        self.onboard_mode = OnboardMode::Onboard;
        let profile = self.store.active_profile().clone();
        let effect = self.current_rgb_effect();
        let macros = self.store.macros.clone();
        let discord = self.store.discord.clone();
        self.hw.send(HwCommand::CommitOnboard {
            profile,
            effect,
            macros,
            discord,
            activate_onboard: true,
            notify_toast,
        });
    }

    fn schedule_onboard_commit(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        self.pending_onboard_commit = Some(Instant::now() + Duration::from_millis(320));
        self.onboard_saving = true;
    }

    fn activate_profile_by_id(&mut self, prof_id: &str) {
        self.macro_runtime.stop_all();
        if self.store.profiles.iter().any(|p| p.id == prof_id) {
            self.store.active_profile_id = prof_id.to_string();
            self.store.save();
            let p = self.store.active_profile().clone();
            let target_dpi = p.dpi_stages[p.active_dpi_idx.min(4)].max(400);
            self.live_dpi = target_dpi;
            self.commit_onboard_now(true);
        }
    }

    fn apply_live_dpi(&mut self, dpi: u16) {
        let clamped = dpi.clamp(self.dpi_min, self.dpi_max);
        let step = self.dpi_step.max(1);
        let snapped = self.dpi_min + ((clamped - self.dpi_min + step / 2) / step) * step;
        self.live_dpi = snapped.min(self.dpi_max);
        self.hw.send(HwCommand::SetDpi(self.live_dpi));
        self.schedule_onboard_commit();
    }

    fn apply_live_rate(&mut self, hz: u16) {
        self.store.active_profile_mut().report_rate_hz = hz;
        self.store.save();
        self.commit_onboard_now(false);
    }

    fn resync_hardware_settings(&mut self) {
        if self.onboard_mode == OnboardMode::Onboard {
            return;
        }
        self.hw.send(HwCommand::SetDpi(self.live_dpi));
        let rate = self.store.active_profile().report_rate_hz;
        if rate > 0 {
            self.hw.send(HwCommand::SetReportRate(rate));
        }
        self.sync_button_map_to_hw();
        if self.lightsync_enabled {
            if self.rgb_tab == LightsyncSubTab::Freestyle {
                let colors = self.perkey_colors;
                self.apply_perkey_rgb(colors, false);
            } else {
                self.apply_rgb_preset(false);
            }
        } else {
            self.hw.send(HwCommand::SetRgbPreset {
                effect: RgbEffect::Off,
                persist: false,
            });
        }
    }

    fn current_rgb_effect(&self) -> RgbEffect {
        if !self.lightsync_enabled {
            return RgbEffect::Off;
        }
        let color = RgbColor::new(
            self.preset_color[0],
            self.preset_color[1],
            self.preset_color[2],
        );
        match self.preset_kind {
            PresetKind::Off => RgbEffect::Off,
            PresetKind::Fixed => RgbEffect::Fixed {
                color,
                brightness: self.preset_brightness,
            },
            PresetKind::Breathing => RgbEffect::Breathing {
                color,
                period_ms: self.preset_period_ms,
                brightness: self.preset_brightness,
            },
            PresetKind::ColorCycle => RgbEffect::ColorCycle {
                period_ms: self.preset_period_ms,
                brightness: self.preset_brightness,
            },
            PresetKind::ColorWave => RgbEffect::ColorWave {
                period_ms: self.preset_period_ms,
                direction: self.wave_dir,
                brightness: self.preset_brightness,
            },
            PresetKind::ColorBlend => RgbEffect::ColorBlend {
                period_ms: self.preset_period_ms,
                brightness: self.preset_brightness,
            },
        }
    }

    fn apply_rgb_preset(&mut self, persist: bool) {
        if persist {
            self.commit_onboard_now(false);
        } else {
            let eff = self.current_rgb_effect();
            self.hw.send(HwCommand::SetRgbPreset {
                effect: eff,
                persist: false,
            });
            self.schedule_onboard_commit();
        }
    }

    fn apply_perkey_rgb(&mut self, colors: [RgbColor; 3], persist: bool) {
        self.onboard_mode = OnboardMode::Host;
        self.hw.send(HwCommand::SetPerKey { colors, persist });
    }

    fn flash_active_profile_to_slot1(&mut self) {
        self.commit_onboard_now(true);
    }

    fn set_onboard_mode_live(&mut self, mode: OnboardMode) {
        self.onboard_mode = mode;
        let profile = self.store.active_profile().clone();
        let effect = self.current_rgb_effect();
        let macros = self.store.macros.clone();
        let discord = self.store.discord.clone();
        if mode == OnboardMode::Onboard {
            self.onboard_saving = true;
            self.pending_onboard_commit = None;
        }
        self.hw.send(HwCommand::SetOnboardMode {
            mode,
            profile,
            effect,
            macros,
            discord,
        });
    }

    fn assign_to_button(&mut self, btn_idx: usize, item: RichAssignment) {
        if btn_idx >= 6 {
            return;
        }
        let lang = self.store.lang;
        if !self.gshift_layer && !matches!(item, RichAssignment::Mouse(MouseCmd::PrimaryClick)) {
            let has_other_primary =
                self.store
                    .active_profile()
                    .buttons
                    .iter()
                    .enumerate()
                    .any(|(i, b)| {
                        i != btn_idx && matches!(b, RichAssignment::Mouse(MouseCmd::PrimaryClick))
                    });
            if !has_other_primary
                && matches!(
                    self.store.active_profile().buttons[btn_idx],
                    RichAssignment::Mouse(MouseCmd::PrimaryClick)
                )
            {
                self.notify(
                    lang.pick(
                        "Birinci Tıklama en az bir düğmede kalmalıdır (önce başka bir düğmeye Birinci Tıklama atayın).",
                        "Primary Click must remain assigned to at least one button.",
                    ),
                    true,
                );
                return;
            }
        }
        let host_only = item.is_host_only();
        let prof = self.store.active_profile_mut();
        if self.gshift_layer {
            prof.gshift_buttons[btn_idx] = item;
        } else {
            prof.buttons[btn_idx] = item;
        }
        self.store.save();
        if host_only {
            self.sync_button_map_to_hw();
            self.notify(
                lang.pick(
                    "Bu özel eylem bilgisayar bağlantısı (Host modu) gerektirir; diğer tüm ayarlar dahili belleğe yazılır.",
                    "This action requires Host mode while LogiTux is running.",
                ),
                false,
            );
        } else {
            self.commit_onboard_now(true);
        }
    }

    fn badge_for_assignment(&self, candidate: &RichAssignment) -> Option<String> {
        let p = self.store.active_profile();
        let arr = if self.gshift_layer {
            &p.gshift_buttons
        } else {
            &p.buttons
        };
        arr.iter()
            .position(|a| a == candidate)
            .map(|idx| format!("G{}", idx + 1))
    }

    fn handle_live_spy_transition(&mut self, prev: u16, next: u16) {
        let prof = self.store.active_profile().clone();
        for btn in 0..6usize {
            let bit = 1u16 << btn;
            let was_down = (prev & bit) != 0;
            let is_down = (next & bit) != 0;
            if was_down == is_down {
                continue;
            }

            if matches!(prof.buttons[btn], RichAssignment::Mouse(MouseCmd::GShift)) {
                self.gshift_held_live = is_down;
                self.sync_button_map_to_hw();
                continue;
            }

            let use_gshift = self.gshift_held_live || self.gshift_layer;
            let binding = if use_gshift {
                prof.gshift_buttons[btn].clone()
            } else {
                prof.buttons[btn].clone()
            };

            let is_hardware_click = matches!(
                &binding,
                RichAssignment::Mouse(
                    MouseCmd::PrimaryClick
                        | MouseCmd::SecondaryClick
                        | MouseCmd::MiddleClick
                        | MouseCmd::Back
                        | MouseCmd::Forward
                )
            );
            if is_hardware_click {
                continue;
            }

            if is_down {
                self.execute_binding_press(btn, &binding, &prof);
            } else {
                self.execute_binding_release(btn, &binding, &prof);
            }
        }
    }

    fn execute_binding_press(
        &mut self,
        btn: usize,
        binding: &RichAssignment,
        prof: &ProfileConfig,
    ) {
        match binding {
            RichAssignment::Mouse(cmd) => match cmd {
                MouseCmd::DpiCycle => {
                    let enabled: Vec<usize> = prof
                        .dpi_stages
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &d)| (d > 0).then_some(i))
                        .collect();
                    if !enabled.is_empty() {
                        let cur_pos = enabled
                            .iter()
                            .position(|&i| i == prof.active_dpi_idx)
                            .unwrap_or(0);
                        let next_i = enabled[(cur_pos + 1) % enabled.len()];
                        self.store.active_profile_mut().active_dpi_idx = next_i;
                        self.store.save();
                        self.apply_live_dpi(prof.dpi_stages[next_i]);
                    }
                }
                MouseCmd::DpiUp => {
                    let enabled: Vec<usize> = prof
                        .dpi_stages
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &d)| (d > 0).then_some(i))
                        .collect();
                    if let Some(&next_i) = enabled.iter().find(|&&i| i > prof.active_dpi_idx) {
                        self.store.active_profile_mut().active_dpi_idx = next_i;
                        self.store.save();
                        self.apply_live_dpi(prof.dpi_stages[next_i]);
                    }
                }
                MouseCmd::DpiDown => {
                    let enabled: Vec<usize> = prof
                        .dpi_stages
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &d)| (d > 0).then_some(i))
                        .collect();
                    if let Some(&prev_i) = enabled.iter().rev().find(|&&i| i < prof.active_dpi_idx)
                    {
                        self.store.active_profile_mut().active_dpi_idx = prev_i;
                        self.store.save();
                        self.apply_live_dpi(prof.dpi_stages[prev_i]);
                    }
                }
                MouseCmd::DpiShift => {
                    let s_dpi = prof.dpi_stages[prof.shift_dpi_idx.min(4)];
                    if s_dpi > 0 {
                        self.dpi_shift_held_live = true;
                        self.apply_live_dpi(s_dpi);
                    }
                }
                MouseCmd::GoToDpiShift => {
                    let s_idx = prof.shift_dpi_idx.min(4);
                    if prof.dpi_stages[s_idx] > 0 {
                        self.store.active_profile_mut().active_dpi_idx = s_idx;
                        self.store.save();
                        self.apply_live_dpi(prof.dpi_stages[s_idx]);
                    }
                }
                MouseCmd::DefaultDpi => {
                    self.apply_live_dpi(800);
                }
                MouseCmd::DoubleClick => {
                    std::thread::spawn(|| {
                        input_synth::mouse_click(1);
                        std::thread::sleep(std::time::Duration::from_millis(35));
                        input_synth::mouse_click(1);
                    });
                }
                MouseCmd::ScrollUp => input_synth::mouse_scroll(0, 1),
                MouseCmd::ScrollDown => input_synth::mouse_scroll(0, -1),
                MouseCmd::ScrollLeft => input_synth::mouse_scroll(-1, 0),
                MouseCmd::ScrollRight => input_synth::mouse_scroll(1, 0),
                MouseCmd::PrimaryClick => input_synth::mouse_click(1),
                MouseCmd::SecondaryClick => input_synth::mouse_click(2),
                MouseCmd::MiddleClick => input_synth::mouse_click(3),
                MouseCmd::Back => input_synth::mouse_click(4),
                MouseCmd::Forward => input_synth::mouse_click(5),
                MouseCmd::OnboardProfileCycle => {
                    let cur_pos = self
                        .store
                        .profiles
                        .iter()
                        .position(|p| p.id == self.store.active_profile_id)
                        .unwrap_or(0);
                    let next_id = self.store.profiles[(cur_pos + 1) % self.store.profiles.len()]
                        .id
                        .clone();
                    self.activate_profile_by_id(&next_id);
                }
                MouseCmd::GShift | MouseCmd::Disabled => {}
            },
            RichAssignment::Media(m) => {
                let mc = *m;
                std::thread::spawn(move || input_synth::trigger_media(mc));
            }
            RichAssignment::Command {
                modifiers, hid_key, ..
            } => {
                let m = *modifiers;
                let k = *hid_key;
                std::thread::spawn(move || input_synth::tap_shortcut(m, k));
            }
            RichAssignment::Key { modifiers, hid_key } => {
                input_synth::key_down(*modifiers, *hid_key);
            }
            RichAssignment::Action(IntegrationAction::Discord(DiscordAction::PushToTalk)) => {
                let ptt = self.store.discord.ptt_shortcut;
                input_synth::key_down(ptt.0, ptt.1);
            }
            RichAssignment::Action(act) => {
                let act_c = act.clone();
                let disc_c = self.store.discord.clone();
                let obs_c = self.store.obs.clone();
                std::thread::spawn(move || {
                    let _ = integrations::execute_integration(&act_c, &disc_c, &obs_c);
                });
            }
            RichAssignment::Macro { macro_id, .. } => {
                if let Some(mac) = self
                    .store
                    .macros
                    .iter()
                    .find(|m| m.id == *macro_id)
                    .cloned()
                {
                    self.macro_runtime.on_button_press(
                        btn,
                        mac,
                        self.store.discord.clone(),
                        self.store.obs.clone(),
                    );
                }
            }
        }
    }

    fn execute_binding_release(
        &mut self,
        btn: usize,
        binding: &RichAssignment,
        prof: &ProfileConfig,
    ) {
        match binding {
            RichAssignment::Mouse(MouseCmd::DpiShift) => {
                if self.dpi_shift_held_live {
                    self.dpi_shift_held_live = false;
                    let restore = prof.dpi_stages[prof.active_dpi_idx.min(4)].max(800);
                    self.apply_live_dpi(restore);
                }
            }
            RichAssignment::Key { modifiers, hid_key } => {
                input_synth::key_up(*modifiers, *hid_key);
            }
            RichAssignment::Action(IntegrationAction::Discord(DiscordAction::PushToTalk)) => {
                let ptt = self.store.discord.ptt_shortcut;
                input_synth::key_up(ptt.0, ptt.1);
            }
            RichAssignment::Macro { .. } => {
                self.macro_runtime.on_button_release(btn);
            }
            _ => {}
        }
    }
}

impl eframe::App for LogiTuxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let lang = self.store.lang;

        if self.macro_wizard.active {
            match self.macro_wizard.show(ctx, lang) {
                MacroWizardOutcome::None => {}
                MacroWizardOutcome::Cancelled => {}
                MacroWizardOutcome::Saved(mac) => {
                    let saved_id = mac.id.clone();
                    if let Some(existing) = self.store.macros.iter_mut().find(|m| m.id == mac.id) {
                        *existing = mac;
                    } else {
                        self.store.macros.push(mac);
                    }
                    self.store.save();
                    let active = self.store.active_profile();
                    let assigned = active
                        .buttons
                        .iter()
                        .chain(active.gshift_buttons.iter())
                        .any(|b| matches!(b, RichAssignment::Macro { macro_id, .. } if *macro_id == saved_id));
                    if assigned {
                        self.commit_onboard_now(true);
                    }
                }
            }
            return;
        }

        if let Some(deadline) = self.pending_onboard_commit {
            if Instant::now() >= deadline {
                self.commit_onboard_now(false);
            } else {
                ctx.request_repaint_after(Duration::from_millis(80));
            }
        }

        while let Ok(ev) = self.hw.rx.try_recv() {
            match ev {
                HwEvent::DevicesUpdated {
                    devices: snaps,
                    blocked_linux,
                } => {
                    let was_empty = self.devices.is_empty();
                    self.devices = snaps;
                    self.blocked_linux_devices = blocked_linux;
                    if self.devices.is_empty() {
                        self.selected_idx = 0;
                        self.screen = RootScreen::HomeDevices;
                        self.pressed_mask = 0;
                        self.onboard_mode = OnboardMode::Onboard;
                        self.onboard_saving = false;
                        self.pending_onboard_commit = None;
                        if cfg!(target_os = "linux")
                            && !self.blocked_linux_devices.is_empty()
                            && !self.linux_perm_auto_tried
                            && !self.linux_perm_busy
                        {
                            self.linux_perm_auto_tried = true;
                            self.linux_perm_busy = true;
                            self.hw.send(HwCommand::GrantLinuxAccess {
                                persistent_rule: self.linux_install_persistent_rule,
                            });
                        }
                    } else {
                        self.selected_idx = self.selected_idx.min(self.devices.len() - 1);
                        self.sync_from_hardware();
                        if was_empty && self.onboard_mode != OnboardMode::Onboard {
                            self.resync_hardware_settings();
                        }
                    }
                }
                HwEvent::LinuxAccessFinished { ok, msg } => {
                    self.linux_perm_busy = false;
                    self.notify(msg, !ok);
                }
                HwEvent::ButtonMask(mask) => {
                    let prev = self.pressed_mask;
                    self.pressed_mask = mask;
                    if prev != mask {
                        self.handle_live_spy_transition(prev, mask);
                    }
                }
                HwEvent::OnboardModeChanged(mode) => {
                    self.onboard_mode = mode;
                    if let Some(d) = self.devices.get_mut(self.selected_idx) {
                        d.summary.onboard_mode = Some(mode);
                    }
                }
                HwEvent::OnboardSaved { ok } => {
                    self.onboard_saving = false;
                    self.onboard_saved_ok = ok;
                }
                HwEvent::StartupEffectChanged(idx, en) => {
                    if let Some(d) = self.devices.get_mut(idx) {
                        d.startup_effect = en;
                    }
                }
                HwEvent::Toast { msg, is_err } => {
                    self.notify(msg, is_err);
                }
            }
        }

        let needs_rgb_anim = matches!(self.screen, RootScreen::HomeDevices)
            || (self.screen == RootScreen::DeviceStudio && self.dev_tab == DeviceTab::Lightsync);
        if needs_rgb_anim {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        if self.rgb_tab == LightsyncSubTab::Animations {
            if let Some(anim) = self.store.animations.get(self.selected_anim_idx).cloned() {
                if !anim.frames.is_empty()
                    && self.last_anim_step.elapsed().as_millis() >= anim.frame_ms.max(80) as u128
                {
                    self.last_anim_step = Instant::now();
                    self.anim_frame_cursor = (self.anim_frame_cursor + 1) % anim.frames.len();
                    let f = anim.frames[self.anim_frame_cursor];
                    let cols = [
                        RgbColor::new(f[0][0], f[0][1], f[0][2]),
                        RgbColor::new(f[1][0], f[1][1], f[1][2]),
                        RgbColor::new(f[2][0], f[2][1], f[2][2]),
                    ];
                    self.apply_perkey_rgb(cols, false);
                }
            }
        }

        if self.store.auto_profile_switching {
            let fg_opt = self.fg_exe.lock().ok().and_then(|g| g.clone());
            if let Some(fg) = fg_opt {
                if let Some(matched_app) = self
                    .store
                    .apps
                    .iter()
                    .find(|a| a.enabled && a.executables.iter().any(|e| fg.contains(e)))
                {
                    if let Some(p) = self
                        .store
                        .profiles
                        .iter()
                        .find(|p| p.app_id == matched_app.id)
                    {
                        if p.id != self.store.active_profile_id {
                            let pid = p.id.clone();
                            self.activate_profile_by_id(&pid);
                        }
                    }
                }
            }
        }

        if self.screen == RootScreen::DeviceSettings {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(theme::BG_BLACK)
                        .inner_margin(egui::Margin::symmetric(24, 18)),
                )
                .show(ctx, |ui| {
                    if let Some(m) = self.devices.get(self.selected_idx) {
                        let summary = m.summary.clone();
                        let se = m.startup_effect;
                        let prof_name = self.store.active_profile().name.clone();
                        if let Some(ev) = views_home::render_device_settings_page(
                            ui,
                            &self.store,
                            &summary,
                            se,
                            self.onboard_mode,
                            &prof_name,
                        ) {
                            match ev {
                                DeviceSettingsEvent::Back => {
                                    self.screen = RootScreen::DeviceStudio;
                                }
                                DeviceSettingsEvent::ToggleStartupEffect(en) => {
                                    if let Some(dm) = self.devices.get_mut(self.selected_idx) {
                                        dm.startup_effect = en;
                                    }
                                    self.hw.send(HwCommand::SetStartupEffect(en));
                                }
                                DeviceSettingsEvent::ToggleOnboardMode(target) => {
                                    self.set_onboard_mode_live(target);
                                }
                                DeviceSettingsEvent::FlashSlot1 => {
                                    self.flash_active_profile_to_slot1();
                                }
                                DeviceSettingsEvent::RestoreDefaults => {
                                    *self.store.active_profile_mut() =
                                        ProfileConfig::default_desktop();
                                    self.store.save();
                                    self.live_dpi = 800;
                                    self.commit_onboard_now(true);
                                }
                            }
                        }
                    } else {
                        self.screen = RootScreen::HomeDevices;
                    }
                });
            return;
        }

        self.render_top_bar(ctx);

        self.render_modals(ctx);

        match self.screen {
            RootScreen::HomeDevices => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(theme::BG_BLACK))
                    .show(ctx, |ui| self.render_home_devices(ctx, ui));
            }
            RootScreen::HomeProfiles => {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(theme::BG_BLACK)
                            .inner_margin(egui::Margin::symmetric(22, 8)),
                    )
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if let Some(pid) = views_home::render_profiles_page(
                                    ui,
                                    &mut self.store,
                                    &mut self.prof_filter,
                                    &mut self.prof_search,
                                    &mut self.adding_app_modal,
                                    &mut self.new_app_name,
                                    &mut self.new_app_exe,
                                ) {
                                    self.activate_profile_by_id(&pid);
                                }
                            });
                    });
            }
            RootScreen::DeviceStudio => {
                if self.devices.is_empty() {
                    self.screen = RootScreen::HomeDevices;
                    return;
                }
                self.render_device_studio(ctx);
            }
            RootScreen::DeviceSettings => {}
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.macro_runtime.stop_all();
        if self.pending_onboard_commit.is_some() && !self.devices.is_empty() {
            self.commit_onboard_now(false);
            std::thread::sleep(Duration::from_millis(280));
        }
        self.hw.send(HwCommand::Shutdown);
        std::thread::sleep(Duration::from_millis(50));
    }
}

impl LogiTuxApp {
    fn render_top_bar(&mut self, ctx: &egui::Context) {
        let lang = self.store.lang;
        let is_inside_device = self.screen == RootScreen::DeviceStudio;

        egui::TopBottomPanel::top("ghub_top_header")
            .exact_height(54.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::symmetric(20, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if is_inside_device {
                        let (br, b_resp) =
                            ui.allocate_exact_size(Vec2::new(32.0, 32.0), Sense::click());
                        icons::draw_icon(
                            &ui.painter_at(br),
                            br.center(),
                            8.5,
                            GhubIcon::BackArrow,
                            if b_resp.hovered() {
                                Color32::WHITE
                            } else {
                                theme::TEXT_LIGHT
                            },
                        );
                        if b_resp.clicked() {
                            self.screen = RootScreen::HomeDevices;
                        }
                        ui.add_space(12.0);
                        let dev_title = self
                            .devices
                            .get(self.selected_idx)
                            .map(|d| clean_device_display_name(&d.summary.name))
                            .unwrap_or_else(|| "G102 LIGHTSYNC".into());
                        ui.label(
                            RichText::new(dev_title)
                                .size(22.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                    } else {
                        let (lr, l_resp) =
                            ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());
                        icons::draw_icon(
                            &ui.painter_at(lr),
                            lr.center(),
                            10.0,
                            GhubIcon::LogoMark,
                            Color32::WHITE,
                        );
                        if l_resp.clicked() {
                            self.screen = RootScreen::HomeDevices;
                        }
                        ui.add_space(24.0);

                        for (target, label) in [
                            (RootScreen::HomeDevices, lang.pick("Cihazlar", "Devices")),
                            (RootScreen::HomeProfiles, lang.pick("Profiller", "Profiles")),
                        ] {
                            let active = self.screen == target;
                            let (tr, tresp) =
                                ui.allocate_exact_size(Vec2::new(66.0, 36.0), Sense::click());
                            let tp = ui.painter_at(tr);
                            tp.text(
                                tr.center(),
                                egui::Align2::CENTER_CENTER,
                                label,
                                egui::FontId::proportional(12.5),
                                if active {
                                    theme::GHUB_BLUE
                                } else if tresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_LIGHT
                                },
                            );
                            if active {
                                tp.line_segment(
                                    [
                                        Pos2::new(tr.left() + 8.0, tr.bottom() - 2.0),
                                        Pos2::new(tr.right() - 8.0, tr.bottom() - 2.0),
                                    ],
                                    Stroke::new(2.0_f32, theme::GHUB_BLUE),
                                );
                            }
                            if tresp.clicked() {
                                self.screen = target;
                            }
                            ui.add_space(8.0);
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (ur, uresp) =
                            ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());
                        icons::draw_icon(
                            &ui.painter_at(ur),
                            ur.center(),
                            8.5,
                            GhubIcon::UserAvatar,
                            if uresp.hovered() {
                                Color32::WHITE
                            } else {
                                theme::TEXT_LIGHT
                            },
                        );
                        if uresp.clicked() {
                            self.about_open = !self.about_open;
                        }

                        if !is_inside_device {
                            ui.add_space(6.0);
                            let (gr, gresp) =
                                ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());
                            icons::draw_icon(
                                &ui.painter_at(gr),
                                gr.center(),
                                8.5,
                                GhubIcon::Gear,
                                if gresp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::TEXT_LIGHT
                                },
                            );
                            if gresp.clicked() {
                                self.global_settings_open = !self.global_settings_open;
                            }
                        }

                        ui.add_space(10.0);

                        if !self.devices.is_empty() {
                            let is_ob = self.onboard_mode == OnboardMode::Onboard;
                            let (dot_col, badge_txt, tip_txt) = if self.onboard_saving {
                                (
                                    theme::GHUB_BLUE,
                                    lang.pick("BELLEĞE YAZILIYOR...", "SAVING TO MOUSE..."),
                                    lang.pick(
                                        "Değişiklikler farenin dahili belleğine yazılıyor.",
                                        "Writing profile to mouse onboard memory.",
                                    ),
                                )
                            } else if is_ob {
                                (
                                    theme::STATUS_GREEN,
                                    lang.pick("DAHİLİ BELLEKTE", "ONBOARD SAVED"),
                                    lang.pick(
                                        "Tüm ayarlar farenin dahili belleğinde kayıtlı. Programı kapatabilirsiniz, her sistemde çalışmaya devam eder. Yeniden yazmak için tıklayın.",
                                        "All settings are stored in mouse onboard memory. You can close LogiTux and keep using them anywhere. Click to re-write.",
                                    ),
                                )
                            } else {
                                (
                                    theme::DPI_AMBER,
                                    lang.pick("BELLEĞE YAZ", "SAVE TO MOUSE"),
                                    lang.pick(
                                        "Mevcut tüm ayarları farenin dahili belleğine yazıp Yerleşik Moda geçmek için tıklayın.",
                                        "Click to write all settings to mouse onboard memory and activate Onboard Mode.",
                                    ),
                                )
                            };
                            let ob_pill = egui::Button::new(
                                RichText::new(format!("● {badge_txt}"))
                                    .size(11.0)
                                    .strong()
                                    .color(dot_col),
                            )
                            .fill(Color32::from_rgb(20, 22, 26))
                            .stroke(Stroke::new(1.0_f32, dot_col.gamma_multiply(0.55)))
                            .corner_radius(CornerRadius::same(14))
                            .min_size(Vec2::new(138.0, 28.0));
                            if ui.add(ob_pill).on_hover_text(tip_txt).clicked() {
                                self.commit_onboard_now(true);
                            }
                            ui.add_space(10.0);
                        }

                        let prof = self.store.active_profile();
                        let app_nm = self.store.app_name_for_profile(prof);
                        let p_text = format!("{app_nm}: {}", prof.name);
                        let (pr, presp) =
                            ui.allocate_exact_size(Vec2::new(212.0, 34.0), Sense::click());
                        let pp = ui.painter_at(pr);
                        pp.rect_filled(pr, CornerRadius::same(5), Color32::from_rgb(24, 25, 29));
                        pp.rect_stroke(
                            pr,
                            CornerRadius::same(5),
                            Stroke::new(
                                1.0_f32,
                                if self.profile_popup_open || presp.hovered() {
                                    Color32::WHITE
                                } else {
                                    theme::BORDER_SUBTLE
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        icons::draw_icon(
                            &pp,
                            Pos2::new(pr.left() + 18.0, pr.center().y),
                            7.5,
                            GhubIcon::ProfileLock,
                            Color32::WHITE,
                        );
                        pp.text(
                            Pos2::new(pr.left() + 34.0, pr.center().y),
                            egui::Align2::LEFT_CENTER,
                            p_text,
                            egui::FontId::proportional(12.0),
                            Color32::WHITE,
                        );
                        pp.text(
                            Pos2::new(pr.right() - 14.0, pr.center().y),
                            egui::Align2::CENTER_CENTER,
                            if self.profile_popup_open {
                                "⌃"
                            } else {
                                "⌄"
                            },
                            egui::FontId::proportional(13.0),
                            Color32::WHITE,
                        );
                        if presp.clicked() {
                            self.profile_popup_open = !self.profile_popup_open;
                        }

                        if self.profile_popup_open {
                            egui::Area::new(egui::Id::new("ghub_profile_dropdown_area"))
                                .fixed_pos(Pos2::new(pr.left(), pr.bottom() + 2.0))
                                .order(egui::Order::Foreground)
                                .show(ctx, |ui| {
                                    egui::Frame::new()
                                        .fill(Color32::from_rgb(27, 29, 34))
                                        .stroke(Stroke::new(1.0_f32, theme::BORDER_DIM))
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(egui::Margin::same(0))
                                        .show(ui, |ui| {
                                            ui.set_width(212.0);
                                            let list = self.store.profiles.clone();
                                            for item in list {
                                                let active =
                                                    item.id == self.store.active_profile_id;
                                                let aname = self.store.app_name_for_profile(&item);
                                                let row_lbl = format!("{aname}: {}", item.name);
                                                let (rr, rresp) = ui.allocate_exact_size(
                                                    Vec2::new(212.0, 40.0),
                                                    Sense::click(),
                                                );
                                                let rp = ui.painter_at(rr);
                                                if rresp.hovered() {
                                                    rp.rect_filled(rr, 0.0, theme::BG_HOVER);
                                                }
                                                let fg = if active {
                                                    theme::GHUB_BLUE
                                                } else {
                                                    Color32::WHITE
                                                };
                                                icons::draw_icon(
                                                    &rp,
                                                    Pos2::new(rr.left() + 20.0, rr.center().y),
                                                    7.5,
                                                    GhubIcon::ProfileLock,
                                                    fg,
                                                );
                                                rp.text(
                                                    Pos2::new(rr.left() + 38.0, rr.center().y),
                                                    egui::Align2::LEFT_CENTER,
                                                    row_lbl,
                                                    egui::FontId::proportional(12.5),
                                                    fg,
                                                );
                                                if rresp.clicked() {
                                                    self.activate_profile_by_id(&item.id);
                                                    self.profile_popup_open = false;
                                                }
                                            }
                                            ui.separator();
                                            let (mr, mresp) = ui.allocate_exact_size(
                                                Vec2::new(212.0, 44.0),
                                                Sense::click(),
                                            );
                                            let mp = ui.painter_at(mr);
                                            mp.rect_filled(
                                                mr,
                                                CornerRadius::same(4),
                                                if mresp.hovered() {
                                                    Color32::from_rgb(32, 34, 40)
                                                } else {
                                                    Color32::from_rgb(18, 19, 22)
                                                },
                                            );
                                            mp.text(
                                                mr.center(),
                                                egui::Align2::CENTER_CENTER,
                                                lang.pick("PROFİLLERİ YÖNET", "MANAGE PROFILES"),
                                                egui::FontId::proportional(11.5),
                                                Color32::WHITE,
                                            );
                                            if mresp.clicked() {
                                                self.screen = RootScreen::HomeProfiles;
                                                self.profile_popup_open = false;
                                            }
                                        });
                                });
                        }
                    });
                });
            });
    }

    fn render_home_devices(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let lang = self.store.lang;
        let time_sec = ui.input(|i| i.time);

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if !self.toast_msg.is_empty() {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(&self.toast_msg)
                        .size(11.5)
                        .color(if self.toast_err {
                            theme::STATUS_RED
                        } else {
                            theme::STATUS_GREEN
                        }),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(26.0);
                let (pill_r, pill_resp) =
                    ui.allocate_exact_size(Vec2::new(62.0, 28.0), Sense::click());
                let pp = ui.painter_at(pill_r);
                pp.rect_filled(
                    pill_r,
                    CornerRadius::same(14),
                    Color32::from_rgb(24, 25, 29),
                );
                pp.line_segment(
                    [
                        Pos2::new(pill_r.center().x, pill_r.top() + 6.0),
                        Pos2::new(pill_r.center().x, pill_r.bottom() - 6.0),
                    ],
                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                );
                icons::draw_icon(
                    &pp,
                    Pos2::new(pill_r.left() + 16.0, pill_r.center().y),
                    6.5,
                    GhubIcon::GridView,
                    if self.store.home_grid_mode {
                        theme::GHUB_BLUE
                    } else {
                        theme::TEXT_LIGHT
                    },
                );
                icons::draw_icon(
                    &pp,
                    Pos2::new(pill_r.right() - 16.0, pill_r.center().y),
                    6.5,
                    GhubIcon::ListView,
                    if !self.store.home_grid_mode {
                        theme::GHUB_BLUE
                    } else {
                        theme::TEXT_LIGHT
                    },
                );
                if pill_resp.clicked() {
                    self.store.home_grid_mode = !self.store.home_grid_mode;
                    self.store.save();
                }
            });
        });

        if self.devices.is_empty() {
            ui.vertical_centered(|ui| {
                if !self.blocked_linux_devices.is_empty() {
                    ui.add_space(95.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "LOGITECH FARE BULUNDU — ERİŞİM İZNİ GEREKİYOR",
                            "LOGITECH MOUSE DETECTED — ACCESS PERMISSION NEEDED",
                        ))
                        .size(21.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    ui.add_space(10.0);
                    for blk in &self.blocked_linux_devices {
                        ui.label(
                            RichText::new(format!(
                                "{} (046D:{:04X}) · {}",
                                blk.name, blk.product_id, blk.path
                            ))
                            .size(13.0)
                            .color(theme::GHUB_BLUE),
                        );
                    }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "Linux çekirdeği /dev/hidraw erişimini varsayılan olarak kısıtlar. Elle kural eklemenize gerek yok; aşağıdaki düğmeye basarak anında izin verip fareyi kullanabilirsiniz.",
                            "Linux restricts /dev/hidraw by default. No manual udev rules needed; click below to grant access immediately.",
                        ))
                        .size(12.5)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(14.0);
                    ui.checkbox(
                        &mut self.linux_install_persistent_rule,
                        RichText::new(lang.pick(
                            "Kalıcı yap (sonraki açılışlarda tekrar şifre sormasın)",
                            "Make persistent (do not ask again on future launches)",
                        ))
                        .size(12.0)
                        .color(theme::TEXT_LIGHT),
                    );
                    ui.add_space(16.0);
                    let btn_lbl = if self.linux_perm_busy {
                        lang.pick("YETKİLENDİRİLİYOR...", "GRANTING ACCESS...")
                    } else {
                        lang.pick("FAREYE ERİŞİM İZNİ VER", "GRANT MOUSE ACCESS")
                    };
                    if ui
                        .add_enabled(
                            !self.linux_perm_busy,
                            egui::Button::new(RichText::new(btn_lbl).strong().color(Color32::WHITE))
                                .fill(theme::GHUB_BLUE)
                                .min_size(Vec2::new(230.0, 40.0)),
                        )
                        .clicked()
                    {
                        self.linux_perm_busy = true;
                        self.hw.send(HwCommand::GrantLinuxAccess {
                            persistent_rule: self.linux_install_persistent_rule,
                        });
                    }
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick("YENİDEN TARA", "RESCAN"))
                                    .color(theme::TEXT_LIGHT),
                            )
                            .fill(theme::BG_CONTROL)
                            .min_size(Vec2::new(150.0, 32.0)),
                        )
                        .clicked()
                    {
                        self.scan_devices();
                    }
                } else {
                    ui.add_space(140.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "LOGITECH GEAR'INIZI BAĞLAYIN",
                            "CONNECT YOUR LOGITECH GEAR",
                        ))
                        .size(22.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "USB veya LIGHTSPEED alıcısını takın.",
                            "Plug in your Logitech USB or LIGHTSPEED mouse.",
                        ))
                        .size(13.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(20.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick("CİHAZLARI TARA", "SCAN FOR DEVICES"))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme::GHUB_BLUE)
                            .min_size(Vec2::new(180.0, 38.0)),
                        )
                        .clicked()
                    {
                        self.scan_devices();
                    }
                }
            });
            return;
        }

        let zone_colors = canvas::compute_preview_zones(
            time_sec,
            if !self.lightsync_enabled {
                0
            } else {
                match self.rgb_tab {
                    LightsyncSubTab::Presets => 0,
                    LightsyncSubTab::Freestyle => 1,
                    LightsyncSubTab::Animations => 2,
                }
            },
            self.perkey_colors,
            None,
            self.current_rgb_effect(),
        );

        let avail = ui.available_size();
        ui.add_space((avail.y - 360.0).max(20.0) * 0.42);

        let mut enter_device: Option<usize> = None;
        let mut toggle_ob_now: Option<OnboardMode> = None;
        let has_modal_open =
            self.profile_popup_open || self.global_settings_open || self.about_open;

        ui.horizontal(|ui| {
            let card_w = if self.store.home_grid_mode {
                232.0
            } else {
                440.0
            };
            let card_h = if self.store.home_grid_mode {
                340.0
            } else {
                150.0
            };
            let count = self.devices.len() as f32;
            let total_w = count * card_w + (count - 1.0).max(0.0) * 24.0;
            ui.add_space((ui.available_width() - total_w).max(0.0) * 0.5);

            for i in 0..self.devices.len() {
                let dev_name = clean_device_display_name(&self.devices[i].summary.name);
                let is_wired = self.devices[i].summary.device_index == 0xFF;
                let is_ob = self.onboard_mode == OnboardMode::Onboard;

                let (card_r, card_resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
                let cp = ui.painter_at(card_r);
                cp.rect_filled(card_r, CornerRadius::same(8), theme::BG_HOME_CARD);

                let pointer_inside = !has_modal_open && ui.rect_contains_pointer(card_r);
                let hover_t = ctx.animate_bool_responsive(
                    ui.id().with(("home_card_hover_anim", i)),
                    pointer_inside,
                );
                if hover_t > 0.01 {
                    let alpha = (hover_t * 235.0).round() as u8;
                    cp.rect_stroke(
                        card_r,
                        CornerRadius::same(8),
                        Stroke::new(
                            1.35_f32,
                            Color32::from_rgba_unmultiplied(222, 226, 234, alpha),
                        ),
                        egui::StrokeKind::Inside,
                    );
                }

                cp.text(
                    Pos2::new(card_r.left() + 18.0, card_r.top() + 30.0),
                    egui::Align2::LEFT_CENTER,
                    &dev_name,
                    egui::FontId::proportional(16.5),
                    Color32::WHITE,
                );

                let usb_r = Rect::from_min_size(
                    Pos2::new(card_r.left() + 16.0, card_r.top() + 48.0),
                    Vec2::new(20.0, 20.0),
                );
                icons::draw_icon(
                    &cp,
                    usb_r.center(),
                    6.5,
                    GhubIcon::UsbPlug,
                    theme::TEXT_MUTED,
                );
                let usb_resp = ui.interact(usb_r, ui.id().with(("usb_tip", i)), Sense::hover());
                if usb_resp.hovered() {
                    theme::draw_speech_tooltip(
                        ctx,
                        usb_r,
                        if is_wired {
                            lang.pick("Bağlantı: USB", "Connection: USB")
                        } else {
                            "Bağlantı: LIGHTSPEED"
                        },
                        true,
                        false,
                    );
                }

                if self.store.home_grid_mode {
                    let stage_r = Rect::from_center_size(
                        Pos2::new(card_r.center().x, card_r.center().y + 6.0),
                        Vec2::new(158.0, 206.0),
                    );
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(stage_r));
                    let _ = canvas::draw_ghub_mouse_stage(
                        &mut child,
                        stage_r.size(),
                        zone_colors,
                        false,
                        None,
                        0,
                        &[],
                        false,
                        false,
                        false,
                        None,
                    );
                }

                let btn_ob_r = Rect::from_min_size(
                    Pos2::new(card_r.right() - 44.0, card_r.bottom() - 42.0),
                    Vec2::new(32.0, 32.0),
                );
                cp.rect_filled(btn_ob_r, CornerRadius::same(5), Color32::from_rgb(42, 43, 47));
                cp.rect_stroke(
                    btn_ob_r,
                    CornerRadius::same(5),
                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE),
                    egui::StrokeKind::Inside,
                );
                icons::draw_icon(
                    &cp,
                    btn_ob_r.center(),
                    8.0,
                    GhubIcon::OnboardChip { slashed: !is_ob },
                    if is_ob {
                        theme::GHUB_BLUE
                    } else {
                        Color32::WHITE
                    },
                );

                let ob_resp = ui.interact(btn_ob_r, ui.id().with(("ob_btn", i)), Sense::click());
                let released_in = |r: Rect| {
                    !has_modal_open
                        && ctx.input(|inp| {
                            inp.pointer.primary_released()
                                && inp.pointer.interact_pos().is_some_and(|p| r.contains(p))
                                && inp.pointer.press_origin().is_some_and(|p| r.contains(p))
                        })
                };
                let ob_clicked = ob_resp.clicked() || released_in(btn_ob_r);
                let card_clicked = card_resp.clicked() || released_in(card_r);
                if ob_resp.hovered() {
                    theme::draw_speech_tooltip(
                        ctx,
                        btn_ob_r,
                        lang.pick(
                            "Yerleşik Modu Etkinleştirin/Devre Dışı Bırakın. Etkinleştirildiğinde, ayarlarınız doğrudan cihazın yerleşik belleğine kaydedilir. Bazı özellikler yalnızca Yerleşik Mod devre dışı bırakıldığında kullanılabilir.",
                            "Enable/Disable Onboard Mode. When enabled, settings run directly from hardware onboard memory.",
                        ),
                        true,
                        false,
                    );
                }
                if ob_clicked {
                    toggle_ob_now = Some(if is_ob {
                        OnboardMode::Host
                    } else {
                        OnboardMode::Onboard
                    });
                } else if card_clicked {
                    enter_device = Some(i);
                }

                ui.add_space(20.0);
            }
        });

        if let Some(target_mode) = toggle_ob_now {
            self.set_onboard_mode_live(target_mode);
        }
        if let Some(idx) = enter_device {
            self.selected_idx = idx;
            self.hw.send(HwCommand::SelectDevice(idx));
            self.sync_from_hardware();
            self.screen = RootScreen::DeviceStudio;
        }
    }

    fn render_device_studio(&mut self, ctx: &egui::Context) {
        let lang = self.store.lang;

        egui::SidePanel::left("ghub_left_rail")
            .exact_width(58.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::symmetric(10, 16)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);
                    for (tab, icon, tip) in [
                        (
                            DeviceTab::Sensitivity,
                            GhubIcon::SensitivityDpi,
                            lang.pick("Duyarlılık (DPI)", "Sensitivity (DPI)"),
                        ),
                        (
                            DeviceTab::Assignments,
                            GhubIcon::AssignmentsPlus,
                            lang.pick("Atamalar", "Assignments"),
                        ),
                        (DeviceTab::Lightsync, GhubIcon::LightsyncSun, "LIGHTSYNC"),
                    ] {
                        let active = self.dev_tab == tab;
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(38.0, 38.0), Sense::click());
                        let p = ui.painter_at(r);
                        if active {
                            p.rect_filled(r, CornerRadius::same(8), theme::GHUB_BLUE);
                        } else if resp.hovered() {
                            p.rect_filled(r, CornerRadius::same(8), theme::BG_DRAWER);
                        }
                        icons::draw_icon(
                            &p,
                            r.center(),
                            10.0,
                            icon,
                            if active || resp.hovered() {
                                Color32::WHITE
                            } else {
                                theme::TEXT_MUTED
                            },
                        );
                        if resp.on_hover_text(tip).clicked() {
                            self.dev_tab = tab;
                        }
                        ui.add_space(10.0);
                    }

                    let rem = (ui.available_height() - 42.0).max(16.0);
                    ui.add_space(rem);
                    let (gr, gresp) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), Sense::click());
                    let gp = ui.painter_at(gr);
                    gp.rect_filled(gr, CornerRadius::same(6), Color32::from_rgb(16, 17, 20));
                    gp.rect_stroke(
                        gr,
                        CornerRadius::same(6),
                        Stroke::new(1.0_f32, theme::BORDER_DIM),
                        egui::StrokeKind::Inside,
                    );
                    icons::draw_icon(&gp, gr.center(), 9.0, GhubIcon::Gear, Color32::WHITE);
                    if gresp.hovered() {
                        theme::draw_speech_tooltip(
                            ctx,
                            gr,
                            lang.pick("Cihaz Ayarları", "Device Settings"),
                            false,
                            true,
                        );
                    }
                    if gresp.clicked() {
                        self.screen = RootScreen::DeviceSettings;
                    }
                });
            });

        egui::SidePanel::left("ghub_floating_drawer")
            .exact_width(366.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin {
                        left: 4,
                        right: 12,
                        top: 4,
                        bottom: 16,
                    }),
            )
            .show(ctx, |ui| {
                let full_h = ui.available_height();
                theme::floating_drawer_frame().show(ui, |ui| {
                    ui.set_min_height((full_h - 40.0).max(200.0));
                    match self.dev_tab {
                        DeviceTab::Sensitivity => self.render_sensitivity_card(ui),
                        DeviceTab::Assignments => self.render_assignments_card(ui),
                        DeviceTab::Lightsync => self.render_lightsync_card(ui),
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| match self.dev_tab {
                DeviceTab::Sensitivity => self.render_dpi_stage(ui),
                DeviceTab::Assignments => self.render_assignments_stage(ui),
                DeviceTab::Lightsync => self.render_lightsync_stage(ui),
            });
    }

    fn render_sensitivity_card(&mut self, ui: &mut egui::Ui) {
        let lang = self.store.lang;
        ui.label(
            RichText::new(lang.pick("Duyarlılık (DPI)", "Sensitivity (DPI)"))
                .size(19.0)
                .strong()
                .color(Color32::WHITE),
        );
        ui.add_space(10.0);
        ui.label(
            RichText::new(lang.pick(
                "DPI, mouse'unuzun ekrandaki hızıdır. DPI hızını kolayca değiştirmek için mouse'unuzdaki DPI tuşlarını kullanın.",
                "DPI is how fast your mouse moves on screen. Use the DPI buttons on your mouse to switch speeds.",
            ))
            .size(11.5)
            .color(theme::TEXT_LIGHT),
        );

        ui.add_space(18.0);
        ui.label(
            RichText::new(lang.pick("DPI HIZLARI", "DPI SPEEDS"))
                .size(11.5)
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);

        let prof = self.store.active_profile().clone();
        let mut clicked_stage = None;
        ui.horizontal_wrapped(|ui| {
            for (idx, &dpi) in prof.dpi_stages.iter().enumerate() {
                if dpi == 0 {
                    continue;
                }
                let is_act = idx == prof.active_dpi_idx;
                let is_shift = idx == prof.shift_dpi_idx;
                let col = if is_shift {
                    theme::DPI_AMBER
                } else {
                    Color32::WHITE
                };
                let mut rt = RichText::new(format!("{dpi}")).size(14.5).color(col);
                if is_act {
                    rt = rt.underline().strong();
                }
                if ui.add(egui::Button::new(rt).frame(false)).clicked() {
                    clicked_stage = Some((idx, dpi));
                }
                ui.add_space(6.0);
            }
        });
        if let Some((idx, dpi)) = clicked_stage {
            self.store.active_profile_mut().active_dpi_idx = idx;
            self.store.save();
            self.apply_live_dpi(dpi);
        }

        ui.add_space(14.0);
        ui.label(
            RichText::new(lang.pick(
                "GEÇERLİ DPI HIZI değerinizin altı çizilidir. Oyun sırasında kolayca başka bir hıza geçmek için DPI KAYDIRMA düğmesini basılı tutun.",
                "Your CURRENT DPI SPEED is underlined. Hold the DPI SHIFT button in game to temporarily switch speeds.",
            ))
            .size(11.5)
            .color(theme::TEXT_LIGHT),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let (ir, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
            icons::draw_icon(
                &ui.painter_at(ir),
                ir.center(),
                7.0,
                GhubIcon::AssignmentsPlus,
                Color32::WHITE,
            );
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(
                            lang.pick("BUNLARI MOUSE'UNUZA ATAYIN", "ASSIGN THESE TO YOUR MOUSE"),
                        )
                        .size(11.5)
                        .underline()
                        .color(Color32::WHITE),
                    )
                    .frame(false),
                )
                .clicked()
            {
                self.dev_tab = DeviceTab::Assignments;
                self.assign_tab = AssignSubTab::System;
                self.assign_search = "DPI".into();
            }
        });

        ui.add_space(22.0);
        ui.label(
            RichText::new(lang.pick("BİLDİRME HIZI (SANİYE BAŞINA)", "REPORT RATE (PER SECOND)"))
                .size(11.5)
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(lang.pick(
                "Mouse'unuzun bilgileri ne sıklıkla bilgisayarınıza bildirdiğini seçin.",
                "Select how often your mouse reports its position to your computer.",
            ))
            .size(11.5)
            .color(theme::TEXT_LIGHT),
        );
        ui.add_space(8.0);

        let cur_hz = self.store.active_profile().report_rate_hz;
        let rates = self.supported_rates.clone();
        let mut chosen_hz = None;
        ui.horizontal(|ui| {
            for hz in rates {
                let is_sel = cur_hz == hz;
                ui.vertical(|ui| {
                    ui.set_min_width(48.0);
                    ui.label(RichText::new(format!("{hz}")).size(12.0).color(if is_sel {
                        Color32::WHITE
                    } else {
                        theme::TEXT_MUTED
                    }));
                    ui.add_space(4.0);
                    let (rr, rresp) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), Sense::click());
                    let rp = ui.painter_at(rr);
                    if is_sel {
                        rp.circle_filled(rr.center(), 5.5, Color32::WHITE);
                    } else {
                        rp.circle_stroke(rr.center(), 5.0, Stroke::new(1.3_f32, theme::TEXT_MUTED));
                    }
                    if rresp.clicked() {
                        chosen_hz = Some(hz);
                    }
                });
                ui.add_space(12.0);
            }
        });
        if let Some(hz) = chosen_hz {
            self.apply_live_rate(hz);
        }

        ui.add_space(28.0);
        if ui
            .add(
                egui::Button::new(
                    RichText::new(
                        lang.pick("VARSAYILAN AYARLARI GERİ YÜKLE", "RESTORE DEFAULT SETTINGS"),
                    )
                    .size(11.5)
                    .strong()
                    .color(Color32::WHITE),
                )
                .fill(theme::BG_CONTROL)
                .min_size(Vec2::new(ui.available_width(), 36.0)),
            )
            .clicked()
        {
            let p = self.store.active_profile_mut();
            p.dpi_stages = [400, 800, 1600, 3200, 0];
            p.active_dpi_idx = 1;
            p.shift_dpi_idx = 0;
            p.report_rate_hz = 1000;
            self.store.save();
            self.apply_live_dpi(800);
            self.apply_live_rate(1000);
        }
    }

    fn render_dpi_stage(&mut self, ui: &mut egui::Ui) {
        let lang = self.store.lang;
        let prof = self.store.active_profile().clone();
        let (ev, committed_inline) = canvas::draw_ghub_full_dpi_stage(
            ui,
            lang,
            self.dpi_min,
            self.dpi_max,
            self.dpi_step,
            &prof.dpi_stages,
            prof.active_dpi_idx,
            prof.shift_dpi_idx,
            self.editing_dpi_stage,
            &mut self.dpi_edit_buf,
        );

        if let Some((idx, new_dpi)) = committed_inline {
            self.editing_dpi_stage = None;
            let p = self.store.active_profile_mut();
            p.dpi_stages[idx] = new_dpi;
            p.active_dpi_idx = idx;
            self.store.save();
            self.apply_live_dpi(new_dpi);
        }

        if let Some(event) = ev {
            match event {
                FullDpiEvent::SelectStage(idx) => {
                    let dpi = self.store.active_profile().dpi_stages[idx];
                    self.store.active_profile_mut().active_dpi_idx = idx;
                    self.store.save();
                    if dpi > 0 {
                        self.apply_live_dpi(dpi);
                    }
                }
                FullDpiEvent::DragStage {
                    index,
                    dpi,
                    committed,
                } => {
                    let p = self.store.active_profile_mut();
                    p.active_dpi_idx = index;
                    p.dpi_stages[index] = dpi;
                    self.live_dpi = dpi;
                    if committed {
                        self.store.save();
                        self.apply_live_dpi(dpi);
                    }
                }
                FullDpiEvent::DeleteStage(idx) => {
                    let p = self.store.active_profile_mut();
                    p.dpi_stages[idx] = 0;
                    if let Some(first) = p.dpi_stages.iter().position(|&d| d > 0) {
                        p.active_dpi_idx = first;
                        if p.shift_dpi_idx == idx {
                            p.shift_dpi_idx = first;
                        }
                        let d = p.dpi_stages[first];
                        self.store.save();
                        self.apply_live_dpi(d);
                    }
                }
                FullDpiEvent::AddStage(dpi) => {
                    let p = self.store.active_profile_mut();
                    if let Some(empty) = p.dpi_stages.iter().position(|&d| d == 0) {
                        p.dpi_stages[empty] = dpi;
                        p.active_dpi_idx = empty;
                        self.store.save();
                        self.apply_live_dpi(dpi);
                    }
                }
                FullDpiEvent::SetShiftStage(idx) => {
                    self.store.active_profile_mut().shift_dpi_idx = idx;
                    self.store.save();
                    self.schedule_onboard_commit();
                }
                FullDpiEvent::StartInlineEdit(idx) => {
                    let cur = self.store.active_profile().dpi_stages[idx];
                    self.dpi_edit_buf = format!("{cur}");
                    self.editing_dpi_stage = Some(idx);
                }
            }
        }
    }

    fn render_assignments_card(&mut self, ui: &mut egui::Ui) {
        let lang = self.store.lang;
        ui.label(
            RichText::new(lang.pick("Atamalar", "Assignments"))
                .size(19.0)
                .strong()
                .color(Color32::WHITE),
        );
        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            for (tab, lbl) in [
                (AssignSubTab::Commands, lang.pick("KOMUTLAR", "COMMANDS")),
                (AssignSubTab::Keys, lang.pick("TUŞLAR", "KEYS")),
                (AssignSubTab::Actions, lang.pick("EYLEMLER", "ACTIONS")),
                (AssignSubTab::Macros, lang.pick("MAKROLAR", "MACROS")),
                (AssignSubTab::System, lang.pick("SİSTEM", "SYSTEM")),
            ] {
                let active = self.assign_tab == tab;
                let mut rt = RichText::new(lbl).size(11.5).color(if active {
                    Color32::WHITE
                } else {
                    theme::TEXT_MUTED
                });
                if active {
                    rt = rt.strong().underline();
                }
                if ui.add(egui::Button::new(rt).frame(false)).clicked() {
                    self.assign_tab = tab;
                }
                ui.add_space(6.0);
            }
        });

        ui.add_space(8.0);
        let helper = match self.assign_tab {
            AssignSubTab::Commands => lang.pick(
                "Bir komutu bir hedefe sürükleyerek veya tıklayarak bu cihaza atayın.",
                "Drag or click a command to assign it to the selected button.",
            ),
            AssignSubTab::Keys => lang.pick(
                "Bir klavye tuşunu seçili fare düğmesine atayın.",
                "Assign a keyboard key to the selected mouse button.",
            ),
            AssignSubTab::Actions => lang.pick(
                "Eylemleri bu profile atamak için düğmelere sürükleyin veya tıklayın.",
                "Drag or click third-party actions (Discord, OBS) onto buttons.",
            ),
            AssignSubTab::Macros => lang.pick(
                "Bir makroyu bir hedefe sürükleyerek veya tıklayarak bu cihaza atayın.",
                "Create and assign macros to your mouse.",
            ),
            AssignSubTab::System => lang.pick(
                "Bir sistem denetimini bir hedefe sürükleyerek veya tıklayarak bu cihaza atayın.",
                "Assign mouse, DPI, or media controls.",
            ),
        };
        ui.label(RichText::new(helper).size(11.0).color(theme::TEXT_LIGHT));

        ui.add_space(8.0);
        let hint = match self.assign_tab {
            AssignSubTab::Commands => lang.pick("Komut ara", "Search commands"),
            AssignSubTab::Keys => lang.pick("Tuş ara", "Search keys"),
            AssignSubTab::Actions => lang.pick("Eylem ara", "Search actions"),
            AssignSubTab::Macros => lang.pick("Makro ara", "Search macros"),
            AssignSubTab::System => lang.pick("Sistem ara", "Search system"),
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(12.0).color(Color32::WHITE));
            ui.add(
                egui::TextEdit::singleline(&mut self.assign_search)
                    .hint_text(hint)
                    .frame(false)
                    .desired_width(ui.available_width()),
            );
        });
        let (ul, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 2.0), Sense::hover());
        ui.painter().line_segment(
            [
                Pos2::new(ul.left(), ul.top()),
                Pos2::new(ul.right(), ul.top()),
            ],
            Stroke::new(1.0_f32, theme::TEXT_LIGHT),
        );
        ui.add_space(8.0);

        let q = self.assign_search.trim().to_lowercase();
        let mut clicked_assign: Option<RichAssignment> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.assign_tab {
                AssignSubTab::Commands => {
                    for group in ["Navigation", "Windows", "Editing", "Productivity"] {
                        let items: Vec<_> = builtin_ghub_commands()
                            .iter()
                            .filter(|(cat, sc, desc, _, _)| {
                                *cat == group
                                    && (q.is_empty()
                                        || sc.to_lowercase().contains(&q)
                                        || desc.to_lowercase().contains(&q))
                            })
                            .collect();
                        if items.is_empty() {
                            continue;
                        }
                        egui::CollapsingHeader::new(
                            RichText::new(group).size(12.5).strong().color(Color32::WHITE),
                        )
                        .default_open(group == "Navigation" || group == "Windows")
                        .show(ui, |ui| {
                            for (cat, sc, desc, mods, key) in items {
                                let cand = RichAssignment::Command {
                                    category: (*cat).into(),
                                    name: (*desc).into(),
                                    shortcut: (*sc).into(),
                                    modifiers: *mods,
                                    hid_key: *key,
                                };
                                let badge = self.badge_for_assignment(&cand);
                                if self.draw_command_row(ui, sc, desc, badge.as_deref(), &cand) {
                                    clicked_assign = Some(cand);
                                }
                            }
                        });
                    }
                }
                AssignSubTab::Keys => {
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.custom_key_ctrl, "Ctrl");
                        ui.checkbox(&mut self.custom_key_shift, "Shift");
                        ui.checkbox(&mut self.custom_key_alt, "Alt");
                        ui.checkbox(&mut self.custom_key_super, "Super/Win");
                    });
                    let mut mod_mask = 0u8;
                    if self.custom_key_ctrl {
                        mod_mask |= 0x01;
                    }
                    if self.custom_key_shift {
                        mod_mask |= 0x02;
                    }
                    if self.custom_key_alt {
                        mod_mask |= 0x04;
                    }
                    if self.custom_key_super {
                        mod_mask |= 0x08;
                    }
                    ui.add_space(4.0);
                    for grp in ["Fonksiyon", "Harfler", "Sayılar", "Kontrol & Gezinme", "Numpad"]
                    {
                        let keys: Vec<_> = all_hid_keys()
                            .iter()
                            .filter(|(g, name, _)| {
                                *g == grp && (q.is_empty() || name.to_lowercase().contains(&q))
                            })
                            .collect();
                        if keys.is_empty() {
                            continue;
                        }
                        egui::CollapsingHeader::new(
                            RichText::new(grp).size(12.5).strong().color(Color32::WHITE),
                        )
                        .default_open(grp == "Harfler" || grp == "Fonksiyon")
                        .show(ui, |ui| {
                            for (_, kname, code) in keys {
                                let cand = RichAssignment::Key {
                                    modifiers: mod_mask,
                                    hid_key: *code,
                                };
                                let disp = cand.label(lang);
                                let badge = self.badge_for_assignment(&cand);
                                if self.draw_simple_assign_row(
                                    ui,
                                    &disp,
                                    badge.as_deref(),
                                    &cand,
                                    kname,
                                ) {
                                    clicked_assign = Some(cand);
                                }
                            }
                        });
                    }
                }
                AssignSubTab::Actions => {
                    egui::CollapsingHeader::new(
                        RichText::new("🐺 Overwolf  ⚠")
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        let cand = RichAssignment::Action(IntegrationAction::Overwolf {
                            command: "Overlay Aç/Kapat".into(),
                        });
                        let badge = self.badge_for_assignment(&cand);
                        if self.draw_simple_assign_row(
                            ui,
                            "Overlay Aç/Kapat",
                            badge.as_deref(),
                            &cand,
                            "ow1",
                        ) {
                            clicked_assign = Some(cand);
                        }
                    });

                    egui::CollapsingHeader::new(
                        RichText::new("🎥 Streamlabs Desktop  ⚠")
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        for (id_s, sa) in [
                            ("sl1", SlobsAction::ToggleStream),
                            ("sl2", SlobsAction::ToggleRecording),
                            ("sl3", SlobsAction::MixerMuteToggle),
                        ] {
                            let lbl = sa.label(lang);
                            if !q.is_empty() && !lbl.to_lowercase().contains(&q) {
                                continue;
                            }
                            let cand = RichAssignment::Action(IntegrationAction::Slobs(sa));
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                &lbl,
                                badge.as_deref(),
                                &cand,
                                id_s,
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }
                    });

                    let is_disc = self.discord_detected.load(Ordering::Relaxed);
                    let disc_title = if is_disc {
                        "💬 Discord  ●"
                    } else {
                        "💬 Discord  ⚠"
                    };
                    egui::CollapsingHeader::new(
                        RichText::new(disc_title)
                            .size(12.5)
                            .strong()
                            .color(if is_disc {
                                Color32::WHITE
                            } else {
                                theme::TEXT_LIGHT
                            }),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        for (id_s, da, sc_opt) in [
                            ("dc1", DiscordAction::ToggleMute, Some(self.store.discord.mute_shortcut)),
                            ("dc2", DiscordAction::ToggleDeafen, Some(self.store.discord.deafen_shortcut)),
                            ("dc3", DiscordAction::PushToTalk, Some(self.store.discord.ptt_shortcut)),
                            ("dc4", DiscordAction::LeaveVoice, None),
                        ] {
                            let base_lbl = da.label(lang);
                            if !q.is_empty() && !base_lbl.to_lowercase().contains(&q) {
                                continue;
                            }
                            let lbl = if self.store.discord.access_token.trim().is_empty() {
                                if let Some((m, k)) = sc_opt {
                                    format!("{base_lbl} ({})", format_hid_combo(m, k))
                                } else {
                                    base_lbl
                                }
                            } else {
                                base_lbl
                            };
                            let cand = RichAssignment::Action(IntegrationAction::Discord(da));
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                &lbl,
                                badge.as_deref(),
                                &cand,
                                id_s,
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }
                        if self.store.discord.access_token.trim().is_empty() {
                            ui.add_space(3.0);
                            ui.label(
                                RichText::new(lang.pick(
                                    "İpucu: Arka planda çalışması için Discord → Tuş Atamaları kısmında aynı kısayolu tanımlayın veya 'Kısayol / RPC' butonundan değiştirin.",
                                    "Tip: Match your Discord → Keybinds shortcut or configure via 'Keybinds / RPC'.",
                                ))
                                .size(10.5)
                                .color(theme::TEXT_DIM),
                            );
                        }

                        let custom_list = self.store.custom_actions.clone();
                        for ca in custom_list {
                            if !q.is_empty() && !ca.name.to_lowercase().contains(&q) {
                                continue;
                            }
                            let nm = ca.name.clone();
                            let id_c = ca.id.clone();
                            let cand = RichAssignment::Action(IntegrationAction::Custom(ca));
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                &nm,
                                badge.as_deref(),
                                &cand,
                                &id_c,
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(lang.pick(
                                            "+ YENİ EYLEM OLUŞTUR",
                                            "+ CREATE NEW ACTION",
                                        ))
                                        .size(11.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.custom_action_modal = true;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(lang.pick("Kısayol / RPC", "Keybinds / RPC"))
                                        .clicked()
                                    {
                                        self.discord_config_open = true;
                                    }
                                },
                            );
                        });
                    });

                    let obs_title = if self.obs_detected.load(Ordering::Relaxed) {
                        "🎬 OBS  ●"
                    } else {
                        "🎬 OBS  ⚠"
                    };
                    egui::CollapsingHeader::new(
                        RichText::new(obs_title)
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        for (idx, oa) in ObsAction::all().iter().enumerate() {
                            let lbl = oa.label(lang);
                            if !q.is_empty() && !lbl.to_lowercase().contains(&q) {
                                continue;
                            }
                            let cand = RichAssignment::Action(IntegrationAction::Obs(*oa));
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                lbl,
                                badge.as_deref(),
                                &cand,
                                &format!("obs_{idx}"),
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }
                        if ui
                            .small_button(lang.pick(
                                "OBS WebSocket Ayarları (4455)",
                                "OBS WebSocket Settings",
                            ))
                            .clicked()
                        {
                            self.obs_config_open = true;
                        }
                    });
                }
                AssignSubTab::Macros => {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick(
                                    "YENİ MAKRO OLUŞTUR",
                                    "CREATE NEW MACRO",
                                ))
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                            )
                            .fill(theme::BG_CONTROL)
                            .min_size(Vec2::new(ui.available_width(), 34.0)),
                        )
                        .clicked()
                    {
                        self.macro_wizard.open_new();
                    }
                    ui.add_space(10.0);

                    let macros_snap = self.store.macros.clone();
                    let mut edit_mac: Option<crate::store::UserMacro> = None;
                    let mut delete_mac_id: Option<String> = None;

                    for mac in macros_snap {
                        if !q.is_empty() && !mac.name.to_lowercase().contains(&q) {
                            continue;
                        }
                        let cand = RichAssignment::Macro {
                            macro_id: mac.id.clone(),
                            macro_name: mac.name.clone(),
                        };
                        let badge = self.badge_for_assignment(&cand);
                        let is_running = self.macro_runtime.is_toggle_running(&mac.id);

                        ui.horizontal(|ui| {
                            if let Some(c) = mac.color {
                                let (dr, _) =
                                    ui.allocate_exact_size(Vec2::new(10.0, 10.0), Sense::hover());
                                ui.painter().circle_filled(
                                    dr.center(),
                                    4.5,
                                    Color32::from_rgb(c[0], c[1], c[2]),
                                );
                            }
                            let label_txt = if is_running {
                                format!("{} (● ÇALIŞIYOR)", mac.name)
                            } else {
                                format!("{} ({})", mac.name, mac.kind.tooltip(lang))
                            };
                            if self.draw_simple_assign_row(
                                ui,
                                &label_txt,
                                badge.as_deref(),
                                &cand,
                                &mac.id,
                            ) {
                                clicked_assign = Some(cand);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.small_button(lang.pick("Düzenle", "Edit")).clicked() {
                                edit_mac = Some(mac.clone());
                            }
                            if ui.small_button(lang.pick("Sil", "Delete")).clicked() {
                                delete_mac_id = Some(mac.id.clone());
                            }
                        });
                        ui.separator();
                    }

                    if let Some(m) = edit_mac {
                        self.macro_wizard.open_edit(&m);
                    }
                    if let Some(did) = delete_mac_id {
                        self.store.macros.retain(|m| m.id != did);
                        self.store.save();
                    }
                }
                AssignSubTab::System => {
                    egui::CollapsingHeader::new(
                        RichText::new("Mouse").size(12.5).strong().color(Color32::WHITE),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        for (idx, mcmd) in MouseCmd::all().iter().enumerate() {
                            let lbl = mcmd.label(lang);
                            if !q.is_empty() && !lbl.to_lowercase().contains(&q) {
                                continue;
                            }
                            let cand = RichAssignment::Mouse(*mcmd);
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                lbl,
                                badge.as_deref(),
                                &cand,
                                &format!("sys_m_{idx}"),
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }
                    });

                    egui::CollapsingHeader::new(
                        RichText::new(lang.pick("Ses ve Medya", "Audio & Media"))
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .default_open(q.is_empty() || !q.contains("dpi"))
                    .show(ui, |ui| {
                        for (idx, acmd) in MediaCmd::all_audio()
                            .iter()
                            .chain(MediaCmd::all_system().iter())
                            .enumerate()
                        {
                            let lbl = acmd.label(lang);
                            if !q.is_empty() && !lbl.to_lowercase().contains(&q) {
                                continue;
                            }
                            let cand = RichAssignment::Media(*acmd);
                            let badge = self.badge_for_assignment(&cand);
                            if self.draw_simple_assign_row(
                                ui,
                                lbl,
                                badge.as_deref(),
                                &cand,
                                &format!("sys_a_{idx}"),
                            ) {
                                clicked_assign = Some(cand);
                            }
                        }
                    });
                }
            });

        if let Some(chosen) = clicked_assign {
            self.assign_to_button(self.selected_button, chosen);
        }
    }

    fn draw_command_row(
        &mut self,
        ui: &mut egui::Ui,
        shortcut: &str,
        desc: &str,
        badge: Option<&str>,
        cand: &RichAssignment,
    ) -> bool {
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 34.0),
            Sense::click_and_drag(),
        );
        let p = ui.painter_at(rect);
        if resp.hovered() {
            p.rect_filled(rect, CornerRadius::same(3), theme::BG_CONTROL);
        }
        p.text(
            Pos2::new(rect.left() + 2.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            shortcut,
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );
        p.text(
            Pos2::new(rect.left() + 148.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            desc,
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
        if let Some(b) = badge {
            p.text(
                Pos2::new(rect.right() - 8.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                b,
                egui::FontId::proportional(11.5),
                Color32::WHITE,
            );
        }
        p.line_segment(
            [
                Pos2::new(rect.left(), rect.bottom()),
                Pos2::new(rect.right(), rect.bottom()),
            ],
            Stroke::new(1.0_f32, theme::BORDER_DIM),
        );
        if resp.drag_started() {
            self.dragging_item = Some(cand.clone());
        }
        resp.clicked()
    }

    fn draw_simple_assign_row(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        badge: Option<&str>,
        cand: &RichAssignment,
        _salt: &str,
    ) -> bool {
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 32.0),
            Sense::click_and_drag(),
        );
        let p = ui.painter_at(rect);
        if resp.hovered() {
            p.rect_filled(rect, CornerRadius::same(3), theme::BG_CONTROL);
        }
        p.text(
            Pos2::new(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.5),
            theme::TEXT_MUTED,
        );
        if let Some(b) = badge {
            p.text(
                Pos2::new(rect.right() - 10.0, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                b,
                egui::FontId::proportional(11.5),
                Color32::WHITE,
            );
        }
        p.line_segment(
            [
                Pos2::new(rect.left(), rect.bottom()),
                Pos2::new(rect.right(), rect.bottom()),
            ],
            Stroke::new(1.0_f32, theme::BORDER_DIM),
        );
        if resp.drag_started() {
            self.dragging_item = Some(cand.clone());
        }
        resp.clicked()
    }

    fn render_assignments_stage(&mut self, ui: &mut egui::Ui) {
        let lang = self.store.lang;
        let prof = self.store.active_profile();
        let arr = if self.gshift_layer {
            &prof.gshift_buttons
        } else {
            &prof.buttons
        };
        let callout_labels: Vec<(String, bool)> = arr
            .iter()
            .map(|a| (a.label(lang), a.is_dpi_highlight()))
            .collect();

        let stage_h = (ui.available_height() - 56.0).max(220.0);
        let stage_out = canvas::draw_ghub_mouse_stage(
            ui,
            Vec2::new(ui.available_width(), stage_h),
            [Color32::WHITE; 3],
            true,
            Some(self.selected_button),
            self.pressed_mask,
            &callout_labels,
            true,
            self.dragging_item.is_some(),
            false,
            None,
        );

        if let Some(btn) = stage_out.clicked_button {
            self.selected_button = btn;
        }
        if let Some(target_btn) = stage_out.dropped_on_button {
            if let Some(item) = self.dragging_item.take() {
                self.selected_button = target_btn;
                self.assign_to_button(target_btn, item);
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            self.dragging_item = None;
        }

        if let Some(ref dragged) = self.dragging_item {
            if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                let lp = ui.ctx().layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("drag_ghost"),
                ));
                let r = Rect::from_min_size(pos + Vec2::new(12.0, 12.0), Vec2::new(150.0, 26.0));
                lp.rect_filled(r, CornerRadius::same(4), theme::GHUB_BLUE);
                lp.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    dragged.label(lang),
                    egui::FontId::proportional(11.5),
                    Color32::WHITE,
                );
            }
        }

        ui.vertical_centered(|ui| {
            let row_w = 186.0;
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - row_w).max(0.0) * 0.5);
                ui.label(
                    RichText::new(lang.pick("VARSAYILAN", "DEFAULT"))
                        .size(11.0)
                        .strong()
                        .color(if !self.gshift_layer {
                            Color32::WHITE
                        } else {
                            theme::TEXT_MUTED
                        }),
                );
                if theme::ghub_pill_switch(ui, &mut self.gshift_layer).changed() {
                    self.sync_button_map_to_hw();
                }
                ui.label(RichText::new("G-SHIFT").size(11.0).strong().color(
                    if self.gshift_layer {
                        Color32::WHITE
                    } else {
                        theme::TEXT_MUTED
                    },
                ));
            });
        });
    }

    fn render_lightsync_card(&mut self, ui: &mut egui::Ui) {
        let lang = self.store.lang;

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("LIGHTSYNC")
                    .size(19.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if theme::ghub_pill_switch(ui, &mut self.lightsync_enabled).changed() {
                    if !self.lightsync_enabled {
                        self.commit_onboard_now(false);
                    } else if self.rgb_tab == LightsyncSubTab::Freestyle {
                        self.apply_perkey_rgb(self.perkey_colors, false);
                    } else {
                        if self.preset_kind == PresetKind::Off {
                            self.preset_kind = PresetKind::ColorWave;
                        }
                        self.commit_onboard_now(false);
                    }
                }
            });
        });

        ui.add_space(14.0);

        let has_perkey = self.devices.get(self.selected_idx).is_some_and(|d| {
            d.summary
                .features
                .iter()
                .any(|f| f.id == FeatureId::PER_KEY_LIGHTING)
        });

        ui.horizontal_wrapped(|ui| {
            for (st, lbl) in [
                (LightsyncSubTab::Presets, lang.pick("ÖN AYARLAR", "PRESETS")),
                (
                    LightsyncSubTab::Freestyle,
                    lang.pick("SERBEST STİL", "FREESTYLE"),
                ),
                (
                    LightsyncSubTab::Animations,
                    lang.pick("ANİMASYONLAR", "ANIMATIONS"),
                ),
            ] {
                if !has_perkey && st != LightsyncSubTab::Presets {
                    continue;
                }
                let active = self.rgb_tab == st;
                let mut rt = RichText::new(lbl).size(11.5).color(if active {
                    Color32::WHITE
                } else {
                    theme::TEXT_MUTED
                });
                if active {
                    rt = rt.strong().underline();
                }
                if ui.add(egui::Button::new(rt).frame(false)).clicked() {
                    self.rgb_tab = st;
                    self.lightsync_enabled = true;
                    if st == LightsyncSubTab::Presets {
                        if self.preset_kind == PresetKind::Off {
                            self.preset_kind = PresetKind::ColorWave;
                        }
                        self.apply_rgb_preset(true);
                    } else if st == LightsyncSubTab::Freestyle {
                        self.apply_perkey_rgb(self.perkey_colors, true);
                    }
                }
                ui.add_space(10.0);
            }
        });

        ui.add_space(18.0);

        match self.rgb_tab {
            LightsyncSubTab::Presets => {
                let mut changed_live = false;
                let mut persist_now = false;

                ui.label(
                    RichText::new(lang.pick("EFEKT", "EFFECT"))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(4.0);
                let eff_label = |k: PresetKind| -> &'static str {
                    match k {
                        PresetKind::Off => lang.pick("KAPALI", "OFF"),
                        PresetKind::Fixed => lang.pick("SABİT", "FIXED"),
                        PresetKind::ColorCycle => lang.pick("RENK DÖNGÜSÜ", "CYCLE"),
                        PresetKind::ColorWave => lang.pick("RENK DALGASI", "COLOR WAVE"),
                        PresetKind::Breathing => lang.pick("NEFES ALMA", "BREATHING"),
                        PresetKind::ColorBlend => lang.pick("RENK KARIŞIMI", "COLOR BLEND"),
                    }
                };
                egui::ComboBox::from_id_salt("ghub_rgb_eff_combo")
                    .selected_text(
                        RichText::new(eff_label(self.preset_kind))
                            .size(12.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .width(210.0)
                    .show_ui(ui, |ui| {
                        for k in [
                            PresetKind::Off,
                            PresetKind::Fixed,
                            PresetKind::ColorCycle,
                            PresetKind::ColorWave,
                            PresetKind::Breathing,
                            PresetKind::ColorBlend,
                        ] {
                            if ui
                                .selectable_value(&mut self.preset_kind, k, eff_label(k))
                                .clicked()
                            {
                                changed_live = true;
                                persist_now = true;
                            }
                        }
                    });

                ui.add_space(16.0);

                if self.preset_kind == PresetKind::ColorWave {
                    ui.label(
                        RichText::new(lang.pick("DÖNGÜ", "CYCLE"))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    let dir_lbl = |d: WaveDirection| -> &'static str {
                        match d {
                            WaveDirection::Horizontal => lang.pick("YATAY", "HORIZONTAL"),
                            WaveDirection::ReverseHorizontal => {
                                lang.pick("TERS YATAY", "REVERSE HORIZONTAL")
                            }
                            WaveDirection::Vertical => lang.pick("DİKEY", "VERTICAL"),
                            WaveDirection::CenterOut => lang.pick("MERKEZDEN DIŞA", "CENTER OUT"),
                        }
                    };
                    egui::ComboBox::from_id_salt("ghub_wave_dir_combo")
                        .selected_text(
                            RichText::new(dir_lbl(self.wave_dir))
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for d in [
                                WaveDirection::Horizontal,
                                WaveDirection::ReverseHorizontal,
                                WaveDirection::Vertical,
                                WaveDirection::CenterOut,
                            ] {
                                if ui
                                    .selectable_value(&mut self.wave_dir, d, dir_lbl(d))
                                    .clicked()
                                {
                                    changed_live = true;
                                    persist_now = true;
                                }
                            }
                        });
                    ui.add_space(16.0);
                }

                if matches!(self.preset_kind, PresetKind::Fixed | PresetKind::Breathing) {
                    ui.label(
                        RichText::new(lang.pick("RENK", "COLOR"))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let cp = ui.color_edit_button_srgb(&mut self.preset_color);
                        if cp.drag_stopped() || (cp.changed() && !cp.dragged()) {
                            changed_live = true;
                            persist_now = true;
                        }
                        ui.label(
                            RichText::new(format!(
                                "#{:02X}{:02X}{:02X}",
                                self.preset_color[0], self.preset_color[1], self.preset_color[2]
                            ))
                            .monospace()
                            .color(Color32::WHITE),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for rgb in [
                            [0, 149, 255],
                            [0, 224, 255],
                            [255, 255, 255],
                            [255, 36, 76],
                            [255, 180, 0],
                            [0, 220, 130],
                            [155, 70, 255],
                        ] {
                            let b = egui::Button::new("")
                                .fill(Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
                                .min_size(Vec2::new(26.0, 20.0));
                            if ui.add(b).clicked() {
                                self.preset_color = rgb;
                                changed_live = true;
                                persist_now = true;
                            }
                        }
                    });
                    ui.add_space(16.0);
                }

                if matches!(
                    self.preset_kind,
                    PresetKind::Breathing
                        | PresetKind::ColorCycle
                        | PresetKind::ColorWave
                        | PresetKind::ColorBlend
                ) {
                    ui.label(
                        RichText::new(lang.pick("EFEKT HIZI", "EFFECT RATE"))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let s = ui.add(
                            egui::Slider::new(&mut self.preset_period_ms, 1000..=20000)
                                .step_by(250.0)
                                .show_value(false),
                        );
                        if s.drag_stopped() || (s.changed() && !s.dragged()) {
                            changed_live = true;
                            persist_now = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{}ms", self.preset_period_ms))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });
                    });
                    ui.add_space(16.0);
                }

                if self.preset_kind != PresetKind::Off {
                    ui.label(
                        RichText::new(lang.pick("EFEKT PARLAKLIK", "EFFECT BRIGHTNESS"))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let b = ui.add(
                            egui::Slider::new(&mut self.preset_brightness, 5..=100)
                                .show_value(false),
                        );
                        if b.drag_stopped() || (b.changed() && !b.dragged()) {
                            changed_live = true;
                            persist_now = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("{}%", self.preset_brightness))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                        });
                    });
                }

                if changed_live {
                    self.lightsync_enabled = self.preset_kind != PresetKind::Off;
                    self.apply_rgb_preset(persist_now);
                }
            }
            LightsyncSubTab::Freestyle => {
                ui.label(
                    RichText::new(lang.pick(
                        "Fare üzerindeki bölgelere tıklayın veya aşağıdan her LED için ayrı renk seçin.",
                        "Select individual colors for each of the 3 LED zones.",
                    ))
                    .size(11.5)
                    .color(theme::TEXT_LIGHT),
                );
                ui.add_space(12.0);

                let mut apply_live = false;
                let mut persist_now = false;
                for (zi, zname) in [
                    (0usize, lang.pick("1. SOL BÖLGE", "1. LEFT ZONE")),
                    (1usize, lang.pick("2. LOGO / ALT BÖLGE", "2. LOGO / BOTTOM")),
                    (2usize, lang.pick("3. SAĞ BÖLGE", "3. RIGHT ZONE")),
                ] {
                    let is_sel = self.selected_zone == zi;
                    ui.horizontal(|ui| {
                        let mut c = [
                            self.perkey_colors[zi].r,
                            self.perkey_colors[zi].g,
                            self.perkey_colors[zi].b,
                        ];
                        let cp = ui.color_edit_button_srgb(&mut c);
                        if cp.changed() {
                            self.selected_zone = zi;
                            self.lightsync_enabled = true;
                            self.perkey_colors[zi] = RgbColor::new(c[0], c[1], c[2]);
                            apply_live = true;
                        }
                        if cp.drag_stopped() || (cp.changed() && !cp.dragged()) {
                            persist_now = true;
                        }
                        ui.add_space(4.0);
                        let lbl_rt = RichText::new(zname).size(12.0).strong().color(if is_sel {
                            theme::GHUB_BLUE
                        } else {
                            Color32::WHITE
                        });
                        if ui.add(egui::Button::new(lbl_rt).frame(false)).clicked() {
                            self.selected_zone = zi;
                        }
                    });
                    ui.add_space(8.0);
                }
                if apply_live {
                    self.apply_perkey_rgb(self.perkey_colors, persist_now);
                }
            }
            LightsyncSubTab::Animations => {
                ui.label(
                    RichText::new(lang.pick("ANİMASYON SEÇ", "SELECT ANIMATION"))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(6.0);
                let anim_names: Vec<String> = self
                    .store
                    .animations
                    .iter()
                    .map(|a| a.name.clone())
                    .collect();
                for (ai, aname) in anim_names.iter().enumerate() {
                    let is_sel = self.selected_anim_idx == ai;
                    if ui
                        .selectable_label(
                            is_sel,
                            RichText::new(aname).size(12.5).strong().color(if is_sel {
                                theme::GHUB_BLUE
                            } else {
                                Color32::WHITE
                            }),
                        )
                        .clicked()
                    {
                        self.selected_anim_idx = ai;
                        self.anim_frame_cursor = 0;
                    }
                }
                if let Some(anim) = self.store.animations.get_mut(self.selected_anim_idx) {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(lang.pick("KARE HIZI (MS)", "FRAME SPEED (MS)"))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    if ui
                        .add(egui::Slider::new(&mut anim.frame_ms, 80..=1000).suffix(" ms"))
                        .changed()
                    {
                        self.store.save();
                    }
                }
            }
        }
    }

    fn render_lightsync_stage(&mut self, ui: &mut egui::Ui) {
        let time_sec = ui.input(|i| i.time);
        let anim_f = (self.rgb_tab == LightsyncSubTab::Animations)
            .then(|| {
                self.store
                    .animations
                    .get(self.selected_anim_idx)
                    .and_then(|a| a.frames.get(self.anim_frame_cursor).copied())
            })
            .flatten();
        let zone_colors = canvas::compute_preview_zones(
            time_sec,
            if !self.lightsync_enabled {
                0
            } else {
                match self.rgb_tab {
                    LightsyncSubTab::Presets => 0,
                    LightsyncSubTab::Freestyle => 1,
                    LightsyncSubTab::Animations => 2,
                }
            },
            self.perkey_colors,
            anim_f,
            self.current_rgb_effect(),
        );
        let is_free = self.rgb_tab == LightsyncSubTab::Freestyle;
        let out = canvas::draw_ghub_mouse_stage(
            ui,
            ui.available_size(),
            zone_colors,
            false,
            None,
            0,
            &[],
            false,
            false,
            is_free,
            is_free.then_some(self.selected_zone),
        );
        if let Some(z) = out.clicked_zone {
            self.selected_zone = z;
        }
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        let lang = self.store.lang;

        if self.global_settings_open {
            let mut gs_open = self.global_settings_open;
            let mut do_rescan = false;
            egui::Window::new(lang.pick("LogiTux Uygulama Ayarları", "LogiTux Global Settings"))
                .collapsible(false)
                .resizable(false)
                .open(&mut gs_open)
                .show(ctx, |ui| {
                    ui.set_min_width(360.0);
                    ui.horizontal(|ui| {
                        ui.label(lang.pick("Dil / Language:", "Language:"));
                        let mut selected = self.store.lang;
                        egui::ComboBox::from_id_salt("logitux_language_combo")
                            .selected_text(selected.native_name())
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for candidate in Lang::all() {
                                    ui.selectable_value(
                                        &mut selected,
                                        *candidate,
                                        candidate.native_name(),
                                    );
                                }
                            });
                        if selected != self.store.lang {
                            self.store.lang = selected;
                            self.store.save();
                        }
                    });
                    ui.add_space(8.0);
                    if ui
                        .button(lang.pick(
                            "USB / HID++ Cihazlarını Yeniden Tara",
                            "Rescan USB / HID++ Devices",
                        ))
                        .clicked()
                    {
                        do_rescan = true;
                    }
                    if let Some(m) = self.devices.get(self.selected_idx) {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.label(
                            RichText::new(format!(
                                "HID++ {}.{} · {} özellik doğrulandı",
                                m.summary.protocol_version.0,
                                m.summary.protocol_version.1,
                                m.summary.features.len()
                            ))
                            .size(11.5)
                            .color(theme::GHUB_BLUE),
                        );
                    }
                });
            self.global_settings_open = gs_open;
            if do_rescan {
                self.scan_devices();
            }
        }

        if self.discord_config_open {
            if let Some(slot) = self.recording_discord_shortcut {
                let mut recorded: Option<(u8, u8)> = None;
                ctx.input(|inp| {
                    let mods = input_synth::egui_modifiers_mask(inp.modifiers);
                    for ev in &inp.events {
                        if let egui::Event::Key {
                            key,
                            pressed: true,
                            repeat: false,
                            ..
                        } = ev
                        {
                            if *key == egui::Key::Escape {
                                recorded = Some((0xFF, 0xFF));
                            } else if let Some(hid) = input_synth::egui_key_to_hid(*key) {
                                recorded = Some((mods, hid));
                            }
                        }
                    }
                });
                if let Some((m, k)) = recorded {
                    if m != 0xFF {
                        match slot {
                            0 => self.store.discord.mute_shortcut = (m, k),
                            1 => self.store.discord.deafen_shortcut = (m, k),
                            _ => self.store.discord.ptt_shortcut = (m, k),
                        }
                        self.store.save();
                        self.commit_onboard_now(false);
                    }
                    self.recording_discord_shortcut = None;
                }
            }

            let mut open = self.discord_config_open;
            egui::Window::new(lang.pick("Discord Kısayol & RPC Ayarları", "Discord Keybinds & RPC"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_min_width(420.0);
                    ui.horizontal(|ui| {
                        let is_disc = self.discord_detected.load(Ordering::Relaxed);
                        let dot = if is_disc {
                            theme::STATUS_GREEN
                        } else {
                            theme::STATUS_RED
                        };
                        ui.label(RichText::new("●").color(dot));
                        ui.label(if is_disc {
                            lang.pick(
                                "Discord Masaüstü algılandı (discord-ipc aktif)",
                                "Discord Desktop detected (discord-ipc active)",
                            )
                        } else {
                            lang.pick(
                                "Discord açık değil (önce Discord uygulamasını başlatın)",
                                "Discord Desktop is not running",
                            )
                        });
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "1. Global Tuş Atamaları (Önerilen — Token gerekmez):\nDiscord → Kullanıcı Ayarları → Tuş Atamaları menüsündeki 'Sesi Aç/Kapat' ve 'Sağırlaştırmayı Aç/Kapat' kısayollarınızla aşağıdaki tuşları eşleştirin (donanım tarama kodu ile tüm oyunlarda arka planda çalışır):",
                            "1. Global Discord Keybinds (Recommended — works in all games via hardware scan codes):",
                        ))
                        .size(11.0)
                        .color(theme::TEXT_LIGHT),
                    );
                    ui.add_space(6.0);

                    for (slot, row_lbl, cur) in [
                        (
                            0u8,
                            lang.pick("Kendini Sessize Al:", "Toggle Mute:"),
                            self.store.discord.mute_shortcut,
                        ),
                        (
                            1u8,
                            lang.pick("Kendini Sağır Et:", "Toggle Deafen:"),
                            self.store.discord.deafen_shortcut,
                        ),
                        (
                            2u8,
                            lang.pick("Bas-Konuş (PTT):", "Push-to-Talk:"),
                            self.store.discord.ptt_shortcut,
                        ),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(row_lbl).size(12.0).color(Color32::WHITE));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let is_rec = self.recording_discord_shortcut == Some(slot);
                                let btn_txt = if is_rec {
                                    lang.pick("Tuşa basın... (Esc iptal)", "Press key...").to_string()
                                } else {
                                    format_hid_combo(cur.0, cur.1)
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(btn_txt).strong().color(Color32::WHITE),
                                        )
                                        .fill(if is_rec {
                                            theme::GHUB_BLUE
                                        } else {
                                            theme::BG_CONTROL
                                        })
                                        .min_size(Vec2::new(175.0, 26.0)),
                                    )
                                    .clicked()
                                {
                                    self.recording_discord_shortcut = Some(slot);
                                }
                            });
                        });
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(
                        RichText::new(lang.pick(
                            "2. Doğrudan Yerel Discord RPC IPC (İsteğe Bağlı OAuth2 Token):",
                            "2. Direct Discord Local RPC IPC (Optional OAuth2 Token):",
                        ))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.label("Discord RPC Client ID:");
                    ui.text_edit_singleline(&mut self.store.discord.client_id);
                    ui.label(lang.pick(
                        "Discord OAuth2 Erişim Anahtarı (İsteğe Bağlı):",
                        "Discord OAuth2 Access Token (Optional):",
                    ));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.store.discord.access_token)
                            .password(true),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(lang.pick(
                                        "BAĞLANTIYI TEST ET",
                                        "TEST DISCORD IPC",
                                    ))
                                    .strong()
                                    .color(Color32::WHITE),
                                )
                                .fill(theme::DISCORD_BLURPLE),
                            )
                            .clicked()
                        {
                            match integrations::discord_test_or_authorize(
                                &self.store.discord.client_id,
                                &self.store.discord.access_token,
                            ) {
                                Ok(user) => {
                                    self.store.discord.connected_user = user.clone();
                                    self.store.save();
                                    self.notify(format!("Discord: {user}"), false);
                                }
                                Err(e) => self.notify(e, true),
                            }
                        }
                        if ui.button(lang.pick("Kaydet", "Save")).clicked() {
                            self.store.save();
                        }
                    });
                    if !self.store.discord.connected_user.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "✓ {}",
                                self.store.discord.connected_user
                            ))
                            .color(theme::STATUS_GREEN),
                        );
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(lang.pick("Ses Kanalı ID:", "Voice Channel ID:"));
                        ui.text_edit_singleline(&mut self.discord_join_chan_buf);
                        if ui.button(lang.pick("Atama Ekle", "Assign")).clicked()
                            && !self.discord_join_chan_buf.trim().is_empty()
                        {
                            let cid = self.discord_join_chan_buf.trim().to_string();
                            self.assign_to_button(
                                self.selected_button,
                                RichAssignment::Action(IntegrationAction::Discord(
                                    DiscordAction::JoinVoice {
                                        channel_id: cid.clone(),
                                        label: cid,
                                    },
                                )),
                            );
                        }
                    });
                });
            if !open {
                self.recording_discord_shortcut = None;
            }
            self.discord_config_open = open;
        }

        if self.obs_config_open {
            let mut open = self.obs_config_open;
            egui::Window::new("OBS Studio WebSocket v5")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut self.store.obs.host);
                        ui.label("Port:");
                        ui.add(egui::DragValue::new(&mut self.store.obs.port).range(1..=65535));
                    });
                    if ui
                        .button(lang.pick("Kaydet ve Test Et", "Save & Test"))
                        .clicked()
                    {
                        self.store.save();
                        let obs_cfg = self.store.obs.clone();
                        let obs_flag = self.obs_detected.clone();
                        std::thread::spawn(move || {
                            obs_flag
                                .store(integrations::is_obs_detected(&obs_cfg), Ordering::Relaxed);
                        });
                    }
                });
            self.obs_config_open = open;
        }

        if self.custom_action_modal {
            let mut open = self.custom_action_modal;
            let mut created_close = false;
            egui::Window::new(lang.pick("Yeni Eylem Oluştur", "Create New Action"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.set_min_width(340.0);
                    ui.label(lang.pick("Eylem Adı:", "Action Name:"));
                    ui.text_edit_singleline(&mut self.new_act_name);
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.new_act_kind_idx,
                            0,
                            lang.pick("Uygulama Başlat", "Launch App"),
                        );
                        ui.selectable_value(
                            &mut self.new_act_kind_idx,
                            1,
                            lang.pick("URL Aç", "Open URL"),
                        );
                        ui.selectable_value(
                            &mut self.new_act_kind_idx,
                            2,
                            lang.pick("Komut Çalıştır", "Shell Command"),
                        );
                    });
                    ui.label(lang.pick("Hedef (Yol / URL / Komut):", "Target:"));
                    ui.text_edit_singleline(&mut self.new_act_target);
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick("OLUŞTUR", "CREATE"))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme::GHUB_BLUE),
                        )
                        .clicked()
                        && !self.new_act_name.trim().is_empty()
                        && !self.new_act_target.trim().is_empty()
                    {
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(5);
                        let kind = match self.new_act_kind_idx {
                            0 => CustomActionKind::LaunchApp {
                                path: self.new_act_target.trim().to_string(),
                                args: String::new(),
                            },
                            1 => CustomActionKind::OpenUrl {
                                url: self.new_act_target.trim().to_string(),
                            },
                            _ => CustomActionKind::ShellCmd {
                                command: self.new_act_target.trim().to_string(),
                            },
                        };
                        self.store.custom_actions.push(CustomAction {
                            id: format!("cact-{ts}"),
                            group: "Custom".into(),
                            name: self.new_act_name.trim().to_string(),
                            kind,
                        });
                        self.store.save();
                        self.new_act_name.clear();
                        self.new_act_target.clear();
                        created_close = true;
                    }
                });
            self.custom_action_modal = open && !created_close;
        }

        if self.about_open {
            egui::Window::new("LogiTux v0.4.0")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.about_open)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("LogiTux")
                            .strong()
                            .color(Color32::WHITE),
                    );
                    ui.label("Logitech mouse configuration for Linux and Windows");
                    ui.hyperlink("https://github.com/furkan5665st/logitux");
                });
        }
    }
}
