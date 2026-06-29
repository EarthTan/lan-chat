// lan-chat/src-tauri/src/app.rs
//
// The eframe::App implementation — drives the entire UI tree.

use crate::messages::{Message, MsgType};
use crate::peers::PeerInfo;
use crate::server::UiEvent;
use crate::ui::setup::{SetupOutcome, SetupState};
use crate::ui::theme::{color, font, space};
use crate::{commands, AppHandle};
use eframe::egui::{self, Align, CentralPanel, Key, Layout, RichText, TopBottomPanel, TextEdit};
use std::time::Instant;

pub struct LanChatApp {
    handle: AppHandle,
    egui_ctx: egui::Context,
    setup: SetupState,
    ready: bool,
    /// First frame after `ready` — drives the boot-step visual.
    boot_anim_t: Instant,
    /// All messages received from the backend, in arrival order.
    messages: Vec<Message>,
    /// Current peer snapshot.
    peers: Vec<PeerInfo>,
    /// Local interfaces (cached at startup).
    interfaces: Vec<crate::network::NetworkInterface>,
    /// Local listening port.
    port: u16,
    /// Local nickname (mirror of the backend).
    nickname: String,
    /// Input bar text.
    input: String,
    /// Transient toast.
    toast: Option<Toast>,
    /// IME composition flag.
    ime_composing: bool,
    /// Cached local IP display string (first non-loopback iface + port).
    local_addr_display: String,
}

struct Toast {
    text: String,
    shown_at: Instant,
}

impl LanChatApp {
    pub fn new(cc: &eframe::CreationContext<'_>, handle: AppHandle) -> Self {
        // Re-hydrate saved nickname from persisted memory.
        let saved: Option<String> = cc
            .egui_ctx
            .memory_mut(|m| m.data.get_persisted(egui::Id::new("lc-name")));

        let mut setup = SetupState::new(saved.is_none());
        if let Some(name) = &saved {
            setup.input = name.clone();
            let state = handle.server_state.clone();
            let n = name.clone();
            handle.runtime.spawn(async move {
                let _ = commands::set_nickname(&state, n).await;
            });
        }

        let port = handle.port;
        let interfaces = commands::get_interfaces();
        let peers = commands::get_peers(&handle.server_state);
        let messages = commands::get_history(&handle.server_state);
        let local_addr_display = interfaces
            .iter()
            .find(|i| i.enabled)
            .map(|i| format!("{}:{}", i.ip, port))
            .unwrap_or_else(|| format!("127.0.0.1:{}", port));

        Self {
            handle,
            egui_ctx: cc.egui_ctx.clone(),
            setup,
            ready: saved.is_some(),
            boot_anim_t: Instant::now(),
            messages,
            peers,
            interfaces,
            port,
            nickname: saved.unwrap_or_default(),
            input: String::new(),
            toast: None,
            ime_composing: false,
            local_addr_display,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.handle.event_rx.try_recv() {
            match ev {
                UiEvent::Message(m) => {
                    if !self.messages.iter().any(|x| x.id == m.id) {
                        self.messages.push(m);
                    }
                }
                UiEvent::Peers(p) => self.peers = p,
                UiEvent::Port(p) => self.port = p,
                UiEvent::Interfaces(i) => {
                    self.interfaces = i.clone();
                    if let Some(first) = i.iter().find(|x| x.enabled) {
                        self.local_addr_display = format!("{}:{}", first.ip, self.port);
                    }
                }
                UiEvent::Nickname(n) => {
                    self.nickname = n.clone();
                    self.egui_ctx.memory_mut(|m| {
                        m.data.insert_persisted(egui::Id::new("lc-name"), n);
                    });
                }
                UiEvent::Notice(text) => self.show_toast(text),
            }
        }
    }

    fn show_toast(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            shown_at: Instant::now(),
        });
    }

    fn draw_setup(&mut self, ctx: &egui::Context) {
        CentralPanel::default()
            .frame(egui::Frame::none().fill(color::BG))
            .show(ctx, |ui| {
                let outcome = crate::ui::setup::ui(ui, &mut self.setup);
                if let SetupOutcome::Submit(name) = outcome {
                    self.nickname = name.clone();
                    self.egui_ctx
                        .memory_mut(|m| m.data.insert_persisted(egui::Id::new("lc-name"), name.clone()));
                    let state = self.handle.server_state.clone();
                    self.handle.runtime.spawn(async move {
                        let _ = commands::set_nickname(&state, name).await;
                    });
                    self.setup.visible = false;
                    self.ready = true;
                    self.boot_anim_t = Instant::now();
                }
            });
    }

    fn draw_main(&mut self, ctx: &egui::Context) {
        // ── Top: header ──────────────────────────────────────
        TopBottomPanel::top("header")
            .frame(egui::Frame::none().fill(color::BG_ELEV).inner_margin(egui::Margin::symmetric(
                space::LG,
                space::SM,
            )))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("█")
                            .font(font::mono(font::BASE))
                            .color(color::AMBER),
                    );
                    ui.add_space(space::SM);
                    ui.label(
                        RichText::new("LAN-CHAT/0")
                            .font(font::mono(font::SM))
                            .color(color::TEXT),
                    );
                    ui.add_space(space::MD);
                    ui.label(
                        RichText::new(format!("● {} peer{}", self.peers.len(), if self.peers.len() == 1 { "" } else { "s" }))
                            .font(font::mono(font::XS))
                            .color(color::MUTED),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("[{}]", self.nickname))
                                .font(font::mono(font::XS))
                                .color(color::MUTED),
                        );
                    });
                });
            });

        // ── Center: log + input bar ──────────────────────────
        CentralPanel::default()
            .frame(egui::Frame::none().fill(color::BG))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Log area
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            if self.messages.is_empty() {
                                ui.add_space(space::XXL);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new("no peers on the wire.")
                                            .font(font::mono(font::BASE))
                                            .color(color::MUTED),
                                    );
                                    ui.label(
                                        RichText::new(format!("listening on {}", self.local_addr_display))
                                            .font(font::mono(font::SM))
                                            .color(color::MUTED),
                                    );
                                });
                            } else {
                                for m in &self.messages {
                                    draw_message_row(ui, m, &self.nickname);
                                }
                            }
                        });

                    ui.add_space(space::SM);

                    // Input bar
                    let frame = egui::Frame::none()
                        .fill(color::BG)
                        .inner_margin(egui::Margin::symmetric(
                            space::LG,
                            space::MD,
                        ));
                    frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let resp = ui.add(
                                TextEdit::singleline(&mut self.input)
                                    .font(font::mono(font::BASE))
                                    .text_color(color::TEXT)
                                    .hint_text(
                                        RichText::new("type a message…")
                                            .font(font::mono(font::BASE))
                                            .color(color::MUTED),
                                    )
                                    .desired_width(ui.available_width() - 80.0),
                            );
                            if resp.lost_focus()
                                && ui.input(|i| i.key_pressed(Key::Enter))
                                && !self.ime_composing
                            {
                                self.try_send_text();
                            }
                            let send_btn = egui::Button::new(
                                RichText::new("[ ↵ ]")
                                    .font(font::mono(font::SM))
                                    .color(if self.input.is_empty() {
                                        color::MUTED
                                    } else {
                                        color::AMBER
                                    }),
                            )
                            .fill(color::BG_ELEV)
                            .stroke(egui::Stroke::new(1.0, color::LINE));
                            if ui.add(send_btn).clicked() {
                                self.try_send_text();
                            }
                        });
                    });
                });
            });

        // ── Bottom: status bar ──────────────────────────────
        TopBottomPanel::bottom("status_bar")
            .frame(
                egui::Frame::none()
                    .fill(color::BG_ELEV)
                    .inner_margin(egui::Margin::symmetric(
                        space::LG,
                        space::SM,
                    )),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("● {} peer{}", self.peers.len(), if self.peers.len() == 1 { "" } else { "s" }))
                            .font(font::mono(font::XS))
                            .color(color::MUTED),
                    );
                    ui.add_space(space::LG);
                    ui.label(
                        RichText::new(format!("addr: {}", self.local_addr_display))
                            .font(font::mono(font::XS))
                            .color(color::MUTED),
                    );
                    ui.add_space(space::LG);
                    ui.label(
                        RichText::new(format!("port: {}", self.port))
                            .font(font::mono(font::XS))
                            .color(color::MUTED),
                    );
                });
            });

        // ── Toast overlay ───────────────────────────────────
        if let Some(t) = &self.toast {
            egui::Area::new(egui::Id::new("toast"))
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -80.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(color::BG_ELEV)
                        .stroke(egui::Stroke::new(1.0, color::LINE))
                        .inner_margin(egui::Margin::symmetric(
                            space::MD,
                            space::XS,
                        ))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&t.text)
                                    .font(font::mono(font::SM))
                                    .color(color::TEXT),
                            );
                        });
                });
        }
    }

    fn try_send_text(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }
        let state = self.handle.server_state.clone();
        let t = text.clone();
        self.handle.runtime.spawn(async move {
            if let Err(e) = commands::send_message(&state, t, "text".into()).await {
                tracing::warn!("send_message failed: {}", e);
            }
        });
        self.input.clear();
    }
}

