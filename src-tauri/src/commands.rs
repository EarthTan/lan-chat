// lan-chat/src-tauri/src/commands.rs
//
// Backend operations callable from the GUI (formerly Tauri IPC commands).
// These are plain async fns over `ServerState` — no Tauri dependency.

use crate::mdns::connect_to_peer;
use crate::messages::{FileMeta, Message, MsgType};
use crate::network::get_candidate_interfaces;
use crate::peers::PeerInfo;
use crate::server::{ServerState, UiEvent};

/// Send a chat / clipboard message.
pub async fn send_message(state: &ServerState, text: String, msg_type: String) -> Result<(), String> {
    let nick = state.nickname.read().await.clone();
    let t = match msg_type.as_str() {
        "clipboard" => MsgType::Clipboard,
        _ => MsgType::Text,
    };
    let msg = Message::new(text, nick, t);
    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;

    if !state.store.insert(msg.clone()) {
        // duplicate (shouldn't happen for self-sent, but safe)
        return Ok(());
    }

    state.pool.broadcast(&json, None);
    let _ = state.events.send(UiEvent::Message(msg));
    Ok(())
}

/// Get message history (called on window load/refresh).
pub fn get_history(state: &ServerState) -> Vec<Message> {
    state.store.history()
}

/// Manually connect to a peer by IP (user input).
pub async fn connect_peer(state: ServerState, ip: String, port: Option<u16>) -> Result<(), String> {
    let p = port.unwrap_or(4242);
    let url = format!("ws://{}:{}/ws", ip, p);
    let addr_str = format!("{}:{}", ip, p);
    let server_state = state.clone();

    tokio::spawn(async move {
        connect_to_peer(url, addr_str, server_state).await;
    });
    Ok(())
}

/// Get current peer list.
pub fn get_peers(state: &ServerState) -> Vec<PeerInfo> {
    state.pool.list()
}

/// Get candidate network interfaces.
pub fn get_interfaces() -> Vec<crate::network::NetworkInterface> {
    get_candidate_interfaces()
}

/// Update nickname.
pub async fn set_nickname(state: &ServerState, nickname: String) -> Result<(), String> {
    let nick = nickname.trim().chars().take(40).collect::<String>();
    *state.nickname.write().await = nick.clone();
    let _ = state.events.send(UiEvent::Nickname(nick));
    Ok(())
}

/// Get current nickname.
pub async fn get_nickname(state: &ServerState) -> String {
    state.nickname.read().await.clone()
}

/// Broadcast a file message and upload its body to every connected peer.
///
/// The body is also cached locally so other peers we discover later (or peers
/// that reconnect) can pull from us.
pub async fn broadcast_file(
    state: &ServerState,
    file: FileMeta,
    body: Vec<u8>,
) -> Result<(), String> {
    // Cache the body locally so peers can fetch.
    state.transfers.put(file.sha256.clone(), body.clone());

    // Broadcast the metadata to all connected peers.
    let nick = state.nickname.read().await.clone();
    let msg = Message::new_file(file.clone(), nick);
    let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    state.store.insert(msg.clone());
    state.pool.broadcast(&json, None);
    let _ = state.events.send(UiEvent::Message(msg));

    // Best-effort: also push the body to each peer so they have it cached
    // (instead of having to pull from us later). We do this in parallel and
    // ignore individual failures.
    let peers: Vec<PeerInfo> = state.pool.list();
    for peer in peers {
        if peer.addr == "inbound" || peer.addr.is_empty() {
            continue; // inbound peers — we don't know their listening port
        }
        let url = format!("http://{}/upload", peer.addr);
        let sha = file.sha256.clone();
        let body = body.clone();
        let state_for_retry = state.clone();
        tokio::spawn(async move {
            match push_body(&url, &sha, &body).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("push to {} failed: {}", url, e);
                    // Fall back: rely on receiver to pull from us
                    let _ = state_for_retry
                        .events
                        .send(UiEvent::Notice(format!("push to {} failed; receiver will pull", peer.addr)));
                }
            }
        });
    }
    Ok(())
}

async fn push_body(url: &str, sha256: &str, body: &[u8]) -> Result<(), String> {
    use bytes::Bytes;
    let resp = reqwest_like_post(url, sha256, Bytes::copy_from_slice(body)).await?;
    if !resp.status().is_success() {
        return Err(format!("status {}", resp.status()._as_u16()));
    }
    Ok(())
}

/// Tiny in-process HTTP POST helper (no `reqwest` dependency).
/// We could use `hyper` directly, but axum is already in our deps — the cleanest
/// way is to spawn a task and use a hyper client. To keep deps minimal, we use
/// the standard library's TCP + manual HTTP/1.1. Acceptable for LAN.
async fn reqwest_like_post(
    url: &str,
    sha256: &str,
    body: bytes::Bytes,
) -> Result<HttpResponse, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    // Parse http://host:port/path
    let stripped = url.strip_prefix("http://").ok_or("not http")?;
    let (host_port, path) = match stripped.find('/') {
        Some(i) => (&stripped[..i], &stripped[i..]),
        None => (stripped, "/"),
    };

    let mut stream = TcpStream::connect(host_port)
        .await
        .map_err(|e| e.to_string())?;

    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/octet-stream\r\nX-Sha256: {sha256}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n",
        path = path,
        host_port = host_port,
        sha256 = sha256,
        len = body.len()
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(&body).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .map_err(|e| e.to_string())?;
    let raw = String::from_utf8_lossy(&buf).to_string();

    // Parse status line "HTTP/1.1 200 OK"
    let status: u16 = raw
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(HttpResponse { status, _raw: raw })
}

struct HttpResponse {
    status: u16,
    _raw: String,
}

impl HttpResponse {
    fn status(&self) -> StatusCode {
        StatusCode(self.status)
    }
}

struct StatusCode(u16);
impl StatusCode {
    fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }
    fn _as_u16(&self) -> u16 {
        self.0
    }
}

/// Download a file body from a peer.
pub async fn download_file(addr: &str, sha256: &str) -> Result<Vec<u8>, String> {
    let url = format!("http://{}/files/{}", addr, sha256);
    let stripped = url.strip_prefix("http://").ok_or("not http")?;
    let (host_port, path) = match stripped.find('/') {
        Some(i) => (&stripped[..i], &stripped[i..]),
        None => (stripped, "/"),
    };

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let mut stream = TcpStream::connect(host_port)
        .await
        .map_err(|e| e.to_string())?;
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .map_err(|e| e.to_string())?;
    let raw = buf;

    // Split header / body at \r\n\r\n
    let split_at = find_double_crlf(&raw).ok_or("malformed response")?;
    let (_header, body_with_body_len) = raw.split_at(split_at);
    let body = body_with_body_len[4..].to_vec(); // skip \r\n\r\n
    Ok(body)
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    for i in 0..buf.len().saturating_sub(3) {
        if &buf[i..i + 4] == b"\r\n\r\n" {
            return Some(i);
        }
    }
    None
}
