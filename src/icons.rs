//! Иконки рисуются кодом — никаких внешних ассетов и шрифтов.

use eframe::egui::{pos2, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke};

fn poly(p: &Painter, pts: Vec<Pos2>, color: Color32) {
    p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

pub fn play(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.5;
    poly(
        p,
        vec![
            pos2(c.x - r * 0.58, c.y - r * 0.92),
            pos2(c.x + r * 0.86, c.y),
            pos2(c.x - r * 0.58, c.y + r * 0.92),
        ],
        color,
    );
}

pub fn pause(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.5;
    let w = r * 0.34;
    let h = r * 0.9;
    p.rect_filled(
        Rect::from_min_max(pos2(c.x - r * 0.62, c.y - h), pos2(c.x - r * 0.62 + w, c.y + h)),
        1,
        color,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(c.x + r * 0.28, c.y - h), pos2(c.x + r * 0.28 + w, c.y + h)),
        1,
        color,
    );
}

pub fn next(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.5;
    poly(
        p,
        vec![
            pos2(c.x - r * 0.92, c.y - r * 0.72),
            pos2(c.x + r * 0.28, c.y),
            pos2(c.x - r * 0.92, c.y + r * 0.72),
        ],
        color,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(c.x + r * 0.44, c.y - r * 0.72), pos2(c.x + r * 0.78, c.y + r * 0.72)),
        1,
        color,
    );
}

pub fn prev(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.5;
    poly(
        p,
        vec![
            pos2(c.x + r * 0.92, c.y - r * 0.72),
            pos2(c.x - r * 0.28, c.y),
            pos2(c.x + r * 0.92, c.y + r * 0.72),
        ],
        color,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(c.x - r * 0.78, c.y - r * 0.72), pos2(c.x - r * 0.44, c.y + r * 0.72)),
        1,
        color,
    );
}

pub fn volume(p: &Painter, c: Pos2, s: f32, color: Color32, muted: bool) {
    let r = s * 0.5;
    // корпус динамика
    poly(
        p,
        vec![
            pos2(c.x - r * 0.9, c.y - r * 0.34),
            pos2(c.x - r * 0.42, c.y - r * 0.34),
            pos2(c.x + r * 0.16, c.y - r * 0.9),
            pos2(c.x + r * 0.16, c.y + r * 0.9),
            pos2(c.x - r * 0.42, c.y + r * 0.34),
            pos2(c.x - r * 0.9, c.y + r * 0.34),
        ],
        color,
    );
    let stroke = Stroke::new(1.4 * (s / 16.0).max(0.8), color);
    if muted {
        let a = pos2(c.x + r * 0.42, c.y - r * 0.42);
        let b = pos2(c.x + r * 0.95, c.y + r * 0.42);
        p.line_segment([a, b], stroke);
        p.line_segment([pos2(b.x, a.y), pos2(a.x, b.y)], stroke);
    } else {
        for (i, k) in [0.55f32, 0.95].iter().enumerate() {
            let w = r * k;
            let pts = vec![
                pos2(c.x + r * 0.34, c.y - w * 0.62),
                pos2(c.x + r * 0.34 + w * 0.32, c.y),
                pos2(c.x + r * 0.34, c.y + w * 0.62),
            ];
            let _ = i;
            p.add(Shape::line(pts, stroke));
        }
    }
}

/// Три «пляшущих» столбика — индикатор играющего трека.
pub fn equalizer(p: &Painter, c: Pos2, s: f32, color: Color32, phase: f32) {
    let w = (s * 0.16).max(1.5);
    let gap = s * 0.22;
    for i in 0..3 {
        let t = phase * 2.6 + i as f32 * 1.7;
        let k = 0.35 + 0.65 * ((t.sin() + 1.0) * 0.5);
        let h = s * 0.8 * k;
        let x = c.x - gap + i as f32 * (w + gap * 0.55);
        p.rect_filled(
            Rect::from_min_max(pos2(x, c.y - h * 0.5), pos2(x + w, c.y + h * 0.5)),
            1,
            color,
        );
    }
}

/// Сердечко из двух кругов и треугольника — только выпуклые фигуры,
/// поэтому заливка всегда корректная.
pub fn heart(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let rr = s * 0.26;
    let dy = -rr * 0.5;
    let lx = c.x - rr * 0.92;
    let rx = c.x + rr * 0.92;
    p.circle_filled(pos2(lx, c.y + dy), rr, color);
    p.circle_filled(pos2(rx, c.y + dy), rr, color);
    p.add(Shape::convex_polygon(
        vec![
            pos2(lx - rr * 0.16, c.y + dy + rr * 0.35),
            pos2(rx + rr * 0.16, c.y + dy + rr * 0.35),
            pos2(c.x, c.y + s * 0.52),
        ],
        color,
        Stroke::NONE,
    ));
}

