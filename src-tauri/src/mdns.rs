// lan-chat/src-tauri/src/mdns.rs
//
// mDNS service advertisement and peer discovery.
// Registers this node on the local network and connects outbound to discovered peers.

use crate::messages::Message;
use crate::peers::{PeerCmd, PeerInfo};
use crate::server::{ServerState, UiEvent};
use futures::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
pub fn start_mdns(
    node_id: String,
    port: u16,
    iface_ips: Vec<String>,
    state: ServerState,
) -> anyhow::Result<()> {
    let mdns = ServiceDaemon::new()?;

    for ip in &iface_ips {
        let mut props: HashMap<String, String> = HashMap::new();
        props.insert("node_id".to_string(), node_id.clone());

        let short_id = node_id.get(..8).unwrap_or(&node_id);
        let instance_name = format!("lanchat-{}-{}", short_id, ip.replace('.', "-"));
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

    let receiver = mdns.browse(SERVICE_TYPE)?;
    let node_id_clone = node_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        let _mdns_keep_alive = mdns;
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

    if remote_node_id.is_empty() || remote_node_id == my_node_id {
        return;
    }

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

pub async fn connect_to_peer(url: String, addr_str: String, state: ServerState) {
    let Ok((ws_stream, _)) = connect_async(&url).await else {
        tracing::warn!("Failed to connect to peer at {}", url);
        let _ = state
            .events
            .send(UiEvent::Notice(format!("connect failed: {}", url)));
        return;
    };

    let (mut sink, mut stream) = ws_stream.split();

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
        return;
    }

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
            }
            Some(Ok(WsMsg::Close(_))) | None => {
                tracing::warn!("Connection to {} closed before hello", url);
                return;
            }
            Some(Err(e)) => {
                tracing::warn!("WS error waiting for hello from {}: {}", url, e);
                return;
            }
            _ => {}
        }
    };

    if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
        return;
    }

    let (tx, rx) = mpsc::unbounded_channel::<PeerCmd>();
    let node_id = peer_info.node_id.clone();
    state.pool.add(peer_info, tx);
    let _ = state.events.send(UiEvent::Peers(state.pool.list()));

    let pool_clone = state.pool.clone();
    let node_id_write = node_id.clone();
    let state_write = state.clone();

    tokio::spawn(async move {
        run_tungstenite_writer(sink, rx).await;
        pool_clone.remove(&node_id_write);
        let _ = state_write.events.send(UiEvent::Peers(state_write.pool.list()));
    });

    while let Some(Ok(msg)) = stream.next().await {
        if let WsMsg::Text(t) = msg {
            handle_peer_message(&t, &node_id, &state);
        }
    }
}

fn handle_peer_message(json: &str, from_node_id: &str, state: &ServerState) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some("history") = v.get("type").and_then(|t| t.as_str()) {
            if let Some(arr) = v.get("messages").and_then(|m| m.as_array()) {
                for item in arr {
                    match serde_json::from_value::<Message>(item.clone()) {
                        Ok(msg) => {
                            if state.store.insert(msg.clone()) {
                                let _ = state.events.send(UiEvent::Message(msg));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to deserialize history message: {}", e);
                        }
                    }
                }
            }
            return;
        }
    }

    if let Ok(msg) = serde_json::from_str::<Message>(json) {
        if state.store.insert(msg.clone()) {
            state.pool.broadcast(json, Some(from_node_id));
            let _ = state.events.send(UiEvent::Message(msg));
        }
    }
}

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
