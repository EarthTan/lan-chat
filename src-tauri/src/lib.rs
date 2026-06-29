// lan-chat/src-tauri/src/lib.rs
//
// App entry: spawn a tokio runtime, start HTTP/WS server + mDNS,
// then hand control to eframe (egui) which drives the rest of the app.

pub mod app;
pub mod commands;
pub mod mdns;
pub mod messages;
pub mod network;
pub mod peers;
pub mod server;
pub mod transfer;
pub mod ui;

use crate::app::LanChatApp;
use crate::messages::MessageStore;
use crate::network::get_candidate_interfaces;
use crate::peers::PeerPool;
use crate::server::{start_server, ServerState, UiEvent};
use crate::transfer::TransferCache;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Application handle shared between the tokio runtime and the eframe GUI.
///
/// The runtime pushes events into `event_rx`; the GUI owns the receiver and
/// drains it each frame. The GUI also holds a `ServerState` (clone) to call
/// backend operations.
pub struct AppHandle {
    pub server_state: ServerState,
    pub port: u16,
    pub event_rx: mpsc::UnboundedReceiver<UiEvent>,
    /// Runtime handle so the GUI can `tokio::spawn` operations that need
    /// to interact with the backend (e.g. connect-to-peer, broadcast-file).
    pub runtime: Arc<tokio::runtime::Runtime>,
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lan_chat=info".parse().unwrap()),
        )
        .init();

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime"),
    );

    let (event_tx, event_rx) = mpsc::unbounded_channel::<UiEvent>();

    let node_id = uuid::Uuid::new_v4().to_string();
    let nickname = Arc::new(RwLock::new(format!("Device-{}", &node_id[..4])));
    let pool = PeerPool::new();
    let store = Arc::new(MessageStore::new());
    let transfers = TransferCache::new();
    let self_addr: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    let server_state = ServerState {
        pool: pool.clone(),
        store: store.clone(),
        transfers: transfers.clone(),
        node_id: node_id.clone(),
        nickname: nickname.clone(),
        events: event_tx.clone(),
        self_addr: self_addr.clone(),
    };

    // Start the HTTP/WS server on a background task.
    let port = {
        let state_clone = server_state.clone();
        runtime.block_on(async move { start_server(state_clone).await })
    };
    let port = match port {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };
    tracing::info!("Server started on port {}", port);
    let _ = event_tx.send(UiEvent::Port(port));

    // Resolve and cache our self-addr: pick the first non-loopback iface IP + port.
    let iface_ips: Vec<String> = get_candidate_interfaces()
        .into_iter()
        .filter(|i| i.enabled)
        .map(|i| i.ip)
        .collect();
    let self_addr_value = iface_ips
        .first()
        .map(|ip| format!("{}:{}", ip, port))
        .unwrap_or_else(|| format!("127.0.0.1:{}", port));
    runtime.block_on(async {
        *self_addr.write().await = Some(self_addr_value.clone());
    });
    let _ = event_tx.send(UiEvent::Interfaces(get_candidate_interfaces()));

    // Start mDNS. `start_mdns` is synchronous but spawns its own tokio task
    // internally; we just need to call it from inside the runtime context.
    let _ = runtime.block_on(async {
        let state = server_state.clone();
        let node_id = node_id.clone();
        let iface_ips = iface_ips.clone();
        // We use spawn_blocking because start_mdns itself is sync.
        tokio::task::spawn_blocking(move || mdns::start_mdns(node_id, port, iface_ips, state))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    });

    let app_handle = AppHandle {
        server_state,
        port,
        event_rx,
        runtime: runtime.clone(),
    };

    // Native window options — wgpu back-end, no decorations we don't need.
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("LAN Chat")
            .with_inner_size([960.0, 700.0])
            .with_min_inner_size([600.0, 480.0])
            .with_resizable(true)
            .with_app_id("io.github.earthtan.lan-chat"),
        vsync: true,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "LAN Chat",
        native_options,
        Box::new(move |cc| {
            crate::ui::theme::install(&cc.egui_ctx);
            Ok(Box::new(LanChatApp::new(cc, app_handle)))
        }),
    );
}
