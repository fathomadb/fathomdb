#![cfg(debug_assertions)]

use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, ReadView, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(logical_id: &str, body: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn in_transaction_refusal_releases_every_reader_snapshot() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("reader-release{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("open");
    opened
        .engine
        .write(&[
            node("erased", r#"{"summary":"erased needle"}"#, "slice80-erased"),
            node("control", "surviving control needle", "slice80-control"),
        ])
        .expect("seed");
    opened.engine.drain(5_000).expect("drain");

    let worker_count = opened.engine.reader_worker_count_for_test();
    assert_eq!(worker_count, 8, "the round-robin proof assumes the shipped eight-reader pool");
    let start = opened.engine.next_reader_worker_index_for_test();
    for index in 0..worker_count {
        assert_eq!(
            opened.engine.next_reader_worker_index_for_test(),
            (start + index) % worker_count,
            "refusal {index} must be routed to the expected reader"
        );
        let error = opened
            .engine
            .search_projected_text("needle", "undeclared", None, &ReadView::default())
            .expect_err("an undeclared projected-text field must be refused inside the reader tx");
        assert!(
            matches!(error, EngineError::InvalidFilter { .. }),
            "reader refusal must retain its typed InvalidFilter outcome: {error:?}"
        );
        assert_eq!(
            opened.engine.next_reader_worker_index_for_test(),
            (start + index + 1) % worker_count,
            "refusal {index} must consume exactly one reader dispatch"
        );
    }

    opened
        .engine
        .erase_source("slice80-erased")
        .expect("all refused reader transactions must release before WAL truncation");
    let result = opened.engine.search("surviving control").expect("search after erasure");
    assert!(result.results.iter().any(|hit| hit.body == "surviving control needle"));
    assert!(result.results.iter().all(|hit| !hit.body.contains("erased needle")));

    opened.engine.close().expect("close");
}
