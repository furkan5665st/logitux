#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod hw_worker;
mod i18n;
mod i18n_data;
mod icons;
mod input_synth;
mod integrations;
mod macro_engine;
mod macro_ui;
mod store;
mod theme;
mod views_home;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon = icons::generate_window_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LogiTux")
            .with_app_id("io.github.furkan5665st.logitux")
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([980.0, 620.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "LogiTux",
        options,
        Box::new(|cc| {
            theme::apply_ghub_theme(&cc.egui_ctx);
            Ok(Box::new(app::LogiTuxApp::new(cc.egui_ctx.clone())))
        }),
    )
}