impl eframe::App for LanChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        if !self.ready {
            self.draw_setup(ctx);
        } else {
            self.draw_main(ctx);
        }

        // Tick the boot step visual once ready
        if self.ready {
            let elapsed = self.boot_anim_t.elapsed().as_millis();
            let step = if elapsed > 600 {
                3
            } else if elapsed > 400 {
                2
            } else if elapsed > 200 {
                1
            } else {
                0
            };
            self.setup.boot_step = step;
        }

        // Toast auto-dismiss after 1.8s
        if let Some(t) = &self.toast {
            if t.shown_at.elapsed().as_secs_f32() > 1.8 {
                self.toast = None;
            }
        }
    }
}

fn draw_message_row(ui: &mut egui::Ui, m: &Message, me: &str) {
    let is_me = m.device == me;
    let is_clip = matches!(m.msg_type, MsgType::Clipboard);
    let _is_file = matches!(m.msg_type, MsgType::File);
    let color_msg = if is_clip {
        color::GREEN
    } else if is_me {
        color::AMBER
    } else {
        color::TEXT
    };
    let time = format_ts(m.ts);
    let row_color = if is_me { color::AMBER } else { color::MUTED };
    let prefix = if is_me { "▌ " } else { "  " };
    let device_str = if is_me { format!("{} (you)", m.device) } else { m.device.clone() };
    ui.add_space(space::SM);
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("[{}] {}{}", time, prefix, device_str))
                .font(font::mono(font::XS))
                .color(row_color),
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_space(space::LG);
        ui.label(
            RichText::new(&m.text)
                .font(font::mono(font::BASE))
                .color(color_msg),
        );
    });
    if let Some(file) = &m.file {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(space::LG);
            ui.label(
                RichText::new(format!("sha256:{}  size:{}B", &file.sha256[..12], file.size))
                    .font(font::mono(font::XS))
                    .color(color::MUTED),
            );
        });
    }
}

fn format_ts(ts: u64) -> String {
    // Format as HH:MM:SS in UTC (LAN tool, timezone precision is fine to skip here).
    let secs = (ts / 1000) % 86_400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
