//! MVCC snapshot isolation with epoch reclamation (Aethelgard pattern).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, RwLock};

pub type SequenceNumber = u64;

const EMPTY_SLOT: SequenceNumber = u64::MAX;
const SLOT_COUNT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValueKind {
    Put = 1,
    Delete = 0,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalKey<K> {
    pub user_key: K,
    pub seq: SequenceNumber,
    pub kind: ValueKind,
}

impl<K: Ord> Ord for InternalKey<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.user_key.cmp(&other.user_key) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match other.seq.cmp(&self.seq) {
            Ordering::Equal => {}
            ord => return ord,
        }
        other.kind.cmp(&self.kind)
    }
}

impl<K: Ord> PartialOrd for InternalKey<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SnapshotCoordinator {
    current_seq: AtomicU64,
    slots: [AtomicU64; SLOT_COUNT],
    overflow: RwLock<Vec<SequenceNumber>>,
}

impl Default for SnapshotCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotCoordinator {
    pub fn new() -> Self {
        Self {
            current_seq: AtomicU64::new(0),
            slots: std::array::from_fn(|_| AtomicU64::new(EMPTY_SLOT)),
            overflow: RwLock::new(Vec::new()),
        }
    }

    pub fn next_seq(&self) -> SequenceNumber {
        self.current_seq.fetch_add(1, AtomicOrdering::SeqCst) + 1
    }

    pub fn acquire_snapshot(self: &Arc<Self>) -> SnapshotHandle {
        let read_seq = self.current_seq.load(AtomicOrdering::SeqCst);
        for (idx, slot) in self.slots.iter().enumerate() {
            if slot
                .compare_exchange(
                    EMPTY_SLOT,
                    read_seq,
                    AtomicOrdering::SeqCst,
                    AtomicOrdering::SeqCst,
                )
                .is_ok()
            {
                return SnapshotHandle {
                    coordinator: Arc::clone(self),
                    read_seq,
                    slot_idx: Some(idx),
                };
            }
        }
        self.overflow.write().unwrap().push(read_seq);
        SnapshotHandle {
            coordinator: Arc::clone(self),
            read_seq,
            slot_idx: None,
        }
    }

    pub fn min_active_seq(&self) -> SequenceNumber {
        let mut min = self.current_seq.load(AtomicOrdering::SeqCst);
        for slot in &self.slots {
            min = min.min(slot.load(AtomicOrdering::SeqCst));
        }
        for seq in self.overflow.read().unwrap().iter() {
            min = min.min(*seq);
        }
        min
    }
}

pub struct SnapshotHandle {
    pub coordinator: Arc<SnapshotCoordinator>,
    pub read_seq: SequenceNumber,
    slot_idx: Option<usize>,
}

impl Drop for SnapshotHandle {
    fn drop(&mut self) {
        match self.slot_idx {
            Some(idx) => {
                self.coordinator.slots[idx].store(EMPTY_SLOT, AtomicOrdering::SeqCst);
            }
            None => {
                let mut overflow = self.coordinator.overflow.write().unwrap();
                if let Some(pos) = overflow.iter().position(|&seq| seq == self.read_seq) {
                    overflow.remove(pos);
                }
            }
        }
    }
}

pub struct MvccStore<K, V> {
    coordinator: Arc<SnapshotCoordinator>,
    entries: RwLock<BTreeMap<InternalKey<K>, Option<V>>>,
}

impl<K, V> Default for MvccStore<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> MvccStore<K, V> {
    pub fn new() -> Self {
        Self {
            coordinator: Arc::new(SnapshotCoordinator::new()),
            entries: RwLock::new(BTreeMap::new()),
        }
    }
}

impl<K: Ord, V> MvccStore<K, V> {
    pub fn insert(&self, key: K, val: V) -> SequenceNumber {
        let seq = self.coordinator.next_seq();
        self.entries.write().unwrap().insert(
            InternalKey {
                user_key: key,
                seq,
                kind: ValueKind::Put,
            },
            Some(val),
        );
        seq
    }

    pub fn delete(&self, key: K) -> SequenceNumber {
        let seq = self.coordinator.next_seq();
        self.entries.write().unwrap().insert(
            InternalKey {
                user_key: key,
                seq,
                kind: ValueKind::Delete,
            },
            None,
        );
        seq
    }

    pub fn snapshot(&self) -> SnapshotHandle {
        SnapshotCoordinator::acquire_snapshot(&self.coordinator)
    }

    // ponytail: linear scan stops at the first greater user key; ceiling is a
    // BTreeMap range query over a borrowed key bound, upgrade if this is hot.
    pub fn get_at(&self, key: &K, read_seq: SequenceNumber) -> Option<V>
    where
        K: Ord,
        V: Clone,
    {
        let entries = self.entries.read().unwrap();
        for (internal_key, value) in entries.iter() {
            match internal_key.user_key.cmp(key) {
                Ordering::Less => continue,
                Ordering::Greater => break,
                Ordering::Equal => {}
            }
            if internal_key.seq <= read_seq {
                return value.clone();
            }
        }
        None
    }

    pub fn get(&self, key: &K, snap: &SnapshotHandle) -> Option<V>
    where
        K: Ord,
        V: Clone,
    {
        self.get_at(key, snap.read_seq)
    }

    pub fn prune_obsolete(&self) -> usize
    where
        K: Ord + Clone,
    {
        let min_seq = self.coordinator.min_active_seq();
        let mut entries = self.entries.write().unwrap();
        let taken = std::mem::take(&mut *entries);
        let mut kept: BTreeMap<InternalKey<K>, Option<V>> = BTreeMap::new();
        let mut pruned = 0;
        let mut prev_key: Option<K> = None;
        let mut kept_at_or_below_min = false;

        for (internal_key, value) in taken.into_iter() {
            let is_new_key = match &prev_key {
                Some(pk) => *pk != internal_key.user_key,
                None => true,
            };
            if is_new_key {
                prev_key = Some(internal_key.user_key.clone());
                kept_at_or_below_min = false;
            }

            let InternalKey {
                user_key,
                seq,
                kind,
            } = internal_key;

            if seq > min_seq {
                // Must retain all revisions above min_active_seq for concurrent snapshots
                kept.insert(InternalKey { user_key, seq, kind }, value);
            } else if !kept_at_or_below_min {
                // First (newest) revision at or below min_active_seq
                kept_at_or_below_min = true;
                if kind == ValueKind::Put {
                    kept.insert(InternalKey { user_key, seq, kind }, value);
                } else {
                    // Delete tombstone with no active snapshots preceding it is elided
                    pruned += 1;
                }
            } else {
                // Obsolete historical revision below min_active_seq safely pruned
                pruned += 1;
            }
        }

        *entries = kept;
        pruned
    }
}
