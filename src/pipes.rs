//! Фоновая анимация «трубы» для «Моей волны» — в духе pipes.sh:
//! несколько змеек ползут по сетке, иногда поворачивают, оставляют затухающий след.

use eframe::egui::{self, Color32, Pos2, Rect};

const STEP: f32 = 26.0; // шаг сетки
const TRAIL: usize = 26; // сколько точек держим в хвосте
const SPEED: f32 = 46.0; // пикселей в секунду

struct Pipe {
    head: Pos2,
    dir: (i32, i32),
    trail: Vec<Pos2>,
    len: usize,
}

pub struct Pipes {
    rng: u64,
    pipes: Vec<Pipe>,
    accum: f32,
    rect: Rect,
}

impl Pipes {
    pub fn new() -> Self {
        Self {
            rng: 0x9E37_79B9_7F4A_7C15,
            pipes: Vec::new(),
            accum: 0.0,
            rect: Rect::NOTHING,
        }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        let r = (self.next_u32() % 10_000) as f32 / 10_000.0;
        lo + (hi - lo) * r
    }

    fn spawn(&mut self, rect: Rect) -> Pipe {
        let x = self.rand_range(rect.left(), rect.right());
        let y = self.rand_range(rect.top(), rect.bottom());
        let dir = match self.next_u32() % 4 {
            0 => (1, 0),
            1 => (-1, 0),
            2 => (0, 1),
            _ => (0, -1),
        };
        Pipe {
            head: Pos2::new(x, y),
            dir,
            trail: vec![Pos2::new(x, y)],
            len: 12 + (self.next_u32() % (TRAIL as u32 - 8)) as usize,
        }
    }

    /// Держим пул змеек и двигаем их.
    pub fn update(&mut self, rect: Rect, dt: f32) {
        if !rect.is_positive() {
            return;
        }
        if self.rect != rect || self.pipes.is_empty() {
            self.rect = rect;
            self.pipes.clear();
            let count = 3 + (self.next_u32() % 3) as usize;
            for _ in 0..count {
                let p = self.spawn(rect);
                self.pipes.push(p);
            }
        }

        self.accum += dt;
        while self.accum >= 0.05 {
            self.accum -= 0.05;
            self.step(rect);
        }
    }

    fn step(&mut self, rect: Rect) {
        for i in 0..self.pipes.len() {
            // иногда поворот на 90°, как у труб
            if self.next_u32() % 9 == 0 {
                let (dx, _dy) = self.pipes[i].dir;
                self.pipes[i].dir = if dx != 0 {
                    (0, if self.next_u32() % 2 == 0 { 1 } else { -1 })
                } else {
                    (if self.next_u32() % 2 == 0 { 1 } else { -1 }, 0)
                };
            }

            let (dx, dy) = self.pipes[i].dir;
            let mut head = self.pipes[i].head;
            head.x += dx as f32 * SPEED * 0.05;
            head.y += dy as f32 * SPEED * 0.05;

            if !rect.contains(head) {
                let fresh = self.spawn(rect);
                self.pipes[i] = fresh;
                continue;
            }

            self.pipes[i].head = head;
            self.pipes[i].trail.push(head);
            let limit = self.pipes[i].len;
            if self.pipes[i].trail.len() > limit {
                self.pipes[i].trail.remove(0);
            }
        }
    }

    pub fn draw(&self, painter: &egui::Painter, accent: Color32) {
        for pipe in &self.pipes {
            let trail = &pipe.trail;
            for w in 1..trail.len() {
                let t = w as f32 / trail.len() as f32;
                let alpha = (t * t * 92.0) as u8;
                let color = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha);
                painter.line_segment([trail[w - 1], trail[w]], egui::Stroke::new(1.6, color));
            }
            // голова светится
            if let Some(last) = trail.last() {
                let head = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 150);
                painter.circle_filled(*last, 2.2, head);
            }
        }
        // едва заметная сетка, чтобы «трубы» читались
        let grid = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 8);
        let mut x = self.rect.left();
        while x < self.rect.right() {
            painter.line_segment(
                [Pos2::new(x, self.rect.top()), Pos2::new(x, self.rect.bottom())],
                egui::Stroke::new(1.0, grid),
            );
            x += STEP;
        }
        let mut y = self.rect.top();
        while y < self.rect.bottom() {
            painter.line_segment(
                [Pos2::new(self.rect.left(), y), Pos2::new(self.rect.right(), y)],
                egui::Stroke::new(1.0, grid),
            );
            y += STEP;
        }
    }
}

impl Default for Pipes {
    fn default() -> Self {
        Self::new()
    }
}
