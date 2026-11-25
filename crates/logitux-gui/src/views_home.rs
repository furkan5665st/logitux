use eframe::egui::{self, Color32, CornerRadius, Pos2, RichText, Sense, Stroke, Vec2};
use logitux_hidpp::{features::onboard::OnboardMode, DeviceSummary};

use crate::canvas;
use crate::icons::{self, GhubIcon};
use crate::store::{AppEntry, AppStore, ProfileConfig};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileFilter {
    All,
    Enabled,
    Disabled,
}

pub enum DeviceSettingsEvent {
    Back,
    ToggleStartupEffect(bool),
    ToggleOnboardMode(OnboardMode),
    FlashSlot1,
    RestoreDefaults,
}

pub fn render_profiles_page(
    ui: &mut egui::Ui,
    store: &mut AppStore,
    filter: &mut ProfileFilter,
    search_q: &mut String,
    adding_app_modal: &mut bool,
    new_app_name: &mut String,
    new_app_exe: &mut String,
) -> Option<String> {
    let lang = store.lang;
    let mut activated_profile: Option<String> = None;

    ui.add_space(16.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui(Vec2::new(225.0, ui.available_height()), |ui| {
            let total = store.apps.len();
            for (f, lbl, count_opt) in [
                (ProfileFilter::All, lang.pick("Tümü", "All"), Some(total)),
                (ProfileFilter::Enabled, lang.pick("Etkin", "Enabled"), None),
                (
                    ProfileFilter::Disabled,
                    lang.pick("Devre dışı bırakıldı", "Disabled"),
                    None,
                ),
            ] {
                let is_sel = *filter == f;
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(205.0, 32.0), Sense::click());
                let p = ui.painter_at(rect);
                if is_sel {
                    p.rect_filled(
                        rect,
                        CornerRadius::same(4),
                        Color32::from_rgb(0, 52, 92),
                    );
                } else if resp.hovered() {
                    p.rect_filled(rect, CornerRadius::same(4), theme::BG_DRAWER);
                }
                p.text(
                    Pos2::new(rect.left() + 12.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    lbl,
                    egui::FontId::proportional(12.5),
                    if is_sel {
                        theme::GHUB_BLUE
                    } else {
                        Color32::WHITE
                    },
                );
                if let Some(c) = count_opt {
                    p.text(
                        Pos2::new(rect.right() - 12.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{c}"),
                        egui::FontId::proportional(12.0),
                        theme::GHUB_BLUE,
                    );
                }
                if resp.clicked() {
                    *filter = f;
                }
            }

            let bottom_space = (ui.available_height() - 125.0).max(24.0);
            ui.add_space(bottom_space);

            ui.label(
                RichText::new(lang.pick("Profil Değiştirme  ⓘ", "Profile Switching  ⓘ"))
                    .size(13.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.add_space(4.0);
            if ui
                .checkbox(
                    &mut store.auto_profile_switching,
                    RichText::new(lang.pick(
                        "Profil değiştirmeyi etkinleştir",
                        "Enable profile switching",
                    ))
                    .size(11.5)
                    .color(theme::TEXT_LIGHT),
                )
                .changed()
            {
                store.save();
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(lang.pick("Kalıcı profil seç", "Select persistent profile"))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
            let cur_pers_id = store
                .persistent_profile_id
                .clone()
                .unwrap_or_else(|| "desktop-default".into());
            let cur_pers_lbl = store
                .profiles
                .iter()
                .find(|p| p.id == cur_pers_id)
                .map(|p| format!("{} - {}", store.app_name_for_profile(p), p.name))
                .unwrap_or_else(|| "Masaüstü - Varsayılan".into());
            egui::ComboBox::from_id_salt("ghub_persistent_prof")
                .selected_text(RichText::new(cur_pers_lbl).size(11.5))
                .width(195.0)
                .show_ui(ui, |ui| {
                    for p in &store.profiles {
                        let lbl = format!("{} - {}", store.app_name_for_profile(p), p.name);
                        if ui
                            .selectable_label(store.persistent_profile_id.as_deref() == Some(&p.id), lbl)
                            .clicked()
                        {
                            store.persistent_profile_id = Some(p.id.clone());
                            activated_profile = Some(p.id.clone());
                        }
                    }
                });
        });

        ui.add_space(20.0);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(lang.pick(
                            "Tüm Oyunlar ve Uygulamalar",
                            "All Games & Applications",
                        ))
                        .size(21.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                    ui.add_space(4.0);
                    ui.set_max_width(580.0);
                    ui.label(
                        RichText::new(lang.pick(
                            "Profillerini, makrolarını ve entegrasyonlarını yönetmek için bir oyun veya uygulama seçin. Bir oyun veya uygulama devre dışı bırakmak, bu oyunun/uygulamanın profillerinin aktif hâle gelmesini önler.",
                            "Select a game or application to manage its profiles, macros, and integrations.",
                        ))
                        .size(11.5)
                        .color(theme::TEXT_MUTED),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.add_space(16.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick(
                                    "+ OYUN VE UYGULAMA EKLE",
                                    "+ ADD GAME OR APPLICATION",
                                ))
                                .size(11.5)
                                .strong()
                                .color(Color32::WHITE),
                            )
                            .fill(theme::GHUB_BLUE)
                            .min_size(Vec2::new(200.0, 36.0)),
                        )
                        .clicked()
                    {
                        *adding_app_modal = true;
                    }
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(theme::BG_BLACK)
                        .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🔍").size(12.0).color(theme::TEXT_MUTED));
                                ui.add(
                                    egui::TextEdit::singleline(search_q)
                                        .hint_text(lang.pick("Ara", "Search"))
                                        .frame(false)
                                        .desired_width(150.0),
                                );
                            });
                        });
                });
            });

            ui.add_space(22.0);

            let q_low = search_q.trim().to_lowercase();
            let apps_snapshot = store.apps.clone();
            let mut toggle_app_id: Option<String> = None;
            let mut add_subprofile_for: Option<String> = None;

            ui.horizontal_wrapped(|ui| {
                for app in &apps_snapshot {
                    if *filter == ProfileFilter::Enabled && !app.enabled {
                        continue;
                    }
                    if *filter == ProfileFilter::Disabled && app.enabled {
                        continue;
                    }
                    if !q_low.is_empty() && !app.name.to_lowercase().contains(&q_low) {
                        continue;
                    }

                    let is_active_app = store.active_profile().app_id == app.id;
                    let card_stroke = if is_active_app {
                        Stroke::new(1.5_f32, theme::GHUB_BLUE)
                    } else {
                        Stroke::new(1.0_f32, theme::BORDER_DIM)
                    };

                    egui::Frame::new()
                        .fill(Color32::from_rgb(6, 22, 38))
                        .stroke(card_stroke)
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.set_width(142.0);
                            let (art_r, art_resp) =
                                ui.allocate_exact_size(Vec2::new(142.0, 156.0), Sense::click());
                            let ap = ui.painter_at(art_r);
                            ap.rect_filled(
                                art_r,
                                CornerRadius::same(4),
                                Color32::from_rgb(24, 26, 31),
                            );
                            ap.circle_filled(
                                Pos2::new(art_r.center().x + 14.0, art_r.center().y + 18.0),
                                32.0,
                                Color32::from_rgb(16, 18, 22),
                            );
                            icons::draw_icon(
                                &ap,
                                Pos2::new(art_r.center().x + 14.0, art_r.center().y + 14.0),
                                8.0,
                                GhubIcon::LogoMark,
                                Color32::from_rgb(
                                    app.accent_rgb[0],
                                    app.accent_rgb[1],
                                    app.accent_rgb[2],
                                ),
                            );

                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(&app.name)
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            if !app.executables.is_empty() {
                                ui.label(
                                    RichText::new(app.executables.join(", "))
                                        .size(10.0)
                                        .color(theme::TEXT_MUTED),
                                );
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let (ic_r, _) =
                                    ui.allocate_exact_size(Vec2::new(18.0, 16.0), Sense::hover());
                                icons::draw_icon(
                                    &ui.painter_at(ic_r),
                                    ic_r.center(),
                                    7.5,
                                    GhubIcon::ProfileLock,
                                    theme::GHUB_BLUE,
                                );
                                if ui.small_button("+").on_hover_text("Yeni alt profil ekle").clicked()
                                {
                                    add_subprofile_for = Some(app.id.clone());
                                }
                                if !app.is_desktop
                                    && ui
                                        .small_button(if app.enabled { "✓" } else { "⊘" })
                                        .clicked()
                                {
                                    toggle_app_id = Some(app.id.clone());
                                }
                            });

                            if art_resp.clicked() {
                                if let Some(first_prof) =
                                    store.profiles.iter().find(|p| p.app_id == app.id)
                                {
                                    activated_profile = Some(first_prof.id.clone());
                                }
                            }
                        });
                }
            });

            if let Some(aid) = toggle_app_id {
                if let Some(a) = store.apps.iter_mut().find(|x| x.id == aid) {
                    a.enabled = !a.enabled;
                    store.save();
                }
            }
            if let Some(aid) = add_subprofile_for {
                let count = store.profiles.iter().filter(|p| p.app_id == aid).count() + 1;
                let mut np = store.active_profile().clone();
                np.id = format!("prof-{aid}-{count}");
                np.app_id = aid;
                np.name = format!("Profil {count}");
                np.locked = false;
                let nid = np.id.clone();
                store.profiles.push(np);
                store.save();
                activated_profile = Some(nid);
            }
        });
    });

    if *adding_app_modal {
        egui::Window::new(lang.pick("Oyun veya Uygulama Ekle", "Add Game or Application"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_min_width(340.0);
                ui.label(lang.pick("Uygulama / Oyun Adı:", "Application / Game Name:"));
                ui.text_edit_singleline(new_app_name);
                ui.add_space(6.0);
                ui.label(lang.pick(
                    "Çalıştırılabilir Dosya Adı (ör. cs2.exe, valorant.exe, firefox):",
                    "Executable Name (e.g. cs2.exe, firefox):",
                ));
                ui.text_edit_singleline(new_app_exe);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(lang.pick("İptal", "Cancel")).clicked() {
                        *adding_app_modal = false;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(lang.pick("EKLE", "ADD"))
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(theme::GHUB_BLUE),
                        )
                        .clicked()
                        && !new_app_name.trim().is_empty()
                    {
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(2);
                        let app_id = format!("app-{ts}");
                        let exe = new_app_exe.trim().to_lowercase();
                        store.apps.push(AppEntry {
                            id: app_id.clone(),
                            name: new_app_name.trim().to_string(),
                            executables: if exe.is_empty() {
                                Vec::new()
                            } else {
                                vec![exe]
                            },
                            enabled: true,
                            accent_rgb: [0, 214, 143],
                            is_desktop: false,
                        });
                        let mut p = ProfileConfig::default_desktop();
                        p.id = format!("prof-{ts}");
                        p.app_id = app_id;
                        p.name = "Varsayılan".into();
                        p.locked = false;
                        let pid = p.id.clone();
                        store.profiles.push(p);
                        store.save();
                        new_app_name.clear();
                        new_app_exe.clear();
                        *adding_app_modal = false;
                        activated_profile = Some(pid);
                    }
                });
            });
    }

    activated_profile
}

