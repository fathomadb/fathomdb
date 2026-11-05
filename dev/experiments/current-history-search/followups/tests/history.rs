use current_history_followups::history;
use rusqlite::Connection;
#[test]
fn contentless_native_preserves_full_compiler_statistics_and_snapshot_identity() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let main = Connection::open_in_memory().unwrap();
    main.execute_batch("CREATE TABLE docs(id INTEGER PRIMARY KEY,body TEXT,eligible INTEGER);")
        .unwrap();
    let bodies = [
        "running runs runner",
        "café naïve 東京",
        "the and or",
        "!!!",
        "",
        "alpha beta alpha",
        "alpha",
        "quoted words",
        "other property alpha",
    ];
    for (i, body) in bodies.iter().enumerate() {
        main.execute("INSERT INTO docs VALUES(?1,?2,?3)", rusqlite::params![i + 1, body, i != 8])
            .unwrap();
    }
    let tx = main.unchecked_transaction().unwrap();
    for query in [
        "running",
        "run runs",
        "café 東京",
        "the and",
        "!!!",
        "\"quoted words\"",
        "alpha",
        "alpha OR beta",
        "",
        "a-b",
    ] {
        let regular = history::search(
            &tx,
            "SELECT id,body,eligible FROM docs",
            "SELECT body FROM docs WHERE id=?1",
            query,
            20,
            false,
            || Ok(()),
        )
        .unwrap();
        let compact = history::search(
            &tx,
            "SELECT id,body,eligible FROM docs",
            "SELECT body FROM docs WHERE id=?1",
            query,
            20,
            true,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(compact.documents, bodies.len());
        assert_eq!(compact.indexed_bytes, bodies.iter().map(|s| s.len()).sum::<usize>());
        assert_eq!(regular.hits, compact.hits, "compiler query {query:?}");
        assert_eq!(compact.temp_store, 2);
        assert!(compact.private_files.iter().all(String::is_empty));
        if query == "alpha" {
            assert!(!compact.hits.is_empty());
            assert!(compact.hits.iter().all(|h| h.id != 9));
        }
    }
}

