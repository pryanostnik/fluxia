#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod app;
mod config;
mod demo;
mod icons;
mod pipes;
mod player;
mod theme;
mod ui;
mod win;

/// Логотип, вшитый в бинарник.
pub const LOGO_BYTES: &[u8] = include_bytes!("../assets/logo.png");

/// Иконка окна/таскбара из того же логотипа.
fn window_icon() -> Option<eframe::egui::IconData> {
    let img = image::load_from_memory(LOGO_BYTES).ok()?.to_rgba8();
    let img = image::imageops::resize(&img, 256, 256, image::imageops::FilterType::Lanczos3);
    Some(eframe::egui::IconData {
        rgba: img.into_raw(),
        width: 256,
        height: 256,
    })
}

fn main() -> Result<(), eframe::Error> {
    if std::env::args().any(|a| a == "--config-info") {
        config::dump_info();
        return Ok(());
    }

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1120.0, 720.0])
        .with_min_inner_size([900.0, 560.0])
        .with_title("fluxia");

    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "fluxia",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
