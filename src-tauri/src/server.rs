// lan-chat/src-tauri/src/server.rs
//
// Starts an Axum HTTP server on this machine, accepting inbound WebSocket connections.
// Handshake protocol:
//   1. After connection, both sides exchange {"type":"hello","node_id":"...","nickname":"...","version":"1"}
//   2. Server sends history to new peer
//   3. Normal message exchange follows

use crate::messages::{Message, MessageStore};
use crate::peers::{PeerCmd, PeerInfo, PeerPool};
use axum::{
    extract::{
        ws::{Message as AxumWsMsg, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
struct HelloMsg {
    #[serde(rename = "type")]
    msg_type: String,
    node_id: String,
    nickname: String,
    version: String,
}

#[derive(Clone)]
pub struct ServerState {
    pub pool: Arc<PeerPool>,
    pub store: Arc<MessageStore>,
    pub node_id: String,
    pub nickname: Arc<tokio::sync::RwLock<String>>,
    pub app: AppHandle,
}

/// Start WebSocket server, trying ports 4242..4252.
/// Returns the actual bound port.
pub async fn start_server(state: ServerState) -> anyhow::Result<u16> {
    let listener = find_free_port(4242, 4252).await?;
    let port = listener.local_addr()?.port();

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    tracing::info!("WebSocket server listening on 0.0.0.0:{}", port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("WebSocket server error: {}", e);
        }
    });

    Ok(port)
}

async fn find_free_port(start: u16, end: u16) -> anyhow::Result<tokio::net::TcpListener> {
    for port in start..=end {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            return Ok(listener);
        }
    }
    anyhow::bail!("No free port found in range {}..{}", start, end)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ServerState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_inbound(socket, state))
}

async fn handle_inbound(socket: WebSocket, state: ServerState) {
    let (mut sink, mut stream) = socket.split();

    // 1. Send hello
    let my_nick = state.nickname.read().await.clone();
    let hello = match serde_json::to_string(&HelloMsg {
        msg_type: "hello".into(),
        node_id: state.node_id.clone(),
        nickname: my_nick,
        version: "1".into(),
    }) {
        Ok(s) => s,
        Err(_) => return,
    };
    if sink.send(AxumWsMsg::Text(hello)).await.is_err() {
        return;
    }

    // 2. Wait for peer's hello
    let peer_info = loop {
        match stream.next().await {
            Some(Ok(AxumWsMsg::Text(t))) => {
                if let Ok(h) = serde_json::from_str::<HelloMsg>(&t) {
                    if h.msg_type == "hello" {
                        break PeerInfo {
                            node_id: h.node_id,
                            nickname: h.nickname,
                            addr: "inbound".into(),
                        };
                    }
                }
            }
            _ => return,
        }
    };

    // 3. Reject self-connection and duplicate connections
    if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
        return;
    }

    // 4. Send history to new peer
    let history = state.store.history();
    if !history.is_empty() {
        let hist_json = match serde_json::to_string(&serde_json::json!({
            "type": "history",
            "messages": history
        })) {
            Ok(s) => s,
            Err(_) => return,
        };
        if sink.send(AxumWsMsg::Text(hist_json)).await.is_err() {
            return;
        }
    }

    // 5. Register peer and start write task
    let (tx, rx) = mpsc::unbounded_channel::<PeerCmd>();
    let node_id = peer_info.node_id.clone();
    state.pool.add(peer_info, tx);
    emit_peer_update(&state);

    let pool_clone = state.pool.clone();
    let node_id_clone = node_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        run_axum_writer(sink, rx).await;
        pool_clone.remove(&node_id_clone);
        emit_peer_update(&state_clone);
    });

    // Read task: receive messages from peer
    while let Some(Ok(msg)) = stream.next().await {
        if let AxumWsMsg::Text(t) = msg {
            handle_incoming_message(&t, &node_id, &state);
        }
    }
}

fn handle_incoming_message(json: &str, from_node_id: &str, state: &ServerState) {
    if let Ok(msg) = serde_json::from_str::<Message>(json) {
        if state.store.insert(msg.clone()) {
            // Broadcast to other peers (excluding sender to prevent loops)
            state.pool.broadcast(json, Some(from_node_id));
            // Push to frontend
            let _ = state.app.emit("message", &msg);
        }
    }
}

fn emit_peer_update(state: &ServerState) {
    let peers = state.pool.list();
    let _ = state.app.emit("peer_update", &peers);
}

/// Axum WebSocket sink writer task
async fn run_axum_writer(
    mut sink: impl futures::Sink<AxumWsMsg, Error = axum::Error> + Unpin,
    mut rx: mpsc::UnboundedReceiver<PeerCmd>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            PeerCmd::Send(json) => {
                if sink.send(AxumWsMsg::Text(json)).await.is_err() {
                    break;
                }
            }
            PeerCmd::Close => {
                let _ = sink.send(AxumWsMsg::Close(None)).await;
                break;
            }
        }
    }
}
