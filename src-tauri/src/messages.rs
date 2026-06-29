// lan-chat/src-tauri/src/messages.rs
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

pub const MAX_MESSAGES: usize = 200;
/// Dedup window: track recently-seen message IDs to prevent P2P broadcast loops
pub const DEDUP_WINDOW: usize = 500;

/// On-the-wire message — text or clipboard or file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub text: String,
    pub device: String,
    #[serde(rename = "type")]
    pub msg_type: MsgType,
    pub ts: u64,
    /// For `File` type: structured file metadata.
    /// For `Text`/`Clipboard`: `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<FileMeta>,
}

/// File metadata shared alongside a `File` message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMeta {
    /// sha256 of the file contents, hex encoded. Also the dedup key.
    pub sha256: String,
    pub filename: String,
    pub size: u64,
    /// "ip:port" of the host that holds the body. Receiver can fetch from
    /// `http://{addr}/files/{sha256}`.
    pub addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MsgType {
    Text,
    Clipboard,
    File,
}

impl Message {
    pub fn new(text: String, device: String, msg_type: MsgType) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let rand: u32 = rand_u32();
        Self {
            id: format!("{}_{:05x}", ts, rand & 0xFFFFF),
            text: text.trim().chars().take(8000).collect(),
            device: device.trim().chars().take(40).collect(),
            msg_type,
            ts,
            file: None,
        }
    }

    pub fn new_file(file: FileMeta, device: String) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let rand: u32 = rand_u32();
        // `text` for files carries a short human label; the real metadata is in `file`.
        let label = format!("[file] {} ({} B)", file.filename, file.size);
        Self {
            id: format!("{}_{:05x}", ts, rand & 0xFFFFF),
            text: label,
            device: device.trim().chars().take(40).collect(),
            msg_type: MsgType::File,
            ts,
            file: Some(file),
        }
    }
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn rand_u32() -> u32 {
    // Mix a monotonic counter with nanosecond timestamp to avoid ID collisions on
    // low-resolution clocks (e.g. Windows 15ms ticks) when called in rapid succession.
    let cnt = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    ((nanos ^ cnt.wrapping_mul(0x9e3779b97f4a7c15)) & 0xFFFFF) as u32
}

/// Thread-safe message store: ring buffer + dedup ID window
pub struct MessageStore {
    messages: Mutex<Vec<Message>>,
    seen_ids: Mutex<VecDeque<String>>, // VecDeque for O(1) pop_front when trimming
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(Vec::with_capacity(MAX_MESSAGES + 1)),
            seen_ids: Mutex::new(VecDeque::with_capacity(DEDUP_WINDOW + 1)),
        }
    }

    /// Try to insert a message. Returns false if this ID was already seen (duplicate).
    pub fn insert(&self, msg: Message) -> bool {
        let mut seen = self.seen_ids.lock().unwrap();
        if seen.contains(&msg.id) {
            return false;
        }
        seen.push_back(msg.id.clone());
        while seen.len() > DEDUP_WINDOW {
            seen.pop_front();
        }
        drop(seen);

        let mut msgs = self.messages.lock().unwrap();
        msgs.push(msg);
        if msgs.len() > MAX_MESSAGES {
            let excess = msgs.len() - MAX_MESSAGES;
            msgs.drain(0..excess);
        }
        true
    }

    pub fn history(&self) -> Vec<Message> {
        self.messages.lock().unwrap().clone()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}
