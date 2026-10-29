//! Экран настроек: все цвета интерфейса, пресеты и автоцвет.

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense, Stroke};

use crate::app::App;
use crate::config::{playlist_key, AutoColor, AutoplaySource};
use crate::theme as t;

impl App {
    pub fn ui_settings(&mut self, ui: &mut egui::Ui) {
        let mut palette = t::palette();
        let mut changed = false;
        let mut auto = self.cfg.auto_color;
        let mut preset: Option<t::Palette> = None;
        let mut autoplay = self.cfg.autoplay;
        let mut source = self.cfg.autoplay_source;
        let mut hidden = self.cfg.hidden_playlists.clone();
        let save_status = self.save_status.clone();
        let path = self.save_path_label();

        // (ключ, название, виден ли) — снимок, чтобы не спорить с заимствованием
        let playlists: Vec<(String, String, bool)> = self
            .playlists
            .iter()
            .map(|p| {
                let key = playlist_key(p.uid, p.kind);
                let visible = !self.cfg.hidden_playlists.contains(&key);
                (key, p.title.clone(), visible)
            })
            .collect();

        egui::CentralPanel::no_frame()
            .frame(
                egui::Frame::NONE
                    .fill(t::bg())
                    .inner_margin(Margin {
                        left: 24,
                        right: 24,
                        top: 20,
                        bottom: 0,
                    }),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Настройки").font(t::bold(24.0)).color(t::text()));
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new("Изменения применяются сразу и сохраняются в конфиг")
                                .font(t::font(12.0))
                                .color(t::text_dim()),
                        );

                        // где именно лежит конфиг — чтобы не гадать, сохранилось ли
                        let (status_text, status_color) = match &save_status {
                            Some(Ok(p)) => (
                                format!("сохранено: {}", p.display()),
                                t::text_faint(),
                            ),
                            Some(Err(e)) => (format!("ошибка записи: {e}"), t::danger()),
                            None => (format!("конфиг: {path}"), t::text_faint()),
                        };
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(status_text)
                                    .font(t::font(10.5))
                                    .color(status_color),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "fluxia {}",
                                            env!("CARGO_PKG_VERSION")
                                        ))
                                        .font(t::font(10.5))
                                        .color(t::text_faint()),
                                    );
                                },
                            );
                        });
                        ui.add_space(16.0);

                        // две колонки: слева цвета, справа поведение
                        ui.columns(2, |cols| {
                        {
                        let ui = &mut cols[0];

                        // ── пресеты ─────────────────────────────────────
                        section(ui, "ПРЕСЕТЫ");
                        ui.horizontal(|ui| {
                            if preset_button(ui, "Тёмная") {
                                preset = Some(t::Palette::dark());
                            }
                            if preset_button(ui, "Светлая") {
                                preset = Some(t::Palette::light());
                            }
                            if preset_button(ui, "Контраст") {
                                preset = Some(t::Palette::dark().with_accent(Color32::from_rgb(
                                    0, 230, 200,
                                )));
                            }
                        });

                        ui.add_space(16.0);

                        // ── цвета ───────────────────────────────────────
                        section(ui, "ЦВЕТА ИНТЕРФЕЙСА");
                        changed |= color_row(ui, "Фон", &mut palette.bg);
                        changed |= color_row(ui, "Боковая панель фон", &mut palette.panel);
                        changed |= color_row(ui, "Поверхность (карточки, поля)", &mut palette.surface);
                        changed |=
                            color_row(ui, "Поверхность при наведении", &mut palette.surface_hover);
                        changed |= color_row(ui, "Границы и линии", &mut palette.border);
                        changed |= color_row(ui, "Основной текст", &mut palette.text);
                        changed |= color_row(ui, "Тусклый текст", &mut palette.text_dim);
                        changed |= color_row(ui, "Еле заметный текст", &mut palette.text_faint);
                        changed |= color_row(ui, "Акцент", &mut palette.accent);
                        changed |= color_row(ui, "Подложка акцента", &mut palette.accent_deep);
                        changed |= color_row(ui, "Текст на акценте", &mut palette.on_accent);
                        changed |= color_row(ui, "Ошибки", &mut palette.danger);

                        }
                        {
                        let ui = &mut cols[1];

                        // ── автоцвет ────────────────────────────────────
                        section(ui, "АВТОЦВЕТ");
                        let mut enabled = auto != AutoColor::Off;
                        if ui
                            .checkbox(
                                &mut enabled,
                                RichText::new("Включать автоматически")
                                    .font(t::font(13.5))
                                    .color(t::text()),
                            )
                            .changed()
                        {
                            auto = if enabled {
                                AutoColor::Cover
                            } else {
                                AutoColor::Off
                            };
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Источник цвета")
                                    .font(t::font(12.5))
                                    .color(t::text_dim()),
                            );
                            ui.add_enabled_ui(enabled, |ui| {
                                egui::ComboBox::from_id_salt("auto_color_mode")
                                    .selected_text(auto.label())
                                    .width(230.0)
                                    .show_ui(ui, |ui| {
                                        for mode in AutoColor::all() {
                                            if mode == AutoColor::Off {
                                                continue;
                                            }
                                            ui.selectable_value(&mut auto, mode, mode.label());
                                        }
                                    });
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(match auto {
                                AutoColor::Off => "Цвета берутся только из палитры ниже.",
                                AutoColor::Cover => {
                                    "Акцент подстраивается под обложку играющего трека."
                                }
                                AutoColor::TimeOfDay => {
                                    "Днём — светлая палитра, вечером и ночью — тёмная."
                                }
                                AutoColor::Random => "Новый акцент при каждом запуске.",
                            })
                            .font(t::font(11.5))
                            .color(t::text_faint()),
                        );

                        ui.add_space(20.0);

                        // ── при запуске ─────────────────────────────────
                        section(ui, "ПРИ ЗАПУСКЕ");
                        ui.checkbox(
                            &mut autoplay,
                            RichText::new("Сразу включать музыку")
                                .font(t::font(13.0))
                                .color(t::text()),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(autoplay, |ui| {
                                ui.label(
                                    RichText::new("играть из")
                                        .font(t::font(12.5))
                                        .color(t::text_dim()),
                                );
                                egui::ComboBox::from_id_salt("autoplay_source")
                                    .selected_text(
                                        RichText::new(source.label())
                                            .font(t::font(12.5))
                                            .color(t::text()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for s in AutoplaySource::all() {
                                            if ui
                                                .selectable_label(source == s, s.label())
                                                .clicked()
                                            {
                                                source = s;
                                            }
                                        }
                                    });
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(match source {
                                AutoplaySource::Wave => {
                                    "Запуск с «Моей волны» — каждый раз новые треки."
                                }
                                AutoplaySource::Liked => {
                                    "Запуск с «Мне нравится» — начнёт с первого трека коллекции."
                                }
                            })
                            .font(t::font(11.5))
                            .color(t::text_faint()),
                        );

                        ui.add_space(20.0);

                        // ── плейлисты в сайдбаре ────────────────────────
                        section(ui, "ПЛЕЙЛИСТЫ В САЙДБАРЕ");
                        if playlists.is_empty() {
                            ui.label(
                                RichText::new("Плейлистов пока нет")
                                    .font(t::font(12.0))
                                    .color(t::text_faint()),
                            );
                        }
                        for (key, title, mut visible) in playlists.clone() {
                            if ui
                                .checkbox(
                                    &mut visible,
                                    RichText::new(title).font(t::font(12.5)).color(t::text()),
                                )
                                .changed()
                            {
                                hidden.retain(|k| k != &key);
                                if !visible {
                                    hidden.push(key);
                                }
                            }
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Снятая галочка прячет плейлист из сайдбара")
                                .font(t::font(11.5))
                                .color(t::text_faint()),
                        );

                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if preset_button(ui, "Сбросить к тёмной") {
                                preset = Some(t::Palette::dark());
                            }
                        });
                        ui.add_space(24.0);
                        }
                        });
                    });
            });

        if let Some(p) = preset {
            self.apply_theme(ui.ctx(), p);
            return;
        }

        if changed {
            self.apply_theme(ui.ctx(), palette);
        }

        if auto != self.cfg.auto_color {
            self.cfg.auto_color = auto;
            self.touch_cfg();
            self.refresh_auto_color(ui.ctx());
        }

        if autoplay != self.cfg.autoplay || source != self.cfg.autoplay_source {
            self.cfg.autoplay = autoplay;
            self.cfg.autoplay_source = source;
            self.touch_cfg();
        }

        if hidden != self.cfg.hidden_playlists {
            self.cfg.hidden_playlists = hidden;
            self.touch_cfg();
        }

        // пишем сразу, чтобы «переключил и закрыл» ничего не теряло
        self.flush_cfg(true);
    }
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .font(t::bold(10.0))
            .color(t::text_faint()),
    );
    ui.add_space(2.0);
    let y = ui.cursor().min.y;
    ui.painter().hline(
        ui.max_rect().x_range(),
        y,
        Stroke::new(1.0, t::border()),
    );
    ui.add_space(10.0);
}

fn preset_button(ui: &mut egui::Ui, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.painter().layout_no_wrap(label.to_owned(), t::font(12.5), t::text()).size().x + 26.0, 30.0),
        Sense::click(),
    );
    let hover = ui.ctx().animate_bool(resp.id, resp.hovered());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(t::RADIUS),
        crate::ui::blend(t::surface(), t::surface_hover(), hover),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(t::RADIUS),
        Stroke::new(1.0, t::border()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        t::font(12.5),
        t::text(),
    );
    let clicked = resp.clicked();
    if resp.hovered() {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    clicked
}

/// Строка «название — образец цвета». Возвращает true, если цвет поменяли.
fn color_row(ui: &mut egui::Ui, label: &str, color: &mut Color32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).font(t::font(13.0)).color(t::text_dim()));
        let hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let resp = ui.color_edit_button_srgba(color);
            changed |= resp.changed();
            ui.add_space(4.0);
            ui.label(RichText::new(hex).font(t::font(11.0)).color(t::text_faint()));
        });
    });
    changed
}
