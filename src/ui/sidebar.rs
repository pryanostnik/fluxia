//! Левая панель: бренд, аккаунт, плейлисты.

use eframe::egui::{self, pos2, vec2, Align2, CornerRadius, Margin, Rect, RichText, Sense};

use crate::app::{App, View};
use crate::theme as t;
use crate::ui::{blend, fit, hairline, avatar};

/// Строка навигации (волна, «мне нравится») — как строка плейлиста, но с иконкой.
fn nav_row(
    ui: &mut egui::Ui,
    selected: bool,
    title: &str,
    subtitle: &str,
    draw_icon: fn(&egui::Painter, eframe::egui::Pos2, f32, eframe::egui::Color32),
    cover: Option<&egui::TextureHandle>,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());

    if selected {
        ui.painter().rect_filled(rect, t::RADIUS, t::surface());
        let bar = Rect::from_min_size(
            pos2(rect.left() + 2.0, rect.center().y - 9.0),
            vec2(3.0, 18.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), t::accent());
    } else if hover > 0.01 {
        ui.painter()
            .rect_filled(rect, t::RADIUS, blend(t::panel(), t::surface_hover(), hover));
    }

    let tile = Rect::from_min_size(
        pos2(rect.left() + 13.0, rect.center().y - 15.0),
        vec2(30.0, 30.0),
    );
    match cover {
        // у коллекции есть настоящая обложка плейлиста — показываем её
        Some(tex) => crate::icons::cover(ui, tile, 5, Some(tex)),
        None => {
            ui.painter().rect_filled(
                tile,
                CornerRadius::same(5),
                if selected { t::accent_deep() } else { t::surface() },
            );
            draw_icon(
                ui.painter(),
                tile.center(),
                14.0,
                if selected {
                    t::accent()
                } else {
                    blend(t::text_dim(), t::text(), hover)
                },
            );
        }
    }

    let text_x = tile.right() + 11.0;
    let max_w = rect.right() - text_x - 8.0;
    let title_color = if selected {
        t::text()
    } else {
        blend(t::text_dim(), t::text(), hover)
    };
    let title = fit(ui, title, t::font(13.0), max_w);
    ui.painter().text(
        pos2(text_x, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        title,
        t::font(13.0),
        title_color,
    );
    let subtitle = fit(ui, subtitle, t::font(11.0), max_w);
    ui.painter().text(
        pos2(text_x, rect.center().y + 8.0),
        Align2::LEFT_CENTER,
        subtitle,
        t::font(11.0),
        t::text_faint(),
    );

    let clicked = resp.clicked();
    if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    clicked
}

