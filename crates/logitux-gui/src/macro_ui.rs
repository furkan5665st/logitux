use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::i18n::Lang;
use crate::icons::{self, GhubIcon};
use crate::input_synth;
use crate::store::{
    DiscordAction, IntegrationAction, MacroKind, MacroStep, MediaCmd, ObsAction, UserMacro,
};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroWizardPhase {
    NamePrompt,
    ChooseType,
    Editor,
}

pub enum MacroWizardOutcome {
    None,
    Cancelled,
    Saved(UserMacro),
}

pub struct MacroWizardState {
    pub active: bool,
    pub phase: MacroWizardPhase,
    pub draft: UserMacro,
    pub seq_tab: usize,
    pub add_menu_open: bool,
    pub recording: bool,
    pub last_event_sec: Option<f64>,
    pub text_buf: String,
    pub custom_delay_ms: u32,
}

impl Default for MacroWizardState {
    fn default() -> Self {
        Self {
            active: false,
            phase: MacroWizardPhase::NamePrompt,
            draft: Self::blank_macro(),
            seq_tab: 0,
            add_menu_open: false,
            recording: false,
            last_event_sec: None,
            text_buf: String::new(),
            custom_delay_ms: 50,
        }
    }
}

impl MacroWizardState {
    fn blank_macro() -> UserMacro {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(1);
        UserMacro {
            id: format!("macro-{ts}"),
            name: String::new(),
            kind: MacroKind::NoRepeat,
            use_standard_delay: true,
            standard_delay_ms: 50,
            show_key_up_down: true,
            color: None,
            steps: Vec::new(),
            on_press: Vec::new(),
            while_holding: Vec::new(),
            on_release: Vec::new(),
        }
    }

    pub fn open_new(&mut self) {
        self.active = true;
        self.phase = MacroWizardPhase::NamePrompt;
        self.draft = Self::blank_macro();
        self.seq_tab = 0;
        self.add_menu_open = false;
        self.recording = false;
        self.last_event_sec = None;
        self.text_buf.clear();
    }

    pub fn open_edit(&mut self, existing: &UserMacro) {
        self.active = true;
        self.phase = MacroWizardPhase::Editor;
        self.draft = existing.clone();
        self.seq_tab = 0;
        self.add_menu_open = false;
        self.recording = false;
        self.last_event_sec = None;
    }

    fn current_track_mut(&mut self) -> &mut Vec<MacroStep> {
        if self.draft.kind == MacroKind::Sequence {
            match self.seq_tab {
                0 => &mut self.draft.on_press,
                1 => &mut self.draft.while_holding,
                _ => &mut self.draft.on_release,
            }
        } else {
            &mut self.draft.steps
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, lang: Lang) -> MacroWizardOutcome {
        let mut outcome = MacroWizardOutcome::None;

        if self.recording && self.phase == MacroWizardPhase::Editor {
            let now = ctx.input(|i| i.time);
            let events = ctx.input(|i| i.events.clone());
            let std_delay = self.draft.use_standard_delay;
            let show_ud = self.draft.show_key_up_down;
            for ev in events {
                if let egui::Event::Key {
                    key,
                    pressed,
                    repeat: false,
                    modifiers,
                    ..
                } = ev
                {
                    if key == egui::Key::Escape {
                        self.recording = false;
                        break;
                    }
                    if let Some(hid) = input_synth::egui_key_to_hid(key) {
                        let mods = input_synth::egui_modifiers_mask(modifiers);
                        if !std_delay {
                            if let Some(prev) = self.last_event_sec {
                                let dt = ((now - prev) * 1000.0).round() as u32;
                                if dt >= 5 {
                                    self.current_track_mut().push(MacroStep::DelayMs(dt));
                                }
                            }
                        }
                        self.last_event_sec = Some(now);
                        if show_ud {
                            if pressed {
                                self.current_track_mut().push(MacroStep::KeyDown {
                                    modifiers: mods,
                                    hid_key: hid,
                                });
                            } else {
                                self.current_track_mut().push(MacroStep::KeyUp {
                                    modifiers: mods,
                                    hid_key: hid,
                                });
                            }
                        } else if pressed {
                            self.current_track_mut().push(MacroStep::KeyTap {
                                modifiers: mods,
                                hid_key: hid,
                            });
                        }
                    }
                }
            }
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG_BLACK)
                    .inner_margin(egui::Margin::symmetric(24, 18)),
            )
            .show(ctx, |ui| {
                let (back_r, back_resp) =
                    ui.allocate_exact_size(Vec2::new(32.0, 28.0), Sense::click());
                icons::draw_icon(
                    &ui.painter_at(back_r),
                    back_r.center(),
                    8.5,
                    GhubIcon::BackArrow,
                    if back_resp.hovered() {
                        Color32::WHITE
                    } else {
                        theme::TEXT_LIGHT
                    },
                );
                if back_resp.clicked() {
                    self.recording = false;
                    match self.phase {
                        MacroWizardPhase::NamePrompt => {
                            self.active = false;
                            outcome = MacroWizardOutcome::Cancelled;
                        }
                        MacroWizardPhase::ChooseType => {
                            self.phase = MacroWizardPhase::NamePrompt;
                        }
                        MacroWizardPhase::Editor => {
                            self.active = false;
                            outcome = MacroWizardOutcome::Cancelled;
                        }
                    }
                }

                match self.phase {
                    MacroWizardPhase::NamePrompt => self.render_name_prompt(ui, lang),
                    MacroWizardPhase::ChooseType => self.render_choose_type(ui, lang),
                    MacroWizardPhase::Editor => {
                        if let Some(saved) = self.render_editor(ctx, ui, lang) {
                            self.active = false;
                            outcome = MacroWizardOutcome::Saved(saved);
                        }
                    }
                }
            });

        outcome
    }

