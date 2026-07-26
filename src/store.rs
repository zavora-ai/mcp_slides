//! In-memory presentation handle store with bounded capacity.
//!
//! Each open/created presentation is keyed by a UUID handle. Access touches the
//! entry's `last_used`; capacity is enforced by LRU eviction and stale entries
//! are swept after a TTL of inactivity.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use uuid::Uuid;
use zavora_slide::Presentation;

const CAPACITY: usize = 25;
const TTL: Duration = Duration::from_secs(30 * 60);

struct Entry {
    pres: Presentation,
    last_used: Instant,
}

pub struct PresentationStore {
    map: HashMap<String, Entry>,
    capacity: usize,
    ttl: Duration,
}

impl PresentationStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            capacity: CAPACITY,
            ttl: TTL,
        }
    }

    /// Insert a presentation, returning its new UUID handle. Sweeps stale
    /// entries first, then evicts the least-recently-used entry if at capacity.
    pub fn insert(&mut self, pres: Presentation) -> String {
        self.sweep();
        if self.map.len() >= self.capacity {
            self.evict_lru();
        }
        let handle = Uuid::new_v4().to_string();
        self.map.insert(
            handle.clone(),
            Entry {
                pres,
                last_used: Instant::now(),
            },
        );
        handle
    }

    /// Borrow a presentation mutably, refreshing its `last_used` timestamp.
    pub fn get_mut(&mut self, handle: &str) -> Option<&mut Presentation> {
        let entry = self.map.get_mut(handle)?;
        entry.last_used = Instant::now();
        Some(&mut entry.pres)
    }

    /// Remove a presentation; returns true if it existed.
    pub fn remove(&mut self, handle: &str) -> bool {
        self.map.remove(handle).is_some()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Drop entries idle longer than the TTL.
    fn sweep(&mut self) {
        let ttl = self.ttl;
        self.map.retain(|_, e| e.last_used.elapsed() < ttl);
    }

    /// Evict the least-recently-used entry.
    fn evict_lru(&mut self) {
        if let Some(handle) = self
            .map
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(h, _)| h.clone())
        {
            self.map.remove(&handle);
        }
    }
}

impl Default for PresentationStore {
    fn default() -> Self {
        Self::new()
    }
}

pub type Shared = Arc<RwLock<PresentationStore>>;

pub fn new_store() -> Shared {
    Arc::new(RwLock::new(PresentationStore::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zavora_slide::Presentation;

    #[test]
    fn insert_get_remove() {
        let mut s = PresentationStore::new();
        let h = s.insert(Presentation::new());
        assert!(s.get_mut(&h).is_some());
        assert_eq!(s.len(), 1);
        assert!(s.remove(&h));
        assert!(!s.remove(&h));
        assert!(s.is_empty());
    }

    #[test]
    fn unknown_handle_is_none() {
        let mut s = PresentationStore::new();
        assert!(s.get_mut("nope").is_none());
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let mut s = PresentationStore {
            map: HashMap::new(),
            capacity: 2,
            ttl: TTL,
        };
        let h1 = s.insert(Presentation::new());
        std::thread::sleep(Duration::from_millis(2));
        let h2 = s.insert(Presentation::new());
        // Touch h1 so h2 becomes the LRU.
        std::thread::sleep(Duration::from_millis(2));
        assert!(s.get_mut(&h1).is_some());
        std::thread::sleep(Duration::from_millis(2));
        let h3 = s.insert(Presentation::new()); // capacity 2 → evicts LRU (h2)
        assert_eq!(s.len(), 2);
        assert!(s.get_mut(&h2).is_none(), "h2 should have been evicted");
        assert!(s.get_mut(&h1).is_some());
        assert!(s.get_mut(&h3).is_some());
    }

    #[test]
    fn ttl_sweep_drops_stale() {
        let mut s = PresentationStore {
            map: HashMap::new(),
            capacity: 25,
            ttl: Duration::from_millis(5),
        };
        let h1 = s.insert(Presentation::new());
        std::thread::sleep(Duration::from_millis(10));
        // Inserting triggers a sweep that removes the now-stale h1.
        let h2 = s.insert(Presentation::new());
        assert!(s.get_mut(&h1).is_none(), "stale entry should be swept");
        assert!(s.get_mut(&h2).is_some());
    }
}
