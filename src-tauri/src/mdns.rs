// lan-chat/src-tauri/src/mdns.rs
//
// mDNS service advertisement and peer discovery.
// Registers this node on the local network and connects outbound to discovered peers.

use crate::messages::Message;
use crate::peers::{PeerCmd, PeerInfo};
use crate::server::ServerState;
use futures::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMsg};

const SERVICE_TYPE: &str = "_lanchat._tcp.local.";

#[derive(Debug, Serialize, Deserialize)]
struct HelloMsg {
    #[serde(rename = "type")]
    msg_type: String,
    node_id: String,
    nickname: String,
    version: String,
}

/// Register this node via mDNS and start browsing for peers.
///
/// `iface_ips` should be the list of local IPv4 addresses to advertise on.
pub fn start_mdns(
    node_id: String,
    _nickname: Arc<tokio::sync::RwLock<String>>,
    port: u16,
    iface_ips: Vec<String>,
    state: ServerState,
) -> anyhow::Result<()> {
    let mdns = ServiceDaemon::new()?;

    // Register one service instance per interface IP
    for ip in &iface_ips {
        let mut props: HashMap<String, String> = HashMap::new();
        props.insert("node_id".to_string(), node_id.clone());

        // Instance name must be unique per IP to avoid collisions when advertising
        // on multiple interfaces from the same host.
        let instance_name = format!("lanchat-{}-{}", &node_id[..8], ip.replace('.', "-"));
        let host_name = format!("{}.local.", hostname());

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &host_name,
            ip.as_str(),
            port,
            props,
        )?;

        mdns.register(service)?;
        tracing::info!("mDNS registered on {}", ip);
    }

    // Browse for peers on the same service type
    let receiver = mdns.browse(SERVICE_TYPE)?;
    let node_id_clone = node_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    handle_resolved(info, &node_id_clone, &state_clone).await;
                }
                ServiceEvent::ServiceRemoved(_, full_name) => {
                    tracing::info!("mDNS peer removed: {}", full_name);
                }
                _ => {}
            }
        }
    });

    Ok(())
}

async fn handle_resolved(info: ServiceInfo, my_node_id: &str, state: &ServerState) {
    let remote_node_id = info
        .get_property_val_str("node_id")
        .unwrap_or("")
        .to_string();

    // Skip self and unknown/empty node IDs
    if remote_node_id.is_empty() || remote_node_id == my_node_id {
        return;
    }

    // Skip already-connected peers
    if state.pool.contains(&remote_node_id) {
        return;
    }

    let addrs: Vec<_> = info.get_addresses().iter().cloned().collect();
    let Some(addr) = addrs.first() else {
        return;
    };
    let peer_port = info.get_port();
    let url = format!("ws://{}:{}/ws", addr, peer_port);
    let addr_str = format!("{}:{}", addr, peer_port);

    tracing::info!("mDNS discovered peer {} at {}", remote_node_id, url);

    let state_for_conn = state.clone();
    tokio::spawn(async move {
        connect_to_peer(url, addr_str, state_for_conn).await;
    });
}

/// Establish an outbound WebSocket connection to a discovered peer.
pub async fn connect_to_peer(url: String, addr_str: String, state: ServerState) {
    let Ok((ws_stream, _)) = connect_async(&url).await else {
        tracing::warn!("Failed to connect to peer at {}", url);
        return;
    };

    let (mut sink, mut stream) = ws_stream.split();

    // 1. Send our hello
    let my_nick = state.nickname.read().await.clone();
    let hello = match serde_json::to_string(&HelloMsg {
        msg_type: "hello".into(),
        node_id: state.node_id.clone(),
        nickname: my_nick,
        version: "1".into(),
    }) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to serialize hello: {}", e);
            return;
        }
    };

    if sink.send(WsMsg::Text(hello)).await.is_err() {
        tracing::warn!("Failed to send hello to {}", url);
        return;
    }

    // 2. Wait for peer's hello response (server may send history first — skip non-hello frames)
    let peer_info = loop {
        match stream.next().await {
            Some(Ok(WsMsg::Text(t))) => {
                if let Ok(h) = serde_json::from_str::<HelloMsg>(&t) {
                    if h.msg_type == "hello" {
                        break PeerInfo {
                            node_id: h.node_id,
                            nickname: h.nickname,
                            addr: addr_str.clone(),
                        };
                    }
                }
                // Non-hello text (e.g. history envelope) — keep waiting
            }
            Some(Ok(WsMsg::Close(_))) | None => {
                tracing::warn!("Connection to {} closed before hello", url);
                return;
            }
            Some(Err(e)) => {
                tracing::warn!("WS error waiting for hello from {}: {}", url, e);
                return;
            }
            _ => {
                // Binary/Ping/Pong — ignore
            }
        }
    };

    // 3. Dedup check — guard against races (another task may have connected concurrently)
    if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
        return;
    }

    // 4. Register peer in pool
    let (tx, rx) = mpsc::unbounded_channel::<PeerCmd>();
    let node_id = peer_info.node_id.clone();
    state.pool.add(peer_info, tx);
    emit_peer_update(&state);

    // 5. Spawn write task (inline tungstenite writer — axum's writer can't be reused here)
    let pool_clone = state.pool.clone();
    let node_id_write = node_id.clone();
    let state_write = state.clone();

    tokio::spawn(async move {
        run_tungstenite_writer(sink, rx).await;
        pool_clone.remove(&node_id_write);
        emit_peer_update(&state_write);
    });

    // 6. Read loop — receive messages (including any deferred history) from this peer
    while let Some(Ok(msg)) = stream.next().await {
        if let WsMsg::Text(t) = msg {
            handle_peer_message(&t, &node_id, &state);
        }
    }

    // Writer task cleanup is handled in its own spawn above
}

fn handle_peer_message(json: &str, from_node_id: &str, state: &ServerState) {
    // Check if this is a history envelope
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some("history") = v.get("type").and_then(|t| t.as_str()) {
            if let Some(arr) = v.get("messages").and_then(|m| m.as_array()) {
                for item in arr {
                    if let Ok(msg) = serde_json::from_value::<Message>(item.clone()) {
                        if state.store.insert(msg.clone()) {
                            let _ = state.app.emit("message", &msg);
                        }
                    }
                }
            }
            return;
        }
    }

    // Regular chat message
    if let Ok(msg) = serde_json::from_str::<Message>(json) {
        if state.store.insert(msg.clone()) {
            state.pool.broadcast(json, Some(from_node_id));
            let _ = state.app.emit("message", &msg);
        }
    }
}

fn emit_peer_update(state: &ServerState) {
    let peers = state.pool.list();
    let _ = state.app.emit("peer_update", &peers);
}

/// Tungstenite WebSocket write task for outbound connections.
/// (Axum's WebSocket sink uses `axum::Error`; tokio-tungstenite uses `tungstenite::Error`.)
async fn run_tungstenite_writer(
    mut sink: impl futures::Sink<WsMsg, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    mut rx: mpsc::UnboundedReceiver<PeerCmd>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            PeerCmd::Send(json) => {
                if sink.send(WsMsg::Text(json)).await.is_err() {
                    break;
                }
            }
            PeerCmd::Close => {
                let _ = sink.send(WsMsg::Close(None)).await;
                break;
            }
        }
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "localhost".to_string())
}