pub fn render_device_settings_page(
    ui: &mut egui::Ui,
    store: &AppStore,
    summary: &DeviceSummary,
    startup_effect: bool,
    onboard_mode: OnboardMode,
    active_profile_name: &str,
) -> Option<DeviceSettingsEvent> {
    let lang = store.lang;
    let mut out = None;

    ui.horizontal(|ui| {
        let (back_r, back_resp) = ui.allocate_exact_size(Vec2::new(32.0, 28.0), Sense::click());
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
            out = Some(DeviceSettingsEvent::Back);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            ui.add_space(90.0);
            ui.vertical(|ui| {
                ui.add_space(28.0);
                let fw_ver = summary
                    .fw_info
                    .as_ref()
                    .map(|f| f.ghub_firmware_string())
                    .unwrap_or_else(|| "152.0.10".into());
                ui.label(
                    RichText::new(lang.pick("FABRİKA YAZILIMI", "FIRMWARE"))
                        .size(11.0)
                        .color(theme::TEXT_LIGHT),
                );
                ui.label(
                    RichText::new(format!("{} : {fw_ver}", summary.name.to_uppercase()))
                        .size(11.5)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(lang.pick("FABRİKA YAZILIMI GÜNCEL", "FIRMWARE IS UP TO DATE"))
                        .size(11.0)
                        .color(theme::TEXT_LIGHT),
                );
            });
        });
    });

    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(summary.name.to_uppercase())
                .size(34.0)
                .strong()
                .color(Color32::from_rgb(120, 123, 130)),
        );
        let _ = canvas::draw_ghub_mouse_stage(
            ui,
            Vec2::new(240.0, 230.0),
            [Color32::from_rgb(150, 154, 162); 3],
            true,
            None,
            0,
            &[],
            false,
            false,
            false,
            None,
        );
    });

    ui.add_space(28.0);

    let total_w = ui.available_width();
    ui.horizontal_top(|ui| {
        ui.add_space((total_w * 0.22).max(40.0));
        ui.allocate_ui(Vec2::new(280.0, 160.0), |ui| {
            ui.label(
                RichText::new(lang.pick("CİHAZ BAŞLATMA EFEKTİ", "DEVICE STARTUP EFFECT"))
                    .size(11.5)
                    .color(theme::TEXT_LIGHT),
            );
            ui.add_space(6.0);
            let mut se = startup_effect;
            if ui
                .checkbox(
                    &mut se,
                    RichText::new(lang.pick("ETKİNLEŞTİR", "ENABLE"))
                        .size(11.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .changed()
            {
                out = Some(DeviceSettingsEvent::ToggleStartupEffect(se));
            }
        });

        ui.add_space((total_w * 0.14).max(32.0));

        ui.allocate_ui(Vec2::new(340.0, 180.0), |ui| {
            let is_ob = onboard_mode == OnboardMode::Onboard;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.pick("YERLEŞİK BELLEK MODU", "ONBOARD MEMORY MODE"))
                        .size(11.5)
                        .color(theme::TEXT_LIGHT),
                );
                ui.add_space(24.0);
                let pill_txt = if is_ob {
                    lang.pick("AÇIK", "ON")
                } else {
                    lang.pick("KAPALI", "OFF")
                };
                let btn = egui::Button::new(RichText::new(pill_txt).size(10.5).strong().color(
                    if is_ob {
                        theme::GHUB_BLUE
                    } else {
                        Color32::WHITE
                    },
                ))
                .fill(theme::BG_BLACK)
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_ob {
                        theme::GHUB_BLUE
                    } else {
                        theme::TEXT_LIGHT
                    },
                ))
                .corner_radius(CornerRadius::same(11));
                if ui.add(btn).clicked() {
                    out = Some(DeviceSettingsEvent::ToggleOnboardMode(if is_ob {
                        OnboardMode::Host
                    } else {
                        OnboardMode::Onboard
                    }));
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").size(10.0).color(if is_ob {
                    theme::GHUB_BLUE
                } else {
                    theme::TEXT_DIM
                }));
                ui.label(
                    RichText::new(lang.pick("1. YUVA", "SLOT 1"))
                        .size(11.5)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(active_profile_name.to_uppercase())
                                .size(11.5)
                                .strong()
                                .color(theme::GHUB_BLUE),
                        )
                        .frame(false),
                    )
                    .on_hover_text(lang.pick(
                        "Geçerli profili farenin yerleşik belleğine (Yuva 1) yazmak için tıklayın",
                        "Click to flash active profile into Slot 1",
                    ))
                    .clicked()
                {
                    out = Some(DeviceSettingsEvent::FlashSlot1);
                }
            });

            ui.add_space(12.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(lang.pick(
                            "TÜM PROFİL YUVALARINI VARSAYILANA\nGERİ YÜKLE",
                            "RESTORE ALL PROFILE SLOTS TO DEFAULT",
                        ))
                        .size(11.0)
                        .strong()
                        .color(theme::TEXT_MUTED),
                    )
                    .frame(false),
                )
                .clicked()
            {
                out = Some(DeviceSettingsEvent::RestoreDefaults);
            }
        });
    });

    out
}
