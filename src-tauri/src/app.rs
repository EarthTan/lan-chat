// lan-chat/src-tauri/src/app.rs
//
// The eframe::App implementation — drives the entire UI tree.

use crate::messages::{Message, MsgType};
use crate::peers::PeerInfo;
use crate::server::UiEvent;
use crate::ui::clip_modal::{ClipModal, ClipOutcome};
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
    /// Set by `try_send_text` so the input bar clears on the *next* frame
    /// after `update` runs. This gives any `Ime::Commit` event that winit
    /// delivered in the same OS event batch a chance to land in `self.input`
    /// before we wipe it — fixing the same-frame race where a RIME Enter-to-
    /// commit would silently lose the committed character.
    pending_clear_input: bool,
    /// Real widget id of the chat input `TextEdit`, captured each frame
    /// from `resp.id`. Needed because egui composes the id from the
    /// current UI's parent id and the `id_source` salt — it is NOT
    /// equal to `Id::new("input_bar_field")`. Used to snap focus back
    /// to the input bar after sending, and to identify the field in
    /// the IME drain.
    input_field_id: Option<egui::Id>,
    /// Same idea for the manual-connect IP:port input in the status bar.
    manual_addr_field_id: Option<egui::Id>,
    /// Same idea for the nickname-edit input in the status bar.
    nickname_field_id: Option<egui::Id>,
    /// Cached local IP display string (first non-loopback iface + port).
    local_addr_display: String,
    /// Clipboard share modal.
    clip_modal: ClipModal,
    /// File actions queued by file cards; drained each frame.
    pending_file_actions: Vec<crate::ui::file_card::FileAction>,
    /// Manual connect address text in status bar.
    manual_addr: String,
    /// Nickname edit text in status bar.
    nickname_input: String,
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
            nickname: saved.as_deref().unwrap_or("").to_string(),
            input: String::new(),
            toast: None,
            ime_composing: false,
            pending_clear_input: false,
            input_field_id: None,
            manual_addr_field_id: None,
            nickname_field_id: None,
            local_addr_display,
            clip_modal: ClipModal::new(),
            pending_file_actions: Vec::new(),
            manual_addr: String::new(),
            nickname_input: saved.as_deref().unwrap_or("").to_string(),
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
                        let clip_btn = egui::Button::new(
                            RichText::new("[ CLIP ]")
                                .font(font::mono(font::SM))
                                .color(color::TEXT),
                        )
                        .fill(color::BG_ELEV)
                        .stroke(egui::Stroke::new(1.0, color::LINE));
                        if ui.add(clip_btn).clicked() {
                            self.clip_modal.open();
                        }
                        ui.add_space(space::SM);
                        ui.label(
                            RichText::new(format!("[{}]", self.nickname))
                                .font(font::mono(font::XS))
                                .color(color::MUTED),
                        );
                    });
                });
            });

        // ── Center: log only (input is a separate bottom panel below) ──
        CentralPanel::default()
            .frame(egui::Frame::none().fill(color::BG))
            .show(ctx, |ui| {
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
                                let actions = draw_message_row(ui, m, &self.nickname);
                                for a in actions {
                                    self.pending_file_actions.push(a);
                                }
                            }
                        }
                    });
            });

        // ── Input bar (its own bottom panel — always visible) ──
        TopBottomPanel::bottom("input_bar")
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(color::BG)
                    .inner_margin(egui::Margin::symmetric(space::LG, space::MD)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // File picker button [ + ]
                    let plus_btn = egui::Button::new(
                        RichText::new("[ + ]")
                            .font(font::mono(font::SM))
                            .color(color::AMBER),
                    )
                    .fill(color::BG_ELEV)
                    .stroke(egui::Stroke::new(1.0, color::LINE));
                    if ui.add(plus_btn).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.handle_local_file(path);
                        }
                    }
                    ui.add_space(space::SM);

                    let resp = ui.add(
                        TextEdit::singleline(&mut self.input)
                            .id_source("input_bar_field")
                            .font(font::mono(font::BASE))
                            .text_color(color::TEXT)
                            .hint_text(
                                RichText::new("type a message…")
                                    .font(font::mono(font::BASE))
                                    .color(color::MUTED),
                            )
                            .desired_width(ui.available_width() - 100.0),
                    );
                    // Capture the widget's actual runtime id. egui composes
                    // the id from the current UI's parent id and the
                    // `id_source` salt, so it is NOT equal to
                    // `Id::new("input_bar_field")`. We need the real id
                    // to (a) snap focus back to the input bar after a
                    // send and (b) identify the field in the IME drain.
                    self.input_field_id = Some(resp.id);
                    if resp.lost_focus()
                        && ui.input(|i| i.key_pressed(Key::Enter))
                        && !self.ime_composing
                    {
                        self.try_send_text();
                    }
                    if resp.has_focus()
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

                    // spacer pushes the rest to the right
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let connect_btn = egui::Button::new(
                            RichText::new("[ CONNECT ]")
                                .font(font::mono(font::XS))
                                .color(color::GREEN),
                        )
                        .fill(color::BG_ELEV)
                        .stroke(egui::Stroke::new(1.0, color::LINE));
                        if ui.add(connect_btn).clicked() {
                            self.show_toast("enter IP:port in the input bar above");
                        }
                        ui.add_space(space::SM);
                        let resp = ui.add(
                            TextEdit::singleline(&mut self.manual_addr)
                                .id_source("manual_addr_field")
                                .font(font::mono(font::XS))
                                .text_color(color::TEXT)
                                .hint_text(
                                    RichText::new("IP:port")
                                        .font(font::mono(font::XS))
                                        .color(color::MUTED),
                                )
                                .desired_width(120.0),
                        );
                        self.manual_addr_field_id = Some(resp.id);
                        if resp.lost_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter))
                            && !self.manual_addr.is_empty()
                        {
                            self.try_manual_connect();
                        }

                        ui.add_space(space::SM);

                        // Nickname editor
                        let nick_resp = ui.add(
                            TextEdit::singleline(&mut self.nickname_input)
                                .id_source("nickname_field")
                                .font(font::mono(font::XS))
                                .text_color(color::TEXT)
                                .hint_text(
                                    RichText::new("nickname")
                                        .font(font::mono(font::XS))
                                        .color(color::MUTED),
                                )
                                .desired_width(120.0),
                        );
                        self.nickname_field_id = Some(nick_resp.id);
                        if nick_resp.lost_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter))
                            && !self.nickname_input.is_empty()
                        {
                            self.try_set_nickname();
                        }
                    });
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

        // ── Clipboard modal overlay ─────────────────────────
        if self.clip_modal.open {
            egui::Area::new(egui::Id::new("clip_overlay"))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_pos(ctx.screen_rect().center())
                .show(ctx, |ui| {
                    // Backdrop
                    let screen = ctx.screen_rect();
                    ui.painter().rect_filled(
                        screen,
                        0.0,
                        eframe::egui::Color32::from_black_alpha(160),
                    );
                    let outcome =
                        crate::ui::clip_modal::ui(ui, &mut self.clip_modal);
                    match outcome {
                        ClipOutcome::Pending => {}
                        ClipOutcome::Send(text) => {
                            let state = self.handle.server_state.clone();
                            self.handle.runtime.spawn(async move {
                                if let Err(e) = commands::send_message(
                                    &state, text, "clipboard".into(),
                                ).await {
                                    tracing::warn!("send clipboard failed: {}", e);
                                }
                            });
                            self.clip_modal.open = false;
                            self.show_toast("transmitted");
                        }
                        ClipOutcome::Close => {
                            self.clip_modal.open = false;
                        }
                    }
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
        // Defer the clear to the next frame so a same-frame `Ime::Commit`
        // event (typical with RIME Enter-to-commit on Linux) can land in
        // `self.input` first. The actual `clear()` happens at the top of
        // `App::update` via `pending_clear_input`.
        self.pending_clear_input = true;

        // Snap focus back to the input bar so the user can keep typing
        // the next message without reaching for the mouse — whether the
        // send was triggered by Enter or by clicking the [↵] button.
        if let Some(id) = self.input_field_id {
            self.egui_ctx.memory_mut(|m| m.request_focus(id));
        }
    }

    fn try_manual_connect(&mut self) {
        let raw = self.manual_addr.trim().to_string();
        if raw.is_empty() {
            return;
        }
        let (ip, port) = match raw.split_once(':') {
            Some((i, p)) => (i.to_string(), p.parse::<u16>().ok()),
            None => (raw.clone(), None),
        };
        let state = self.handle.server_state.clone();
        self.handle.runtime.spawn(async move {
            if let Err(e) = commands::connect_peer(state, ip, port).await {
                tracing::warn!("connect_peer failed: {}", e);
            }
        });
        self.manual_addr.clear();
        self.show_toast("connecting…");
    }

    fn try_set_nickname(&mut self) {
        let n = self.nickname_input.trim().to_string();
        if n.is_empty() {
            return;
        }
        let state = self.handle.server_state.clone();
        let n_clone = n.clone();
        self.handle.runtime.spawn(async move {
            let _ = commands::set_nickname(&state, n_clone).await;
        });
        self.nickname = n;
        self.show_toast("nickname updated");
    }

    fn handle_file_action(&mut self, a: crate::ui::file_card::FileAction) {
        use crate::ui::file_card::FileAction;
        match a {
            FileAction::SaveLocal { sha256, suggested_name } => {
                let state = self.handle.server_state.clone();
                let events = self.handle.server_state.events.clone();
                self.handle.runtime.spawn(async move {
                    let body = match state.transfers.get(&sha256) {
                        Some(b) => b,
                        None => {
                            let _ = events.send(UiEvent::Notice("file not in cache".into()));
                            return;
                        }
                    };
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&suggested_name)
                        .save_file()
                    {
                        if let Err(e) = std::fs::write(&path, &body) {
                            let _ = events.send(UiEvent::Notice(format!("save failed: {}", e)));
                        } else {
                            let _ = events.send(UiEvent::Notice(format!("saved {}", path.display())));
                        }
                    }
                });
            }
            FileAction::SaveRemote { addr, sha256, suggested_name } => {
                let events = self.handle.server_state.events.clone();
                self.handle.runtime.spawn(async move {
                    let body = match commands::download_file(&addr, &sha256).await {
                        Ok(b) => b,
                        Err(e) => {
                            let _ = events.send(UiEvent::Notice(format!("download failed: {}", e)));
                            return;
                        }
                    };
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&suggested_name)
                        .save_file()
                    {
                        if let Err(e) = std::fs::write(&path, &body) {
                            let _ = events.send(UiEvent::Notice(format!("save failed: {}", e)));
                        } else {
                            let _ = events.send(UiEvent::Notice(format!("saved {}", path.display())));
                        }
                    }
                });
            }
            FileAction::Open(_path) => {
                // not implemented in v1
            }
        }
    }

    fn handle_local_file(&mut self, path: std::path::PathBuf) {
        let state = self.handle.server_state.clone();
        let events = self.handle.server_state.events.clone();
        self.handle.runtime.spawn(async move {
            let body = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    let _ = events.send(UiEvent::Notice(format!("read failed: {}", e)));
                    return;
                }
            };
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            Digest::update(&mut hasher, &body);
            let sha256 = hex::encode(Digest::finalize(hasher));
            let size = body.len() as u64;
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let file = crate::messages::FileMeta {
                sha256,
                filename,
                size,
                addr: state.self_addr.read().await.clone().unwrap_or_default(),
            };
            if let Err(e) = commands::broadcast_file(&state, file, body).await {
                let _ = events.send(UiEvent::Notice(format!("send failed: {}", e)));
            } else {
                let _ = events.send(UiEvent::Notice("file sent".into()));
            }
        });
    }
}

