use std::f32::consts::{PI, TAU};

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use logitux_hidpp::features::rgb::{RgbColor, RgbEffect, WaveDirection};

use crate::i18n::Lang;
use crate::theme;

pub fn compute_preview_zones(
    time_sec: f64,
    mode_tag: u8,
    perkey_colors: [RgbColor; 3],
    anim_frame: Option<[[u8; 3]; 3]>,
    effect: RgbEffect,
) -> [Color32; 3] {
    if mode_tag == 1 {
        return perkey_colors.map(|c| Color32::from_rgb(c.r, c.g, c.b));
    }
    if mode_tag == 2 {
        if let Some(f) = anim_frame {
            return f.map(|rgb| Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        }
    }
    match effect {
        RgbEffect::Off => [Color32::from_rgb(42, 44, 48); 3],
        RgbEffect::Fixed { color, brightness } => {
            let c = color.scaled(brightness);
            [Color32::from_rgb(c.r, c.g, c.b); 3]
        }
        RgbEffect::Breathing {
            color,
            period_ms,
            brightness,
        } => {
            let p = (period_ms.max(400) as f64) / 1000.0;
            let wave = (((time_sec / p) * (TAU as f64)).sin() * 0.5 + 0.5) as f32;
            let scale = (brightness as f32 / 100.0) * (0.12 + 0.88 * wave);
            let c = Color32::from_rgb(
                (color.r as f32 * scale) as u8,
                (color.g as f32 * scale) as u8,
                (color.b as f32 * scale) as u8,
            );
            [c; 3]
        }
        RgbEffect::ColorCycle {
            period_ms,
            brightness,
        }
        | RgbEffect::ColorBlend {
            period_ms,
            brightness,
        } => {
            let p = (period_ms.max(500) as f64) / 1000.0;
            let hue = ((time_sec / p) % 1.0) as f32;
            let c = hsv_to_color32(hue, 0.88, brightness as f32 / 100.0);
            [c; 3]
        }
        RgbEffect::ColorWave {
            period_ms,
            direction,
            brightness,
        } => {
            let p = (period_ms.max(500) as f64) / 1000.0;
            let base = ((time_sec / p) % 1.0) as f32;
            let dir_sign = match direction {
                WaveDirection::ReverseHorizontal => -1.0,
                _ => 1.0,
            };
            let v = brightness as f32 / 100.0;
            [
                hsv_to_color32((base + 0.00 * dir_sign).rem_euclid(1.0), 0.88, v),
                hsv_to_color32((base + 0.18 * dir_sign).rem_euclid(1.0), 0.88, v),
                hsv_to_color32((base + 0.36 * dir_sign).rem_euclid(1.0), 0.88, v),
            ]
        }
    }
}

fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
    let h6 = h * 6.0;
    let i = h6.floor() as i32;
    let f = h6 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

pub enum FullDpiEvent {
    SelectStage(usize),
    DragStage {
        index: usize,
        dpi: u16,
        committed: bool,
    },
    DeleteStage(usize),
    AddStage(u16),
    SetShiftStage(usize),
    StartInlineEdit(usize),
}

fn ghub_dpi_to_t(dpi: u16, max_dpi: u16) -> f32 {
    if max_dpi > 8000 {
        return ((dpi as f32 - 100.0) / (max_dpi as f32 - 100.0)).clamp(0.0, 1.0);
    }
    let d = (dpi as f32).clamp(200.0, 8000.0);
    let knots: [(f32, f32); 7] = [
        (200.0, 0.0),
        (1000.0, 1.0 / 6.0),
        (2000.0, 2.0 / 6.0),
        (3000.0, 3.0 / 6.0),
        (4000.0, 4.0 / 6.0),
        (6000.0, 5.0 / 6.0),
        (8000.0, 1.0),
    ];
    for w in knots.windows(2) {
        if d <= w[1].0 {
            let f = (d - w[0].0) / (w[1].0 - w[0].0);
            return w[0].1 + f * (w[1].1 - w[0].1);
        }
    }
    1.0
}

fn ghub_t_to_dpi(t: f32, min_dpi: u16, max_dpi: u16, step: u16) -> u16 {
    let t = t.clamp(0.0, 1.0);
    let raw = if max_dpi > 8000 {
        min_dpi as f32 + t * (max_dpi - min_dpi) as f32
    } else {
        let knots: [(f32, f32); 7] = [
            (200.0, 0.0),
            (1000.0, 1.0 / 6.0),
            (2000.0, 2.0 / 6.0),
            (3000.0, 3.0 / 6.0),
            (4000.0, 4.0 / 6.0),
            (6000.0, 5.0 / 6.0),
            (8000.0, 1.0),
        ];
        let mut val = 8000.0;
        for w in knots.windows(2) {
            if t <= w[1].1 {
                let f = (t - w[0].1) / (w[1].1 - w[0].1);
                val = w[0].0 + f * (w[1].0 - w[0].0);
                break;
            }
        }
        val
    };
    let s = step.max(1) as f32;
    let snapped = ((raw / s).round() * s) as u16;
    snapped.clamp(min_dpi, max_dpi)
}

#[allow(clippy::too_many_arguments)]
pub fn draw_ghub_full_dpi_stage(
    ui: &mut egui::Ui,
    lang: Lang,
    dpi_min: u16,
    dpi_max: u16,
    dpi_step: u16,
    stages: &[u16; 5],
    active_idx: usize,
    shift_idx: usize,
    editing_stage: Option<usize>,
    edit_buffer: &mut String,
) -> (Option<FullDpiEvent>, Option<(usize, u16)>) {
    let mut out_event: Option<FullDpiEvent> = None;
    let mut committed_inline: Option<(usize, u16)> = None;

    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
    let painter = ui.painter_at(rect);

    let cx = rect.center().x;
    let top = rect.top() + (avail.y * 0.08).clamp(24.0, 64.0);

    painter.text(
        Pos2::new(cx, top),
        egui::Align2::CENTER_CENTER,
        lang.pick("DPI HIZLARI", "DPI SPEEDS"),
        egui::FontId::proportional(21.0),
        theme::TEXT_WHITE,
    );

    let leg_y = top + 46.0;
    let leg_x = cx - 72.0;
    draw_diamond(&painter, Pos2::new(leg_x, leg_y), 7.5, theme::DPI_AMBER);
    painter.text(
        Pos2::new(leg_x + 16.0, leg_y),
        egui::Align2::LEFT_CENTER,
        lang.pick("DPI KAYDIRMA HIZI", "DPI SHIFT SPEED"),
        egui::FontId::proportional(11.5),
        theme::TEXT_WHITE,
    );

    painter.text(
        Pos2::new(cx, leg_y + 34.0),
        egui::Align2::CENTER_CENTER,
        lang.pick(
            "Kendiniz değiştirmek için DPI hız numarasına tıklayın.",
            "Click a DPI speed number to type a custom value.",
        ),
        egui::FontId::proportional(12.0),
        theme::TEXT_LIGHT,
    );
    painter.text(
        Pos2::new(cx, leg_y + 56.0),
        egui::Align2::CENTER_CENTER,
        lang.pick(
            "Yeni bir DPI hızı oluşturmak için kaydırıcıda herhangi bir yere tıklayın.",
            "Click anywhere on the slider to create a new DPI speed.",
        ),
        egui::FontId::proportional(12.0),
        theme::TEXT_LIGHT,
    );

    let margin = (avail.x * 0.09).clamp(42.0, 110.0);
    let left_x = rect.left() + margin;
    let right_x = rect.right() - margin;
    let track_w = (right_x - left_x).max(160.0);
    let track_y = rect.center().y + 18.0;

    painter.line_segment(
        [Pos2::new(left_x, track_y), Pos2::new(right_x, track_y)],
        Stroke::new(2.0_f32, Color32::from_rgb(76, 80, 88)),
    );

    let major_labels = if dpi_max <= 8000 {
        [200u16, 1000, 2000, 3000, 4000, 6000, 8000]
    } else {
        [
            dpi_min,
            dpi_max / 6,
            dpi_max * 2 / 6,
            dpi_max * 3 / 6,
            dpi_max * 4 / 6,
            dpi_max * 5 / 6,
            dpi_max,
        ]
    };

    for tick in 0..=12 {
        let frac = tick as f32 / 12.0;
        let tx = left_x + frac * track_w;
        let half_h = if tick % 2 == 0 { 5.5 } else { 3.5 };
        painter.line_segment(
            [
                Pos2::new(tx, track_y - half_h),
                Pos2::new(tx, track_y + half_h),
            ],
            Stroke::new(1.2_f32, Color32::from_rgb(104, 108, 118)),
        );
        if tick % 2 == 0 {
            let lbl = major_labels[tick / 2];
            painter.text(
                Pos2::new(tx, track_y + 28.0),
                egui::Align2::CENTER_CENTER,
                format!("{lbl}"),
                egui::FontId::proportional(11.5),
                theme::TEXT_MUTED,
            );
        }
    }

    let track_hit = Rect::from_min_max(
        Pos2::new(left_x, track_y - 11.0),
        Pos2::new(right_x, track_y + 11.0),
    );
    let track_resp = ui.interact(track_hit, ui.id().with("ghub_dpi_add_line"), Sense::click());

    let enabled_count = stages.iter().filter(|&&d| d > 0).count();
    let mut clicked_any_node = false;

    for (idx, &dpi) in stages.iter().enumerate() {
        if dpi == 0 {
            continue;
        }
        let t = ghub_dpi_to_t(dpi, dpi_max);
        let nx = left_x + t * track_w;
        let node_pos = Pos2::new(nx, track_y);
        let is_active = idx == active_idx;
        let is_shift = idx == shift_idx;

        let node_hit = Rect::from_center_size(node_pos, Vec2::new(24.0, 26.0));
        let resp = ui.interact(
            node_hit,
            ui.id().with(("ghub_dpi_thumb", idx)),
            Sense::click_and_drag(),
        );

        if resp.clicked() {
            clicked_any_node = true;
            out_event = Some(FullDpiEvent::SelectStage(idx));
        } else if resp.dragged() {
            clicked_any_node = true;
            if let Some(ptr) = resp.interact_pointer_pos() {
                if (ptr.y - track_y).abs() > 48.0 && enabled_count > 1 {
                } else {
                    let nt = ((ptr.x - left_x) / track_w).clamp(0.0, 1.0);
                    let ndpi = ghub_t_to_dpi(nt, dpi_min, dpi_max, dpi_step);
                    out_event = Some(FullDpiEvent::DragStage {
                        index: idx,
                        dpi: ndpi,
                        committed: false,
                    });
                }
            }
        } else if resp.drag_stopped() {
            clicked_any_node = true;
            if let Some(ptr) = resp.interact_pointer_pos() {
                if (ptr.y - track_y).abs() > 48.0 && enabled_count > 1 {
                    out_event = Some(FullDpiEvent::DeleteStage(idx));
                } else {
                    let nt = ((ptr.x - left_x) / track_w).clamp(0.0, 1.0);
                    let ndpi = ghub_t_to_dpi(nt, dpi_min, dpi_max, dpi_step);
                    out_event = Some(FullDpiEvent::DragStage {
                        index: idx,
                        dpi: ndpi,
                        committed: true,
                    });
                }
            }
        }

        if is_shift {
            draw_diamond(
                &painter,
                node_pos,
                if is_active || resp.hovered() {
                    8.5
                } else {
                    7.2
                },
                theme::DPI_AMBER,
            );
        } else {
            painter.circle_filled(
                node_pos,
                if is_active || resp.hovered() {
                    7.0
                } else {
                    5.8
                },
                Color32::WHITE,
            );
        }

        let num_y = if is_shift {
            track_y - 36.0
        } else if is_active {
            track_y - 22.0
        } else {
            track_y - 32.0
        };
        let num_rect = Rect::from_center_size(Pos2::new(nx, num_y), Vec2::new(56.0, 22.0));

        if editing_stage == Some(idx) {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(num_rect));
            let te = child.add(
                egui::TextEdit::singleline(edit_buffer)
                    .desired_width(52.0)
                    .font(egui::FontId::proportional(14.0)),
            );
            te.request_focus();
            if te.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Ok(parsed) = edit_buffer.trim().parse::<u16>() {
                    committed_inline = Some((idx, parsed.clamp(dpi_min, dpi_max)));
                } else {
                    committed_inline = Some((idx, dpi));
                }
            }
        } else {
            let num_resp = ui.interact(
                num_rect,
                ui.id().with(("ghub_dpi_num", idx)),
                Sense::click(),
            );
            if num_resp.clicked() {
                clicked_any_node = true;
                out_event = Some(FullDpiEvent::StartInlineEdit(idx));
            }
            let num_col = if is_shift {
                theme::DPI_AMBER
            } else {
                Color32::WHITE
            };
            let galley =
                painter.layout_no_wrap(format!("{dpi}"), egui::FontId::proportional(15.5), num_col);
            let gpos = Pos2::new(nx - galley.size().x * 0.5, num_y - galley.size().y * 0.5);
            painter.galley(gpos, galley.clone(), num_col);
            if is_active {
                painter.line_segment(
                    [
                        Pos2::new(gpos.x, gpos.y + galley.size().y + 1.5),
                        Pos2::new(gpos.x + galley.size().x, gpos.y + galley.size().y + 1.5),
                    ],
                    Stroke::new(1.6_f32, num_col),
                );
            }
        }
    }

    if track_resp.clicked() && !clicked_any_node && enabled_count < 5 {
        if let Some(ptr) = track_resp.interact_pointer_pos() {
            let nt = ((ptr.x - left_x) / track_w).clamp(0.0, 1.0);
            let ndpi = ghub_t_to_dpi(nt, dpi_min, dpi_max, dpi_step);
            out_event = Some(FullDpiEvent::AddStage(ndpi));
        }
    }

    let bot_y = (track_y + 118.0).min(rect.bottom() - 48.0);
    painter.text(
        Pos2::new(cx, bot_y),
        egui::Align2::CENTER_CENTER,
        lang.pick(
            "DPI hızını silmek için kaydırıcının dışına sürükleyin.",
            "Drag a DPI speed off the slider to delete it.",
        ),
        egui::FontId::proportional(12.0),
        theme::TEXT_LIGHT,
    );

    let shift_txt = lang.pick(
        "DPI Kaydırma hızını değiştirmek için buna tıklayın:",
        "Click this to set active stage as DPI Shift speed:",
    );
    let g_shift = painter.layout_no_wrap(
        shift_txt.to_string(),
        egui::FontId::proportional(12.0),
        theme::DPI_AMBER,
    );
    let row_y = bot_y + 28.0;
    let total_w = g_shift.size().x + 24.0;
    let start_x = cx - total_w * 0.5;
    painter.galley(
        Pos2::new(start_x, row_y - g_shift.size().y * 0.5),
        g_shift.clone(),
        theme::DPI_AMBER,
    );
    let dia_center = Pos2::new(start_x + g_shift.size().x + 14.0, row_y);
    draw_diamond(&painter, dia_center, 8.0, theme::DPI_AMBER);
    let dia_hit = Rect::from_center_size(Pos2::new(cx, row_y), Vec2::new(total_w + 16.0, 24.0));
    if ui
        .interact(dia_hit, ui.id().with("ghub_set_shift_btn"), Sense::click())
        .clicked()
    {
        out_event = Some(FullDpiEvent::SetShiftStage(active_idx));
    }

    (out_event, committed_inline)
}

