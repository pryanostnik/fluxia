//! Правая часть: заголовок, поиск и список треков.

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke,
};

use crate::app::{App, View};
use crate::theme as t;
use crate::ui::{blend, fit, human_duration, plural_tracks};

impl App {
    pub fn ui_content(&mut self, ui: &mut egui::Ui) {
        if self.view == View::Settings {
            return self.ui_settings(ui);
        }

        let tracks = &self.tracks;
        let covers = &self.covers;
        let likes = &self.likes;
        let current = self.current;
        let playing = self.snapshot().playing;
        let phase = self.phase;
        let busy = self.tracks_busy || self.search_busy || self.wave_busy;
        let audio_busy = self.audio_busy.is_some();
        let banner = self.banner.clone();
        let view = self.view;
        let search_query = self.search_query.clone();
        let mut search_input = self.search_input.clone();
        // берём анимацию «на время», чтобы не конфликтовать с заимствованием списка
        let mut pipes = std::mem::take(&mut self.pipes);

        let playlist = self.active_playlist.and_then(|i| self.playlists.get(i));
        let total_secs: u64 = tracks.iter().map(|t| t.duration.as_secs()).sum();
        let count = tracks.len();

        let (title, subtitle) = match view {
            View::Playlist => {
                let title = playlist.map(|p| p.title.clone()).unwrap_or_default();
                let owner = playlist.map(|p| p.owner.clone()).unwrap_or_default();
                let meta = if count == 0 {
                    owner
                } else {
                    format!(
                        "{} {} · {}{}",
                        count,
                        plural_tracks(count),
                        human_duration(total_secs),
                        if owner.is_empty() {
                            String::new()
                        } else {
                            format!(" · {owner}")
                        }
                    )
                };
                (title, meta)
            }
            View::Liked => {
                let meta = if count == 0 {
                    "Пока ничего не лайкнуто".to_owned()
                } else {
                    format!("{} {} · {}", count, plural_tracks(count), human_duration(total_secs))
                };
                ("Мне нравится".to_owned(), meta)
            }
            View::Wave => {
                let meta = if busy && count == 0 {
                    "Подбираю треки…".to_owned()
                } else {
                    format!("{} {} · станция не заканчивается", count, plural_tracks(count))
                };
                ("Моя волна".to_owned(), meta)
            }
            View::Search => {
                let meta = if search_query.is_empty() {
                    "Введите запрос и нажмите Enter".to_owned()
                } else if busy {
                    "Ищу…".to_owned()
                } else {
                    format!("«{}» · {} {}", search_query, count, plural_tracks(count))
                };
                ("Поиск".to_owned(), meta)
            }
            View::Settings => (String::new(), String::new()),
        };

        let mut clicked: Option<usize> = None;
        let mut play_all = false;
        let mut like_clicked: Option<usize> = None;
        let mut do_search = false;

        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.fill(t::bg()).inner_margin(Margin {
                left: 24,
                right: 16,
                top: 20,
                bottom: 0,
            }))
            .show(ui, |ui| {
                // ── шапка ────────────────────────────────────────────────
                ui.horizontal(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::click());
                    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
                    let fill = blend(t::accent(), Color32::from_rgb(255, 230, 150), hover);
                    ui.painter().circle_filled(rect.center(), 23.0, fill);
                    crate::icons::play(ui.painter(), rect.center(), 15.0, t::on_accent());
                    if resp.clicked() && !tracks.is_empty() {
                        play_all = true;
                    }

                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        let w = (ui.available_width() - 250.0).max(80.0);
                        let name = fit(ui, &title, t::bold(24.0), w);
                        ui.label(RichText::new(name).font(t::bold(24.0)).color(t::text()));
                        let meta = fit(ui, &subtitle, t::font(12.0), w);
                        ui.label(RichText::new(meta).font(t::font(12.0)).color(t::text_dim()));
                    });

