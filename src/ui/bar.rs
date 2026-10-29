//! Нижняя панель: трек, управление, прогресс, громкость.

use std::time::Duration;

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, Margin, Rect, Sense, Stroke,
};

use crate::app::App;
use crate::theme as t;
use crate::ui::blend;

impl App {
    pub fn ui_player_bar(&mut self, ui: &mut egui::Ui) {
        let snap = self.snapshot();
        let current = self.current.and_then(|i| self.tracks.get(i)).cloned();
        let covers = &self.covers;
        let volume = self.volume;
        let busy = self.audio_busy.is_some();

        let mut toggle = false;
        let mut prev = false;
        let mut next = false;
        let mut seek_to: Option<f32> = None;
        let mut vol_to: Option<f32> = None;
        let mut queue_toggle = false;

        egui::Panel::bottom("player")
            .resizable(false)
            .exact_size(t::BAR_H)
            .frame(
                egui::Frame::NONE
                    .fill(t::panel())
                    .inner_margin(Margin::symmetric(20, 15)),
            )
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let cy = rect.top() + 20.0;
                let cx = rect.center().x;
                let painter = ui.painter();

                painter.hline(
                    rect.x_range(),
                    rect.top() - 15.0,
                    Stroke::new(1.0, t::border()),
                );

                // ── текущий трек ────────────────────────────────────────
                let cover_rect =
                    Rect::from_min_size(pos2(rect.left(), cy - 23.0), vec2(46.0, 46.0));
                let tex = current
                    .as_ref()
                    .and_then(|tr| tr.cover.as_ref())
                    .and_then(|u| covers.get(u));
                crate::icons::cover(ui, cover_rect, 6, tex);

                let text_x = cover_rect.right() + 12.0;
                let max_w = 230.0f32.min(rect.width() * 0.28);
                let (title, artist) = match &current {
                    Some(tr) => (tr.title.clone(), tr.artists.clone()),
                    None => ("Ничего не играет".to_owned(), String::new()),
                };
                let title = crate::ui::fit(ui, &title, t::font(13.0), max_w);
                painter.text(
                    pos2(text_x, cy - 8.0),
                    Align2::LEFT_CENTER,
                    title,
                    t::font(13.0),
                    if current.is_some() { t::text() } else { t::text_faint() },
                );
                let artist = crate::ui::fit(ui, &artist, t::font(11.0), max_w);
                painter.text(
                    pos2(text_x, cy + 8.0),
                    Align2::LEFT_CENTER,
                    artist,
                    t::font(11.0),
                    t::text_dim(),
                );

                // ── кнопки ──────────────────────────────────────────────
                let prev_hit = Rect::from_center_size(pos2(cx - 66.0, cy), vec2(34.0, 34.0));
                let prev_resp = ui.interact(prev_hit, ui.scope_id().with("prev"), Sense::click());
                let prev_hover = ui.ctx().animate_bool(prev_resp.id, prev_resp.hovered());
                if prev_hover > 0.01 {
                    painter.circle_filled(
                        prev_hit.center(),
                        14.0,
                        blend(t::panel(), t::surface_hover(), prev_hover),
                    );
                }
                crate::icons::prev(
                    painter,
                    prev_hit.center(),
                    13.0,
                    blend(t::text_dim(), t::text(), prev_hover),
                );
                if prev_resp.clicked() {
                    prev = true;
                }

                let next_hit = Rect::from_center_size(pos2(cx + 66.0, cy), vec2(34.0, 34.0));
                let next_resp = ui.interact(next_hit, ui.scope_id().with("next"), Sense::click());
                let next_hover = ui.ctx().animate_bool(next_resp.id, next_resp.hovered());
                if next_hover > 0.01 {
                    painter.circle_filled(
                        next_hit.center(),
                        14.0,
                        blend(t::panel(), t::surface_hover(), next_hover),
                    );
                }
                crate::icons::next(
                    painter,
                    next_hit.center(),
                    13.0,
                    blend(t::text_dim(), t::text(), next_hover),
                );
                if next_resp.clicked() {
                    next = true;
                }

                let play_hit = Rect::from_center_size(pos2(cx, cy), vec2(46.0, 46.0));
                let play_resp = ui.interact(play_hit, ui.scope_id().with("play"), Sense::click());
                let play_hover = ui.ctx().animate_bool(play_resp.id, play_resp.hovered());
                let fill = if current.is_some() {
                    blend(t::accent(), Color32::from_rgb(255, 232, 158), play_hover)
                } else {
                    t::surface_hover()
                };
                painter.circle_filled(play_hit.center(), 20.0, fill);
                let icon_color = if current.is_some() {
                    t::on_accent()
                } else {
                    t::text_faint()
                };
                if snap.playing {
                    crate::icons::pause(painter, play_hit.center(), 14.0, icon_color);
                } else {
                    crate::icons::play(painter, play_hit.center() + vec2(1.0, 0.0), 14.0, icon_color);
                }
                if play_resp.clicked() {
                    toggle = true;
                }
                if play_resp.hovered() {
                    play_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                }

                // ── прогресс ────────────────────────────────────────────
                let pb_w = (rect.width() * 0.34).clamp(180.0, 460.0);
                let pb_left = cx - pb_w * 0.5;
                let pb_right = pb_left + pb_w;
                let pb_y = rect.bottom() - 4.0;
                let dur = snap.duration.as_secs_f32();

                let hit = Rect::from_min_max(pos2(pb_left, pb_y - 9.0), pos2(pb_right, pb_y + 9.0));
                let resp = ui.interact(hit, ui.scope_id().with("seek"), Sense::click_and_drag());

                let pointer_frac = if resp.dragged() {
                    resp.interact_pointer_pos()
                        .map(|p| ((p.x - pb_left) / pb_w).clamp(0.0, 1.0))
                } else {
                    None
                };
                let frac = pointer_frac.unwrap_or_else(|| {
                    if dur > 0.0 {
                        (snap.position.as_secs_f32() / dur).clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                });

                if dur > 0.0 && (resp.dragged() || resp.clicked()) {
                    if let Some(f) = pointer_frac.or_else(|| {
                        resp.interact_pointer_pos()
                            .map(|p| ((p.x - pb_left) / pb_w).clamp(0.0, 1.0))
                    }) {
                        seek_to = Some(f * dur);
                    }
                }

                let track_rect = Rect::from_min_max(
                    pos2(pb_left, pb_y - 1.5),
                    pos2(pb_right, pb_y + 1.5),
                );
                painter.rect_filled(track_rect, CornerRadius::same(2), t::surface_hover());
                let played = (pb_w * frac).max(0.0);
                if played > 0.5 {
                    painter.rect_filled(
                        Rect::from_min_max(pos2(pb_left, pb_y - 1.5), pos2(pb_left + played, pb_y + 1.5)),
                        CornerRadius::same(2),
                        t::accent(),
                    );
                }
                if resp.hovered() || resp.dragged() {
                    painter.circle_filled(pos2(pb_left + played, pb_y), 5.0, t::accent());
                }

                let elapsed = if let Some(f) = pointer_frac {
                    Duration::from_secs_f32(f * dur)
                } else {
                    snap.position
                };
                painter.text(
                    pos2(pb_left - 8.0, pb_y),
                    Align2::RIGHT_CENTER,
                    crate::api::fmt_duration(elapsed),
                    t::font(10.5),
                    t::text_faint(),
                );
                painter.text(
                    pos2(pb_right + 8.0, pb_y),
                    Align2::LEFT_CENTER,
                    if dur <= 0.0 {
                        "--:--".to_owned()
                    } else {
                        crate::api::fmt_duration(snap.duration)
                    },
                    t::font(10.5),
                    t::text_faint(),
                );

                // ── громкость ───────────────────────────────────────────
                let vw = 78.0;
                let v_left = rect.right() - vw;

                // ── очередь ─────────────────────────────────────────────
                let q_c = pos2(v_left - 52.0, cy);
                let q_hit = Rect::from_center_size(q_c, vec2(26.0, 26.0));
                let q_resp = ui.interact(q_hit, ui.scope_id().with("queue"), Sense::click());
                let q_open = self.show_queue;
                crate::icons::queue(
                    painter,
                    q_c,
                    15.0,
                    if q_open || q_resp.hovered() {
                        t::accent()
                    } else {
                        t::text_dim()
                    },
                );
                if q_resp.clicked() {
                    queue_toggle = true;
                }

                let v_icon_c = pos2(v_left - 20.0, cy);

                let mute_hit = Rect::from_center_size(v_icon_c, vec2(26.0, 26.0));
                let mute_resp = ui.interact(mute_hit, ui.scope_id().with("mute"), Sense::click());
                let muted = volume <= 0.01;
                crate::icons::volume(
                    painter,
                    v_icon_c,
                    15.0,
                    if mute_resp.hovered() { t::text() } else { t::text_dim() },
                    muted,
                );
                if mute_resp.clicked() {
                    vol_to = Some(if muted { 0.7 } else { 0.0 });
                }

                let vhit = Rect::from_min_max(pos2(v_left, cy - 8.0), pos2(v_left + vw, cy + 8.0));
                let vresp = ui.interact(vhit, ui.scope_id().with("vol"), Sense::click_and_drag());
                if (vresp.dragged() || vresp.clicked())
                    && vresp.interact_pointer_pos().is_some()
                {
                    let p = vresp.interact_pointer_pos().unwrap();
                    vol_to = Some(((p.x - v_left) / vw).clamp(0.0, 1.0));
                }

                painter.rect_filled(
                    Rect::from_min_max(pos2(v_left, cy - 1.5), pos2(v_left + vw, cy + 1.5)),
                    CornerRadius::same(2),
                    t::surface_hover(),
                );
                let vfill = vw * volume.clamp(0.0, 1.0);
                if vfill > 0.5 {
                    painter.rect_filled(
                        Rect::from_min_max(pos2(v_left, cy - 1.5), pos2(v_left + vfill, cy + 1.5)),
                        CornerRadius::same(2),
                        t::text_dim(),
                    );
                }
                if vresp.hovered() || vresp.dragged() {
                    painter.circle_filled(pos2(v_left + vfill, cy), 4.5, t::text());
                }

                let _ = busy;
            });

        if toggle {
            self.toggle();
        }
        if prev {
            self.prev_track();
        }
        if next {
            self.next_track();
        }
        if queue_toggle {
            self.show_queue = !self.show_queue;
        }
        if let Some(s) = seek_to {
            self.audio.seek(Duration::from_secs_f32(s.max(0.0)));
        }
        if let Some(v) = vol_to {
            self.volume = v;
            self.audio.set_volume(v);
            self.cfg.volume = v;
            self.touch_cfg();
        }
    }
}