    fn render_name_prompt(&mut self, ui: &mut egui::Ui, lang: Lang) {
        let avail = ui.available_size();
        ui.vertical_centered(|ui| {
            ui.add_space((avail.y * 0.40).max(80.0));
            let te = ui.add(
                egui::TextEdit::singleline(&mut self.draft.name)
                    .hint_text(
                        RichText::new(lang.pick("Bu makroyu adlandır", "Name this macro"))
                            .size(21.0)
                            .color(theme::TEXT_MUTED),
                    )
                    .font(egui::FontId::proportional(21.0))
                    .frame(false)
                    .desired_width(320.0)
                    .horizontal_align(egui::Align::Center),
            );
            te.request_focus();
            let line_w = 260.0;
            let (lrect, _) = ui.allocate_exact_size(Vec2::new(line_w, 4.0), Sense::hover());
            ui.painter().line_segment(
                [
                    Pos2::new(lrect.left(), lrect.top()),
                    Pos2::new(lrect.right(), lrect.top()),
                ],
                Stroke::new(1.0_f32, theme::BORDER_DIM),
            );
            ui.add_space(18.0);
            if !self.draft.name.trim().is_empty() {
                if ui.input(|i| i.key_pressed(egui::Key::Enter))
                    || ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick("DEVAM ET →", "CONTINUE →"))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme::GHUB_BLUE),
                        )
                        .clicked()
                {
                    self.phase = MacroWizardPhase::ChooseType;
                }
            }
        });
    }

    fn render_choose_type(&mut self, ui: &mut egui::Ui, lang: Lang) {
        let avail = ui.available_size();
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            ui.label(
                RichText::new(&self.draft.name)
                    .size(20.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.add_space((avail.y * 0.20).clamp(48.0, 150.0));
            ui.label(
                RichText::new(lang.pick(
                    "OLUŞTURMAK İSTEDİĞİNİZ MAKRO TÜRÜNÜ SEÇİN",
                    "SELECT THE TYPE OF MACRO YOU WANT TO CREATE",
                ))
                .size(12.0)
                .strong()
                .color(Color32::WHITE),
            );
            ui.add_space(28.0);

            let cards = [
                (MacroKind::NoRepeat, GhubIcon::MacroNoRepeat),
                (MacroKind::RepeatWhileHolding, GhubIcon::MacroRepeatHold),
                (MacroKind::Toggle, GhubIcon::MacroToggle),
                (MacroKind::Sequence, GhubIcon::MacroSequence),
            ];

            let total_w = 4.0 * 124.0 + 3.0 * 28.0;
            let start_x = (ui.available_width() - total_w).max(0.0) * 0.5;
            ui.horizontal(|ui| {
                ui.add_space(start_x);
                for (kind, icon) in cards {
                    ui.vertical(|ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::new(124.0, 118.0), Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(
                            rect,
                            CornerRadius::same(3),
                            if resp.hovered() {
                                Color32::from_rgb(38, 40, 46)
                            } else {
                                Color32::from_rgb(24, 25, 28)
                            },
                        );
                        if resp.hovered() {
                            painter.rect_stroke(
                                rect,
                                CornerRadius::same(3),
                                Stroke::new(1.5_f32, theme::GHUB_BLUE),
                                egui::StrokeKind::Inside,
                            );
                        }
                        icons::draw_icon(&painter, rect.center(), 15.0, icon, Color32::WHITE);

                        let lbl_rect = Rect::from_min_size(
                            Pos2::new(rect.left() - 8.0, rect.bottom() + 12.0),
                            Vec2::new(140.0, 34.0),
                        );
                        ui.painter().text(
                            Pos2::new(lbl_rect.center().x, lbl_rect.top() + 8.0),
                            egui::Align2::CENTER_TOP,
                            kind.title(lang),
                            egui::FontId::proportional(10.5),
                            Color32::WHITE,
                        );
                        ui.allocate_space(Vec2::new(124.0, 36.0));

                        if resp.clicked() {
                            self.draft.kind = kind;
                            self.phase = MacroWizardPhase::Editor;
                        }
                    });
                    ui.add_space(20.0);
                }
            });
        });
    }

    fn render_editor(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        lang: Lang,
    ) -> Option<UserMacro> {
        let mut saved_out = None;

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(130.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.draft.name)
                        .font(egui::FontId::proportional(20.0))
                        .frame(false)
                        .desired_width(240.0),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(lang.pick("Macro türleri", "Macro types"))
                            .size(12.0)
                            .color(theme::TEXT_LIGHT),
                    );
                    ui.add_space(6.0);
                    for (kind, ic) in [
                        (MacroKind::NoRepeat, GhubIcon::MacroNoRepeat),
                        (MacroKind::RepeatWhileHolding, GhubIcon::MacroRepeatHold),
                        (MacroKind::Toggle, GhubIcon::MacroToggle),
                        (MacroKind::Sequence, GhubIcon::MacroSequence),
                    ] {
                        let is_sel = self.draft.kind == kind;
                        let (r, resp) =
                            ui.allocate_exact_size(Vec2::new(28.0, 24.0), Sense::click());
                        let p = ui.painter_at(r);
                        p.rect_filled(r, CornerRadius::same(3), Color32::from_rgb(20, 21, 24));
                        p.rect_stroke(
                            r,
                            CornerRadius::same(3),
                            Stroke::new(
                                if is_sel { 1.5_f32 } else { 1.0_f32 },
                                if is_sel {
                                    theme::GHUB_BLUE
                                } else {
                                    theme::BORDER_SUBTLE
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        icons::draw_icon(&p, r.center(), 7.5, ic, Color32::WHITE);
                        if resp.hovered() {
                            let tip_rect = Rect::from_center_size(
                                Pos2::new(r.center().x, r.bottom() + 18.0),
                                Vec2::new(130.0, 24.0),
                            );
                            let lp = ctx.layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                egui::Id::new("macro_type_tip"),
                            ));
                            lp.rect_filled(
                                tip_rect,
                                CornerRadius::same(3),
                                Color32::from_rgb(43, 44, 49),
                            );
                            lp.text(
                                tip_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                kind.tooltip(lang),
                                egui::FontId::proportional(11.0),
                                Color32::WHITE,
                            );
                        }
                        if resp.clicked() {
                            self.draft.kind = kind;
                        }
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.add_space(130.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.draft.use_standard_delay,
                            RichText::new(
                                lang.pick("STANDART GECİKMELERİ KULLAN", "USE STANDARD DELAYS"),
                            )
                            .size(11.0)
                            .strong()
                            .color(Color32::WHITE),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.draft.standard_delay_ms)
                                .range(5..=5000)
                                .suffix(" ms"),
                        );
                    });
                    ui.label(
                        RichText::new(lang.pick(
                            "    Eylemler arası standart gecikme kullanılıyor",
                            "    Using standard delay between actions",
                        ))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.checkbox(
                        &mut self.draft.show_key_up_down,
                        RichText::new(
                            lang.pick("TUŞ AŞAĞI/TUŞ YUKARI GÖSTER", "SHOW KEY DOWN / KEY UP"),
                        )
                        .size(11.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let col_preview = self
                            .draft
                            .color
                            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                            .unwrap_or(theme::STATUS_RED);
                        let (cr, c_resp) =
                            ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::click());
                        let cp = ui.painter_at(cr);
                        if self.draft.color.is_some() {
                            cp.circle_filled(cr.center(), 7.5, col_preview);
                        } else {
                            cp.circle_stroke(
                                cr.center(),
                                7.0,
                                Stroke::new(1.8_f32, theme::STATUS_RED),
                            );
                            cp.line_segment(
                                [
                                    Pos2::new(cr.center().x - 4.5, cr.center().y - 4.5),
                                    Pos2::new(cr.center().x + 4.5, cr.center().y + 4.5),
                                ],
                                Stroke::new(1.8_f32, theme::STATUS_RED),
                            );
                        }
                        if c_resp.clicked()
                            || ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(lang.pick("MAKRO RENGİ", "MACRO COLOR"))
                                            .size(11.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                        {
                            let palette: [Option<[u8; 3]>; 6] = [
                                None,
                                Some([0, 149, 255]),
                                Some([255, 67, 144]),
                                Some([255, 180, 0]),
                                Some([0, 214, 143]),
                                Some([155, 93, 229]),
                            ];
                            let cur_i = palette
                                .iter()
                                .position(|p| *p == self.draft.color)
                                .unwrap_or(0);
                            self.draft.color = palette[(cur_i + 1) % palette.len()];
                        }
                    });
                });
            });
        });

        ui.add_space(36.0);

        if self.draft.kind == MacroKind::Sequence {
            ui.horizontal(|ui| {
                ui.add_space(130.0);
                for (ti, tname) in [
                    (0usize, lang.pick("1. BASILDIĞINDA", "1. ON PRESS")),
                    (
                        1usize,
                        lang.pick("2. BASILI TUTULURKEN", "2. WHILE HOLDING"),
                    ),
                    (2usize, lang.pick("3. BIRAKILDIĞINDA", "3. ON RELEASE")),
                ] {
                    let active = self.seq_tab == ti;
                    if ui
                        .selectable_label(
                            active,
                            RichText::new(tname).size(12.0).strong().color(if active {
                                theme::GHUB_BLUE
                            } else {
                                theme::TEXT_MUTED
                            }),
                        )
                        .clicked()
                    {
                        self.seq_tab = ti;
                    }
                    ui.add_space(12.0);
                }
            });
            ui.add_space(16.0);
        }

        let body_h = (ui.available_height() - 80.0).max(180.0);
        ui.allocate_ui(Vec2::new(ui.available_width(), body_h), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(130.0);
                ui.vertical(|ui| {
                    let mut remove_idx: Option<usize> = None;
                    let steps_snapshot = self.current_track_mut().clone();
                    if !steps_snapshot.is_empty() {
                        egui::ScrollArea::vertical()
                            .max_height((body_h - 110.0).max(90.0))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    for (idx, step) in steps_snapshot.iter().enumerate() {
                                        egui::Frame::new()
                                            .fill(theme::BG_DRAWER)
                                            .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                                            .corner_radius(CornerRadius::same(4))
                                            .inner_margin(egui::Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(step.summary(lang))
                                                            .size(12.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                    if ui
                                                        .small_button("×")
                                                        .on_hover_text(lang.pick("Sil", "Remove"))
                                                        .clicked()
                                                    {
                                                        remove_idx = Some(idx);
                                                    }
                                                });
                                            });
                                    }
                                });
                            });
                        if let Some(ri) = remove_idx {
                            let track = self.current_track_mut();
                            if ri < track.len() {
                                track.remove(ri);
                            }
                        }
                        ui.add_space(18.0);
                    } else {
                        ui.add_space((body_h * 0.28).clamp(24.0, 110.0));
                    }

                    ui.horizontal(|ui| {
                        let (plus_r, plus_resp) =
                            ui.allocate_exact_size(Vec2::new(38.0, 38.0), Sense::click());
                        let pp = ui.painter_at(plus_r);
                        pp.rect_filled(
                            plus_r,
                            CornerRadius::same(3),
                            if self.recording {
                                theme::STATUS_RED
                            } else if plus_resp.hovered() {
                                theme::BG_HOVER
                            } else {
                                Color32::from_rgb(52, 54, 60)
                            },
                        );
                        let c = plus_r.center();
                        pp.line_segment(
                            [Pos2::new(c.x - 7.0, c.y), Pos2::new(c.x + 7.0, c.y)],
                            Stroke::new(1.8_f32, Color32::WHITE),
                        );
                        pp.line_segment(
                            [Pos2::new(c.x, c.y - 7.0), Pos2::new(c.x, c.y + 7.0)],
                            Stroke::new(1.8_f32, Color32::WHITE),
                        );

                        ui.add_space(8.0);
                        let lbl = if self.recording {
                            lang.pick(
                                "KAYIT YAPILIYOR... (Durdurmak için tıkla veya ESC)",
                                "RECORDING KEYSTROKES... (Click or ESC to stop)",
                            )
                        } else if steps_snapshot.is_empty() {
                            lang.pick("ŞİMDİ BAŞLA", "START NOW")
                        } else {
                            lang.pick("EYLEM EKLE", "ADD ACTION")
                        };
                        let txt_resp = ui.add(
                            egui::Button::new(
                                RichText::new(lbl).size(12.0).strong().color(Color32::WHITE),
                            )
                            .frame(false),
                        );
                        if plus_resp.clicked() || txt_resp.clicked() {
                            if self.recording {
                                self.recording = false;
                            } else {
                                self.add_menu_open = !self.add_menu_open;
                            }
                        }
                    });

                    if self.add_menu_open {
                        ui.add_space(10.0);
                        egui::Frame::new()
                            .fill(theme::BG_DRAWER)
                            .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(egui::Margin::same(14))
                            .show(ui, |ui| {
                                ui.set_max_width(580.0);
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .button(lang.pick(
                                            "● Tuş Vuruşlarını Kaydet",
                                            "● Record Keystrokes",
                                        ))
                                        .clicked()
                                    {
                                        self.recording = true;
                                        self.last_event_sec = None;
                                        self.add_menu_open = false;
                                    }
                                    for (m_lbl, b_id) in [
                                        (lang.pick("+ Sol Tık", "+ Left Click"), 1u8),
                                        (lang.pick("+ Sağ Tık", "+ Right Click"), 2u8),
                                        (lang.pick("+ Orta Tık", "+ Middle Click"), 3u8),
                                    ] {
                                        if ui.button(m_lbl).clicked() {
                                            self.current_track_mut()
                                                .push(MacroStep::MouseClick { button: b_id });
                                        }
                                    }
                                    if ui
                                        .button(
                                            lang.pick(
                                                "+ Discord Sessize Al",
                                                "+ Discord Toggle Mute",
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.current_track_mut().push(MacroStep::Action(
                                            IntegrationAction::Discord(DiscordAction::ToggleMute),
                                        ));
                                    }
                                    if ui
                                        .button(lang.pick("+ OBS Kayıt", "+ OBS Toggle Record"))
                                        .clicked()
                                    {
                                        self.current_track_mut().push(MacroStep::Action(
                                            IntegrationAction::Obs(ObsAction::ToggleRecording),
                                        ));
                                    }
                                    if ui
                                        .button(lang.pick("+ Oynat/Duraklat", "+ Play/Pause"))
                                        .clicked()
                                    {
                                        self.current_track_mut()
                                            .push(MacroStep::Media(MediaCmd::PlayPause));
                                    }
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label(lang.pick("Metin / Emoji:", "Text / Emoji:"));
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.text_buf)
                                            .desired_width(180.0),
                                    );
                                    if ui.button(lang.pick("Metin Ekle", "Add Text")).clicked()
                                        && !self.text_buf.is_empty()
                                    {
                                        let t = self.text_buf.clone();
                                        self.current_track_mut().push(MacroStep::TypeText(t));
                                        self.text_buf.clear();
                                    }
                                    ui.separator();
                                    ui.add(
                                        egui::DragValue::new(&mut self.custom_delay_ms)
                                            .range(5..=10000)
                                            .suffix(" ms"),
                                    );
                                    if ui.button(lang.pick("Gecikme Ekle", "Add Delay")).clicked() {
                                        let d = self.custom_delay_ms;
                                        self.current_track_mut().push(MacroStep::DelayMs(d));
                                    }
                                });
                            });
                    }
                });
            });
        });

        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                let half_pad = (ui.available_width() - 340.0).max(0.0) * 0.5;
                ui.add_space(half_pad);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(lang.pick("SIFIRLA", "RESET"))
                                .size(11.5)
                                .strong()
                                .color(theme::TEXT_LIGHT),
                        )
                        .fill(Color32::from_rgb(24, 25, 28))
                        .min_size(Vec2::new(160.0, 34.0)),
                    )
                    .clicked()
                {
                    self.draft.steps.clear();
                    self.draft.on_press.clear();
                    self.draft.while_holding.clear();
                    self.draft.on_release.clear();
                    self.recording = false;
                }
                ui.add_space(16.0);
                let has_any = !self.draft.steps.is_empty()
                    || !self.draft.on_press.is_empty()
                    || !self.draft.while_holding.is_empty()
                    || !self.draft.on_release.is_empty();
                let save_fill = if has_any {
                    theme::GHUB_BLUE
                } else {
                    theme::GHUB_TEAL_SAVE
                };
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(lang.pick("KAYDET", "SAVE"))
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(save_fill)
                        .min_size(Vec2::new(160.0, 34.0)),
                    )
                    .clicked()
                {
                    self.recording = false;
                    if self.draft.name.trim().is_empty() {
                        self.draft.name = "Makro 1".into();
                    }
                    saved_out = Some(self.draft.clone());
                }
            });
        });

        saved_out
    }
}