impl eframe::App for LanChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        // Apply the deferred input-bar clear from the previous frame
        // BEFORE we drain IME events, so a same-frame `Ime::Commit`
        // gets a chance to land in `self.input` first.
        if self.pending_clear_input {
            self.input.clear();
            self.pending_clear_input = false;
        }

        // ── IME drain (workaround for egui-winit 0.30 dropping
        //    `Ime::Commit` on Linux + `TextEdit::singleline`, see
        //    emilk/egui#5683 / emilk/egui-winit#254).
        //
        // We manually observe `egui::Event::Ime(...)` and:
        //   - keep `ime_composing` in sync (previously dead state)
        //   - forward `Commit(s)` into the focused TextEdit's backing
        //     String, bypassing the egui-winit drop bug.
        //
        // Only the input bar and nickname/addr editors need this —
        // the rest of the UI is read-only.
        let focused_id = ctx.memory(|m| m.focused());
        ctx.input(|i| {
            for ev in &i.events {
                if let egui::Event::Ime(ime) = ev {
                    match ime {
                        egui::ImeEvent::Enabled
                        | egui::ImeEvent::Preedit(_) => {
                            self.ime_composing = true;
                        }
                        egui::ImeEvent::Disabled
                        | egui::ImeEvent::Commit(_) => {
                            self.ime_composing = false;
                        }
                    }
                    if let egui::ImeEvent::Commit(text) = ime {
                        if !text.is_empty() {
                            // Match the focused widget by its REAL runtime
                            // id (captured each frame into the *_field_id
                            // fields), not by `Id::new("..._field")` — those
                            // are salta and never equal to the composed
                            // widget id.
                            let target = match focused_id {
                                Some(id) if Some(id) == self.input_field_id => {
                                    Some(&mut self.input)
                                }
                                Some(id) if Some(id) == self.manual_addr_field_id => {
                                    Some(&mut self.manual_addr)
                                }
                                Some(id) if Some(id) == self.nickname_field_id => {
                                    Some(&mut self.nickname_input)
                                }
                                _ => None,
                            };
                            if let Some(buf) = target {
                                buf.push_str(text);
                            }
                        }
                    }
                }
            }
        });
        // Note: re-entering `ctx.input(|i| ...)` multiple times in the
        // same frame is the documented pattern; egui drains events into
        // the input state once per frame at the top of `update`, so
        // subsequent reads of `i.events` are stable within this frame.

        // (kept for clarity — the legacy Enter-guard comments around
        //  `try_send_text` rely on `ime_composing` being correct, which
        //  the drain above now guarantees.)

        // Tick the boot step visual. Run on every frame so the [..] → [OK]
        // sequence is visible *while* the setup overlay is still up — not
        // only after the user submits a name.
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

        if !self.ready {
            self.draw_setup(ctx);
        } else {
            self.draw_main(ctx);
        }

        // Toast auto-dismiss after 1.8s
        if let Some(t) = &self.toast {
            if t.shown_at.elapsed().as_secs_f32() > 1.8 {
                self.toast = None;
            }
        }

        // Process file card actions
        let actions: Vec<_> = self.pending_file_actions.drain(..).collect();
        for a in actions {
            self.handle_file_action(a);
        }

        // Dropped files (drag & drop)
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        for d in dropped {
            if let Some(path) = d.path {
                self.handle_local_file(path);
            }
        }
    }
}

fn draw_message_row(ui: &mut egui::Ui, m: &Message, me: &str) -> Vec<crate::ui::file_card::FileAction> {
    // File messages render as a card
    if matches!(m.msg_type, MsgType::File) {
        return crate::ui::file_card::draw(ui, m, me);
    }

    let is_me = m.device == me;
    let is_clip = matches!(m.msg_type, MsgType::Clipboard);
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
    Vec::new()
}

fn format_ts(ts: u64) -> String {
    // Format as HH:MM:SS in UTC (LAN tool, timezone precision is fine to skip here).
    let secs = (ts / 1000) % 86_400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