impl App {
    pub fn ui_sidebar(&mut self, ui: &mut egui::Ui) {
        let playlists = self.playlists.clone();
        let account = self.account.clone();
        let active = self.active_playlist;
        let covers = &self.covers;
        let logo = self.logo.clone();
        let view = self.view;
        let liked_pl = self.liked_playlist.clone();
        let likes_count = liked_pl
            .as_ref()
            .map(|p| p.count as usize)
            .unwrap_or_else(|| self.liked.len());
        let mut nav: Option<View> = None;
        let mut hide_pl: Option<String> = None;
        let hidden: Vec<String> = self.cfg.hidden_playlists.clone();

        let clicked = egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(t::SIDEBAR_W)
            .frame(
                egui::Frame::NONE
                    .fill(t::panel())
                    .inner_margin(Margin::symmetric(12, 16)),
            )
            .show(ui, |ui| -> Option<usize> {
                let mut clicked: Option<usize> = None;

                // бренд
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                    crate::icons::logo_tile(ui, rect, 6, logo.as_ref(), 2.0);
                    ui.add_space(2.0);
                    ui.label(RichText::new("fluxia").font(t::bold(16.0)).color(t::text()));
                    ui.add_space(2.0);
                    ui.label(RichText::new("музыка").font(t::font(11.0)).color(t::text_faint()));
                });

                ui.add_space(20.0);

                // аккаунт
                if let Some(acc) = &account {
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
                        avatar(ui, rect.center(), 15.0, acc.name.chars().next().unwrap_or('?'));
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.add_space(1.0);
                            let name =
                                fit(ui, &acc.name, t::font(13.0), t::SIDEBAR_W - 90.0);
                            ui.label(
                                RichText::new(name).font(t::font(13.0)).color(t::text()),
                            );
                            ui.label(
                                RichText::new(format!("{} плейлистов", playlists.len()))
                                    .font(t::font(11.0))
                                    .color(t::text_faint()),
                            );
                        });
                    });
                }

                ui.add_space(16.0);
                hairline(ui, ui.cursor().min.y);
                ui.add_space(10.0);

                if nav_row(
                    ui,
                    view == View::Wave,
                    "Моя волна",
                    "персональная станция",
                    crate::icons::wave,
                    None,
                ) {
                    nav = Some(View::Wave);
                }
                let liked_tex = liked_pl
                    .as_ref()
                    .and_then(|p| p.cover.as_ref())
                    .and_then(|c| covers.get(c));
                if nav_row(
                    ui,
                    view == View::Liked,
                    "Мне нравится",
                    &format!("{} треков", likes_count),
                    crate::icons::heart,
                    liked_tex,
                ) {
                    nav = Some(View::Liked);
                }

                ui.add_space(12.0);
                if nav_row(
                    ui,
                    view == View::Settings,
                    "Настройки",
                    "цвета, автоцвет",
                    crate::icons::sliders,
                    None,
                ) {
                    nav = Some(View::Settings);
                }

                ui.add_space(12.0);
                ui.label(
                    RichText::new("ПЛЕЙЛИСТЫ")
                        .font(t::bold(10.0))
                        .color(t::text_faint()),
                );
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, pl) in playlists.iter().enumerate() {
                            // коллекция уже есть отдельной строкой выше — не дублируем
                            if liked_pl
                                .as_ref()
                                .map_or(false, |lp| lp.kind == pl.kind && lp.uid == pl.uid)
                            {
                                continue;
                            }
                            // спрятанные в настройках не показываем вовсе
                            let key = crate::config::playlist_key(pl.uid, pl.kind);
                            if hidden.contains(&key) {
                                continue;
                            }

                            let (rect, resp) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 46.0),
                                Sense::click(),
                            );
                            let selected = active == Some(i);
                            let hover = ui.ctx().animate_bool(resp.id, resp.hovered());

                            if selected {
                                ui.painter().rect_filled(rect, t::RADIUS, t::surface());
                                let bar = Rect::from_min_size(
                                    pos2(rect.left() + 2.0, rect.center().y - 9.0),
                                    vec2(3.0, 18.0),
                                );
                                ui.painter()
                                    .rect_filled(bar, CornerRadius::same(2), t::accent());
                            } else if hover > 0.01 {
                                ui.painter().rect_filled(
                                    rect,
                                    t::RADIUS,
                                    blend(t::panel(), t::surface_hover(), hover),
                                );
                            }

                            let cover_rect = Rect::from_min_size(
                                pos2(rect.left() + 13.0, rect.center().y - 15.0),
                                vec2(30.0, 30.0),
                            );
                            let tex = pl
                                .cover
                                .as_ref()
                                .and_then(|c| covers.get(c));
                            crate::icons::cover(ui, cover_rect, 5, tex);

                            let text_x = cover_rect.right() + 11.0;
                            let max_w = rect.right() - text_x - 8.0;
                            let title_color = if selected {
                                t::text()
                            } else {
                                blend(t::text_dim(), t::text(), hover)
                            };
                            let title = fit(ui, &pl.title, t::font(13.0), max_w);
                            ui.painter().text(
                                pos2(text_x, rect.center().y - 8.0),
                                Align2::LEFT_CENTER,
                                title,
                                t::font(13.0),
                                title_color,
                            );
                            ui.painter().text(
                                pos2(text_x, rect.center().y + 8.0),
                                Align2::LEFT_CENTER,
                                format!("{} треков", pl.count),
                                t::font(11.0),
                                t::text_faint(),
                            );

                            let was_clicked = resp.clicked();
                            let was_hovered = resp.hovered();
                            if was_clicked {
                                clicked = Some(i);
                            }
                            if was_hovered {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            // правый клик — спрятать плейлист
                            let key = crate::config::playlist_key(pl.uid, pl.kind);
                            resp.context_menu(|ui| {
                                ui.set_min_width(150.0);
                                if ui.button("Скрыть плейлист").clicked() {
                                    hide_pl = Some(key.clone());
                                    ui.close();
                                }
                            });
                        }
                    });

                clicked
            })
            .inner;

        match nav {
            Some(View::Wave) => self.open_wave(),
            Some(View::Liked) => self.open_liked(),
            Some(View::Settings) => self.open_settings(),
            _ => {}
        }

        if let Some(key) = hide_pl {
            if !self.cfg.hidden_playlists.contains(&key) {
                self.cfg.hidden_playlists.push(key);
                self.touch_cfg();
            }
        }

        if let Some(i) = clicked {
            self.load_playlist(i);
        }
    }
}
