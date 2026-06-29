//! Clipboard share modal — pre-filled with system clipboard, edit, send.

use crate::ui::theme::{color, font, space};
use eframe::egui::{self, Align, Key, Layout, RichText, TextEdit, Ui};

pub struct ClipModal {
    pub open: bool,
    pub text: String,
}

impl ClipModal {
    pub fn new() -> Self {
        Self {
            open: false,
            text: String::new(),
        }
    }

    /// Open the modal and pre-fill with current system clipboard content.
    pub fn open(&mut self) {
        self.text.clear();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(s) = cb.get_text() {
                self.text = s;
            }
        }
        self.open = true;
    }
}

pub enum ClipOutcome {
    Pending,
    Send(String),
    Close,
}

pub fn ui(ui: &mut Ui, modal: &mut ClipModal) -> ClipOutcome {
    let mut outcome = ClipOutcome::Pending;

    egui::Frame::none()
        .fill(color::BG_ELEV)
        .stroke(egui::Stroke::new(1.0, color::LINE))
        .inner_margin(egui::Margin::same(space::XL))
        .show(ui, |ui| {
            // Head
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("CLIPBOARD.SHARE")
                        .font(font::mono(font::SM))
                        .color(color::TEXT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("×")
                                    .font(font::mono(font::LG))
                                    .color(color::MUTED),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        outcome = ClipOutcome::Close;
                    }
                });
            });

            ui.add_space(space::LG);

            // Textarea
            let resp = ui.add(
                TextEdit::multiline(&mut modal.text)
                    .font(font::mono(font::BASE))
                    .text_color(color::TEXT)
                    .hint_text(
                        RichText::new("paste anything — code, url, notes. ⌘V / Ctrl+V")
                            .font(font::mono(font::BASE))
                            .color(color::MUTED),
                    )
                    .desired_rows(8)
                    .desired_width(ui.available_width()),
            );
            // Border styling — draw a dashed amber border
            let rect = resp.rect;
            ui.painter().rect_stroke(
                rect.expand(2.0),
                0.0,
                egui::Stroke::new(1.0, color::AMBER),
            );
            if modal.open {
                resp.request_focus();
            }

            ui.add_space(space::LG);

            // Foot
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("no network egress · LAN only")
                        .font(font::mono(font::XS))
                        .color(color::MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let send_btn = egui::Button::new(
                        RichText::new("[ SEND ↵ ]")
                            .font(font::mono(font::SM))
                            .color(color::BG),
                    )
                    .fill(color::AMBER)
                    .stroke(egui::Stroke::NONE);
                    if ui.add(send_btn).clicked() {
                        let t = modal.text.trim().to_string();
                        if !t.is_empty() {
                            outcome = ClipOutcome::Send(t);
                        }
                    }
                    ui.add_space(space::SM);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("[ CANCEL ]")
                                    .font(font::mono(font::SM))
                                    .color(color::MUTED),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        outcome = ClipOutcome::Close;
                    }
                });
            });
        });

    // Esc closes
    if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
        outcome = ClipOutcome::Close;
    }
    // Ctrl+Enter sends
    if ui.ctx().input(|i| i.modifiers.ctrl && i.key_pressed(Key::Enter)) {
        let t = modal.text.trim().to_string();
        if !t.is_empty() {
            outcome = ClipOutcome::Send(t);
        }
    }

    outcome
}
