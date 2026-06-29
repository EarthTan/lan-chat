// lan-chat/src-tauri/src/lib.rs

mod commands;
mod mdns;
mod messages;
mod network;
mod peers;
mod server;

use commands::AppState;
use messages::MessageStore;
use network::get_candidate_interfaces;
use peers::PeerPool;
use server::{start_server, ServerState};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lan_chat=info".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::get_history,
            commands::connect_peer,
            commands::get_peers,
            commands::get_interfaces,
            commands::get_port,
            commands::set_nickname,
            commands::get_nickname,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let node_id = uuid::Uuid::new_v4().to_string();
            let nickname = Arc::new(RwLock::new(format!("Device-{}", &node_id[..4])));
            // PeerPool::new() already returns Arc<PeerPool>
            let pool = PeerPool::new();
            let store = Arc::new(MessageStore::new());

            let server_state = ServerState {
                pool: pool.clone(),
                store: store.clone(),
                node_id: node_id.clone(),
                nickname: nickname.clone(),
                app: handle.clone(),
            };

            let server_state_clone = server_state.clone();
            let handle_clone = handle.clone();

            tauri::async_runtime::spawn(async move {
                let port = match start_server(server_state_clone.clone()).await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Failed to start server: {}", e);
                        return;
                    }
                };

                tracing::info!("Server started on port {}", port);

                handle_clone.manage(AppState {
                    server_state: server_state_clone.clone(),
                    port,
                });

                let iface_ips: Vec<String> = get_candidate_interfaces()
                    .into_iter()
                    .filter(|i| i.enabled)
                    .map(|i| i.ip)
                    .collect();

                tracing::info!("Broadcasting on interfaces: {:?}", iface_ips);

                if let Err(e) = mdns::start_mdns(node_id, port, iface_ips, server_state_clone) {
                    tracing::warn!("mDNS failed to start: {}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
