use std::env;
use std::fs;
use std::path::PathBuf;

use logitux_hidpp::features::onboard::{ButtonAction, OnboardMacroOp};
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseCmd {
    PrimaryClick,
    SecondaryClick,
    MiddleClick,
    Forward,
    Back,
    DpiCycle,
    DpiShift,
    DpiUp,
    DpiDown,
    GoToDpiShift,
    DefaultDpi,
    GShift,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    DoubleClick,
    OnboardProfileCycle,
    Disabled,
}

impl MouseCmd {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::PrimaryClick => lang.pick("Birinci Tıklama", "Primary Click"),
            Self::SecondaryClick => lang.pick("İkinci Tıklama", "Secondary Click"),
            Self::MiddleClick => lang.pick("Orta Tıklama", "Middle Click"),
            Self::Forward => lang.pick("İleri", "Forward"),
            Self::Back => lang.pick("Geri", "Back"),
            Self::DpiCycle => lang.pick("DPI Döngüsü", "DPI Cycle"),
            Self::DpiShift => lang.pick("DPI Kaydırma", "DPI Shift"),
            Self::DpiUp => lang.pick("DPI Yukarı", "DPI Up"),
            Self::DpiDown => lang.pick("DPI Aşağı", "DPI Down"),
            Self::GoToDpiShift => lang.pick("DPI Kaydırmaya git", "Go to DPI Shift"),
            Self::DefaultDpi => lang.pick("Varsayılan DPI", "Default DPI"),
            Self::GShift => "G-Shift",
            Self::ScrollUp => lang.pick("Yukarı Kaydır", "Scroll Up"),
            Self::ScrollDown => lang.pick("Aşağı Kaydır", "Scroll Down"),
            Self::ScrollLeft => lang.pick("Sola Kaydır", "Scroll Left"),
            Self::ScrollRight => lang.pick("Sağa Kaydır", "Scroll Right"),
            Self::DoubleClick => lang.pick("Çift Tıklama", "Double Click"),
            Self::OnboardProfileCycle => {
                lang.pick("Yerleşik Profil Döngüsü", "Onboard Profile Cycle")
            }
            Self::Disabled => lang.pick("Devre Dışı", "Disabled"),
        }
    }

    pub fn is_dpi_related(self) -> bool {
        matches!(
            self,
            Self::DpiCycle
                | Self::DpiShift
                | Self::DpiUp
                | Self::DpiDown
                | Self::GoToDpiShift
                | Self::DefaultDpi
        )
    }

    pub fn all() -> &'static [MouseCmd] {
        &[
            Self::PrimaryClick,
            Self::SecondaryClick,
            Self::MiddleClick,
            Self::Forward,
            Self::Back,
            Self::DpiCycle,
            Self::DpiShift,
            Self::DpiUp,
            Self::DpiDown,
            Self::GoToDpiShift,
            Self::DefaultDpi,
            Self::GShift,
            Self::ScrollUp,
            Self::ScrollDown,
            Self::ScrollLeft,
            Self::ScrollRight,
            Self::DoubleClick,
            Self::OnboardProfileCycle,
            Self::Disabled,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaCmd {
    PlayPause,
    Stop,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    Mute,
    Calculator,
    BrowserHome,
    Mail,
    Search,
    Screenshot,
}

impl MediaCmd {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::PlayPause => lang.pick("Oynat / Duraklat", "Play / Pause"),
            Self::Stop => lang.pick("Durdur", "Stop"),
            Self::NextTrack => lang.pick("Sonraki Parça", "Next Track"),
            Self::PrevTrack => lang.pick("Önceki Parça", "Previous Track"),
            Self::VolumeUp => lang.pick("Sesi Artır", "Volume Up"),
            Self::VolumeDown => lang.pick("Sesi Azalt", "Volume Down"),
            Self::Mute => lang.pick("Sesi Kapat", "Mute"),
            Self::Calculator => lang.pick("Hesap Makinesi", "Calculator"),
            Self::BrowserHome => lang.pick("Tarayıcı Ana Sayfası", "Browser Home"),
            Self::Mail => lang.pick("E-Posta", "Mail"),
            Self::Search => lang.pick("Arama", "Search"),
            Self::Screenshot => lang.pick("Ekran Görüntüsü", "Screenshot"),
        }
    }

    pub fn all_audio() -> &'static [MediaCmd] {
        &[
            Self::PlayPause,
            Self::Stop,
            Self::NextTrack,
            Self::PrevTrack,
            Self::VolumeUp,
            Self::VolumeDown,
            Self::Mute,
        ]
    }

    pub fn all_system() -> &'static [MediaCmd] {
        &[
            Self::Calculator,
            Self::BrowserHome,
            Self::Mail,
            Self::Search,
            Self::Screenshot,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscordAction {
    ToggleMute,
    ToggleDeafen,
    PushToTalk,
    JoinVoice { channel_id: String, label: String },
    LeaveVoice,
}

impl DiscordAction {
    pub fn label(&self, lang: Lang) -> String {
        match self {
            Self::ToggleMute => lang.pick("Kendini Sessize Al", "Toggle Self Mute").into(),
            Self::ToggleDeafen => lang.pick("Kendini Sağır Et", "Toggle Self Deafen").into(),
            Self::PushToTalk => lang.pick("Bas-Konuş (PTT)", "Push to Talk").into(),
            Self::JoinVoice { label, .. } => {
                if label.is_empty() {
                    lang.pick("Ses Kanalına Katıl", "Join Voice Channel").into()
                } else {
                    format!("{}: {label}", lang.pick("Kanala Katıl", "Join"))
                }
            }
            Self::LeaveVoice => lang.pick("Ses Kanalından Ayrıl", "Disconnect Voice").into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObsAction {
    StartStream,
    StopStream,
    ToggleStream,
    StartRecording,
    StopRecording,
    ToggleRecording,
    StartBuffer,
    StopBuffer,
    ToggleBuffer,
    SaveBuffer,
    DesktopMute,
    DesktopUnmute,
    DesktopMuteToggle,
    MicMute,
    MicUnmute,
    MicMuteToggle,
}

impl ObsAction {
    pub fn label(self, lang: Lang) -> &'static str {
        match self {
            Self::StartStream => lang.pick("Yayını Başlat", "Start Stream"),
            Self::StopStream => lang.pick("Yayını Durdur", "Stop Stream"),
            Self::ToggleStream => lang.pick("Yayını Aç/Kapat", "Toggle Stream"),
            Self::StartRecording => lang.pick("Kaydı Başlat", "Start Recording"),
            Self::StopRecording => lang.pick("Kaydı Durdur", "Stop Recording"),
            Self::ToggleRecording => lang.pick("Kaydı Aç/Kapat", "Toggle Recording"),
            Self::StartBuffer => lang.pick("Tekrar Arabelleğini Başlat", "Start Replay Buffer"),
            Self::StopBuffer => lang.pick("Tekrar Arabelleğini Durdur", "Stop Replay Buffer"),
            Self::ToggleBuffer => lang.pick("Tekrar Arabelleğini Aç/Kapat", "Toggle Replay Buffer"),
            Self::SaveBuffer => lang.pick("Tekrar Arabelleğini Kaydet", "Save Replay Buffer"),
            Self::DesktopMute => lang.pick("Masaüstü Sesini Kapat", "Mute Desktop Audio"),
            Self::DesktopUnmute => lang.pick("Masaüstü Sesini Aç", "Unmute Desktop Audio"),
            Self::DesktopMuteToggle => {
                lang.pick("Masaüstü Sesini Değiştir", "Toggle Desktop Audio")
            }
            Self::MicMute => lang.pick("Mikrofonu Kapat", "Mute Mic"),
            Self::MicUnmute => lang.pick("Mikrofonu Aç", "Unmute Mic"),
            Self::MicMuteToggle => lang.pick("Mikrofonu Değiştir", "Toggle Mic"),
        }
    }

    pub fn all() -> &'static [ObsAction] {
        &[
            Self::ToggleStream,
            Self::StartStream,
            Self::StopStream,
            Self::ToggleRecording,
            Self::StartRecording,
            Self::StopRecording,
            Self::SaveBuffer,
            Self::ToggleBuffer,
            Self::StartBuffer,
            Self::StopBuffer,
            Self::MicMuteToggle,
            Self::MicMute,
            Self::MicUnmute,
            Self::DesktopMuteToggle,
            Self::DesktopMute,
            Self::DesktopUnmute,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlobsAction {
    ToggleStream,
    ToggleRecording,
    ActivateScene(String),
    ToggleSource(String),
    MixerMuteToggle,
}

impl SlobsAction {
    pub fn label(&self, lang: Lang) -> String {
        match self {
            Self::ToggleStream => lang.pick("Yayını Değiştir", "Toggle Stream").into(),
            Self::ToggleRecording => lang.pick("Kaydı Değiştir", "Toggle Recording").into(),
            Self::ActivateScene(s) => {
                format!("{}: {s}", lang.pick("Sahne Etkinleştir", "Activate Scene"))
            }
            Self::ToggleSource(s) => {
                format!("{}: {s}", lang.pick("Kaynak Değiştir", "Toggle Source"))
            }
            Self::MixerMuteToggle => lang
                .pick("Karıştırıcı Sesini Değiştir", "Toggle Mixer Mute")
                .into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomActionKind {
    LaunchApp { path: String, args: String },
    OpenUrl { url: String },
    ShellCmd { command: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomAction {
    pub id: String,
    pub group: String,
    pub name: String,
    pub kind: CustomActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationAction {
    Discord(DiscordAction),
    Obs(ObsAction),
    Slobs(SlobsAction),
    Overwolf { command: String },
    Custom(CustomAction),
}

impl IntegrationAction {
    pub fn label(&self, lang: Lang) -> String {
        match self {
            Self::Discord(d) => format!("Discord · {}", d.label(lang)),
            Self::Obs(o) => format!("OBS · {}", o.label(lang)),
            Self::Slobs(s) => format!("Streamlabs · {}", s.label(lang)),
            Self::Overwolf { command } => format!("Overwolf · {command}"),
            Self::Custom(c) => c.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RichAssignment {
    Mouse(MouseCmd),
    Media(MediaCmd),
    Command {
        category: String,
        name: String,
        shortcut: String,
        modifiers: u8,
        hid_key: u8,
    },
    Key {
        modifiers: u8,
        hid_key: u8,
    },
    Action(IntegrationAction),
    Macro {
        macro_id: String,
        macro_name: String,
    },
}

impl RichAssignment {
    pub fn label(&self, lang: Lang) -> String {
        match self {
            Self::Mouse(m) => m.label(lang).to_string(),
            Self::Media(m) => m.label(lang).to_string(),
            Self::Command { name, .. } => name.clone(),
            Self::Key { modifiers, hid_key } => format_hid_combo(*modifiers, *hid_key),
            Self::Action(a) => a.label(lang),
            Self::Macro { macro_name, .. } => macro_name.clone(),
        }
    }

    pub fn is_dpi_highlight(&self) -> bool {
        matches!(self, Self::Mouse(m) if m.is_dpi_related())
    }

    pub fn to_onboard_with_discord(&self, discord: &DiscordSettings) -> ButtonAction {
        match self {
            Self::Mouse(MouseCmd::PrimaryClick) => ButtonAction::MouseButton(0x0001),
            Self::Mouse(MouseCmd::SecondaryClick) => ButtonAction::MouseButton(0x0002),
            Self::Mouse(MouseCmd::MiddleClick) => ButtonAction::MouseButton(0x0004),
            Self::Mouse(MouseCmd::Back) => ButtonAction::MouseButton(0x0008),
            Self::Mouse(MouseCmd::Forward) => ButtonAction::MouseButton(0x0010),
            Self::Mouse(MouseCmd::ScrollLeft) => ButtonAction::Special(0x01),
            Self::Mouse(MouseCmd::ScrollRight) => ButtonAction::Special(0x02),
            Self::Mouse(MouseCmd::DpiUp) => ButtonAction::Special(0x03),
            Self::Mouse(MouseCmd::DpiDown) => ButtonAction::Special(0x04),
            Self::Mouse(MouseCmd::DpiCycle) => ButtonAction::Special(0x05),
            Self::Mouse(MouseCmd::DefaultDpi) => ButtonAction::Special(0x06),
            Self::Mouse(MouseCmd::DpiShift | MouseCmd::GoToDpiShift) => ButtonAction::Special(0x07),
            Self::Mouse(MouseCmd::OnboardProfileCycle) => ButtonAction::Special(0x0A),
            Self::Mouse(MouseCmd::GShift) => ButtonAction::Special(0x0B),
            Self::Mouse(MouseCmd::ScrollDown) => ButtonAction::Special(0x10),
            Self::Mouse(MouseCmd::ScrollUp) => ButtonAction::Special(0x11),
            Self::Mouse(MouseCmd::Disabled) => ButtonAction::Disabled,
            Self::Mouse(MouseCmd::DoubleClick) => ButtonAction::MouseButton(0x0001),
            Self::Media(m) => media_cmd_to_onboard(*m),
            Self::Command {
                modifiers, hid_key, ..
            }
            | Self::Key { modifiers, hid_key } => ButtonAction::Keyboard {
                modifiers: *modifiers,
                key: *hid_key,
            },
            Self::Action(IntegrationAction::Discord(DiscordAction::ToggleMute)) => {
                ButtonAction::Keyboard {
                    modifiers: discord.mute_shortcut.0,
                    key: discord.mute_shortcut.1,
                }
            }
            Self::Action(IntegrationAction::Discord(DiscordAction::ToggleDeafen)) => {
                ButtonAction::Keyboard {
                    modifiers: discord.deafen_shortcut.0,
                    key: discord.deafen_shortcut.1,
                }
            }
            Self::Action(IntegrationAction::Discord(DiscordAction::PushToTalk)) => {
                ButtonAction::Keyboard {
                    modifiers: discord.ptt_shortcut.0,
                    key: discord.ptt_shortcut.1,
                }
            }
            Self::Action(_) | Self::Macro { .. } => ButtonAction::Inherit,
        }
    }

    pub fn is_host_only(&self) -> bool {
        matches!(
            self,
            Self::Action(
                IntegrationAction::Obs(_)
                    | IntegrationAction::Slobs(_)
                    | IntegrationAction::Overwolf { .. }
                    | IntegrationAction::Custom(_)
                    | IntegrationAction::Discord(
                        DiscordAction::JoinVoice { .. } | DiscordAction::LeaveVoice
                    )
            )
        )
    }

    pub fn from_onboard(act: ButtonAction) -> Self {
        match act {
            ButtonAction::Disabled => Self::Mouse(MouseCmd::Disabled),
            ButtonAction::MouseButton(0x0001) => Self::Mouse(MouseCmd::PrimaryClick),
            ButtonAction::MouseButton(0x0002) => Self::Mouse(MouseCmd::SecondaryClick),
            ButtonAction::MouseButton(0x0004) => Self::Mouse(MouseCmd::MiddleClick),
            ButtonAction::MouseButton(0x0008) => Self::Mouse(MouseCmd::Back),
            ButtonAction::MouseButton(0x0010) => Self::Mouse(MouseCmd::Forward),
            ButtonAction::Special(0x01) => Self::Mouse(MouseCmd::ScrollLeft),
            ButtonAction::Special(0x02) => Self::Mouse(MouseCmd::ScrollRight),
            ButtonAction::Special(0x03) => Self::Mouse(MouseCmd::DpiUp),
            ButtonAction::Special(0x04) => Self::Mouse(MouseCmd::DpiDown),
            ButtonAction::Special(0x05) => Self::Mouse(MouseCmd::DpiCycle),
            ButtonAction::Special(0x06) => Self::Mouse(MouseCmd::DefaultDpi),
            ButtonAction::Special(0x07) => Self::Mouse(MouseCmd::DpiShift),
            ButtonAction::Special(0x08 | 0x09 | 0x0A) => Self::Mouse(MouseCmd::OnboardProfileCycle),
            ButtonAction::Special(0x0B) => Self::Mouse(MouseCmd::GShift),
            ButtonAction::Special(0x10) => Self::Mouse(MouseCmd::ScrollDown),
            ButtonAction::Special(0x11) => Self::Mouse(MouseCmd::ScrollUp),
            ButtonAction::ConsumerKey(0x00CD) => Self::Media(MediaCmd::PlayPause),
            ButtonAction::ConsumerKey(0x00B7) => Self::Media(MediaCmd::Stop),
            ButtonAction::ConsumerKey(0x00B5) => Self::Media(MediaCmd::NextTrack),
            ButtonAction::ConsumerKey(0x00B6) => Self::Media(MediaCmd::PrevTrack),
            ButtonAction::ConsumerKey(0x00E9) => Self::Media(MediaCmd::VolumeUp),
            ButtonAction::ConsumerKey(0x00EA) => Self::Media(MediaCmd::VolumeDown),
            ButtonAction::ConsumerKey(0x00E2) => Self::Media(MediaCmd::Mute),
            ButtonAction::ConsumerKey(0x0192) => Self::Media(MediaCmd::Calculator),
            ButtonAction::ConsumerKey(0x0223) => Self::Media(MediaCmd::BrowserHome),
            ButtonAction::ConsumerKey(0x018A) => Self::Media(MediaCmd::Mail),
            ButtonAction::ConsumerKey(0x0221) => Self::Media(MediaCmd::Search),
            ButtonAction::Keyboard {
                modifiers: 0x00,
                key: 0x46,
            } => Self::Media(MediaCmd::Screenshot),
            ButtonAction::Keyboard { modifiers, key } => Self::Key {
                modifiers,
                hid_key: key,
            },
            ButtonAction::Macro { page, offset } => Self::Macro {
                macro_id: format!("hw-macro-{page}-{offset}"),
                macro_name: format!("Onboard Macro #{page}"),
            },
            _ => Self::Mouse(MouseCmd::PrimaryClick),
        }
    }
}

pub fn media_cmd_to_consumer_usage(cmd: MediaCmd) -> Option<u16> {
    match cmd {
        MediaCmd::PlayPause => Some(0x00CD),
        MediaCmd::Stop => Some(0x00B7),
        MediaCmd::NextTrack => Some(0x00B5),
        MediaCmd::PrevTrack => Some(0x00B6),
        MediaCmd::VolumeUp => Some(0x00E9),
        MediaCmd::VolumeDown => Some(0x00EA),
        MediaCmd::Mute => Some(0x00E2),
        MediaCmd::Calculator => Some(0x0192),
        MediaCmd::BrowserHome => Some(0x0223),
        MediaCmd::Mail => Some(0x018A),
        MediaCmd::Search => Some(0x0221),
        MediaCmd::Screenshot => None,
    }
}

fn media_cmd_to_onboard(cmd: MediaCmd) -> ButtonAction {
    if let Some(usage) = media_cmd_to_consumer_usage(cmd) {
        ButtonAction::ConsumerKey(usage)
    } else {
        ButtonAction::Keyboard {
            modifiers: 0x00,
            key: 0x46,
        }
    }
}

fn ascii_char_to_hid(ch: char) -> Option<(u8, u8)> {
    match ch {
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
        '`' => Some((0x00, 0x35)),
        '~' => Some((0x02, 0x35)),
        ',' => Some((0x00, 0x36)),
        '<' => Some((0x02, 0x36)),
        '.' => Some((0x00, 0x37)),
        '>' => Some((0x02, 0x37)),
        '/' => Some((0x00, 0x38)),
        '?' => Some((0x02, 0x38)),
        '!' => Some((0x02, 0x1E)),
        '@' => Some((0x02, 0x1F)),
        '#' => Some((0x02, 0x20)),
        '$' => Some((0x02, 0x21)),
        '%' => Some((0x02, 0x22)),
        '^' => Some((0x02, 0x23)),
        '&' => Some((0x02, 0x24)),
        '*' => Some((0x02, 0x25)),
        '(' => Some((0x02, 0x26)),
        ')' => Some((0x02, 0x27)),
        _ => None,
    }
}

fn push_key_press_ops(ops: &mut Vec<OnboardMacroOp>, modifiers: u8, hid_key: u8) {
    if modifiers != 0 {
        ops.push(OnboardMacroOp::KeyPress { modifiers, key: 0 });
    }
    if hid_key != 0 {
        ops.push(OnboardMacroOp::KeyPress {
            modifiers: 0,
            key: hid_key,
        });
    }
}

fn push_key_release_ops(ops: &mut Vec<OnboardMacroOp>, modifiers: u8, hid_key: u8) {
    if hid_key != 0 {
        ops.push(OnboardMacroOp::KeyRelease {
            modifiers: 0,
            key: hid_key,
        });
    }
    if modifiers != 0 {
        ops.push(OnboardMacroOp::KeyRelease { modifiers, key: 0 });
    }
}

fn append_steps_to_onboard_ops(
    steps: &[MacroStep],
    default_delay: u16,
    discord: &DiscordSettings,
    ops: &mut Vec<OnboardMacroOp>,
) {
    for step in steps {
        match step {
            MacroStep::KeyDown { modifiers, hid_key } => {
                push_key_press_ops(ops, *modifiers, *hid_key);
                if default_delay > 0 {
                    ops.push(OnboardMacroOp::DelayMs(default_delay));
                }
            }
            MacroStep::KeyUp { modifiers, hid_key } => {
                push_key_release_ops(ops, *modifiers, *hid_key);
                if default_delay > 0 {
                    ops.push(OnboardMacroOp::DelayMs(default_delay));
                }
            }
            MacroStep::KeyTap { modifiers, hid_key } => {
                push_key_press_ops(ops, *modifiers, *hid_key);
                ops.push(OnboardMacroOp::DelayMs(default_delay.max(15)));
                push_key_release_ops(ops, *modifiers, *hid_key);
                if default_delay > 0 {
                    ops.push(OnboardMacroOp::DelayMs(default_delay));
                }
            }
            MacroStep::MouseClick { button } => {
                let mask = 1u16.checked_shl((*button).saturating_sub(1) as u32).unwrap_or(1);
                ops.push(OnboardMacroOp::ButtonDown(mask));
                ops.push(OnboardMacroOp::DelayMs(default_delay.max(15)));
                ops.push(OnboardMacroOp::ButtonUp(mask));
                if default_delay > 0 {
                    ops.push(OnboardMacroOp::DelayMs(default_delay));
                }
            }
            MacroStep::DelayMs(ms) => {
                let mut rem = (*ms).max(1);
                while rem > 0 {
                    let chunk = rem.min(60000) as u16;
                    ops.push(OnboardMacroOp::DelayMs(chunk));
                    rem -= chunk as u32;
                }
            }
            MacroStep::TypeText(txt) => {
                for ch in txt.chars() {
                    if let Some((mods, key)) = ascii_char_to_hid(ch) {
                        push_key_press_ops(ops, mods, key);
                        ops.push(OnboardMacroOp::DelayMs(12));
                        push_key_release_ops(ops, mods, key);
                        ops.push(OnboardMacroOp::DelayMs(12));
                    }
                }
            }
            MacroStep::Media(m) => {
                if let Some(usage) = media_cmd_to_consumer_usage(*m) {
                    ops.push(OnboardMacroOp::ConsumerDown(usage));
                    ops.push(OnboardMacroOp::DelayMs(20));
                    ops.push(OnboardMacroOp::ConsumerUp(usage));
                } else {
                    push_key_press_ops(ops, 0, 0x46);
                    ops.push(OnboardMacroOp::DelayMs(20));
                    push_key_release_ops(ops, 0, 0x46);
                }
            }
            MacroStep::Action(IntegrationAction::Discord(d)) => {
                let sc = match d {
                    DiscordAction::ToggleMute => Some(discord.mute_shortcut),
                    DiscordAction::ToggleDeafen => Some(discord.deafen_shortcut),
                    DiscordAction::PushToTalk => Some(discord.ptt_shortcut),
                    _ => None,
                };
                if let Some((mods, key)) = sc {
                    push_key_press_ops(ops, mods, key);
                    ops.push(OnboardMacroOp::DelayMs(25));
                    push_key_release_ops(ops, mods, key);
                }
            }
            MacroStep::Action(_) => {}
        }
    }
}

pub fn compile_assignment_to_onboard_macro(
    item: &RichAssignment,
    macros: &[UserMacro],
    discord: &DiscordSettings,
) -> Option<Vec<OnboardMacroOp>> {
    match item {
        RichAssignment::Mouse(MouseCmd::DoubleClick) => Some(vec![
            OnboardMacroOp::ButtonDown(0x0001),
            OnboardMacroOp::DelayMs(20),
            OnboardMacroOp::ButtonUp(0x0001),
            OnboardMacroOp::DelayMs(35),
            OnboardMacroOp::ButtonDown(0x0001),
            OnboardMacroOp::DelayMs(20),
            OnboardMacroOp::ButtonUp(0x0001),
        ]),
        RichAssignment::Macro { macro_id, .. } => {
            let mac = macros.iter().find(|m| m.id == *macro_id)?;
            let delay = if mac.use_standard_delay {
                mac.standard_delay_ms.clamp(5, 5000) as u16
            } else {
                15
            };
            let mut ops = Vec::new();
            match mac.kind {
                MacroKind::NoRepeat => {
                    append_steps_to_onboard_ops(&mac.steps, delay, discord, &mut ops);
                }
                MacroKind::RepeatWhileHolding => {
                    append_steps_to_onboard_ops(&mac.steps, delay, discord, &mut ops);
                    ops.push(OnboardMacroOp::DelayMs(delay));
                    ops.push(OnboardMacroOp::RepeatWhilePressed);
                }
                MacroKind::Toggle => {
                    append_steps_to_onboard_ops(&mac.steps, delay, discord, &mut ops);
                    ops.push(OnboardMacroOp::DelayMs(delay));
                    ops.push(OnboardMacroOp::RepeatUntilCanceled);
                }
                MacroKind::Sequence => {
                    append_steps_to_onboard_ops(&mac.on_press, delay, discord, &mut ops);
                    if mac.while_holding.is_empty() {
                        ops.push(OnboardMacroOp::WaitForRelease);
                    } else {
                        append_steps_to_onboard_ops(&mac.while_holding, delay, discord, &mut ops);
                        ops.push(OnboardMacroOp::DelayMs(delay));
                        ops.push(OnboardMacroOp::RepeatWhilePressed);
                    }
                    append_steps_to_onboard_ops(&mac.on_release, delay, discord, &mut ops);
                }
            }
            (!ops.is_empty()).then_some(ops)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroKind {
    NoRepeat,
    RepeatWhileHolding,
    Toggle,
    Sequence,
}

impl MacroKind {
    pub fn title(self, lang: Lang) -> &'static str {
        match self {
            Self::NoRepeat => lang.pick("TEKRAR YOK", "NO REPEAT"),
            Self::RepeatWhileHolding => {
                lang.pick("BASILI TUTARKEN TEKRAR ET", "REPEAT WHILE HOLDING")
            }
            Self::Toggle => lang.pick("DEĞİŞTİR", "TOGGLE"),
            Self::Sequence => lang.pick("SIRA", "SEQUENCE"),
        }
    }

    pub fn tooltip(self, lang: Lang) -> &'static str {
        match self {
            Self::NoRepeat => lang.pick("Tekrar Yok", "No Repeat"),
            Self::RepeatWhileHolding => {
                lang.pick("Basılı Tutarken Tekrar Et", "Repeat While Holding")
            }
            Self::Toggle => lang.pick("Değiştir", "Toggle"),
            Self::Sequence => lang.pick("Sıra", "Sequence"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacroStep {
    KeyDown { modifiers: u8, hid_key: u8 },
    KeyUp { modifiers: u8, hid_key: u8 },
    KeyTap { modifiers: u8, hid_key: u8 },
    MouseClick { button: u8 },
    DelayMs(u32),
    TypeText(String),
    Media(MediaCmd),
    Action(IntegrationAction),
}

impl MacroStep {
    pub fn summary(&self, lang: Lang) -> String {
        match self {
            Self::KeyDown { modifiers, hid_key } => {
                format!("↓ {}", format_hid_combo(*modifiers, *hid_key))
            }
            Self::KeyUp { modifiers, hid_key } => {
                format!("↑ {}", format_hid_combo(*modifiers, *hid_key))
            }
            Self::KeyTap { modifiers, hid_key } => {
                format!("⏎ {}", format_hid_combo(*modifiers, *hid_key))
            }
            Self::MouseClick { button } => match button {
                1 => lang.pick("🖱 Sol Tıklama", "🖱 Left Click").into(),
                2 => lang.pick("🖱 Sağ Tıklama", "🖱 Right Click").into(),
                3 => lang.pick("🖱 Orta Tıklama", "🖱 Middle Click").into(),
                4 => lang.pick("🖱 Geri (G4)", "🖱 Back (G4)").into(),
                5 => lang.pick("🖱 İleri (G5)", "🖱 Forward (G5)").into(),
                _ => format!("🖱 Button {button}"),
            },
            Self::DelayMs(ms) => format!("⏱ {ms} ms"),
            Self::TypeText(t) => format!("⌨ \"{t}\""),
            Self::Media(m) => format!("♪ {}", m.label(lang)),
            Self::Action(a) => format!("⚡ {}", a.label(lang)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMacro {
    pub id: String,
    pub name: String,
    pub kind: MacroKind,
    pub use_standard_delay: bool,
    pub standard_delay_ms: u32,
    pub show_key_up_down: bool,
    pub color: Option<[u8; 3]>,
    pub steps: Vec<MacroStep>,
    pub on_press: Vec<MacroStep>,
    pub while_holding: Vec<MacroStep>,
    pub on_release: Vec<MacroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbAnimation {
    pub id: String,
    pub name: String,
    pub frame_ms: u32,
    pub frames: Vec<[[u8; 3]; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub executables: Vec<String>,
    pub enabled: bool,
    pub accent_rgb: [u8; 3],
    pub is_desktop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub locked: bool,
    pub dpi_stages: [u16; 5],
    pub active_dpi_idx: usize,
    pub shift_dpi_idx: usize,
    pub report_rate_hz: u16,
    pub buttons: [RichAssignment; 6],
    pub gshift_buttons: [RichAssignment; 6],
}

impl ProfileConfig {
    pub fn default_desktop() -> Self {
        Self {
            id: "desktop-default".into(),
            app_id: "app-desktop".into(),
            name: "Varsayılan".into(),
            locked: true,
            dpi_stages: [400, 800, 1600, 3200, 0],
            active_dpi_idx: 1,
            shift_dpi_idx: 0,
            report_rate_hz: 1000,
            buttons: [
                RichAssignment::Mouse(MouseCmd::PrimaryClick),
                RichAssignment::Mouse(MouseCmd::SecondaryClick),
                RichAssignment::Mouse(MouseCmd::MiddleClick),
                RichAssignment::Mouse(MouseCmd::Back),
                RichAssignment::Mouse(MouseCmd::Forward),
                RichAssignment::Mouse(MouseCmd::DpiCycle),
            ],
            gshift_buttons: [
                RichAssignment::Mouse(MouseCmd::PrimaryClick),
                RichAssignment::Mouse(MouseCmd::SecondaryClick),
                RichAssignment::Mouse(MouseCmd::MiddleClick),
                RichAssignment::Media(MediaCmd::VolumeDown),
                RichAssignment::Media(MediaCmd::VolumeUp),
                RichAssignment::Mouse(MouseCmd::DpiShift),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordSettings {
    pub client_id: String,
    pub access_token: String,
    pub connected_user: String,
    pub mute_shortcut: (u8, u8),
    pub deafen_shortcut: (u8, u8),
    pub ptt_shortcut: (u8, u8),
    pub prefer_rpc: bool,
}

impl Default for DiscordSettings {
    fn default() -> Self {
        Self {
            client_id: "227491271223017472".into(),
            access_token: String::new(),
            connected_user: String::new(),
            mute_shortcut: (0x03, 0x10),
            deafen_shortcut: (0x03, 0x07),
            ptt_shortcut: (0x03, 0x13),
            prefer_rpc: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsSettings {
    pub host: String,
    pub port: u16,
    pub password: String,
}

impl Default for ObsSettings {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 4455,
            password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStore {
    pub lang: Lang,
    pub home_grid_mode: bool,
    pub dynamic_lighting: bool,
    pub auto_profile_switching: bool,
    pub persistent_profile_id: Option<String>,
    pub active_profile_id: String,
    pub apps: Vec<AppEntry>,
    pub profiles: Vec<ProfileConfig>,
    pub macros: Vec<UserMacro>,
    pub custom_actions: Vec<CustomAction>,
    pub custom_commands: Vec<(String, String, u8, u8)>,
    pub animations: Vec<RgbAnimation>,
    pub discord: DiscordSettings,
    pub obs: ObsSettings,
}

impl Default for AppStore {
    fn default() -> Self {
        Self {
            lang: Lang::default(),
            home_grid_mode: true,
            dynamic_lighting: false,
            auto_profile_switching: false,
            persistent_profile_id: Some("desktop-default".into()),
            active_profile_id: "desktop-default".into(),
            apps: vec![AppEntry {
                id: "app-desktop".into(),
                name: "Masaüstü".into(),
                executables: Vec::new(),
                enabled: true,
                accent_rgb: [0, 149, 255],
                is_desktop: true,
            }],
            profiles: vec![ProfileConfig::default_desktop()],
            macros: Vec::new(),
            custom_actions: Vec::new(),
            custom_commands: Vec::new(),
            animations: default_animations(),
            discord: DiscordSettings::default(),
            obs: ObsSettings::default(),
        }
    }
}

impl AppStore {
    pub fn config_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = env::var("APPDATA") {
                return PathBuf::from(appdata).join("LogiTux").join("settings.json");
            }
        }
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("logitux").join("settings.json");
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("logitux")
                .join("settings.json");
        }
        PathBuf::from("logitux_settings.json")
    }

    pub fn load() -> Self {
        let p = Self::config_path();
        if let Ok(txt) = fs::read_to_string(&p) {
            if let Ok(mut parsed) = serde_json::from_str::<Self>(&txt) {
                if parsed.profiles.is_empty() {
                    parsed.profiles.push(ProfileConfig::default_desktop());
                }
                if parsed.apps.is_empty() {
                    parsed.apps = Self::default().apps;
                }
                if parsed.animations.is_empty() {
                    parsed.animations = default_animations();
                }
                return parsed;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let p = Self::config_path();
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&p, json);
        }
    }

    pub fn active_profile(&self) -> &ProfileConfig {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile_id)
            .unwrap_or(&self.profiles[0])
    }

    pub fn active_profile_mut(&mut self) -> &mut ProfileConfig {
        let id = self.active_profile_id.clone();
        if let Some(pos) = self.profiles.iter().position(|p| p.id == id) {
            &mut self.profiles[pos]
        } else {
            &mut self.profiles[0]
        }
    }

    pub fn app_name_for_profile(&self, prof: &ProfileConfig) -> String {
        self.apps
            .iter()
            .find(|a| a.id == prof.app_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Masaüstü".into())
    }
}

pub fn default_animations() -> Vec<RgbAnimation> {
    vec![
        RgbAnimation {
            id: "anim-ocean".into(),
            name: "Okyanus Dalgası".into(),
            frame_ms: 180,
            frames: vec![
                [[0, 149, 255], [0, 220, 255], [0, 90, 200]],
                [[0, 90, 200], [0, 149, 255], [0, 220, 255]],
                [[0, 220, 255], [0, 90, 200], [0, 149, 255]],
            ],
        },
        RgbAnimation {
            id: "anim-cyber".into(),
            name: "Siber Nabız".into(),
            frame_ms: 220,
            frames: vec![
                [[255, 0, 128], [0, 184, 252], [150, 40, 255]],
                [[150, 40, 255], [255, 0, 128], [0, 184, 252]],
                [[0, 184, 252], [150, 40, 255], [255, 0, 128]],
            ],
        },
        RgbAnimation {
            id: "anim-ember".into(),
            name: "Kızıl Kor".into(),
            frame_ms: 160,
            frames: vec![
                [[255, 30, 0], [255, 120, 0], [180, 10, 0]],
                [[180, 10, 0], [255, 45, 0], [255, 150, 0]],
                [[255, 140, 0], [200, 15, 0], [255, 50, 0]],
            ],
        },
        RgbAnimation {
            id: "anim-police".into(),
            name: "Çakar (Kırmızı / Mavi)".into(),
            frame_ms: 140,
            frames: vec![
                [[255, 0, 30], [255, 255, 255], [0, 80, 255]],
                [[0, 80, 255], [20, 20, 20], [255, 0, 30]],
            ],
        },
    ]
}

pub fn builtin_ghub_commands() -> &'static [(&'static str, &'static str, &'static str, u8, u8)] {
    &[
        ("Navigation", "Alt + Right", "Go Forward", 0x04, 0x4F),
        ("Navigation", "Alt + Left", "Go Back", 0x04, 0x50),
        (
            "Windows",
            "Ctrl + Left Windows + Left",
            "Previous Desktop",
            0x09,
            0x50,
        ),
        (
            "Windows",
            "Ctrl + Left Windows + Right",
            "Next Desktop",
            0x09,
            0x4F,
        ),
        (
            "Windows",
            "Left Windows + K",
            "Open Connect Quick Action",
            0x08,
            0x0E,
        ),
        (
            "Windows",
            "Left Windows + A",
            "Open Action Center",
            0x08,
            0x04,
        ),
        (
            "Windows",
            "Left Windows + B",
            "Set Focus in Notification Area",
            0x08,
            0x05,
        ),
        (
            "Windows",
            "Left Windows + U",
            "Open Ease of Access Center",
            0x08,
            0x18,
        ),
        (
            "Windows",
            "Left Windows + .",
            "Open Emoji Panel",
            0x08,
            0x37,
        ),
        (
            "Windows",
            "Left Windows + I",
            "Open Windows Setting",
            0x08,
            0x0C,
        ),
        ("Windows", "Left Windows + L", "Lock PC", 0x08, 0x0F),
        (
            "Windows",
            "Left Windows + D",
            "Hide/Show Desktop",
            0x08,
            0x07,
        ),
        (
            "Windows",
            "Left Windows + X",
            "Open Quick Links",
            0x08,
            0x1B,
        ),
        (
            "Windows",
            "Left Windows + E",
            "Open File Explorer",
            0x08,
            0x08,
        ),
        ("Windows", "Left Windows + S", "Open Search", 0x08, 0x16),
        (
            "Windows",
            "Left Windows + G",
            "Open Windows Game Bar",
            0x08,
            0x0A,
        ),
        (
            "Windows",
            "Left Windows + M",
            "Minimize All Windows",
            0x08,
            0x10,
        ),
        ("Windows", "Left Windows + R", "Run dialog", 0x08, 0x15),
        (
            "Windows",
            "Left Windows + T",
            "Cycle Task Bar Apps",
            0x08,
            0x17,
        ),
        (
            "Windows",
            "Left Windows + Tab",
            "Open Task View",
            0x08,
            0x2B,
        ),
        ("Windows", "Left Windows + +", "Open Magnifier", 0x08, 0x2E),
        (
            "Windows",
            "Ctrl + Left Windows + Enter",
            "Open Narrator",
            0x09,
            0x28,
        ),
        ("Windows", "Left Windows + C", "Open Copilot", 0x08, 0x06),
        ("Editing", "Ctrl + X", "Cut", 0x01, 0x1B),
        ("Editing", "Ctrl + C", "Copy", 0x01, 0x06),
        ("Editing", "Ctrl + V", "Paste", 0x01, 0x19),
        ("Editing", "Ctrl + Z", "Undo", 0x01, 0x1D),
        ("Editing", "Ctrl + Y", "Redo", 0x01, 0x1C),
        ("Editing", "Ctrl + A", "Select All", 0x01, 0x04),
        ("Editing", "Ctrl + S", "Save", 0x01, 0x16),
        ("Editing", "Ctrl + N", "New", 0x01, 0x11),
        ("Editing", "Ctrl + O", "Open", 0x01, 0x12),
        ("Editing", "Ctrl + T", "New tab", 0x01, 0x17),
        ("Editing", "Ctrl + W", "Close tab", 0x01, 0x1A),
        ("Editing", "Ctrl + Tab", "Next Tab", 0x01, 0x2B),
        ("Editing", "Ctrl + Shift + Tab", "Previous Tab", 0x03, 0x2B),
        ("Editing", "Ctrl + +", "Zoom In", 0x01, 0x2E),
        ("Editing", "Ctrl + -", "Zoom Out", 0x01, 0x2D),
        ("Editing", "Ctrl + 0", "Zoom Reset", 0x01, 0x27),
        (
            "Productivity",
            "Alt + Tab",
            "Switch Between Apps",
            0x04,
            0x2B,
        ),
        ("Productivity", "Alt + F4", "Exit Active App", 0x04, 0x3D),
        (
            "Productivity",
            "Alt + Esc",
            "Cycle Through Apps",
            0x04,
            0x29,
        ),
        (
            "Productivity",
            "Ctrl + Shift + Esc",
            "Open Task Manager",
            0x03,
            0x29,
        ),
        ("Productivity", "Left Windows", "Open Start", 0x08, 0x00),
    ]
}

pub fn all_hid_keys() -> &'static [(&'static str, &'static str, u8)] {
    &[
        ("Fonksiyon", "F1", 0x3A),
        ("Fonksiyon", "F2", 0x3B),
        ("Fonksiyon", "F3", 0x3C),
        ("Fonksiyon", "F4", 0x3D),
        ("Fonksiyon", "F5", 0x3E),
        ("Fonksiyon", "F6", 0x3F),
        ("Fonksiyon", "F7", 0x40),
        ("Fonksiyon", "F8", 0x41),
        ("Fonksiyon", "F9", 0x42),
        ("Fonksiyon", "F10", 0x43),
        ("Fonksiyon", "F11", 0x44),
        ("Fonksiyon", "F12", 0x45),
        ("Fonksiyon", "F13", 0x68),
        ("Fonksiyon", "F14", 0x69),
        ("Fonksiyon", "F15", 0x6A),
        ("Fonksiyon", "F16", 0x6B),
        ("Fonksiyon", "F17", 0x6C),
        ("Fonksiyon", "F18", 0x6D),
        ("Fonksiyon", "F19", 0x6E),
        ("Fonksiyon", "F20", 0x6F),
        ("Fonksiyon", "F21", 0x70),
        ("Fonksiyon", "F22", 0x71),
        ("Fonksiyon", "F23", 0x72),
        ("Fonksiyon", "F24", 0x73),
        ("Harfler", "A", 0x04),
        ("Harfler", "B", 0x05),
        ("Harfler", "C", 0x06),
        ("Harfler", "D", 0x07),
        ("Harfler", "E", 0x08),
        ("Harfler", "F", 0x09),
        ("Harfler", "G", 0x0A),
        ("Harfler", "H", 0x0B),
        ("Harfler", "I", 0x0C),
        ("Harfler", "J", 0x0D),
        ("Harfler", "K", 0x0E),
        ("Harfler", "L", 0x0F),
        ("Harfler", "M", 0x10),
        ("Harfler", "N", 0x11),
        ("Harfler", "O", 0x12),
        ("Harfler", "P", 0x13),
        ("Harfler", "Q", 0x14),
        ("Harfler", "R", 0x15),
        ("Harfler", "S", 0x16),
        ("Harfler", "T", 0x17),
        ("Harfler", "U", 0x18),
        ("Harfler", "V", 0x19),
        ("Harfler", "W", 0x1A),
        ("Harfler", "X", 0x1B),
        ("Harfler", "Y", 0x1C),
        ("Harfler", "Z", 0x1D),
        ("Sayılar", "1", 0x1E),
        ("Sayılar", "2", 0x1F),
        ("Sayılar", "3", 0x20),
        ("Sayılar", "4", 0x21),
        ("Sayılar", "5", 0x22),
        ("Sayılar", "6", 0x23),
        ("Sayılar", "7", 0x24),
        ("Sayılar", "8", 0x25),
        ("Sayılar", "9", 0x26),
        ("Sayılar", "0", 0x27),
        ("Kontrol & Gezinme", "Enter", 0x28),
        ("Kontrol & Gezinme", "Escape", 0x29),
        ("Kontrol & Gezinme", "Backspace", 0x2A),
        ("Kontrol & Gezinme", "Tab", 0x2B),
        ("Kontrol & Gezinme", "Space", 0x2C),
        ("Kontrol & Gezinme", "Caps Lock", 0x39),
        ("Kontrol & Gezinme", "Print Screen", 0x46),
        ("Kontrol & Gezinme", "Scroll Lock", 0x47),
        ("Kontrol & Gezinme", "Pause", 0x48),
        ("Kontrol & Gezinme", "Insert", 0x49),
        ("Kontrol & Gezinme", "Home", 0x4A),
        ("Kontrol & Gezinme", "Page Up", 0x4B),
        ("Kontrol & Gezinme", "Delete", 0x4C),
        ("Kontrol & Gezinme", "End", 0x4D),
        ("Kontrol & Gezinme", "Page Down", 0x4E),
        ("Kontrol & Gezinme", "Right Arrow", 0x4F),
        ("Kontrol & Gezinme", "Left Arrow", 0x50),
        ("Kontrol & Gezinme", "Down Arrow", 0x51),
        ("Kontrol & Gezinme", "Up Arrow", 0x52),
        ("Numpad", "Num 0", 0x62),
        ("Numpad", "Num 1", 0x59),
        ("Numpad", "Num 2", 0x5A),
        ("Numpad", "Num 3", 0x5B),
        ("Numpad", "Num 4", 0x5C),
        ("Numpad", "Num 5", 0x5D),
        ("Numpad", "Num 6", 0x5E),
        ("Numpad", "Num 7", 0x5F),
        ("Numpad", "Num 8", 0x60),
        ("Numpad", "Num 9", 0x61),
        ("Numpad", "Num +", 0x57),
        ("Numpad", "Num -", 0x56),
        ("Numpad", "Num *", 0x55),
        ("Numpad", "Num /", 0x54),
        ("Numpad", "Num Enter", 0x58),
    ]
}

pub fn hid_key_name(code: u8) -> String {
    if code == 0 {
        return String::new();
    }
    for (_, name, c) in all_hid_keys() {
        if *c == code {
            return (*name).to_string();
        }
    }
    match code {
        0x2D => "-".into(),
        0x2E => "+".into(),
        0x36 => ",".into(),
        0x37 => ".".into(),
        _ => format!("Key 0x{code:02X}"),
    }
}

pub fn format_hid_combo(modifiers: u8, hid_key: u8) -> String {
    let mut parts: Vec<String> = Vec::new();
    if (modifiers & 0x01) != 0 {
        parts.push("Ctrl".into());
    }
    if (modifiers & 0x02) != 0 {
        parts.push("Shift".into());
    }
    if (modifiers & 0x04) != 0 {
        parts.push("Alt".into());
    }
    if (modifiers & 0x08) != 0 {
        parts.push("Super".into());
    }
    let k = hid_key_name(hid_key);
    if !k.is_empty() {
        parts.push(k);
    }
    if parts.is_empty() {
        "Key".into()
    } else {
        parts.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_and_gshift_map_to_onboard() {
        let disc = DiscordSettings::default();
        let mute = RichAssignment::Action(IntegrationAction::Discord(DiscordAction::ToggleMute));
        assert_eq!(
            mute.to_onboard_with_discord(&disc),
            ButtonAction::Keyboard {
                modifiers: 0x03,
                key: 0x10
            }
        );
        assert!(!mute.is_host_only());

        let gshift = RichAssignment::Mouse(MouseCmd::GShift);
        assert_eq!(gshift.to_onboard_with_discord(&disc), ButtonAction::Special(0x0B));
        assert_eq!(
            RichAssignment::from_onboard(ButtonAction::Special(0x0B)),
            RichAssignment::Mouse(MouseCmd::GShift)
        );
    }

    #[test]
    fn test_double_click_and_user_macro_compile_to_onboard_ops() {
        let disc = DiscordSettings::default();
        let dbl = RichAssignment::Mouse(MouseCmd::DoubleClick);
        let ops = compile_assignment_to_onboard_macro(&dbl, &[], &disc).unwrap();
        assert_eq!(ops.len(), 7);

        let mac = UserMacro {
            id: "m1".into(),
            name: "Test".into(),
            kind: MacroKind::RepeatWhileHolding,
            use_standard_delay: true,
            standard_delay_ms: 30,
            show_key_up_down: false,
            color: None,
            steps: vec![MacroStep::KeyTap {
                modifiers: 0x01,
                hid_key: 0x06,
            }],
            on_press: vec![],
            while_holding: vec![],
            on_release: vec![],
        };
        let item = RichAssignment::Macro {
            macro_id: "m1".into(),
            macro_name: "Test".into(),
        };
        let mops = compile_assignment_to_onboard_macro(&item, &[mac], &disc).unwrap();
        assert!(matches!(mops.last(), Some(OnboardMacroOp::RepeatWhilePressed)));
    }
}
