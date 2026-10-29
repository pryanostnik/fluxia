//! Экран входа по OAuth-токену.

use eframe::egui::{self, vec2, Align, CornerRadius, Layout, Margin, RichText, Sense, Stroke};
use eframe::egui::Color32;

use crate::app::App;
use crate::theme as t;

impl App {
    pub fn ui_login(&mut self, ui: &mut egui::Ui) {
        let mut do_login = false;

        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.fill(t::bg()))
            .show(ui, |ui| {
                let top = ((ui.available_height() - 420.0) * 0.5).max(24.0);
                ui.add_space(top);

                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), 0.0),
                    Layout::top_down(Align::Center),
                    |ui| {
                        ui.set_width(420.0);

                        // логотип
                        let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
                        crate::icons::logo_tile(ui, rect, 16, self.logo.as_ref(), 6.0);

                        ui.add_space(18.0);
                        ui.label(
                            RichText::new("fluxia").font(t::bold(26.0)).color(t::text()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Клиент Яндекс Музыки. Вставьте OAuth-токен, чтобы войти")
                                .font(t::font(13.0))
                                .color(t::text_dim()),
                        );
                        ui.add_space(24.0);

                        // поле токена
                        let frame = egui::Frame::NONE
                            .fill(t::surface())
                            .stroke(Stroke::new(1.0, t::border()))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(12, 10));

                        let field = frame.show(ui, |ui| {
                            ui.set_width(392.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.token_input)
                                    .hint_text("y0_AgAAAAB…  (Ctrl+V)")
                                    .desired_width(f32::INFINITY)
                                    .font(t::font(13.0))
                                    .margin(Margin::ZERO)
                                    .frame(egui::Frame::NONE),
                            )
                        });

                        // сразу ставим курсор в поле, чтобы можно было жать Ctrl+V
                        if !self.login_focused {
                            field.inner.request_focus();
                            self.login_focused = true;
                        }

                        ui.add_space(6.0);

                        // кнопка-фолбэк: читаем буфер напрямую, без хоткея
                        ui.horizontal(|ui| {
                            let paste = egui::Button::new(
                                RichText::new("Вставить из буфера").font(t::font(12.0)),
                            )
                            .frame(false);
                            if ui.add(paste).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(text) = cb.get_text() {
                                        let cleaned = text.trim().to_owned();
                                        if !cleaned.is_empty() {
                                            self.token_input = cleaned;
                                        }
                                    }
                                }
                            }
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("или Ctrl+V в поле")
                                    .font(t::font(12.0))
                                    .color(t::text_faint()),
                            );
                        });

                        ui.add_space(10.0);

                        let width = ui.available_width();
                        let btn = egui::Button::new(
                            RichText::new(if self.login_busy {
                                "Подключаюсь…"
                            } else {
                                "Войти"
                            })
                            .font(t::bold(14.0))
                            .color(t::on_accent()),
                        )
                        .fill(t::accent())
                        .corner_radius(CornerRadius::same(10))
                        .min_size(vec2(width, 40.0));

                        if ui.add(btn).clicked() && !self.login_busy {
                            do_login = true;
                        }

                        let enter =
                            field.inner.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if enter && !self.login_busy {
                            do_login = true;
                        }

                        if let Some(err) = &self.login_error {
                            ui.add_space(10.0);
                            ui.label(RichText::new(err).font(t::font(12.0)).color(t::danger()));
                        }

                        ui.add_space(18.0);
                        ui.label(
                            RichText::new(
                                "Токен можно взять в своём аккаунте Яндекса (OAuth). \
                                 Он сохранится локально в %APPDATA%\\fluxia\\config.json",
                            )
                            .font(t::font(11.0))
                            .color(Color32::from_rgb(70, 70, 80)),
                        );
                    },
                );
            });

        if do_login {
            let token = self.token_input.clone();
            self.start_login(token);
        }
    }
}
