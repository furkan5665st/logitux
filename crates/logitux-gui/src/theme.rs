use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, Vec2, Visuals};

pub const BG_BLACK: Color32 = Color32::from_rgb(0, 0, 0);
pub const BG_DRAWER: Color32 = Color32::from_rgb(27, 28, 32);
pub const BG_HOME_CARD: Color32 = Color32::from_rgb(32, 33, 36);
pub const BG_CONTROL: Color32 = Color32::from_rgb(42, 44, 50);
pub const BG_HOVER: Color32 = Color32::from_rgb(54, 57, 65);
pub const BG_MODAL: Color32 = Color32::from_rgb(22, 23, 27);

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(62, 65, 74);
pub const BORDER_DIM: Color32 = Color32::from_rgb(42, 44, 50);

pub const GHUB_BLUE: Color32 = Color32::from_rgb(0, 149, 255);
pub const GHUB_BLUE_HOVER: Color32 = Color32::from_rgb(28, 168, 255);
pub const GHUB_TEAL_SAVE: Color32 = Color32::from_rgb(0, 108, 148);

pub const DPI_AMBER: Color32 = Color32::from_rgb(255, 180, 0);
pub const STATUS_GREEN: Color32 = Color32::from_rgb(0, 214, 143);
pub const STATUS_RED: Color32 = Color32::from_rgb(235, 64, 76);
pub const DISCORD_BLURPLE: Color32 = Color32::from_rgb(88, 101, 242);

pub const TEXT_WHITE: Color32 = Color32::from_rgb(245, 246, 248);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(202, 206, 214);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(142, 146, 156);
pub const TEXT_DIM: Color32 = Color32::from_rgb(98, 102, 112);

pub fn apply_ghub_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_BLACK;
    visuals.window_fill = BG_MODAL;
    visuals.extreme_bg_color = Color32::from_rgb(14, 15, 18);
    visuals.faint_bg_color = BG_DRAWER;

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 149, 255, 55);
    visuals.selection.stroke = Stroke::new(1.5_f32, GHUB_BLUE);

    visuals.widgets.noninteractive.bg_fill = BG_DRAWER;
    visuals.widgets.noninteractive.weak_bg_fill = BG_DRAWER;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_DIM);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_WHITE);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = BG_CONTROL;
    visuals.widgets.inactive.weak_bg_fill = BG_CONTROL;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_DIM);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_WHITE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    visuals.widgets.active.bg_fill = GHUB_BLUE;
    visuals.widgets.active.weak_bg_fill = GHUB_BLUE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, GHUB_BLUE_HOVER);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(4);

    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.slider_trailing_fill = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.slider_width = 195.0;
    style.spacing.slider_rail_height = 2.0;
    ctx.set_style(style);
}

pub fn floating_drawer_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(BG_DRAWER)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(20, 20))
}

pub fn draw_speech_tooltip(
    ctx: &egui::Context,
    anchor: Rect,
    text: &str,
    white_style: bool,
    point_left: bool,
) {
    let layer_id = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("ghub_speech_tip"));
    let painter = ctx.layer_painter(layer_id);

    let bg = if white_style {
        Color32::WHITE
    } else {
        Color32::from_rgb(43, 44, 49)
    };
    let fg = if white_style {
        Color32::from_rgb(20, 21, 24)
    } else {
        Color32::WHITE
    };

    let galley = painter.layout(
        text.to_string(),
        egui::FontId::proportional(12.5),
        fg,
        250.0,
    );
    let pad = Vec2::new(14.0, 10.0);
    let box_size = galley.size() + pad * 2.0;

    let (box_rect, tri) = if point_left {
        let min = Pos2::new(anchor.right() + 10.0, anchor.center().y - box_size.y * 0.5);
        let r = Rect::from_min_size(min, box_size);
        let cy = anchor.center().y;
        let pts = vec![
            Pos2::new(r.left() - 7.0, cy),
            Pos2::new(r.left() + 1.0, cy - 6.5),
            Pos2::new(r.left() + 1.0, cy + 6.5),
        ];
        (r, pts)
    } else {
        let cx = anchor.center().x;
        let min = Pos2::new(cx - box_size.x * 0.5, anchor.top() - box_size.y - 10.0);
        let r = Rect::from_min_size(min, box_size);
        let pts = vec![
            Pos2::new(cx, r.bottom() + 7.0),
            Pos2::new(cx - 6.5, r.bottom() - 1.0),
            Pos2::new(cx + 6.5, r.bottom() - 1.0),
        ];
        (r, pts)
    };

    painter.rect_filled(
        box_rect.translate(Vec2::new(0.0, 3.0)),
        CornerRadius::same(6),
        Color32::from_rgba_unmultiplied(0, 0, 0, 120),
    );
    painter.rect_filled(box_rect, CornerRadius::same(6), bg);
    painter.add(Shape::convex_polygon(tri, bg, Stroke::NONE));
    painter.galley(box_rect.min + pad, galley, fg);
}

pub fn ghub_pill_switch(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(36.0, 18.0), egui::Sense::click());
    if resp.clicked() {
        *value = !*value;
        resp.mark_changed();
    }
    let painter = ui.painter_at(rect);
    let bg = if *value {
        GHUB_BLUE
    } else {
        Color32::from_rgb(92, 95, 104)
    };
    painter.rect_filled(rect, CornerRadius::same(9), bg);
    let knob_x = if *value {
        rect.right() - 9.0
    } else {
        rect.left() + 9.0
    };
    painter.circle_filled(Pos2::new(knob_x, rect.center().y), 6.5, Color32::WHITE);
    resp
}
