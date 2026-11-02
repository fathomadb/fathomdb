use current_history_followups::current::Certificate;
#[test]
fn certificate_requires_exact_scope_generation_and_half_open_time() {
    let cert = Certificate {
        published_at: 10,
        next_boundary: 20,
        visibility: 3,
        scope: "strict-v1".into(),
    };
    assert!(cert.covers(10, 3, "strict-v1"));
    assert!(cert.covers(19, 3, "strict-v1"));
    assert!(!cert.covers(20, 3, "strict-v1"), "clock boundary must invalidate even with no write");
    assert!(!cert.covers(9, 3, "strict-v1"));
    assert!(!cert.covers(10, 4, "strict-v1"));
    assert!(!cert.covers(10, 3, "archive"));
}

fn timed(
    id: &str,
    body: &str,
    from: Option<i64>,
    until: Option<i64>,
) -> fathomdb_engine::PreparedWrite {
    fathomdb_engine::PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: fathomdb_engine::SourceId::new("source").unwrap(),
        logical_id: Some(id.into()),
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: from,
        valid_until: until,
    }
}
#[test]
fn actual_readonly_pool_refreshes_both_outgoing_and_previously_absent_future_rows() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("current.sqlite")).unwrap();
    store
        .write(&[
            timed("old", "needle old", None, Some(100)),
            timed("future", "needle future", Some(100), None),
        ])
        .unwrap();
    store.bootstrap(50).unwrap();
    assert_eq!(store.search("needle", 50, 10).unwrap().rows[0].body, "needle old");
    let after = store.search("needle", 100, 10).unwrap();
    assert_eq!(
        after.rows.iter().map(|r| r.body.as_str()).collect::<Vec<_>>(),
        vec!["needle future"]
    );
    store.close().unwrap();
}

#[test]
fn compatible_publication_coalesces_and_healthy_reads_never_contact_writer() {
    use current_history_followups::current::CurrentStore;
    use std::sync::atomic::Ordering;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("coalesce.sqlite")).unwrap();
    store.write(&[timed("a", "needle", None, None)]).unwrap();
    store.bootstrap(50).unwrap();
    let contacts = store.writer_contacts.load(Ordering::Relaxed);
    for _ in 0..16 {
        assert_eq!(store.search("needle", 50, 10).unwrap().rows.len(), 1);
    }
    assert_eq!(store.writer_contacts.load(Ordering::Relaxed), contacts);
    store.bootstrap(51).unwrap();
    assert_eq!(
        store.refreshes.load(Ordering::Relaxed),
        1,
        "compatible queued publication must reuse the existing interval"
    );
    store.close().unwrap();
}

#[test]
fn pinned_v1_survives_v2_refresh_without_stale_publication_or_reader_starvation() {
    use current_history_followups::current::{CurrentStore, Failure, Pause};
    use std::sync::{mpsc, Arc};
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(CurrentStore::open(&dir.path().join("pinned.sqlite")).unwrap());
    store
        .write(&[
            timed("a", "needle old", None, Some(100)),
            timed("b", "needle new", Some(100), None),
        ])
        .unwrap();
    store.bootstrap(50).unwrap();
    let (ready, receive_ready) = mpsc::sync_channel(1);
    let (release, receive_release) = mpsc::sync_channel(1);
    let old_store = store.clone();
    let old = std::thread::spawn(move || {
        old_store
            .search_paused("needle", 50, 10, Some(Pause { ready, release: receive_release }))
            .unwrap()
    });
    receive_ready.recv_timeout(Duration::from_secs(5)).unwrap();
    let new = store.search("needle", 100, 10).unwrap();
    assert_eq!(new.rows[0].body, "needle new");
    assert_eq!(store.search("needle", 50, 10).unwrap_err(), Failure::BackwardTime);
    release.send(()).unwrap();
    let old = old.join().unwrap();
    assert_eq!(old.rows[0].body, "needle old");
    assert_ne!(old.worker, new.worker);
    store.close().unwrap();
    assert_eq!(store.search("needle", 100, 10).unwrap_err(), Failure::Closed);
}

#[test]
fn repair_publication_reconciles_candidate_and_failed_refresh_rolls_back() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("repair.sqlite")).unwrap();
    store
        .write(&[
            timed("a", "needle old", None, Some(100)),
            timed("b", "needle new", Some(100), None),
        ])
        .unwrap();
    store.bootstrap(50).unwrap();
    store.with_writer(|engine|engine.execute_for_test("CREATE TRIGGER poc_fail_refresh BEFORE UPDATE ON poc_certificate BEGIN SELECT RAISE(ABORT,'injected refresh rollback'); END;")).unwrap();
    assert!(store.search("needle", 100, 10).is_err());
    assert_eq!(store.search("needle", 50, 10).unwrap().rows[0].body, "needle old");
    store.with_writer(|engine| engine.execute_for_test("DROP TRIGGER poc_fail_refresh")).unwrap();
    assert_eq!(store.search("needle", 100, 10).unwrap().rows[0].body, "needle new");
    store
        .with_writer(|engine| {
            engine.execute_for_test("DELETE FROM poc_current").unwrap();
            engine.rebuild_projections()
        })
        .unwrap();
    assert_eq!(
        store.search("needle", 100, 10).unwrap().rows.len(),
        1,
        "generation-changing repair must republish missing current projection"
    );
    store.close().unwrap();
}

#[test]
fn first_install_over_populated_engine_seeds_boundary_frontier() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("existing.sqlite");
    let engine = fathomdb_engine::Engine::open(&path).unwrap().engine;
    engine
        .write(&[
            timed("a", "needle old", None, Some(100)),
            timed("b", "needle future", Some(100), None),
        ])
        .unwrap();
    engine.close().unwrap();
    let store = CurrentStore::open(&path).unwrap();
    store.bootstrap(50).unwrap();
    assert_eq!(store.search("needle", 100, 10).unwrap().rows[0].body, "needle future");
    store.close().unwrap();
}

#[test]
fn empty_limit_is_empty_and_admission_deadline_is_not_retry_count() {
    use current_history_followups::current::{CurrentStore, Failure};
    use std::sync::{mpsc, Arc};
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(CurrentStore::open(&dir.path().join("deadline.sqlite")).unwrap());
    store.write(&[timed("a", "needle", None, Some(100))]).unwrap();
    store.bootstrap(50).unwrap();
    let (held, ready) = mpsc::sync_channel(1);
    let writer = store.clone();
    let task = std::thread::spawn(move || {
        writer.with_writer(|_| {
            held.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        })
    });
    ready.recv().unwrap();
    assert_eq!(
        store.search_budget("needle", 100, 10, Duration::from_millis(10)).unwrap_err(),
        Failure::Deadline
    );
    task.join().unwrap();
    assert!(store.search("needle", 50, 0).unwrap().rows.is_empty());
    store.close().unwrap();
}

#[test]
fn zero_limit_never_emits_a_native_match() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("zero.sqlite")).unwrap();
    store.write(&[timed("a", "needle", None, None)]).unwrap();
    store.bootstrap(50).unwrap();
    assert!(store.search("needle", 50, 0).unwrap().rows.is_empty());
    store.close().unwrap();
}
