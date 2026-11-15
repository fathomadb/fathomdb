use super::*;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Barrier};

fn session(capacity: usize) -> (Arc<Collector>, Arc<Slice30ProjectionObservationGuard>) {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    let collector = Arc::new(Collector {
        engine_id: 17,
        session_id: 3,
        origin: Instant::now(),
        capacity,
        active: AtomicBool::new(true),
        next_span: AtomicU64::new(1),
        take_sequence: AtomicU64::new(0),
        dropped: AtomicU64::new(0),
        records: Mutex::new(Vec::new()),
        projection: true,
        projection_sender: Some(sender),
        next_projection_operation: AtomicU64::new(1_u64 << 63),
    });
    let guard = Arc::new(Slice30ProjectionObservationGuard {
        owner: Weak::new(),
        collector: Arc::clone(&collector),
        receiver: Mutex::new(receiver),
    });
    (collector, guard)
}

fn record(id: u64) -> Slice30ObservationRecord {
    Slice30ObservationRecord {
        engine_id: 17,
        session_id: 3,
        operation_id: id,
        span_id: id,
        parent_span_id: None,
        phase: "transport_control",
        started_ns: None,
        ended_ns: None,
        status: "ok",
        facts: serde_json::Value::Null,
    }
}

#[test]
fn four_producers_and_drain_preserve_256_records_with_space() {
    let (collector, guard) = session(512);
    let done = AtomicBool::new(false);
    let start = Barrier::new(5);
    let records = std::thread::scope(|scope| {
        let drain = scope.spawn(|| {
            let mut records = Vec::new();
            while !done.load(Ordering::Acquire) {
                let batch = guard.take();
                assert!(batch.records.len() <= 512);
                assert!(batch.issues.is_empty());
                assert_eq!(batch.dropped_records, 0);
                records.extend(batch.records);
                std::thread::yield_now();
            }
            let final_batch = guard.take();
            assert!(final_batch.records.len() <= 512);
            assert_eq!(final_batch.dropped_records, 0);
            assert!(final_batch.issues.is_empty());
            records.extend(final_batch.records);
            records
        });
        let producers = (0..4)
            .map(|producer| {
                let collector = &collector;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    for offset in 1..=64 {
                        collector.append(record(producer * 64 + offset));
                    }
                })
            })
            .collect::<Vec<_>>();
        start.wait();
        for producer in producers {
            producer.join().unwrap();
        }
        done.store(true, Ordering::Release);
        drain.join().unwrap()
    });
    assert_eq!(records.len(), 256);
    assert_eq!(
        records.iter().map(|record| record.span_id).collect::<BTreeSet<_>>(),
        (1..=256).collect()
    );
    assert!(records.iter().all(|record| record.engine_id == 17 && record.session_id == 3));
    assert_eq!(collector.dropped.load(Ordering::Relaxed), 0);
}

#[test]
fn full_capacity_two_counts_one_loss_and_retains_later_record() {
    let (collector, guard) = session(2);
    for id in 1..=3 {
        collector.append(record(id));
    }
    let first = guard.take();
    assert_eq!(first.records.iter().map(|record| record.span_id).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(first.dropped_records, 1);
    assert!(first.issues.is_empty());
    collector.append(record(4));
    let second = guard.take();
    assert_eq!(second.records.iter().map(|record| record.span_id).collect::<Vec<_>>(), [4]);
    assert_eq!(second.dropped_records, 1);
    assert_eq!(second.session_sequence, first.session_sequence + 1);
    assert!(second.issues.is_empty());
}

#[test]
fn disconnected_active_append_is_loss_but_retired_append_is_suppressed() {
    let (collector, guard) = session(2);
    let (_, replacement) = mpsc::sync_channel(1);
    drop(std::mem::replace(&mut *guard.receiver.lock().unwrap(), replacement));
    collector.append(record(1));
    assert_eq!(collector.dropped.load(Ordering::Relaxed), 1);
    collector.active.store(false, Ordering::Release);
    collector.append(record(2));
    assert_eq!(collector.dropped.load(Ordering::Relaxed), 1);
    let batch = guard.take();
    assert!(batch.records.is_empty());
    assert_eq!(batch.dropped_records, 1);
    assert!(batch.issues.is_empty());
}

#[test]
fn active_producers_keep_each_take_bounded_and_conserve_256_attempts() {
    let (collector, guard) = session(2);
    let done = AtomicBool::new(false);
    let start = Barrier::new(5);
    let records = std::thread::scope(|scope| {
        let drain = scope.spawn(|| {
            let mut records = Vec::new();
            while !done.load(Ordering::Acquire) {
                let batch = guard.take();
                assert!(batch.records.len() <= 2);
                assert!(batch.issues.is_empty());
                records.extend(batch.records);
                std::thread::yield_now();
            }
            let batch = guard.take();
            assert!(batch.records.len() <= 2);
            assert!(batch.issues.is_empty());
            records.extend(batch.records);
            records
        });
        let producers = (0..4)
            .map(|producer| {
                let collector = &collector;
                let start = &start;
                scope.spawn(move || {
                    start.wait();
                    for offset in 1..=64 {
                        collector.append(record(producer * 64 + offset));
                    }
                })
            })
            .collect::<Vec<_>>();
        start.wait();
        for producer in producers {
            producer.join().unwrap();
        }
        done.store(true, Ordering::Release);
        drain.join().unwrap()
    });
    let dropped = collector.dropped.load(Ordering::Relaxed);
    assert_eq!(records.len() as u64 + dropped, 256);
    let ids = records.iter().map(|record| record.span_id).collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), records.len());
    assert!(ids.iter().all(|id| (1..=256).contains(id)));
}
