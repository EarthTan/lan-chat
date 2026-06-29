//! Setup overlay — full-window modal that collects a terminal name before
//! the chat UI becomes interactive.

use crate::ui::theme::{color, font, space};
use eframe::egui::{self, Align, Key, Layout, RichText, TextEdit, Ui};

/// Outcome of one frame of the setup overlay.
pub enum SetupOutcome {
    /// User is still filling in.
    Pending,
    /// User submitted a name.
    Submit(String),
}

/// State held by the App.
pub struct SetupState {
    pub visible: bool,
    pub input: String,
    /// Cached hint for the placeholder (set once on first show).
    pub hint: Option<String>,
    /// Boot sequence step counter: 0..=3 (visual only).
    pub boot_step: u8,
}

impl SetupState {
    pub fn new(visible: bool) -> Self {
        Self {
            visible,
            input: String::new(),
            hint: None,
            boot_step: 0,
        }
    }

    /// Pre-fill the input with a suggested name on first show.
    pub fn ensure_hint(&mut self, hint: String) {
        if self.hint.is_none() {
            self.input = hint.clone();
            self.hint = Some(hint);
        }
    }
}

pub fn ui(ui: &mut Ui, state: &mut SetupState) -> SetupOutcome {
    let mut outcome = SetupOutcome::Pending;

    egui::Frame::none()
        .fill(color::BG)
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.add_space(space::XXXL);

                    ui.allocate_ui(egui::vec2(420.0, ui.available_height()), |ui| {
                        // ── Head row: brand + version ─────────────────
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("LAN-CHAT/0")
                                    .font(font::mono(font::SM))
                                    .color(color::TEXT),
                            );
                            ui.add_space(space::SM);
                            let rule = ui.available_width() - 60.0;
                            ui.allocate_ui(egui::vec2(rule.max(0.0), 1.0), |ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width().max(0.0), 1.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 0.0, color::LINE);
                            });
                            ui.label(
                                RichText::new("b.1.0.0")
                                    .font(font::mono(font::XS))
                                    .color(color::MUTED),
                            );
                        });

                        ui.add_space(space::XL);

                        // ── Boot status list ─────────────────────────
                        boot_line(ui, "initializing p2p socket", state.boot_step >= 1);
                        ui.add_space(space::SM);
                        boot_line(ui, "binding 0.0.0.0:4242", state.boot_step >= 2);
                        ui.add_space(space::SM);
                        boot_line(ui, "awaiting peer handshake", state.boot_step >= 3);

                        ui.add_space(space::XL);

                        // ── Input prompt ─────────────────────────────
                        ui.label(
                            RichText::new("identify this terminal:")
                                .font(font::mono(font::XS))
                                .color(color::MUTED),
                        );
                        ui.add_space(space::SM);
                        egui::Frame::none()
                            .show(ui, |ui| {
                                let mut resp_opt: Option<egui::Response> = None;
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("›")
                                            .font(font::mono(font::BASE))
                                            .color(color::AMBER),
                                    );
                                    ui.add_space(space::SM);
                                    let resp = ui.add(
                                        TextEdit::singleline(&mut state.input)
                                            .font(font::mono(font::BASE))
                                            .text_color(color::TEXT)
                                            .hint_text(
                                                RichText::new("name this terminal")
                                                    .font(font::mono(font::BASE))
                                                    .color(color::MUTED),
                                            )
                                            .desired_width(ui.available_width() - space::SM),
                                    );
                                    if state.input.is_empty() {
                                        resp.request_focus();
                                    }
                                    resp_opt = Some(resp);
                                });
                                ui.add_space(space::SM);
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 1.0),
                                    egui::Sense::hover(),
                                );
                                let stroke_color = if resp_opt.map(|r| r.has_focus()).unwrap_or(false) {
                                    color::AMBER
                                } else {
                                    color::LINE
                                };
                                ui.painter().rect_filled(rect, 0.0, stroke_color);
                            });

                        ui.add_space(space::XL);

                        // ── Foot row: hint + button ──────────────────
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("enter accepts · esc dismisses")
                                    .font(font::mono(font::XS))
                                    .color(color::MUTED),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let btn = egui::Button::new(
                                    RichText::new("[ ENTER ↵ ]  connect")
                                        .font(font::mono(font::SM))
                                        .color(color::BG),
                                )
                                .fill(color::AMBER)
                                .stroke(egui::Stroke::NONE);
                                if ui.add(btn).clicked() {
                                    let v = state.input.trim().to_string();
                                    if !v.is_empty() {
                                        outcome = SetupOutcome::Submit(v);
                                    }
                                }
                            });
                        });
                    });
                },
            );
        });

    // Keyboard: Enter submits, Esc dismisses
    if ui.ctx().input(|i| i.key_pressed(Key::Enter)) {
        let v = state.input.trim().to_string();
        if !v.is_empty() {
            outcome = SetupOutcome::Submit(v);
        }
    }

    outcome
}

fn boot_line(ui: &mut Ui, msg: &str, done: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(msg)
                .font(font::mono(font::SM))
                .color(color::MUTED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if done {
                ui.label(
                    RichText::new("[ OK ]")
                        .font(font::mono(font::SM))
                        .color(color::GREEN),
                );
            } else {
                ui.label(
                    RichText::new("[ .. ]")
                        .font(font::mono(font::SM))
                        .color(color::AMBER),
                );
            }
        });
    });
}
