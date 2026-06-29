//! File message card — renders an inbound or outbound file message
//! and provides a "save" button that triggers a native save dialog.

use crate::messages::Message;
use crate::ui::theme::{color, font, space};
use eframe::egui::{self, RichText, Ui};

/// What the file card wants the App to do this frame.
pub enum FileAction {
    /// Save the body (already cached locally) to disk via a native dialog.
    SaveLocal { sha256: String, suggested_name: String },
    /// Pull the body from a peer's `addr` via HTTP and save to disk.
    SaveRemote { addr: String, sha256: String, suggested_name: String },
    /// Open the file with the OS's default app after download.
    Open(String),
}

pub fn draw(ui: &mut Ui, msg: &Message, me: &str) -> Vec<FileAction> {
    let mut actions = Vec::new();
    let Some(file) = &msg.file else {
        return actions;
    };

    let is_me = msg.device == me;
    let color = if is_me { color::AMBER } else { color::TEXT };

    ui.add_space(space::SM);

    // Header line
    ui.horizontal_wrapped(|ui| {
        ui.add_space(space::LG);
        ui.label(
            RichText::new(format!("[file] {}", msg.device))
                .font(font::mono(font::XS))
                .color(color::MUTED),
        );
    });

    // Card body
    egui::Frame::none()
        .fill(color::BG_ELEV)
        .stroke(egui::Stroke::new(1.0, color::LINE))
        .inner_margin(egui::Margin::symmetric(space::LG, space::MD))
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - space::LG * 2.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("📄")
                        .font(font::mono(font::LG))
                        .color(color::AMBER),
                );
                ui.add_space(space::MD);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&file.filename)
                            .font(font::mono(font::BASE))
                            .color(color),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} · sha256:{}",
                            humanize_bytes(file.size),
                            &file.sha256[..12.min(file.sha256.len())]
                        ))
                        .font(font::mono(font::XS))
                        .color(color::MUTED),
                    );
                });
            });

            ui.add_space(space::SM);

            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    RichText::new("[ SAVE ]")
                        .font(font::mono(font::SM))
                        .color(color::AMBER),
                )
                .fill(color::BG)
                .stroke(egui::Stroke::new(1.0, color::AMBER));
                if ui.add(save_btn).clicked() {
                    if file.addr.is_empty() || file.addr == "inbound" {
                        // No remote — must be local cache. Defer to App.
                        actions.push(FileAction::SaveLocal {
                            sha256: file.sha256.clone(),
                            suggested_name: file.filename.clone(),
                        });
                    } else {
                        actions.push(FileAction::SaveRemote {
                            addr: file.addr.clone(),
                            sha256: file.sha256.clone(),
                            suggested_name: file.filename.clone(),
                        });
                    }
                }
            });
        });

    actions
}

fn humanize_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", n, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}
