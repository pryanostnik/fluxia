//! Вспомогательные функции отрисовки.

pub mod bar;
pub mod login;
pub mod settings;
pub mod sidebar;
pub mod tracklist;

use eframe::egui::{self, Color32, FontId};

pub fn blend(a: Color32, b: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

#[allow(dead_code)]
pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Обрезка длинного текста по ширине с многоточием.
pub fn fit(ui: &egui::Ui, text: &str, font: FontId, max_w: f32) -> String {
    if max_w <= 8.0 {
        return String::new();
    }
    let width = |s: &str| {
        ui.painter()
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    if width(text) <= max_w {
        return text.to_owned();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let mut candidate = out.clone();
        candidate.push(ch);
        candidate.push('…');
        if width(&candidate) > max_w {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// Тонкая горизонтальная линия.
pub fn hairline(ui: &egui::Ui, y: f32) {
    let rect = ui.max_rect();
    ui.painter()
        .hline(rect.x_range(), y, egui::Stroke::new(1.0, crate::theme::border()));
}

/// «1 трек», «2 трека», «5 треков»
pub fn plural_tracks(n: usize) -> &'static str {
    let n100 = n % 100;
    let n10 = n % 10;
    if (11..=14).contains(&n100) {
        "треков"
    } else if n10 == 1 {
        "трек"
    } else if (2..=4).contains(&n10) {
        "трека"
    } else {
        "треков"
    }
}

/// «48 мин», «2 ч 51 мин»
pub fn human_duration(total_secs: u64) -> String {
    let mins = total_secs / 60;
    if mins >= 60 {
        format!("{} ч {} мин", mins / 60, mins % 60)
    } else {
        format!("{} мин", mins.max(1))
    }
}

/// Кружок с инициалом (аватар без картинки).
pub fn avatar(ui: &egui::Ui, center: egui::Pos2, radius: f32, letter: char) {
    ui.painter()
        .circle_filled(center, radius, crate::theme::surface_hover());
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        letter.to_uppercase().to_string(),
        crate::theme::bold(radius * 0.9),
        crate::theme::text_dim(),
    );
}
