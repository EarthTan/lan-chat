// lan-chat/src-tauri/src/transfer.rs
//
// In-memory cache of file bodies keyed by sha256, plus outbound HTTP upload/download helpers.
//
// Design:
// - Both sender and receiver may cache the body. The sender caches so the receiver
//   can pull from us; the receiver caches so we can re-serve it (and so multiple
//   peers can fetch without re-uploading).
// - We do NOT cap size: a 1 GB file means ~1 GB of RAM. For a LAN chat app this is
//   acceptable; if needed later we can add an LRU cap or spill-to-disk.

use dashmap::DashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct TransferCache {
    /// sha256 (lowercase hex) -> raw bytes
    bodies: DashMap<String, Vec<u8>>,
}

impl TransferCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bodies: DashMap::new(),
        })
    }

    pub fn put(&self, sha256: String, body: Vec<u8>) {
        self.bodies.insert(sha256, body);
    }

    pub fn get(&self, sha256: &str) -> Option<Vec<u8>> {
        self.bodies.get(sha256).map(|v| v.clone())
    }

    pub fn contains(&self, sha256: &str) -> bool {
        self.bodies.contains_key(sha256)
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get() {
        let c = TransferCache::new();
        c.put("abc".into(), vec![1, 2, 3]);
        assert_eq!(c.get("abc").unwrap(), vec![1, 2, 3]);
        assert!(!c.contains("xyz"));
    }
}
