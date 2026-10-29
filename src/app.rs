//! Состояние приложения и обработка событий.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, RichText};
use yandex_music::YandexMusicClient;

use crate::api::{self, AccountInfo, PlaylistItem, TrackItem};
use crate::config::{AutoColor, Config};
use crate::player::Audio;
use crate::theme as t;

/// Варианты акцента для «случайного» автоцвета.
const RANDOM_ACCENTS: [egui::Color32; 6] = [
    egui::Color32::from_rgb(255, 210, 74),
    egui::Color32::from_rgb(122, 200, 255),
    egui::Color32::from_rgb(150, 245, 175),
    egui::Color32::from_rgb(255, 145, 195),
    egui::Color32::from_rgb(198, 160, 255),
    egui::Color32::from_rgb(255, 158, 110),
];

fn random_accent() -> egui::Color32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    RANDOM_ACCENTS[(nanos as usize) % RANDOM_ACCENTS.len()]
}

/// Днём светлее, вечером и ночью темнее — акцент сохраняем.
fn palette_for_time(current: t::Palette) -> t::Palette {
    let hour = crate::win::local_hour();
    let base = if (7..19).contains(&hour) {
        t::Palette::light()
    } else {
        t::Palette::dark()
    };
    base.with_accent(current.accent)
}

/// «Цветной» акцент из обложки: среднее со усиленной насыщенностью.
fn cover_accent(rgba: &image::RgbaImage) -> egui::Color32 {
    let (w, h) = (rgba.width().max(1), rgba.height().max(1));
    let step = (w.min(h) / 24).max(1);
    let (mut r, mut g, mut b, mut n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);

    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let px = rgba.get_pixel(x, y).0;
            let (pr, pg, pb) = (px[0] as f64, px[1] as f64, px[2] as f64);
            // белый/чёрный фон обложек не должен размывать цвет
            let lum = (pr + pg + pb) / 3.0;
            if (30.0..240.0).contains(&lum) {
                r += pr;
                g += pg;
                b += pb;
                n += 1.0;
            }
            x += step;
        }
        y += step;
    }

    if n == 0.0 {
        return egui::Color32::from_rgb(255, 210, 74);
    }

    let (r, g, b) = (r / n, g / n, b / n);
    let gray = (r + g + b) / 3.0;
    let boost = |c: f64| (gray + (c - gray) * 1.85).clamp(0.0, 255.0);
    let (r, g, b) = (boost(r), boost(g), boost(b));

    // подтягиваем яркость, чтобы акцент не был тёмным
    let peak = r.max(g).max(b);
    let k = if peak < 200.0 && peak > 1.0 {
        225.0 / peak
    } else {
        1.0
    };
    egui::Color32::from_rgb(
        (r * k).clamp(0.0, 255.0) as u8,
        (g * k).clamp(0.0, 255.0) as u8,
        (b * k).clamp(0.0, 255.0) as u8,
    )
}

pub enum Msg {
    Login(Result<(AccountInfo, Vec<PlaylistItem>), String>),
    Tracks(usize, Result<Vec<TrackItem>, String>),
    Audio(String, Result<Vec<u8>, String>),
    Cover(String, Result<Vec<u8>, String>),
    Liked(Result<Vec<TrackItem>, String>),
    Wave(Result<Vec<TrackItem>, String>),
    Search(Result<Vec<TrackItem>, String>),
    Like(String, bool, Result<(), String>),
}

/// Что сейчас показано в правой части.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Playlist,
    Liked,
    Wave,
    Search,
    Settings,
}

pub struct App {
    pub cfg: Config,
    pub tx: Sender<Msg>,
    rx: Receiver<Msg>,
    pub rt: tokio::runtime::Runtime,
    pub client: Option<Arc<YandexMusicClient>>,
    pub token: String,

    // экран входа
    pub token_input: String,
    pub login_busy: bool,
    pub login_error: Option<String>,
    pub login_focused: bool,

    // данные
    pub account: Option<AccountInfo>,
    pub playlists: Vec<PlaylistItem>,
    pub active_playlist: Option<usize>,
    pub tracks: Vec<TrackItem>,
    pub tracks_busy: bool,
    pub view: View,

