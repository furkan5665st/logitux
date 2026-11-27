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

#[cfg(target_os = "linux")]
fn prepare_linux_graphics_env() {
    let on_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    let has_nvidia = std::path::Path::new("/proc/driver/nvidia/version").exists();
    if on_wayland && has_nvidia && std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none() {
        std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
    }
}

#[cfg(target_os = "linux")]
fn try_linux_x11_relaunch() -> bool {
    let on_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    let has_x11 = std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty());
    let already_tried = std::env::var_os("LOGITUX_X11_FALLBACK").is_some();
    if !on_wayland || !has_x11 || already_tried {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    std::process::Command::new(exe)
        .args(args)
        .env("WAYLAND_DISPLAY", "")
        .env("LOGITUX_X11_FALLBACK", "1")
        .status()
        .is_ok_and(|s| s.success())
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "linux")]
    prepare_linux_graphics_env();

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

    let res = eframe::run_native(
        "LogiTux",
        options,
        Box::new(|cc| {
            theme::apply_ghub_theme(&cc.egui_ctx);
            Ok(Box::new(app::LogiTuxApp::new(cc.egui_ctx.clone())))
        }),
    );

    #[cfg(target_os = "linux")]
    if res.is_err() && try_linux_x11_relaunch() {
        return Ok(());
    }

    res
}