fn draw_diamond(painter: &egui::Painter, center: Pos2, r: f32, color: Color32) {
    let pts = vec![
        Pos2::new(center.x, center.y - r),
        Pos2::new(center.x + r, center.y),
        Pos2::new(center.x, center.y + r),
        Pos2::new(center.x - r, center.y),
    ];
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

pub struct MouseStageOutput {
    pub clicked_button: Option<usize>,
    pub dropped_on_button: Option<usize>,
    pub clicked_zone: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_ghub_mouse_stage(
    ui: &mut egui::Ui,
    size: Vec2,
    zone_colors: [Color32; 3],
    grayscale_mode: bool,
    selected_button: Option<usize>,
    pressed_mask: u16,
    button_labels: &[(String, bool)],
    show_callouts: bool,
    dragging_assignment: bool,
    interactive_zones: bool,
    selected_zone: Option<usize>,
) -> MouseStageOutput {
    let sense = if show_callouts || dragging_assignment || interactive_zones {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let painter = ui.painter_at(rect);

    let center = Pos2::new(rect.center().x, rect.center().y);
    let mh = (size.y * 0.56).clamp(150.0, 345.0);
    let mw = mh * 0.525;
    let top_y = center.y - mh * 0.5;
    let bot_y = center.y + mh * 0.5;

    let eff_zones = if grayscale_mode {
        [Color32::from_rgb(135, 138, 144); 3]
    } else {
        zone_colors
    };

    for seg in 0..8 {
        let t0 = seg as f32 / 8.0;
        let t1 = (seg + 1) as f32 / 8.0;
        let y0 = top_y - 36.0 * (1.0 - t0);
        let y1 = top_y - 36.0 * (1.0 - t1);
        let alpha = (t1 * 180.0) as u8;
        painter.line_segment(
            [Pos2::new(center.x, y0), Pos2::new(center.x, y1)],
            Stroke::new(4.2_f32, Color32::from_rgba_unmultiplied(38, 40, 45, alpha)),
        );
    }

    let contour = build_g102_contour(center, mw, mh, 64);
    painter.add(Shape::convex_polygon(
        contour.clone(),
        Color32::from_rgb(23, 24, 27),
        Stroke::new(1.4_f32, Color32::from_rgb(46, 48, 54)),
    ));

    for (scale, col) in [
        (0.88_f32, Color32::from_rgba_unmultiplied(34, 36, 40, 140)),
        (0.72_f32, Color32::from_rgba_unmultiplied(44, 46, 52, 95)),
        (0.50_f32, Color32::from_rgba_unmultiplied(54, 57, 64, 55)),
    ] {
        let inner = build_g102_contour(
            Pos2::new(center.x, center.y - mh * 0.03),
            mw * scale,
            mh * scale,
            48,
        );
        painter.add(Shape::convex_polygon(inner, col, Stroke::NONE));
    }

    let split_y = top_y + mh * 0.375;
    let seam_pts = 16usize;
    for i in 0..seam_pts {
        let t0 = i as f32 / seam_pts as f32;
        let t1 = (i + 1) as f32 / seam_pts as f32;
        let x0 = center.x - mw * 0.475 + t0 * mw * 0.95;
        let x1 = center.x - mw * 0.475 + t1 * mw * 0.95;
        let arch0 = (t0 - 0.5).powi(2) * mh * 0.07;
        let arch1 = (t1 - 0.5).powi(2) * mh * 0.07;
        painter.line_segment(
            [
                Pos2::new(x0, split_y + arch0),
                Pos2::new(x1, split_y + arch1),
            ],
            Stroke::new(2.0_f32, Color32::from_rgb(8, 9, 11)),
        );
    }
    painter.line_segment(
        [
            Pos2::new(center.x, top_y + 2.0),
            Pos2::new(center.x, split_y),
        ],
        Stroke::new(2.2_f32, Color32::from_rgb(8, 9, 11)),
    );

    let spine = Rect::from_center_size(
        Pos2::new(center.x, top_y + mh * 0.22),
        Vec2::new(mw * 0.165, mh * 0.38),
    );
    painter.rect_filled(spine, CornerRadius::same(9), Color32::from_rgb(13, 14, 16));
    painter.rect_stroke(
        spine,
        CornerRadius::same(9),
        Stroke::new(1.0_f32, Color32::from_rgb(52, 55, 62)),
        egui::StrokeKind::Inside,
    );

    let wheel_rect = Rect::from_center_size(
        Pos2::new(center.x, top_y + mh * 0.195),
        Vec2::new(mw * 0.115, mh * 0.155),
    );
    painter.rect_filled(
        wheel_rect,
        CornerRadius::same(5),
        Color32::from_rgb(26, 28, 32),
    );
    for i in 0..8 {
        let ry = wheel_rect.top() + 4.0 + (i as f32) * ((wheel_rect.height() - 8.0) / 7.0);
        painter.line_segment(
            [
                Pos2::new(wheel_rect.left() + 2.0, ry),
                Pos2::new(wheel_rect.right() - 2.0, ry),
            ],
            Stroke::new(1.2_f32, Color32::from_rgb(12, 13, 15)),
        );
    }

    let dpi_btn = Rect::from_center_size(
        Pos2::new(center.x, top_y + mh * 0.345),
        Vec2::new(mw * 0.085, mh * 0.068),
    );
    painter.rect_filled(
        dpi_btn,
        CornerRadius::same(5),
        Color32::from_rgb(28, 30, 35),
    );
    painter.rect_stroke(
        dpi_btn,
        CornerRadius::same(5),
        Stroke::new(1.2_f32, Color32::from_rgb(76, 80, 90)),
        egui::StrokeKind::Inside,
    );

    let flank_x = center.x - mw * 0.485;
    let g5_rect = Rect::from_center_size(
        Pos2::new(flank_x, top_y + mh * 0.43),
        Vec2::new(6.0, mh * 0.105),
    );
    let g4_rect = Rect::from_center_size(
        Pos2::new(flank_x + 1.5, top_y + mh * 0.56),
        Vec2::new(6.0, mh * 0.105),
    );
    for r in [g5_rect, g4_rect] {
        painter.rect_filled(r, CornerRadius::same(3), Color32::from_rgb(34, 36, 42));
    }

    let arc_steps = 48usize;
    let rx = mw * 0.462;
    let ry = mh * 0.362;
    let arc_cy = bot_y - ry - mh * 0.012;

    if !grayscale_mode {
        let mut glow_mesh = egui::Mesh::default();
        let half_glow = 4.5_f32;
        for i in 0..=arc_steps {
            let t = i as f32 / arc_steps as f32;
            let angle = PI - 0.16 - t * (PI - 0.32);
            let ca = angle.cos();
            let sa = angle.sin();
            let center_pt = Pos2::new(center.x + rx * ca, arc_cy + ry * sa);
            let n = Vec2::new(ca / rx, sa / ry).normalized();
            let col = if t < 0.5 {
                lerp_color(eff_zones[0], eff_zones[1], t * 2.0)
            } else {
                lerp_color(eff_zones[1], eff_zones[2], (t - 0.5) * 2.0)
            };
            let gcol = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 36);
            glow_mesh.colored_vertex(center_pt - n * half_glow, gcol);
            glow_mesh.colored_vertex(center_pt + n * half_glow, gcol);
            if i < arc_steps {
                let base = (i * 2) as u32;
                glow_mesh.add_triangle(base, base + 1, base + 2);
                glow_mesh.add_triangle(base + 1, base + 3, base + 2);
            }
        }
        painter.add(Shape::mesh(glow_mesh));
    }

    let mut core_mesh = egui::Mesh::default();
    let half_core = 1.85_f32;
    for i in 0..=arc_steps {
        let t = i as f32 / arc_steps as f32;
        let angle = PI - 0.16 - t * (PI - 0.32);
        let ca = angle.cos();
        let sa = angle.sin();
        let center_pt = Pos2::new(center.x + rx * ca, arc_cy + ry * sa);
        let n = Vec2::new(ca / rx, sa / ry).normalized();
        let col = if t < 0.5 {
            lerp_color(eff_zones[0], eff_zones[1], t * 2.0)
        } else {
            lerp_color(eff_zones[1], eff_zones[2], (t - 0.5) * 2.0)
        };
        core_mesh.colored_vertex(center_pt - n * half_core, col);
        core_mesh.colored_vertex(center_pt + n * half_core, col);
        if i < arc_steps {
            let base = (i * 2) as u32;
            core_mesh.add_triangle(base, base + 1, base + 2);
            core_mesh.add_triangle(base + 1, base + 3, base + 2);
        }
    }
    painter.add(Shape::mesh(core_mesh));

    let emblem_pos = Pos2::new(center.x, bot_y - mh * 0.165);
    let c_mid = eff_zones[1];
    if !grayscale_mode {
        painter.circle_filled(
            emblem_pos,
            mw * 0.13,
            Color32::from_rgba_unmultiplied(c_mid.r(), c_mid.g(), c_mid.b(), 30),
        );
    }
    draw_ghub_emblem(&painter, emblem_pos, mw * 0.082, c_mid);

    let mut out = MouseStageOutput {
        clicked_button: None,
        dropped_on_button: None,
        clicked_zone: None,
    };

    let ptr_pos = ui.input(|i| i.pointer.interact_pos());
    let released = ui.input(|i| i.pointer.any_released());
    let click_pos = response
        .clicked()
        .then(|| response.interact_pointer_pos())
        .flatten();

    if interactive_zones {
        let anchors = [
            (
                0usize,
                "1 · SOL",
                Pos2::new(center.x - mw * 0.58, bot_y - mh * 0.18),
            ),
            (1usize, "2 · LOGO / ALT", Pos2::new(center.x, bot_y + 22.0)),
            (
                2usize,
                "3 · SAĞ",
                Pos2::new(center.x + mw * 0.58, bot_y - mh * 0.18),
            ),
        ];
        for (zi, lbl, pos) in anchors {
            let is_sel = selected_zone == Some(zi);
            let pill = Rect::from_center_size(pos, Vec2::new(98.0, 22.0));
            painter.rect_filled(
                pill,
                CornerRadius::same(11),
                if is_sel {
                    theme::GHUB_BLUE
                } else {
                    theme::BG_DRAWER
                },
            );
            painter.rect_stroke(
                pill,
                CornerRadius::same(11),
                Stroke::new(1.2_f32, zone_colors[zi]),
                egui::StrokeKind::Inside,
            );
            painter.text(
                pill.center(),
                egui::Align2::CENTER_CENTER,
                lbl,
                egui::FontId::proportional(10.5),
                Color32::WHITE,
            );
            if let Some(cp) = click_pos {
                if pill.expand(6.0).contains(cp) {
                    out.clicked_zone = Some(zi);
                }
            }
        }
    }

    if show_callouts && button_labels.len() >= 6 {
        let span_x = (mw * 1.38).min(size.x * 0.42);
        let callouts: [(usize, Pos2, Pos2, egui::Align2); 6] = [
            (
                0,
                Pos2::new(center.x - mw * 0.24, top_y + mh * 0.22),
                Pos2::new(center.x - span_x, top_y + mh * 0.22),
                egui::Align2::LEFT_BOTTOM,
            ),
            (
                1,
                Pos2::new(center.x + mw * 0.24, top_y + mh * 0.22),
                Pos2::new(center.x + span_x, top_y + mh * 0.22),
                egui::Align2::RIGHT_BOTTOM,
            ),
            (
                2,
                Pos2::new(center.x, wheel_rect.center().y),
                Pos2::new(center.x, (top_y - 42.0).max(rect.top() + 18.0)),
                egui::Align2::CENTER_BOTTOM,
            ),
            (
                4,
                Pos2::new(g5_rect.center().x, g5_rect.center().y),
                Pos2::new(center.x - span_x, g5_rect.center().y),
                egui::Align2::LEFT_BOTTOM,
            ),
            (
                3,
                Pos2::new(g4_rect.center().x, g4_rect.center().y),
                Pos2::new(center.x - span_x, g4_rect.center().y),
                egui::Align2::LEFT_BOTTOM,
            ),
            (
                5,
                Pos2::new(dpi_btn.center().x, dpi_btn.center().y),
                Pos2::new(center.x + span_x, dpi_btn.center().y),
                egui::Align2::RIGHT_BOTTOM,
            ),
        ];

        for (idx, pin, end, align) in callouts {
            let (ref txt, is_amber) = button_labels[idx];
            let is_sel = selected_button == Some(idx);
            let is_pressed = (pressed_mask & (1 << idx)) != 0;

            let hit_box = match align {
                egui::Align2::CENTER_BOTTOM => {
                    Rect::from_center_size(Pos2::new(end.x, end.y - 10.0), Vec2::new(130.0, 28.0))
                }
                egui::Align2::LEFT_BOTTOM => {
                    Rect::from_min_size(Pos2::new(end.x, end.y - 24.0), Vec2::new(140.0, 28.0))
                }
                _ => Rect::from_min_size(
                    Pos2::new(end.x - 140.0, end.y - 24.0),
                    Vec2::new(140.0, 28.0),
                ),
            };

            let hovered_drop = dragging_assignment
                && ptr_pos
                    .is_some_and(|p| hit_box.expand(8.0).contains(p) || p.distance(pin) < 18.0);

            let line_col = if is_pressed {
                theme::STATUS_GREEN
            } else if hovered_drop || is_sel {
                theme::GHUB_BLUE
            } else {
                Color32::from_rgb(92, 95, 102)
            };

            painter.line_segment(
                [pin, end],
                Stroke::new(
                    if is_sel || hovered_drop {
                        1.6_f32
                    } else {
                        1.0_f32
                    },
                    line_col,
                ),
            );

            painter.circle_filled(
                pin,
                7.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, if is_sel { 75 } else { 32 }),
            );
            painter.circle_filled(
                pin,
                4.4,
                if is_pressed {
                    theme::STATUS_GREEN
                } else if is_sel || hovered_drop {
                    theme::GHUB_BLUE
                } else {
                    Color32::WHITE
                },
            );

            let text_col = if is_pressed {
                theme::STATUS_GREEN
            } else if hovered_drop || is_sel {
                theme::GHUB_BLUE_HOVER
            } else if is_amber {
                theme::DPI_AMBER
            } else {
                Color32::WHITE
            };

            let text_pos = match align {
                egui::Align2::CENTER_BOTTOM => Pos2::new(end.x, end.y - 4.0),
                _ => Pos2::new(end.x, end.y - 4.0),
            };
            painter.text(
                text_pos,
                align,
                txt,
                egui::FontId::proportional(12.5),
                text_col,
            );

            if hovered_drop {
                painter.rect_stroke(
                    hit_box,
                    CornerRadius::same(4),
                    Stroke::new(1.5_f32, theme::GHUB_BLUE),
                    egui::StrokeKind::Outside,
                );
            }
            if hovered_drop && released {
                out.dropped_on_button = Some(idx);
            }
            if let Some(cp) = click_pos {
                if hit_box.contains(cp) || cp.distance(pin) < 16.0 {
                    out.clicked_button = Some(idx);
                }
            }
        }
    }

    out
}

fn build_g102_contour(center: Pos2, width: f32, height: f32, steps: usize) -> Vec<Pos2> {
    let rx = width * 0.5;
    let ry = height * 0.5;
    let mut pts = Vec::with_capacity(steps);
    for i in 0..steps {
        let a = (i as f32 / steps as f32) * TAU;
        let sin_a = a.sin();
        let cos_a = a.cos();
        let width_mod = 0.96 + 0.045 * sin_a - 0.025 * (2.0 * a).cos();
        let sx = cos_a.signum() * cos_a.abs().powf(0.84) * rx * width_mod;
        let sy = sin_a.signum() * sin_a.abs().powf(0.88) * ry;
        pts.push(Pos2::new(center.x + sx, center.y + sy));
    }
    pts
}

fn draw_ghub_emblem(painter: &egui::Painter, c: Pos2, r: f32, col: Color32) {
    let s = Stroke::new(2.8_f32, col);
    let segs = 20usize;
    for i in 0..segs {
        let a0 = 0.38 + (i as f32 / segs as f32) * (TAU - 0.80);
        let a1 = 0.38 + ((i + 1) as f32 / segs as f32) * (TAU - 0.80);
        painter.line_segment(
            [
                Pos2::new(c.x + r * a0.cos(), c.y + r * a0.sin()),
                Pos2::new(c.x + r * a1.cos(), c.y + r * a1.sin()),
            ],
            s,
        );
    }
    painter.line_segment([c, Pos2::new(c.x + r * 0.90, c.y)], s);
    painter.line_segment(
        [
            Pos2::new(c.x + r * 0.90, c.y),
            Pos2::new(c.x + r * 0.90, c.y + r * 0.50),
        ],
        s,
    );
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}
