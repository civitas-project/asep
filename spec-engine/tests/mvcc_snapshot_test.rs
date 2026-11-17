use asep::snapshot::MvccStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[test]
fn test_snapshot_isolation() {
    let store: MvccStore<&'static str, i32> = MvccStore::new();
    let key1 = "key1";
    let key2 = "key2";

    let seq_v1 = store.insert(key1, 1);
    let snap1 = store.snapshot();
    let seq_v2 = store.insert(key2, 2);
    let snap2 = store.snapshot();
    store.delete(key1);
    let snap3 = store.snapshot();
    let snap_now = store.snapshot();

    assert_eq!(store.get(&key1, &snap1), Some(1));
    assert_eq!(store.get(&key2, &snap2), Some(2));
    assert_eq!(store.get(&key1, &snap3), None);
    assert_eq!(store.get(&key1, &snap_now), None);

    assert_eq!(store.get_at(&key1, seq_v1), Some(1));
    assert_eq!(store.get_at(&key2, seq_v2), Some(2));
    assert_eq!(store.get_at(&key1, u64::MAX), None);
}

#[test]
fn test_concurrent_readers_see_consistent_snapshots() {
    let store: MvccStore<&'static str, i32> = MvccStore::new();
    let key = "k";
    let writing = Arc::new(AtomicBool::new(true));
    let writer_writing = Arc::clone(&writing);

    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut value = 0i32;
            while writer_writing.load(Ordering::Acquire) {
                store.insert(key, value);
                value = value.wrapping_add(1);
                thread::sleep(std::time::Duration::from_micros(50));
            }
        });

        let mut readers = Vec::new();
        for _ in 0..8 {
            readers.push(scope.spawn(|| {
                let mut iterations = 0u32;
                while iterations < 50 {
                    let snap = store.snapshot();
                    let first = store.get(&key, &snap);
                    let second = store.get(&key, &snap);
                    assert_eq!(
                        first,
                        second,
                        "pinned snapshot must return a stable read"
                    );
                    iterations += 1;
                }
            }));
        }

        for reader in readers {
            reader.join().unwrap();
        }
        writing.store(false, Ordering::Release);
        writer.join().unwrap();
    });
}

#[test]
fn test_prune_obsolete_epoch_reclamation() {
    let store: MvccStore<&'static str, i32> = MvccStore::new();
    let key = "k";

    let seq_v1 = store.insert(key, 1);
    let snap_old = store.snapshot();
    let seq_v2 = store.insert(key, 2);
    let seq_tomb = store.delete(key);

    let pruned = store.prune_obsolete();
    assert_eq!(pruned, 0);
    assert_eq!(store.get_at(&key, seq_v1), Some(1));
    assert_eq!(store.get_at(&key, seq_v2), Some(2));
    assert_eq!(store.get(&key, &snap_old), Some(1));

    drop(snap_old);

    let pruned = store.prune_obsolete();
    assert!(pruned > 0);
    assert_eq!(store.get_at(&key, seq_tomb), None);
    assert_eq!(store.get_at(&key, u64::MAX), None);
}
