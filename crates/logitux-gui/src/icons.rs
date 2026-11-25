use std::f32::consts::TAU;

use eframe::egui::{self, Color32, CornerRadius, IconData, Painter, Pos2, Rect, Stroke, Vec2};

pub fn generate_window_icon() -> IconData {
    IconData {
        rgba: include_bytes!("../../../assets/logitux-64.rgba").to_vec(),
        width: 64,
        height: 64,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhubIcon {
    LogoMark,
    BackArrow,
    SensitivityDpi,
    AssignmentsPlus,
    LightsyncSun,
    Gear,
    UserAvatar,
    ProfileLock,
    UsbPlug,
    OnboardChip { slashed: bool },
    GridView,
    ListView,
    MacroNoRepeat,
    MacroRepeatHold,
    MacroToggle,
    MacroSequence,
}

pub fn draw_icon(painter: &Painter, center: Pos2, r: f32, icon: GhubIcon, color: Color32) {
    let s = Stroke::new(1.65_f32, color);
    match icon {
        GhubIcon::LogoMark => {
            let bold = Stroke::new(2.8_f32, color);
            let segs = 18usize;
            for i in 0..segs {
                let a0 = 0.42 + (i as f32 / segs as f32) * (TAU - 0.85);
                let a1 = 0.42 + ((i + 1) as f32 / segs as f32) * (TAU - 0.85);
                painter.line_segment(
                    [
                        Pos2::new(center.x + r * a0.cos(), center.y + r * a0.sin()),
                        Pos2::new(center.x + r * a1.cos(), center.y + r * a1.sin()),
                    ],
                    bold,
                );
            }
            painter.line_segment([center, Pos2::new(center.x + r * 0.92, center.y)], bold);
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.92, center.y),
                    Pos2::new(center.x + r * 0.92, center.y + r * 0.52),
                ],
                bold,
            );
        }
        GhubIcon::BackArrow => {
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.85, center.y),
                    Pos2::new(center.x - r * 0.85, center.y),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.85, center.y),
                    Pos2::new(center.x - r * 0.25, center.y - r * 0.55),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.85, center.y),
                    Pos2::new(center.x - r * 0.25, center.y + r * 0.55),
                ],
                s,
            );
        }
        GhubIcon::SensitivityDpi => {
            painter.circle_filled(center, r * 0.18, color);
            let d = r * 0.92;
            let arm = r * 0.30;
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - d),
                    Pos2::new(center.x - arm, center.y - d + arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - d),
                    Pos2::new(center.x + arm, center.y - d + arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y + d),
                    Pos2::new(center.x - arm, center.y + d - arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y + d),
                    Pos2::new(center.x + arm, center.y + d - arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - d, center.y),
                    Pos2::new(center.x - d + arm, center.y - arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - d, center.y),
                    Pos2::new(center.x - d + arm, center.y + arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + d, center.y),
                    Pos2::new(center.x + d - arm, center.y - arm),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + d, center.y),
                    Pos2::new(center.x + d - arm, center.y + arm),
                ],
                s,
            );
        }
        GhubIcon::AssignmentsPlus => {
            let rect = Rect::from_center_size(center, Vec2::splat(r * 1.68));
            painter.rect_stroke(rect, CornerRadius::same(2), s, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.42, center.y),
                    Pos2::new(center.x + r * 0.42, center.y),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.42),
                    Pos2::new(center.x, center.y + r * 0.42),
                ],
                s,
            );
        }
        GhubIcon::LightsyncSun => {
            painter.circle_stroke(center, r * 0.36, s);
            for i in 0..8 {
                let a = (i as f32 / 8.0) * TAU;
                painter.circle_filled(
                    Pos2::new(center.x + a.cos() * r * 0.66, center.y + a.sin() * r * 0.66),
                    1.2,
                    color,
                );
                painter.circle_filled(
                    Pos2::new(center.x + a.cos() * r * 0.95, center.y + a.sin() * r * 0.95),
                    1.2,
                    color,
                );
            }
        }
        GhubIcon::Gear => {
            painter.circle_stroke(center, r * 0.36, s);
            painter.circle_stroke(center, r * 0.72, s);
            for i in 0..8 {
                let a = (i as f32 / 8.0) * TAU;
                painter.line_segment(
                    [
                        Pos2::new(center.x + a.cos() * r * 0.70, center.y + a.sin() * r * 0.70),
                        Pos2::new(center.x + a.cos() * r * 0.98, center.y + a.sin() * r * 0.98),
                    ],
                    Stroke::new(2.1_f32, color),
                );
            }
        }
        GhubIcon::UserAvatar => {
            painter.circle_stroke(Pos2::new(center.x, center.y - r * 0.32), r * 0.36, s);
            let arc_y = center.y + r * 0.72;
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.72, arc_y),
                    Pos2::new(center.x - r * 0.45, center.y + r * 0.18),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.45, center.y + r * 0.18),
                    Pos2::new(center.x + r * 0.45, center.y + r * 0.18),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.45, center.y + r * 0.18),
                    Pos2::new(center.x + r * 0.72, arc_y),
                ],
                s,
            );
        }
        GhubIcon::ProfileLock => {
            let rect = Rect::from_min_size(
                Pos2::new(center.x - r * 0.95, center.y - r * 0.68),
                Vec2::new(r * 1.55, r * 1.15),
            );
            painter.rect_stroke(rect, CornerRadius::same(2), s, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(rect.left() + 3.0, rect.top() + 4.0),
                    Pos2::new(rect.right() - 4.0, rect.top() + 4.0),
                ],
                Stroke::new(1.2_f32, color),
            );
            let lock_r = Rect::from_center_size(
                Pos2::new(center.x + r * 0.48, center.y + r * 0.42),
                Vec2::new(r * 0.75, r * 0.58),
            );
            painter.rect_filled(lock_r, CornerRadius::same(1), color);
        }
        GhubIcon::UsbPlug => {
            let head = Rect::from_center_size(
                Pos2::new(center.x, center.y + r * 0.12),
                Vec2::new(r * 1.15, r * 0.95),
            );
            painter.rect_stroke(head, CornerRadius::same(1), s, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(center.x, center.y - r * 0.85),
                    Pos2::new(center.x, head.top()),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(head.left() + 2.0, head.bottom() + 3.0),
                    Pos2::new(head.right() - 2.0, head.bottom() + 3.0),
                ],
                s,
            );
        }
        GhubIcon::OnboardChip { slashed } => {
            let rect = Rect::from_center_size(center, Vec2::splat(r * 1.15));
            painter.rect_stroke(rect, CornerRadius::same(2), s, egui::StrokeKind::Inside);
            for off in [-0.32_f32, 0.0, 0.32] {
                let d = off * r;
                painter.line_segment(
                    [
                        Pos2::new(rect.left() - r * 0.32, center.y + d),
                        Pos2::new(rect.left(), center.y + d),
                    ],
                    s,
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.right(), center.y + d),
                        Pos2::new(rect.right() + r * 0.32, center.y + d),
                    ],
                    s,
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x + d, rect.top() - r * 0.32),
                        Pos2::new(center.x + d, rect.top()),
                    ],
                    s,
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x + d, rect.bottom()),
                        Pos2::new(center.x + d, rect.bottom() + r * 0.32),
                    ],
                    s,
                );
            }
            if slashed {
                painter.line_segment(
                    [
                        Pos2::new(center.x - r * 0.98, center.y - r * 0.98),
                        Pos2::new(center.x + r * 0.98, center.y + r * 0.98),
                    ],
                    Stroke::new(2.0_f32, color),
                );
            }
        }
        GhubIcon::GridView => {
            let s2 = r * 0.65;
            let g = r * 0.20;
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let c = Pos2::new(
                    center.x + dx * (s2 * 0.5 + g),
                    center.y + dy * (s2 * 0.5 + g),
                );
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(s2)),
                    CornerRadius::same(1),
                    s,
                    egui::StrokeKind::Inside,
                );
            }
        }
        GhubIcon::ListView => {
            for dy in [-0.45_f32, 0.45] {
                let r_row = Rect::from_center_size(
                    Pos2::new(center.x, center.y + dy * r),
                    Vec2::new(r * 1.55, r * 0.55),
                );
                painter.rect_stroke(r_row, CornerRadius::same(1), s, egui::StrokeKind::Inside);
            }
        }
        GhubIcon::MacroNoRepeat => {
            painter.line_segment(
                [
                    Pos2::new(center.x - r * 0.75, center.y),
                    Pos2::new(center.x + r * 0.75, center.y),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.75, center.y),
                    Pos2::new(center.x + r * 0.22, center.y - r * 0.48),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.75, center.y),
                    Pos2::new(center.x + r * 0.22, center.y + r * 0.48),
                ],
                s,
            );
        }
        GhubIcon::MacroRepeatHold => {
            let segs = 14usize;
            for i in 0..segs {
                let a0 = -0.8 + (i as f32 / segs as f32) * (TAU * 0.75);
                let a1 = -0.8 + ((i + 1) as f32 / segs as f32) * (TAU * 0.75);
                painter.line_segment(
                    [
                        Pos2::new(
                            center.x + r * 0.72 * a0.cos(),
                            center.y + r * 0.72 * a0.sin(),
                        ),
                        Pos2::new(
                            center.x + r * 0.72 * a1.cos(),
                            center.y + r * 0.72 * a1.sin(),
                        ),
                    ],
                    s,
                );
            }
            let tip = Pos2::new(
                center.x + r * 0.72 * (-0.8_f32).cos(),
                center.y + r * 0.72 * (-0.8_f32).sin(),
            );
            painter.line_segment([tip, Pos2::new(tip.x - r * 0.36, tip.y - r * 0.10)], s);
            painter.line_segment([tip, Pos2::new(tip.x + r * 0.08, tip.y - r * 0.38)], s);
        }
        GhubIcon::MacroToggle => {
            let pill = Rect::from_center_size(center, Vec2::new(r * 1.75, r * 0.92));
            painter.rect_stroke(pill, CornerRadius::same(6), s, egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(center.x + r * 0.38, center.y), r * 0.28, color);
        }
        GhubIcon::MacroSequence => {
            let back = Rect::from_center_size(
                Pos2::new(center.x - r * 0.25, center.y - r * 0.15),
                Vec2::new(r * 1.05, r * 0.75),
            );
            let front = Rect::from_center_size(
                Pos2::new(center.x + r * 0.28, center.y + r * 0.18),
                Vec2::new(r * 1.05, r * 0.75),
            );
            painter.rect_filled(back, CornerRadius::same(2), color);
            painter.rect_stroke(front, CornerRadius::same(2), s, egui::StrokeKind::Inside);
        }
    }
}
