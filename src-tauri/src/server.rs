// lan-chat/src-tauri/src/server.rs
//
// Starts an Axum HTTP server on this machine, accepting inbound WebSocket connections
// and HTTP file transfer requests.
// Handshake protocol:
//   1. After WS connection, both sides exchange {"type":"hello","node_id":...,"nickname":...,"version":"1"}
//   2. Server sends history to new peer
//   3. Normal message exchange follows
//
// File transfer:
//   - POST /upload   body: raw bytes; sets X-Sha256 + X-Filename; receiver stores in cache
//   - GET  /files/:sha256   returns raw bytes
//   - File metadata is broadcast through the existing WebSocket channel as a "file" message

use crate::messages::{FileMeta, Message, MessageStore};
use crate::peers::{PeerCmd, PeerInfo, PeerPool};
use crate::transfer::TransferCache;
use axum::{
    body::Bytes,
    extract::{
        ws::{Message as AxumWsMsg, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize)]
struct HelloMsg {
    #[serde(rename = "type")]
    msg_type: String,
    node_id: String,
    nickname: String,
    version: String,
}

/// Events pushed from the backend (server / mDNS / peer connections) to the GUI.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// A new chat/clipboard/file message was just stored locally.
    Message(Message),
    /// The connected peer list changed.
    Peers(Vec<PeerInfo>),
    /// This node's listening port (sent once at startup).
    Port(u16),
    /// Network interfaces (sent once at startup).
    Interfaces(Vec<crate::network::NetworkInterface>),
    /// Current nickname (sent once at startup, then on update).
    Nickname(String),
    /// Background info / error message to show in the toast layer.
    Notice(String),
}

#[derive(Clone)]
pub struct ServerState {
    pub pool: Arc<PeerPool>,
    pub store: Arc<MessageStore>,
    pub transfers: Arc<TransferCache>,
    pub node_id: String,
    pub nickname: Arc<tokio::sync::RwLock<String>>,
    /// Channel to push events to the GUI.
    pub events: mpsc::UnboundedSender<UiEvent>,
    /// Our own listening addr "ip:port" — used for file messages we send so receivers
    /// can fetch the body from us.
    pub self_addr: Arc<tokio::sync::RwLock<Option<String>>>,
}

/// Start HTTP/WS server, trying ports 4242..4252.
pub async fn start_server(state: ServerState) -> anyhow::Result<u16> {
    let listener = find_free_port(4242, 4252).await?;
    let port = listener.local_addr()?.port();

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .route("/upload", post(upload_handler))
        .route("/files/:sha256", get(download_handler))
        .with_state(state);

    tracing::info!("HTTP+WS server listening on 0.0.0.0:{}", port);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("HTTP server error: {}", e);
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

    if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
        return;
    }

    // 3. Send history to new peer
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

    while let Some(Ok(msg)) = stream.next().await {
        if let AxumWsMsg::Text(t) = msg {
            handle_incoming_message(&t, &node_id, &state);
        }
    }
}

fn handle_incoming_message(json: &str, from_node_id: &str, state: &ServerState) {
    if let Ok(msg) = serde_json::from_str::<Message>(json) {
        if state.store.insert(msg.clone()) {
            state.pool.broadcast(json, Some(from_node_id));
            let _ = state.events.send(UiEvent::Message(msg));
        }
    }
}

fn emit_peer_update(state: &ServerState) {
    let peers = state.pool.list();
    let _ = state.events.send(UiEvent::Peers(peers));
}

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

// ── File transfer HTTP handlers ─────────────────────────────

/// `POST /upload` — receiver stores the body in the transfer cache, keyed by sha256.
async fn upload_handler(
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // The sender must include X-Sha256 (hex) so we don't have to recompute on the
    // receiver side (and to verify the body matches the announced hash).
    let announced = match headers
        .get("x-sha256")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
    {
        Some(s) => s,
        None => return (StatusCode::BAD_REQUEST, "missing x-sha256").into_response(),
    };

    let mut hasher = Sha256::new();
    hasher.update(&body);
    let actual = hex::encode(hasher.finalize());

    if actual != announced.to_lowercase() {
        return (StatusCode::BAD_REQUEST, "sha256 mismatch").into_response();
    }

    state.transfers.put(actual.clone(), body.to_vec());
    (StatusCode::OK, "ok").into_response()
}

/// `GET /files/:sha256` — returns the cached body.
async fn download_handler(
    State(state): State<ServerState>,
    Path(sha256): Path<String>,
) -> impl IntoResponse {
    match state.transfers.get(&sha256) {
        Some(bytes) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
            bytes,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

/// Build a `FileMeta` for an outbound file message, addressing ourselves.
pub async fn make_file_meta_for_self(
    state: &ServerState,
    sha256: String,
    filename: String,
    size: u64,
) -> FileMeta {
    let addr = state.self_addr.read().await.clone().unwrap_or_default();
    FileMeta {
        sha256,
        filename,
        size,
        addr,
    }
}