#[test]
fn private_corpus_holds_one_snapshot_and_drops_on_error_before_erasure_retry() {
    use fathomdb_engine::*;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    let write = |body: &str| PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        logical_id: Some("claim".into()),
        source_id: SourceId::new("source").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    };
    engine.write(&[write("needle old")]).unwrap();
    engine.drain(10_000).unwrap();
    let mut main =
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    main.execute_batch("PRAGMA query_only=ON").unwrap();
    let select="SELECT write_cursor,body,1 FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL";
    let hydrate = "SELECT body FROM canonical_nodes WHERE write_cursor=?1";
    {
        let tx = main.transaction().unwrap();
        let old = history::search(&tx, select, hydrate, "needle", 10, true, || {
            engine.write(&[write("needle new")])?;
            Ok(())
        })
        .unwrap();
        assert_eq!(old.hits[0].body, "needle old");
    }
    {
        let tx = main.transaction().unwrap();
        let new = history::search(&tx, select, hydrate, "needle", 10, true, || Ok(())).unwrap();
        assert_eq!(new.hits[0].body, "needle new");
    }
    {
        let tx = main.transaction().unwrap();
        let failed = history::search(&tx, select, hydrate, "needle", 10, true, || {
            assert!(
                matches!(engine.erase_source("source"),Err(EngineError::ErasureIncomplete{ref stage,..}) if stage=="wal_checkpoint")
            );
            Err("forced post-population error".into())
        });
        assert!(failed.is_err());
    }
    engine.erase_source("source").unwrap();
    let tx = main.transaction().unwrap();
    let erased = history::search(&tx, select, hydrate, "needle", 10, true, || Ok(())).unwrap();
    assert_eq!(erased.documents, 0);
    assert!(erased.hits.is_empty());
    drop(tx);
    assert_eq!(main.query_row("PRAGMA query_only", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    engine.close().unwrap();
}

#[test]
fn real_temporal_selector_feeds_corrected_and_prior_native_corpora() {
    use current_history_followups::ledger::Ledger;
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("temporal.sqlite")).unwrap();
    let a = ledger.change("claim", "needle A", "source", (0, 100), false).unwrap();
    let b = ledger.change("claim", "needle B", "source", (20, 40), false).unwrap();
    for (valid, known, expected) in
        [(10, b, "needle A"), (30, b, "needle B"), (50, b, "needle A"), (30, a, "needle A")]
    {
        let tx = ledger.read.unchecked_transaction().unwrap();
        let selected = ledger.selected("claim", valid, known as i64).unwrap();
        let ids = selected.iter().map(|(id, _)| id.to_string()).collect::<Vec<_>>().join(",");
        let result = history::search(
            &tx,
            &format!(
                "SELECT write_cursor,body,1 FROM canonical_nodes WHERE write_cursor IN ({ids})"
            ),
            "SELECT body FROM canonical_nodes WHERE write_cursor=?1",
            "needle",
            10,
            true,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(result.hits[0].body, expected);
    }
    ledger.engine.close().unwrap();
}

#[test]
fn abandoned_receiver_keeps_owned_snapshot_until_worker_finishes_even_after_engine_close() {
    use fathomdb_engine::*;
    use std::sync::mpsc;
    use std::time::Duration;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("receiver.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "needle owned".into(),
            logical_id: Some("owner".into()),
            source_id: SourceId::new("source").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.drain(10_000).unwrap();
    let (ready, wait_ready) = mpsc::sync_channel(1);
    let (release, wait_release) = mpsc::sync_channel(1);
    let (reply, receiver) = mpsc::sync_channel(1);
    let reader_path = path.clone();
    let worker = std::thread::spawn(move || {
        let mut main =
            Connection::open_with_flags(reader_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        main.execute_batch("PRAGMA query_only=ON").unwrap();
        let tx = main.transaction().unwrap();
        let result = history::search(
            &tx,
            "SELECT write_cursor,body,1 FROM canonical_nodes",
            "SELECT body FROM canonical_nodes WHERE write_cursor=?1",
            "needle",
            10,
            true,
            || {
                ready.send(())?;
                wait_release.recv_timeout(Duration::from_secs(5))?;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result.hits[0].body, "needle owned");
        assert!(reply.send(result.hits.len()).is_err());
    });
    wait_ready.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(receiver);
    assert!(
        matches!(engine.erase_source("source"),Err(EngineError::ErasureIncomplete{ref stage,..}) if stage=="wal_checkpoint")
    );
    engine.close().unwrap();
    release.send(()).unwrap();
    worker.join().unwrap();
    let reopened = Engine::open(&path).unwrap().engine;
    reopened.erase_source("source").unwrap();
    reopened.close().unwrap();
}

#[test]
fn property_grouping_matches_real_engine_for_identical_owners_and_revisions() {
    use fathomdb_engine::*;
    use std::collections::BTreeSet;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("property-duplicates.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "p1".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    let write = |id: &str| PreparedWrite::Node {
        kind: "doc".into(),
        body: r#"{"p1":"needle identical"}"#.into(),
        logical_id: Some(id.into()),
        source_id: SourceId::new("source").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    };
    engine.write(&[write("a"), write("b")]).unwrap();
    engine.write(&[write("a")]).unwrap();
    engine.drain(10_000).unwrap();
    let reference = engine
        .search_projected_text(
            "needle",
            "p1",
            None,
            &ReadView { include_superseded: true, ..Default::default() },
        )
        .unwrap();
    assert_eq!(reference.results.len(), 3);
    let mut main =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tx = main.transaction().unwrap();
    let result = history::search_options(
        &tx,
        "SELECT write_cursor,json_extract(body,'$.p1'),1 FROM canonical_nodes",
        "SELECT body FROM canonical_nodes WHERE write_cursor=?1",
        "needle",
        history::Options { grouping: history::Grouping::Revision, ..Default::default() },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(result.hits.len(), reference.results.len());
    assert_eq!(
        result.hits.iter().map(|h| h.id as u64).collect::<Vec<_>>(),
        reference.results.iter().map(|h| h.write_cursor).collect::<Vec<_>>()
    );
    let body_grouped = history::search_options(
        &tx,
        "SELECT write_cursor,json_extract(body,'$.p1'),1 FROM canonical_nodes",
        "SELECT body FROM canonical_nodes WHERE write_cursor=?1",
        "needle",
        history::Options { limit: 2, grouping: history::Grouping::Body, ..Default::default() },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(body_grouped.hits.len(), 1);
    assert_eq!(body_grouped.hits[0].id, result.hits[0].id);
    drop(tx);
    engine.close().unwrap();
}