    // лайки и поиск
    pub likes: HashSet<String>,
    pub liked: Vec<TrackItem>,
    /// Коллекция «Мне нравится» как плейлист Яндекса (kind = 3).
    pub liked_playlist: Option<PlaylistItem>,
    pub search_input: String,
    pub search_query: String,
    pub search_busy: bool,
    pub wave_busy: bool,
    wave_autoplay: bool,

    // воспроизведение
    pub audio: Audio,
    pub current: Option<usize>,
    pub audio_busy: Option<String>,
    pub volume: f32,
    pub seeking: Option<f32>,

    // ui
    pub covers: HashMap<String, egui::TextureHandle>,
    covers_requested: HashSet<String>,
    pub banner: Option<String>,
    pub phase: f32,
    pub demo: bool,
    pub demo_playing: bool,

    // служебное: `--shot file.png` — сохранить кадр окна и выйти
    /// Текстура логотипа (вшита в бинарник).
    pub logo: Option<egui::TextureHandle>,

    /// Фоновая анимация «труб» в «Моей волне».
    pub pipes: crate::pipes::Pipes,
    /// Окно «что играет дальше».
    pub show_queue: bool,
    /// Какой трек сейчас тащат в очереди.
    pub queue_drag: Option<usize>,
    /// Средний цвет обложки (для автоцвета по треку).
    pub cover_color: HashMap<String, egui::Color32>,
    /// Палитра, под которую уже перекрашено окно.
    styled: Option<t::Palette>,
    /// Ждём загрузки лайков, чтобы включить музыку.
    autoplay_pending: bool,
    /// Последний результат сохранения конфига (для экрана настроек).
    pub save_status: Option<Result<std::path::PathBuf, String>>,
    /// Конфиг изменён, но ещё не записан (дебаунс).
    dirty: bool,
    dirty_at: f32,

    shot_path: Option<String>,
    shot_taken: bool,
    frames: u32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = Config::load();

        // палитра из конфига (+ автоцвет при запуске)
        let mut palette = cfg.theme.to_palette();
        match cfg.auto_color {
            AutoColor::Random => palette = palette.with_accent(random_accent()),
            AutoColor::TimeOfDay => palette = palette_for_time(palette),
            _ => {}
        }
        t::set_palette(palette);
        t::install(&cc.egui_ctx);

        let (tx, rx) = mpsc::channel();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("не удалось запустить tokio");

