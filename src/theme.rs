//! Вся визуальная часть в одном месте: палитра, типографика, размеры.
//! Палитра изменяемая: её правят настройки, поэтому цвета берутся функциями
//! (`t::accent()`), а не константами.

use std::sync::RwLock;

use eframe::egui::{self, Color32, FontFamily, FontId};

// ── палитра ──────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub accent: Color32,
    pub accent_deep: Color32,
    pub danger: Color32,
    pub on_accent: Color32,
}

impl Palette {
    /// Тёмная (по умолчанию).
    pub const fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(14, 14, 16),
            panel: Color32::from_rgb(17, 17, 20),
            surface: Color32::from_rgb(23, 23, 27),
            surface_hover: Color32::from_rgb(30, 30, 35),
            border: Color32::from_rgb(37, 37, 43),
            text: Color32::from_rgb(237, 237, 240),
            text_dim: Color32::from_rgb(138, 138, 147),
            text_faint: Color32::from_rgb(88, 88, 98),
            accent: Color32::from_rgb(255, 210, 74),
            accent_deep: Color32::from_rgb(32, 28, 12),
            danger: Color32::from_rgb(232, 92, 92),
            on_accent: Color32::from_rgb(18, 16, 8),
        }
    }

    /// Светлая.
    pub const fn light() -> Self {
        Self {
            bg: Color32::from_rgb(247, 247, 250),
            panel: Color32::from_rgb(255, 255, 255),
            surface: Color32::from_rgb(238, 238, 243),
            surface_hover: Color32::from_rgb(228, 228, 235),
            border: Color32::from_rgb(214, 214, 222),
            text: Color32::from_rgb(22, 22, 26),
            text_dim: Color32::from_rgb(94, 94, 104),
            text_faint: Color32::from_rgb(142, 142, 152),
            accent: Color32::from_rgb(226, 160, 10),
            accent_deep: Color32::from_rgb(250, 240, 210),
            danger: Color32::from_rgb(198, 60, 60),
            on_accent: Color32::from_rgb(255, 255, 255),
        }
    }

    /// Акцент меняем, сохраняя контраст подложки под ним.
    pub fn with_accent(mut self, accent: Color32) -> Self {
        self.accent = accent;
        self.accent_deep = Color32::from_rgba_unmultiplied(
            accent.r(),
            accent.g(),
            accent.b(),
            if self.is_dark() { 42 } else { 56 },
        );
        self.on_accent = if luminance(accent) > 0.55 {
            Color32::from_rgb(18, 16, 8)
        } else {
            Color32::from_rgb(250, 250, 250)
        };
        self
    }

    pub fn is_dark(&self) -> bool {
        luminance(self.bg) < 0.5
    }
}

fn luminance(c: Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

static PALETTE: RwLock<Palette> = RwLock::new(Palette::dark());

pub fn palette() -> Palette {
    *PALETTE.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set_palette(p: Palette) {
    *PALETTE.write().unwrap_or_else(|e| e.into_inner()) = p;
}

fn c(pick: impl Fn(&Palette) -> Color32) -> Color32 {
    pick(&PALETTE.read().unwrap_or_else(|e| e.into_inner()))
}

pub fn bg() -> Color32 {
    c(|p| p.bg)
}
pub fn panel() -> Color32 {
    c(|p| p.panel)
}
pub fn surface() -> Color32 {
    c(|p| p.surface)
}
pub fn surface_hover() -> Color32 {
    c(|p| p.surface_hover)
}
pub fn border() -> Color32 {
    c(|p| p.border)
}
pub fn text() -> Color32 {
    c(|p| p.text)
}
pub fn text_dim() -> Color32 {
    c(|p| p.text_dim)
}
pub fn text_faint() -> Color32 {
    c(|p| p.text_faint)
}
pub fn accent() -> Color32 {
    c(|p| p.accent)
}
pub fn accent_deep() -> Color32 {
    c(|p| p.accent_deep)
}
pub fn danger() -> Color32 {
    c(|p| p.danger)
}
pub fn on_accent() -> Color32 {
    c(|p| p.on_accent)
}

// ── метрики ──────────────────────────────────────────────────────────────────
pub const SIDEBAR_W: f32 = 240.0;
pub const BAR_H: f32 = 80.0;
pub const ROW_H: f32 = 52.0;
pub const RADIUS: u8 = 8;

// ── типографика ──────────────────────────────────────────────────────────────
pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Жирное начертание (Segoe UI Semibold, если он есть в системе).
pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Bold".into()))
}

/// Шрифты один раз при старте.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let win = std::path::Path::new("C:/Windows/Fonts");

    let mut load = |key: &str, file: &str| -> bool {
        match std::fs::read(win.join(file)) {
            Ok(bytes) => {
                fonts.font_data.insert(
                    key.to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                true
            }
            Err(_) => false,
        }
    };

    let regular = load("segoe", "segoeui.ttf");
    let semibold = load("segoe_sb", "segoeuisb.ttf");
    let bold_font = load("segoe_b", "segoeuib.ttf");

    if regular {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".to_owned());
    }

    let bold_key = if semibold {
        Some("segoe_sb")
    } else if bold_font {
        Some("segoe_b")
    } else {
        None
    };
    let mut bold_family = Vec::new();
    if let Some(key) = bold_key {
        bold_family.push(key.to_owned());
    }
    bold_family.push(if regular { "segoe" } else { "Ubuntu-Light" }.to_owned());
    fonts
        .families
        .insert(FontFamily::Name("Bold".into()), bold_family);

    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.scroll = egui::style::ScrollStyle::thin();

        style.text_styles = [
            (egui::TextStyle::Small, font(12.0)),
            (egui::TextStyle::Body, font(14.0)),
            (egui::TextStyle::Button, font(14.0)),
            (egui::TextStyle::Heading, bold(18.0)),
            (egui::TextStyle::Monospace, font(12.0)),
        ]
        .into();
    });

    apply(ctx);
}

/// Перекрашивает виджеты egui под текущую палитру (вызывается после её смены).
pub fn apply(ctx: &egui::Context) {
    let p = palette();
    ctx.all_styles_mut(move |style| {
        let mut visuals = if p.is_dark() {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = p.bg;
        visuals.window_fill = p.panel;
        visuals.extreme_bg_color = p.surface;
        visuals.faint_bg_color = p.surface;
        visuals.override_text_color = Some(p.text);
        visuals.selection.bg_fill = p.accent.gamma_multiply(0.22);
        visuals.selection.stroke = egui::Stroke::new(1.0, p.accent);
        visuals.hyperlink_color = p.accent;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.border);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.border);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.text_faint);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.accent);
        style.visuals = visuals;
    });
}

/// Смена палитры «на лету».
pub fn set_palette_live(ctx: &egui::Context, p: Palette) {
    set_palette(p);
    apply(ctx);
}
