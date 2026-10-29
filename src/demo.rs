//! Демо-данные для `fluxia --demo`: посмотреть дизайн без токена и сети.

use std::time::Duration;

use eframe::egui::{self, TextureOptions};

use crate::api::{AccountInfo, PlaylistItem, TrackItem};

const PALETTE: [(u8, u8, u8); 6] = [
    (44, 38, 30),
    (28, 33, 40),
    (38, 29, 38),
    (26, 38, 34),
    (46, 40, 28),
    (30, 30, 38),
];

pub fn account() -> AccountInfo {
    AccountInfo {
        uid: 0,
        name: "Демо-режим".to_owned(),
    }
}

pub fn playlists() -> Vec<PlaylistItem> {
    let titles = [
        ("Мне нравится", 412u32),
        ("Дорога", 38),
        ("Ночной город", 27),
        ("Работа", 64),
        ("Тёплые вещи", 19),
        ("Нечто новое", 31),
        ("Для дорожки", 22),
        ("Фоновое", 45),
    ];
    titles
        .into_iter()
        .enumerate()
        .map(|(i, (title, count))| PlaylistItem {
            kind: i as u32,
            uid: 0,
            title: title.to_owned(),
            count,
            cover: Some(format!("demo:p{i}")),
            owner: "Демо-режим".to_owned(),
        })
        .collect()
}

pub fn tracks() -> Vec<TrackItem> {
    let data: [(&str, &str, u32); 18] = [
        ("Before The Fall", "RP", 214),
        ("Морозный воздух", "Семён Слепаков", 187),
        ("Neon Rain", "Kavinsky", 302),
        ("Забери меня домой", "Zemfira", 245),
        ("Slow Motion", "Saint Jhn", 198),
        ("Город под подошвой", "Oxxxymiron", 267),
        ("Nocturne", "Ólafur Arnalds", 224),
        ("Солнце в колодце", "Дельфин", 291),
        ("Prism Break", "Little Dragon", 176),
        ("Провода", "Мельница", 203),
        ("Dust & Quiet", "Gesaffelstein", 254),
        ("Половина пути", "Noize MC", 231),
        ("Undertow", "Warpaint", 288),
        ("Тише травы", "Аукцыон", 219),
        ("Afterglow", "Tame Impala", 342),
        ("Каждый день", "Мумий Тролль", 197),
        ("Cold Harbour", "Brodinski", 265),
        ("Свободный полёт", "Kino", 238),
    ];

    data.iter()
        .enumerate()
        .map(|(i, (title, artist, secs))| TrackItem {
            id: format!("demo-track-{i}"),
            title: (*title).to_owned(),
            artists: (*artist).to_owned(),
            duration: Duration::from_secs(*secs as u64),
            cover: Some(format!("demo:p{}", i % 6)),
            available: true,
        })
        .collect()
}

/// Плоские обложки-заглушки, чтобы список не выглядел дырявым.
pub fn covers(ctx: &egui::Context, count: usize) -> Vec<(String, egui::TextureHandle)> {
    let mut out = Vec::new();
    for i in 0..count {
        let (r, g, b) = PALETTE[i % PALETTE.len()];
        let size = 64usize;
        let mut rgba = Vec::with_capacity(size * size * 4);
        for y in 0..size {
            for x in 0..size {
                let k = 1.0 + (x as f32 / size as f32) * 0.22 + (y as f32 / size as f32) * 0.08;
                rgba.push(((r as f32 * k).min(255.0)) as u8);
                rgba.push(((g as f32 * k).min(255.0)) as u8);
                rgba.push(((b as f32 * k).min(255.0)) as u8);
                rgba.push(255);
            }
        }
        let image = egui::ColorImage::from_rgba_unmultiplied([size, size], &rgba);
        let key = format!("demo:p{i}");
        let tex = ctx.load_texture(key.clone(), image, TextureOptions::LINEAR);
        out.push((key, tex));
    }
    out
}