        let audio = Audio::new(cfg.volume);
        let mut app = Self {
            token: cfg.token.clone().unwrap_or_default(),
            token_input: String::new(),
            cfg,
            tx,
            rx,
            rt,
            client: None,
            login_busy: false,
            login_error: None,
            login_focused: false,
            account: None,
            playlists: Vec::new(),
            active_playlist: None,
            tracks: Vec::new(),
            tracks_busy: false,
            view: View::Playlist,
            likes: HashSet::new(),
            liked: Vec::new(),
            liked_playlist: None,
            search_input: String::new(),
            search_query: String::new(),
            search_busy: false,
            wave_busy: false,
            wave_autoplay: false,
            audio,
            current: None,
            audio_busy: None,
            volume: 0.7,
            seeking: None,
            covers: HashMap::new(),
            covers_requested: HashSet::new(),
            banner: None,
            phase: 0.0,
            demo: false,
            demo_playing: false,
            logo: crate::icons::logo_texture(&cc.egui_ctx),
            pipes: crate::pipes::Pipes::new(),
            show_queue: false,
            queue_drag: None,
            cover_color: HashMap::new(),
            styled: None,
            autoplay_pending: false,
            save_status: None,
            dirty: false,
            dirty_at: 0.0f32,
            shot_path: None,
            shot_taken: false,
            frames: 0,
        };
        app.volume = app.cfg.volume;

        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--shot") {
            app.shot_path = args.get(pos + 1).cloned();
        }

        if args.iter().any(|a| a == "--demo") {
            app.demo = true;
            app.account = Some(crate::demo::account());
            app.playlists = crate::demo::playlists();
            app.tracks = crate::demo::tracks();
            app.active_playlist = Some(0);
            app.covers.extend(crate::demo::covers(&cc.egui_ctx, 8));
            app.current = Some(2);
            app.demo_playing = true;
            app.liked = app.tracks.iter().take(6).cloned().collect();
            app.likes = app.liked.iter().map(|t| t.id.clone()).collect();

            // для скриншотов: --view liked|wave|search
            if let Some(pos) = args.iter().position(|a| a == "--view") {
                match args.get(pos + 1).map(|s| s.as_str()) {
                    Some("liked") => {
                        app.view = View::Liked;
                        app.active_playlist = None;
                        app.tracks = app.liked.clone();
                    }
                    Some("wave") => {
                        app.view = View::Wave;
                        app.active_playlist = None;
                    }
                    Some("search") => {
                        app.view = View::Search;
                        app.active_playlist = None;
                        app.search_input = "neon".to_owned();
                        app.search_query = "neon".to_owned();
                        app.tracks = crate::demo::tracks()
                            .into_iter()
                            .filter(|t| t.title.to_lowercase().contains("neon"))
                            .collect();
                    }
                    Some("settings") => {
                        app.view = View::Settings;
                        app.active_playlist = None;
                    }
                    // служебные экраны для скриншотов
                    Some("login") => app.account = None,
                    Some("queue") => app.show_queue = true,
                    _ => {}
                }
            }

            return app;
        }

        if let Some(token) = app.cfg.token.clone().filter(|s| !s.trim().is_empty()) {
            app.start_login(token);
        }
        app
    }

    /// Снимок воспроизведения (в демо-режиме — синтетический).
    pub fn snapshot(&self) -> crate::player::Snapshot {
        if self.demo {
            let duration = self
                .current
                .and_then(|i| self.tracks.get(i))
                .map(|t| t.duration)
                .unwrap_or(Duration::from_secs(214));
            return crate::player::Snapshot {
                position: Duration::from_secs(83),
                duration,
                playing: self.demo_playing,
                loaded: true,
            };
        }
        self.audio.snapshot()
    }

    pub fn toggle(&mut self) {
        if self.demo {
            self.demo_playing = !self.demo_playing;
        } else {
            self.audio.toggle();
        }
    }

    // ── сеть ────────────────────────────────────────────────────────────────

    pub fn start_login(&mut self, token: String) {
        let token = token.trim().to_owned();
        if token.is_empty() {
            self.login_error = Some("Вставьте токен".to_owned());
            return;
        }
        self.login_busy = true;
        self.login_error = None;
        self.token = token.clone();

        // сохраняем сразу, не дожидаясь сети: иначе токен теряется при перезапуске
        self.cfg.token = Some(token.clone());
        self.dirty = false;
        self.save_cfg();
        if let Some(Err(e)) = &self.save_status {
            self.login_error = Some(format!("Токен не сохранится: {e}"));
        }

        let client = match api::new_client(&token) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                self.login_busy = false;
                self.login_error = Some(e);
                return;
            }
        };
        self.client = Some(client.clone());

        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = async {
                let acc = api::account(&client).await?;
                let pls = api::playlists(&client, acc.uid).await?;
                Ok::<_, String>((acc, pls))
            }
            .await;
            let _ = tx.send(Msg::Login(res));
        });
    }

    pub fn load_playlist(&mut self, idx: usize) {
        let Some(pl) = self.playlists.get(idx).cloned() else {
            return;
        };
        let Some(client) = self.client.clone() else {
            return;
        };

        self.active_playlist = Some(idx);
        self.view = View::Playlist;
        self.tracks_busy = true;
        self.banner = None;

        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = api::playlist_tracks(&client, pl.uid, pl.kind).await;
            let _ = tx.send(Msg::Tracks(idx, res));
        });
    }

    pub fn play_index(&mut self, idx: usize) {
        let Some(track) = self.tracks.get(idx).cloned() else {
            return;
        };
        if self.demo {
            self.current = Some(idx);
            self.demo_playing = true;
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };

        self.current = Some(idx);
        self.audio_busy = Some(track.id.clone());
        self.banner = None;

        let tx = self.tx.clone();
        let bitrate = self.cfg.bitrate;
        self.rt.spawn(async move {
            let res = api::stream(&client, &track.id, bitrate).await;
            let _ = tx.send(Msg::Audio(track.id, res));
        });
    }

    pub fn next_track(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let next = match self.current {
            Some(i) if i + 1 < self.tracks.len() => i + 1,
            _ => {
                // в волне докидываем новую порцию вместо зацикливания
                if self.view == View::Wave && !self.demo {
                    self.wave_autoplay = true;
                    self.fetch_wave();
                    return;
                }
                0
            }
        };
        self.play_index(next);
    }

    pub fn prev_track(&mut self) {
        let Some(i) = self.current else { return };
        if self.audio.snapshot().position > Duration::from_secs(3) {
            self.audio.seek(Duration::ZERO);
        } else if i > 0 {
            self.play_index(i - 1);
        } else {
            self.audio.seek(Duration::ZERO);
        }
    }

    pub fn seeks_by(&mut self, delta: i64) {
        if self.demo {
            return;
        }
        let snap = self.audio.snapshot();
        if !snap.loaded {
            return;
        }
        let pos = snap.position.as_secs() as i64 + delta;
        let target = pos.clamp(0, snap.duration.as_secs() as i64) as u64;
        self.audio.seek(Duration::from_secs(target));
    }

    // ── «Мне нравится», волна, поиск ────────────────────────────────────────

    pub fn uid(&self) -> u64 {
        self.account.as_ref().map(|a| a.uid).unwrap_or(0)
    }

    pub fn open_liked(&mut self) {
        self.view = View::Liked;
        self.active_playlist = None;
        self.current = None;
        self.tracks = self.liked.clone();
        self.banner = None;
        self.reload_liked();
        self.request_covers();
    }

    pub fn reload_liked(&mut self) {
        let Some(client) = self.client.clone() else { return };
        let uid = self.uid();
        if uid == 0 || self.demo {
            return;
        }
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = api::liked_tracks(&client, uid).await;
            let _ = tx.send(Msg::Liked(res));
        });
    }

    pub fn open_wave(&mut self) {
        self.view = View::Wave;
        self.active_playlist = None;
        self.banner = None;

        if self.demo {
            self.tracks = crate::demo::tracks();
            self.current = None;
            self.request_covers();
            self.play_index(0);
            return;
        }

        self.wave_autoplay = true;
        self.fetch_wave();
    }

    pub fn fetch_wave(&mut self) {
        if self.wave_busy {
            return;
        }
        let Some(client) = self.client.clone() else { return };
        self.wave_busy = true;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = api::wave_tracks(&client).await;
            let _ = tx.send(Msg::Wave(res));
        });
    }

    pub fn open_settings(&mut self) {
        self.view = View::Settings;
        self.banner = None;
    }

    /// Применяет выбранный режим автоцвета прямо сейчас.
    pub fn refresh_auto_color(&mut self, ctx: &egui::Context) {
        match self.cfg.auto_color {
            AutoColor::Random => {
                let palette = t::palette().with_accent(random_accent());
                self.apply_theme(ctx, palette);
            }
            AutoColor::TimeOfDay => {
                let palette = palette_for_time(t::palette());
                self.apply_theme(ctx, palette);
            }
            AutoColor::Cover => self.apply_cover_auto_color(ctx),
            AutoColor::Off => {}
        }
    }

    /// Красит рамку окна под текущую палитру (Windows 11: цвет заголовка, скругление).
    pub fn sync_window_style(&mut self) {
        let palette = t::palette();
        if self.styled == Some(palette) {
            return;
        }
        // окно появляется не сразу — пробуем каждый кадр, пока не найдём
        if crate::win::style_window(&palette) {
            self.styled = Some(palette);
        }
    }

    /// Запуск музыки при старте (если включено в настройках).
    pub fn start_autoplay(&mut self) {
        if !self.cfg.autoplay {
            return;
        }
        match self.cfg.autoplay_source {
            crate::config::AutoplaySource::Wave => self.open_wave(),
            crate::config::AutoplaySource::Liked => {
                self.autoplay_pending = true;
                self.open_liked();
            }
        }
    }

    /// Перестановка трека в текущем списке (для очереди).
    pub fn move_track(&mut self, from: usize, to: usize) {
        let len = self.tracks.len();
        if from == to || from >= len || to >= len {
            return;
        }
        let item = self.tracks.remove(from);
        self.tracks.insert(to, item);

        if let Some(cur) = self.current {
            self.current = Some(if cur == from {
                to
            } else if from < cur && to >= cur {
                cur - 1
            } else if from > cur && to <= cur {
                cur + 1
            } else {
                cur
            });
        }
    }

    /// Окно «что играет дальше».
    fn ui_queue_window(&mut self, ctx: &egui::Context) {
        if !self.show_queue {
            return;
        }
        let start = self.current.map(|i| i + 1).unwrap_or(0);
        let upcoming: Vec<(usize, String, String)> = self
            .tracks
            .iter()
            .enumerate()
            .skip(start)
            .take(14)
            .map(|(i, t)| (i, t.title.clone(), t.artists.clone()))
            .collect();

        let mut open = true;
        let mut close = false;
        let mut jump = None;
        // перетаскивание
        let mut dragging = self.queue_drag;
        let mut drop_to: Option<usize> = None;

        egui::Window::new("Далее")
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(
                egui::Align2::RIGHT_BOTTOM,
                [-22.0, -(t::BAR_H + 18.0)],
            )
            .frame(
                egui::Frame::NONE
                    .fill(t::panel())
                    .corner_radius(egui::CornerRadius::same(t::RADIUS))
                    .stroke(egui::Stroke::new(1.0, t::border()))
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ДАЛЕЕ").font(t::bold(10.0)).color(t::text_faint()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("×").font(t::font(14.0)).color(t::text_dim()),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(4.0);

                if upcoming.is_empty() {
                    ui.label(
                        RichText::new("Дальше ничего нет")
                            .font(t::font(12.0))
                            .color(t::text_dim()),
                    );
                    return;
                }

                for (i, title, artist) in &upcoming {
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 40.0),
                        egui::Sense::click_and_drag(),
                    );
                    let is_src = dragging == Some(*i);
                    let hovered = resp.hovered();

                    if hovered && !is_src {
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(6),
                            t::surface(),
                        );
                    }
                    if is_src {
                        // строка, которую тащат
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same(6),
                            t::surface_hover(),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            egui::Stroke::new(1.0, t::accent()),
                            egui::StrokeKind::Inside,
                        );
                    }

                    // ручка перетаскивания слева
                    let grip = egui::pos2(rect.left() + 9.0, rect.center().y);
                    for k in -1..=1 {
                        ui.painter().circle_filled(
                            egui::pos2(grip.x, grip.y + k as f32 * 5.0),
                            1.4,
                            if is_src || hovered {
                                t::accent()
                            } else {
                                t::text_faint()
                            },
                        );
                    }

                    let text_x = rect.left() + 22.0;
                    let text_w = rect.width() - 30.0;
                    let title = crate::ui::fit(ui, title, t::font(12.5), text_w);
                    ui.painter().text(
                        egui::pos2(text_x, rect.center().y - 7.0),
                        egui::Align2::LEFT_CENTER,
                        title,
                        t::font(12.5),
                        t::text(),
                    );
                    let artist = crate::ui::fit(ui, artist, t::font(10.5), text_w);
                    ui.painter().text(
                        egui::pos2(text_x, rect.center().y + 7.5),
                        egui::Align2::LEFT_CENTER,
                        artist,
                        t::font(10.5),
                        t::text_dim(),
                    );

                    if resp.drag_started() {
                        dragging = Some(*i);
                    }
                    if dragging.is_some() && hovered && ui.input(|inp| inp.pointer.any_released()) {
                        drop_to = Some(*i);
                    }
                    if resp.clicked() {
                        jump = Some(*i);
                    }
                    if hovered {
                        let cursor = if dragging.is_some() {
                            egui::CursorIcon::Grabbing
                        } else {
                            egui::CursorIcon::PointingHand
                        };
                        resp.on_hover_cursor(cursor);
                    }
                }

                if dragging.is_some() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("отпусти на нужном месте")
                            .font(t::font(10.0))
                            .color(t::text_faint()),
                    );
                }
            });

        if let Some(to) = drop_to {
            if let Some(from) = dragging {
                self.move_track(from, to);
            }
            dragging = None;
        }
        if dragging.is_some() && ctx.input(|i| i.pointer.any_released()) {
            dragging = None;
        }
        self.queue_drag = dragging;

        if let Some(i) = jump {
            self.play_index(i);
        }
        if !open || close {
            self.show_queue = false;
        }
    }

    /// Применяет палитру и планирует сохранение в конфиг.
    pub fn apply_theme(&mut self, ctx: &egui::Context, palette: t::Palette) {
        t::set_palette_live(ctx, palette);
        self.cfg.theme = crate::config::ThemeConfig::from_palette(palette);
        self.touch_cfg();
    }

    /// Помечает конфиг изменённым: пишем не каждый кадр, а с задержкой.
    pub fn touch_cfg(&mut self) {
        self.dirty = true;
        self.dirty_at = self.phase;
    }

    /// Сбрасывает конфиг на диск, если есть несохранённые изменения.
    pub fn flush_cfg(&mut self, force: bool) {
        if !self.dirty {
            return;
        }
        if !force && self.phase - self.dirty_at < 0.4 {
            return;
        }
        self.dirty = false;
        self.save_cfg();
    }

    /// Пишет конфиг сразу и запоминает путь (его показываем в настройках).
    pub fn save_cfg(&mut self) {
        match self.cfg.save() {
            Ok(p) => self.save_status = Some(Ok(p)),
            Err(e) => {
                eprintln!("fluxia: не удалось сохранить конфиг: {e}");
                self.banner = Some(format!("Настройки не сохраняются: {e}"));
                self.save_status = Some(Err(e));
            }
        }
    }

    /// Строка «куда сохраняем» для экрана настроек.
    pub fn save_path_label(&self) -> String {
        if let Some(Ok(p)) = &self.save_status {
            return p.display().to_string();
        }
        crate::config::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    }

    /// Автоцвет по обложке играющего трека (если включён).
    pub fn apply_cover_auto_color(&mut self, ctx: &egui::Context) {
        if self.cfg.auto_color != AutoColor::Cover {
            return;
        }
        let Some(track) = self.current.and_then(|i| self.tracks.get(i)) else {
            return;
        };
        let Some(uri) = track.cover.clone() else {
            return;
        };
        let Some(color) = self.cover_color.get(&uri).copied() else {
            return;
        };
        let palette = t::palette().with_accent(color);
        self.apply_theme(ctx, palette);
    }

    pub fn run_search(&mut self) {
        let query = self.search_input.trim().to_owned();
        if query.is_empty() {
            return;
        }
        self.search_query = query.clone();
        self.view = View::Search;
        self.active_playlist = None;
        self.tracks.clear();
        self.current = None;
        self.banner = None;

        if self.demo {
            let q = query.to_lowercase();
            self.tracks = crate::demo::tracks()
                .into_iter()
                .filter(|t| {
                    t.title.to_lowercase().contains(&q) || t.artists.to_lowercase().contains(&q)
                })
                .collect();
            self.request_covers();
            return;
        }

        let Some(client) = self.client.clone() else { return };
        self.search_busy = true;
        let tx = self.tx.clone();
        self.rt.spawn(async move {
            let res = api::search_tracks(&client, &query).await;
            let _ = tx.send(Msg::Search(res));
        });
    }

    pub fn toggle_like(&mut self, idx: usize) {
        let Some(track) = self.tracks.get(idx).cloned() else {
            return;
        };
        let now_liked = !self.likes.contains(&track.id);

        if now_liked {
            self.likes.insert(track.id.clone());
            self.liked.insert(0, track.clone());
        } else {
            self.likes.remove(&track.id);
            self.liked.retain(|t| t.id != track.id);
        }
        if let Some(pl) = self.liked_playlist.as_mut() {
            pl.count = if now_liked {
                pl.count.saturating_add(1)
            } else {
                pl.count.saturating_sub(1)
            };
        }
        if self.view == View::Liked && !now_liked {
            self.tracks = self.liked.clone();
            self.current = None;
        }

        if self.demo {
            return;
        }
        let Some(client) = self.client.clone() else { return };
        let uid = self.uid();
        let tx = self.tx.clone();
        let id = track.id.clone();
        self.rt.spawn(async move {
            let res = api::like_track(&client, uid, &id, now_liked).await;
            let _ = tx.send(Msg::Like(id, now_liked, res));
        });
    }

    fn request_covers(&mut self) {
        let covers: Vec<Option<String>> = self.tracks.iter().take(80).map(|t| t.cover.clone()).collect();
        for c in covers {
            self.request_cover(c.as_deref());
        }
    }

    fn request_cover(&mut self, url: Option<&str>) {
        let Some(url) = url else { return };
        if self.covers.contains_key(url) || self.covers_requested.contains(url) {
            return;
        }
        let Some(client) = self.client.clone() else { return };

        self.covers_requested.insert(url.to_owned());
        let tx = self.tx.clone();
        let url = url.to_owned();
        self.rt.spawn(async move {
            let res = api::cover_bytes(&client, &url).await;
            let _ = tx.send(Msg::Cover(url, res));
        });
    }

    fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Login(Ok((acc, pls))) => {
                    self.login_busy = false;
                    self.login_error = None;
                    let uid = acc.uid;
                    self.account = Some(acc);

                    // коллекция «Мне нравится» — это плейлист kind = 3
                    self.liked_playlist = pls
                        .iter()
                        .find(|p| p.kind == 3 && p.uid == uid)
                        .cloned();
                    self.playlists = pls;
                    self.cfg.token = Some(self.token.clone());
                    if let Err(e) = self.cfg.save() {
                        self.banner = Some(format!("Токен не сохранился: {e}"));
                    }
                    self.reload_liked();
                    let covers: Vec<Option<String>> =
                        self.playlists.iter().map(|p| p.cover.clone()).collect();
                    for c in covers {
                        self.request_cover(c.as_deref());
                    }
                    let liked_cover = self.liked_playlist.as_ref().and_then(|p| p.cover.clone());
                    self.request_cover(liked_cover.as_deref());
                    if !self.playlists.is_empty() {
                        self.load_playlist(0);
                    }
                    // «включить музыку при запуске» перебивает стартовый плейлист
                    self.start_autoplay();
                }
                Msg::Login(Err(e)) => {
                    self.login_busy = false;
                    self.login_error = Some(e);
                    self.account = None;
                }
                Msg::Tracks(idx, res) => {
                    if self.active_playlist != Some(idx) {
                        continue;
                    }
                    self.tracks_busy = false;
                    match res {
                        Ok(list) => {
                            self.tracks = list;
                            self.current = None;
                            let covers: Vec<Option<String>> =
                                self.tracks.iter().take(60).map(|t| t.cover.clone()).collect();
                            for c in covers {
                                self.request_cover(c.as_deref());
                            }
                        }
                        Err(e) => {
                            self.tracks.clear();
                            self.banner = Some(format!("Не удалось загрузить плейлист: {e}"));
                        }
                    }
                }
                Msg::Audio(id, res) => {
                    let still_wanted = self
                        .current
                        .and_then(|i| self.tracks.get(i))
                        .map(|t| t.id == id)
                        .unwrap_or(false);
                    if !still_wanted {
                        continue;
                    }
                    self.audio_busy = None;
                    match res {
                        Ok(bytes) => {
                            let duration = self
                                .current
                                .and_then(|i| self.tracks.get(i))
                                .map(|t| t.duration)
                                .unwrap_or(Duration::ZERO);
                            self.audio
                                .load(Arc::<[u8]>::from(bytes.as_slice()), duration);
                        }
                        Err(e) => self.banner = Some(format!("Не удалось получить трек: {e}")),
                    }
                }
                Msg::Liked(res) => match res {
                    Ok(list) => {
                        self.likes = list.iter().map(|t| t.id.clone()).collect();
                        if let Some(pl) = self.liked_playlist.as_mut() {
                            pl.count = list.len() as u32;
                        }
                        self.liked = list.clone();
                        if self.view == View::Liked {
                            self.tracks = list;
                            self.current = None;
                            self.request_covers();
                        }
                        if self.autoplay_pending {
                            self.autoplay_pending = false;
                            if self.view == View::Liked && !self.tracks.is_empty() {
                                self.play_index(0);
                            }
                        }
                    }
                    Err(e) => {
                        self.banner = Some(format!("Не удалось загрузить «Мне нравится»: {e}"))
                    }
                },
                Msg::Wave(res) => {
                    self.wave_busy = false;
                    match res {
                        Ok(list) if !list.is_empty() => {
                            self.tracks = list;
                            self.current = None;
                            self.request_covers();
                            if self.wave_autoplay {
                                self.wave_autoplay = false;
                                self.play_index(0);
                            }
                        }
                        Ok(_) => self.banner = Some("Волна пришла пустой".to_owned()),
                        Err(e) => {
                            self.wave_autoplay = false;
                            self.banner = Some(format!("Не удалось запустить волну: {e}"));
                        }
                    }
                }
                Msg::Search(res) => {
                    self.search_busy = false;
                    match res {
                        Ok(list) => {
                            self.tracks = list;
                            self.current = None;
                            self.request_covers();
                        }
                        Err(e) => self.banner = Some(format!("Поиск не удался: {e}")),
                    }
                }
                Msg::Like(id, liked, res) => {
                    if let Err(e) = res {
                        if liked {
                            self.likes.remove(&id);
                            self.liked.retain(|t| t.id != id);
                        } else {
                            self.likes.insert(id);
                        }
                        self.banner = Some(format!("Лайк не сохранился: {e}"));
                    }
                }
                Msg::Cover(url, res) => match res {
                    Ok(bytes) => {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            let rgba = img.to_rgba8();
                            let accent = cover_accent(&rgba);
                            self.cover_color.insert(url.clone(), accent);

                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let color =
                                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                            let tex = ctx.load_texture(
                                format!("cover:{url}"),
                                color,
                                egui::TextureOptions::LINEAR,
                            );
                            self.covers.insert(url, tex);
                            self.apply_cover_auto_color(ctx);
                        }
                    }
                    Err(_) => {
                        self.covers_requested.remove(&url);
                    }
                },
            }
        }

        if self.audio.take_finished() {
            self.next_track();
        }

        if self.banner.is_none() {
            if let Some(e) = self.audio.error() {
                self.banner = Some(e);
            }
        }
    }

    fn hotkeys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (space, left, right, ctrl) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.modifiers.ctrl,
            )
        });

        if space {
            self.toggle();
        }
        if left {
            if ctrl {
                self.prev_track();
            } else {
                self.seeks_by(-5);
            }
        }
        if right {
            if ctrl {
                self.next_track();
            } else {
                self.seeks_by(5);
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.phase += ctx.input(|i| i.stable_dt);
        self.poll_messages(&ctx);
        self.hotkeys(&ctx);
        // отложенная запись конфига (не чаще двух раз в секунду)
        self.flush_cfg(false);
        // рамка окна (цвет заголовка, скругление) — под палитру
        self.sync_window_style();

        if self.account.is_none() {
            self.ui_login(ui);
        } else {
            self.ui_sidebar(ui);
            self.ui_player_bar(ui);
            self.ui_content(ui);
            self.ui_queue_window(&ctx);

            if self.view == View::Wave {
                // анимация фона должна идти всегда, даже без воспроизведения
                ctx.request_repaint_after(Duration::from_millis(33));
            }
        }

        let snap = self.snapshot();
        if snap.playing
            || self.tracks_busy
            || self.audio_busy.is_some()
            || self.seeking.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        if self.shot_path.is_some() && !self.shot_taken {
            ctx.request_repaint();
        }

        self.frames += 1;
        if let Some(path) = self.shot_path.clone() {
            if !self.shot_taken && self.frames > 20 {
                self.shot_taken = true;
                let ctx2 = ctx.clone();
                ctx.request_screenshot(move |img| {
                    let [w, h] = img.size;
                    let mut buf = Vec::with_capacity(w * h * 4);
                    for px in img.pixels.iter() {
                        let [r, g, b, a] = px.to_array();
                        buf.extend_from_slice(&[r, g, b, a]);
                    }
                    if let Some(bitmap) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                        let _ = bitmap.save(&path);
                    }
                    ctx2.send_viewport_cmd(egui::ViewportCommand::Close);
                });
            }
        }
    }
}

impl Drop for App {
    /// Последний шанс дописать настройки — при выходе.
    fn drop(&mut self) {
        if self.dirty {
            let _ = self.cfg.save();
        }
    }
}