                    // ── поиск ────────────────────────────────────────────
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let frame = egui::Frame::NONE
                            .fill(t::surface())
                            .stroke(Stroke::new(1.0, t::border()))
                            .corner_radius(CornerRadius::same(9))
                            .inner_margin(Margin::symmetric(10, 7));
                        frame.show(ui, |ui| {
                            ui.set_width(210.0);
                            ui.horizontal(|ui| {
                                let (ico, _) =
                                    ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                                crate::icons::search(
                                    ui.painter(),
                                    ico.center(),
                                    14.0,
                                    t::text_faint(),
                                );
                                ui.add_space(2.0);
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut search_input)
                                        .hint_text("Поиск")
                                        .desired_width(f32::INFINITY)
                                        .font(t::font(13.0))
                                        .margin(Margin::ZERO)
                                        .frame(egui::Frame::NONE),
                                );
                                if resp.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    do_search = true;
                                }
                            });
                        });
                    });
                });

                ui.add_space(12.0);

                // фон «Моей волны»: анимированные трубы
                if view == View::Wave {
                    let area = ui.available_rect_before_wrap();
                    let dt = ui.ctx().input(|i| i.stable_dt).min(0.1);
                    pipes.update(area, dt);
                    pipes.draw(ui.painter(), t::accent());
                }

                if let Some(b) = &banner {
                    ui.label(RichText::new(b).font(t::font(12.0)).color(t::danger()));
                    ui.add_space(6.0);
                }

                // ── список ───────────────────────────────────────────────
                if tracks.is_empty() {
                    ui.add_space(100.0);
                    ui.vertical_centered(|ui| {
                        if busy {
                            ui.spinner();
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Загружаю…")
                                    .font(t::font(13.0))
                                    .color(t::text_faint()),
                            );
                        } else {
                            let text = match view {
                                View::Liked => "Пока ничего не лайкнуто",
                                View::Wave => "Волна пока пустая",
                                View::Search => {
                                    if search_query.is_empty() {
                                        "Введите запрос в поле справа"
                                    } else {
                                        "Ничего не нашлось"
                                    }
                                }
                                View::Playlist => "Здесь пока пусто",
                                View::Settings => "",
                            };
                            ui.label(RichText::new(text).font(t::font(13.0)).color(t::text_faint()));
                        }
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_rows(ui, t::ROW_H, tracks.len(), |ui, range| {
                            for i in range {
                                let track = &tracks[i];
                                let (rect, resp) = ui.allocate_exact_size(
                                    vec2(ui.available_width(), t::ROW_H),
                                    Sense::click(),
                                );
                                let is_current = current == Some(i);
                                let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
                                let row = rect.shrink2(vec2(4.0, 1.0));
                                let painter = ui.painter();

                                if is_current {
                                    painter.rect_filled(row, t::RADIUS, t::accent_deep());
                                } else if hover > 0.01 {
                                    painter.rect_filled(
                                        row,
                                        t::RADIUS,
                                        blend(t::bg(), t::surface_hover(), hover),
                                    );
                                }

                                // индекс / иконка / эквалайзер
                                let idx_center = pos2(rect.left() + 16.0, rect.center().y);
                                let is_loading = audio_busy && is_current;
                                if is_current && playing && !is_loading {
                                    crate::icons::equalizer(
                                        painter,
                                        idx_center,
                                        12.0,
                                        t::accent(),
                                        phase,
                                    );
                                } else if resp.hovered() {
                                    crate::icons::play(
                                        painter,
                                        idx_center,
                                        10.0,
                                        if is_current { t::accent() } else { t::text() },
                                    );
                                } else {
                                    painter.text(
                                        idx_center,
                                        Align2::CENTER_CENTER,
                                        format!("{}", i + 1),
                                        t::font(11.0),
                                        if is_current { t::accent() } else { t::text_faint() },
                                    );
                                }

                                // обложка
                                let cover_rect = Rect::from_min_size(
                                    pos2(rect.left() + 34.0, rect.center().y - 16.0),
                                    vec2(32.0, 32.0),
                                );
                                let tex = track.cover.as_ref().and_then(|c| covers.get(c));
                                crate::icons::cover(ui, cover_rect, 4, tex);

                                // текст
                                let text_x = cover_rect.right() + 12.0;
                                let dur_w = 46.0;
                                // оставляем место под сердечко, чтобы оно не лезло на время
                                let max_w = (rect.right() - dur_w - 72.0 - text_x).max(40.0);
                                let title_color = if is_current {
                                    t::accent()
                                } else if track.available {
                                    t::text()
                                } else {
                                    t::text_faint()
                                };
                                let title = fit(ui, &track.title, t::font(13.5), max_w);
                                painter.text(
                                    pos2(text_x, rect.center().y - 8.0),
                                    Align2::LEFT_CENTER,
                                    title,
                                    t::font(13.5),
                                    title_color,
                                );
                                let artist = fit(ui, &track.artists, t::font(11.5), max_w);
                                painter.text(
                                    pos2(text_x, rect.center().y + 9.0),
                                    Align2::LEFT_CENTER,
                                    artist,
                                    t::font(11.5),
                                    t::text_dim(),
                                );

                                // длительность
                                painter.text(
                                    pos2(rect.right() - 12.0, rect.center().y),
                                    Align2::RIGHT_CENTER,
                                    crate::api::fmt_duration(track.duration),
                                    t::font(11.5),
                                    t::text_faint(),
                                );

                                // лайк
                                let liked = likes.contains(&track.id);
                                let heart_hit = Rect::from_center_size(
                                    pos2(rect.right() - 62.0, rect.center().y),
                                    vec2(26.0, 26.0),
                                );
                                let heart_resp = ui.interact(
                                    heart_hit,
                                    ui.scope_id().with(("like", i)),
                                    Sense::click(),
                                );
                                if liked || heart_resp.hovered() || hover > 0.01 {
                                    let color = if liked {
                                        t::accent()
                                    } else if heart_resp.hovered() {
                                        t::text()
                                    } else {
                                        t::text_faint()
                                    };
                                    crate::icons::heart(
                                        painter,
                                        heart_hit.center(),
                                        13.0,
                                        color,
                                    );
                                }
                                if heart_resp.clicked() {
                                    like_clicked = Some(i);
                                } else if resp.clicked() {
                                    clicked = Some(i);
                                }
                                if resp.hovered() {
                                    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                }
                            }
                        });
                }
            });

        if search_input != self.search_input {
            self.search_input = search_input;
        }
        self.pipes = pipes;

        if do_search {
            self.run_search();
        } else if let Some(i) = like_clicked {
            self.toggle_like(i);
        } else if play_all {
            self.play_index(0);
        } else if let Some(i) = clicked {
            self.play_index(i);
        }
    }
}
