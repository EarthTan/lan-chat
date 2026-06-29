// lan-chat/src-tauri/src/peers.rs
//
// Manages all WebSocket peer connections (outbound + inbound).
// Each peer is uniquely identified by node_id (UUID); same node_id won't be connected twice.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Metadata for each connected peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub nickname: String,
    pub addr: String, // "ip:port"
}

/// Command sent to a peer's write task
#[derive(Debug)]
pub enum PeerCmd {
    Send(String), // JSON string
    Close,
}

/// Global peer connection pool
pub struct PeerPool {
    /// node_id -> sender channel (write end of the WebSocket writer task)
    senders: DashMap<String, mpsc::UnboundedSender<PeerCmd>>,
    /// node_id -> PeerInfo
    infos: DashMap<String, PeerInfo>,
}

impl PeerPool {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            senders: DashMap::new(),
            infos: DashMap::new(),
        })
    }

    /// Register a new peer (called after connection is established)
    pub fn add(&self, info: PeerInfo, tx: mpsc::UnboundedSender<PeerCmd>) {
        self.senders.insert(info.node_id.clone(), tx);
        self.infos.insert(info.node_id.clone(), info);
    }

    /// Disconnect and remove a peer
    pub fn remove(&self, node_id: &str) {
        if let Some((_, tx)) = self.senders.remove(node_id) {
            let _ = tx.send(PeerCmd::Close);
        }
        self.infos.remove(node_id);
    }

    /// Check if this node_id is already connected
    pub fn contains(&self, node_id: &str) -> bool {
        self.senders.contains_key(node_id)
    }

    /// Broadcast JSON to all peers (excluding exclude_node_id to prevent loops)
    pub fn broadcast(&self, json: &str, exclude_node_id: Option<&str>) {
        for entry in self.senders.iter() {
            if let Some(excl) = exclude_node_id {
                if entry.key().as_str() == excl {
                    continue;
                }
            }
            let _ = entry.value().send(PeerCmd::Send(json.to_string()));
        }
    }

    /// Get all peer info (for frontend display)
    pub fn list(&self) -> Vec<PeerInfo> {
        self.infos.iter().map(|e| e.value().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.senders.len()
    }
}

impl Default for PeerPool {
    fn default() -> Self {
        Self {
            senders: DashMap::new(),
            infos: DashMap::new(),
        }
    }
}
