//! Slice 135 diagnostic probe. Copy this file to
//! `src/rust/crates/fathomdb-engine/tests/slice135_logic_probe.rs`, run the
//! command in the measurement note, then remove the temporary test file.
//! This is a deliberately RED contract probe, not a regular suite test.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    DataPlaneIntegrityCheckV1, DataPlaneIntegrityRequestV1, EmbedderChoice, Engine, EngineError,
    InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::Arc;
use tempfile::TempDir;

fn node(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("node {id}"),
        source_id: SourceId::new("slice135:logic-probe").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn malformed_edge_fts_kind_must_not_silently_remove_a_committed_hit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("logic-probe{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_choice(
        &path,
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .unwrap();
    let receipt = opened
        .engine
        .write(&[
            node("left"),
            node("right"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("slice135:logic-probe").unwrap(),
                logical_id: Some("edge".into()),
                body: Some("slice135uniqueedgefact".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control = opened.engine.search_text_only("slice135uniqueedgefact").unwrap();
    assert!(control.projection_cursor >= receipt.cursor);
    assert!(control.results.iter().any(|hit| hit.body == "slice135uniqueedgefact"));

    // FTS5 UNINDEXED columns accept SQLite dynamic types. A BLOB containing
    // invalid UTF-8 cannot decode as the String expected by the edge-hit mapper.
    // The independent integrity scan classifies this projection mismatch.
    opened
        .engine
        .execute_for_test(
            "UPDATE search_index_edges SET kind=x'ff' WHERE body='slice135uniqueedgefact'",
        )
        .unwrap();
    let physical: (String, i64) = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT typeof(kind), length(kind) FROM search_index_edges WHERE search_index_edges MATCH 'slice135uniqueedgefact'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    eprintln!("fts_kind_type_and_length={physical:?}");
    let integrity = opened
        .engine
        .check_data_plane_integrity(
            DataPlaneIntegrityRequestV1::new(
                vec![DataPlaneIntegrityCheckV1::ActiveSearchableOrphans],
                10_000,
                100,
            )
            .unwrap(),
        )
        .unwrap();
    eprintln!("integrity_findings={:?}", integrity.findings);
    assert!(!integrity.findings.is_empty(), "the fault must be independently visible");
    let observed = opened.engine.search_text_only("slice135uniqueedgefact");
    eprintln!("search_after_fault={observed:?}");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "a malformed FTS row must not produce a successful incomplete search"
    );
}
