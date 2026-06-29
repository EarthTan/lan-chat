// lan-chat/src-tauri/src/commands.rs
//
// All Tauri IPC commands callable from the frontend.

use crate::mdns::connect_to_peer;
use crate::messages::{Message, MsgType};
use crate::network::{get_candidate_interfaces, NetworkInterface};
use crate::peers::PeerInfo;
use crate::server::ServerState;
use tauri::{Emitter, State};

/// Application global state (registered via tauri::Manager::manage)
pub struct AppState {
    pub server_state: ServerState,
    pub port: u16,
}

// ── Messages ──────────────────────────────────────────────

/// Send a message: store locally, broadcast to all peers, push to frontend
#[tauri::command]
pub async fn send_message(
    text: String,
    msg_type: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let nick = state.server_state.nickname.read().await.clone();
    let t = if msg_type == "clipboard" {
        MsgType::Clipboard
    } else {
        MsgType::Text
    };
    let msg = Message::new(text, nick, t);
    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;

    state.server_state.store.insert(msg.clone());
    state.server_state.pool.broadcast(&json, None);
    let _ = state.server_state.app.emit("message", &msg);
    Ok(())
}

/// Get message history (called on window load/refresh)
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<Message> {
    state.server_state.store.history()
}

// ── Peer Management ──────────────────────────────────────────

/// Manually connect to a peer by IP (user input)
#[tauri::command]
pub async fn connect_peer(
    ip: String,
    port: Option<u16>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let p = port.unwrap_or(4242);
    let url = format!("ws://{}:{}/ws", ip, p);
    let addr_str = format!("{}:{}", ip, p);
    let server_state = state.server_state.clone();

    tokio::spawn(async move {
        connect_to_peer(url, addr_str, server_state).await;
    });
    Ok(())
}

/// Get current peer list
#[tauri::command]
pub fn get_peers(state: State<'_, AppState>) -> Vec<PeerInfo> {
    state.server_state.pool.list()
}

// ── Network ──────────────────────────────────────────────

/// Get candidate network interfaces
#[tauri::command]
pub fn get_interfaces() -> Vec<NetworkInterface> {
    get_candidate_interfaces()
}

/// Get the local WebSocket listening port
#[tauri::command]
pub fn get_port(state: State<'_, AppState>) -> u16 {
    state.port
}

// ── Settings ──────────────────────────────────────────────

/// Update nickname
#[tauri::command]
pub async fn set_nickname(nickname: String, state: State<'_, AppState>) -> Result<(), String> {
    let nick = nickname.trim().chars().take(40).collect::<String>();
    *state.server_state.nickname.write().await = nick;
    Ok(())
}

/// Get current nickname
#[tauri::command]
pub async fn get_nickname(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.server_state.nickname.read().await.clone())
}