/// «Моя волна»: три столбика разной высоты.
pub fn wave(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let w = (s * 0.16).max(1.6);
    let gap = s * 0.13;
    let heights = [0.5_f32, 1.0, 0.7];
    let total = w * 3.0 + gap * 2.0;
    let left = c.x - total * 0.5;
    for (i, h) in heights.iter().enumerate() {
        let hh = s * 0.8 * h;
        let x = left + i as f32 * (w + gap);
        p.rect_filled(
            Rect::from_min_max(pos2(x, c.y - hh * 0.5), pos2(x + w, c.y + hh * 0.5)),
            CornerRadius::same((w * 0.5).round() as u8),
            color,
        );
    }
}

/// Лупа.
pub fn search(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let r = s * 0.32;
    let center = pos2(c.x - s * 0.06, c.y - s * 0.06);
    p.circle_stroke(center, r, Stroke::new(1.6, color));
    let a = std::f32::consts::FRAC_PI_4;
    let start = pos2(center.x + r * a.cos(), center.y + r * a.sin());
    let end = pos2(c.x + s * 0.34, c.y + s * 0.34);
    p.line_segment([start, end], Stroke::new(1.6, color));
}

/// Очередь: три строки с «маркерами» слева.
pub fn queue(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    let left = c.x - s * 0.5;
    let right = c.x + s * 0.5;
    for (i, dy) in [-0.34_f32, 0.0, 0.34].iter().enumerate() {
        let y = c.y + s * dy;
        p.circle_filled(pos2(left + s * 0.02, y), if i == 0 { 1.7 } else { 1.2 }, color);
        p.line_segment(
            [pos2(left + s * 0.20, y), pos2(right, y)],
            stroke,
        );
    }
}

/// Ползунки (настройки).
pub fn sliders(p: &Painter, c: Pos2, s: f32, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    for (i, (dy, knob)) in [(-0.32_f32, -0.16_f32), (0.0, 0.22), (0.32, -0.02)]
        .iter()
        .enumerate()
    {
        let y = c.y + s * dy;
        p.line_segment(
            [pos2(c.x - s * 0.5, y), pos2(c.x + s * 0.5, y)],
            stroke,
        );
        p.circle_filled(pos2(c.x + s * knob, y), if i == 1 { 2.6 } else { 2.2 }, color);
    }
}

/// Текстура логотипа из вшитого PNG (грузим один раз на старте).
/// Логотип вписывается в квадрат по большей стороне — без искажений.
pub fn logo_texture(ctx: &eframe::egui::Context) -> Option<eframe::egui::TextureHandle> {
    let src = image::load_from_memory(crate::LOGO_BYTES).ok()?.to_rgba8();
    let side = src.width().max(src.height()).max(1);
    let mut square =
        image::RgbaImage::from_pixel(side, side, image::Rgba([0, 0, 0, 0]));
    let ox = (side - src.width()) / 2;
    let oy = (side - src.height()) / 2;
    image::imageops::overlay(&mut square, &src, ox as i64, oy as i64);

    let img = image::imageops::resize(&square, 128, 128, image::imageops::FilterType::Lanczos3);
    let size = [img.width() as usize, img.height() as usize];
    let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture("logo", color_image, eframe::egui::TextureOptions::LINEAR))
}

/// Плитка с логотипом: белый скруглённый квадрат + картинка внутри.
pub fn logo_tile(
    ui: &eframe::egui::Ui,
    rect: Rect,
    radius: u8,
    tex: Option<&eframe::egui::TextureHandle>,
    inset: f32,
) {
    ui.painter().rect_filled(rect, radius, Color32::WHITE);
    if let Some(t) = tex {
        let inner = rect.shrink(inset);
        eframe::egui::Image::new(eframe::egui::load::SizedTexture::new(t.id(), inner.size()))
            .paint_at(ui, inner);
    }
}

/// Обложка со скруглением (или заглушка, пока картинка грузится).
pub fn cover(
    ui: &eframe::egui::Ui,
    rect: Rect,
    radius: u8,
    tex: Option<&eframe::egui::TextureHandle>,
) {
    match tex {
        Some(t) => {
            eframe::egui::Image::new(eframe::egui::load::SizedTexture::new(t.id(), rect.size()))
                .corner_radius(radius)
                .paint_at(ui, rect);
        }
        None => {
            ui.painter()
                .rect_filled(rect, radius, crate::theme::surface_hover());
        }
    }
}
