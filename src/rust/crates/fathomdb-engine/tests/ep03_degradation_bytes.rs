//! EP-03 byte test: with a healthy embedder, adding `degradations` changes
//! nothing else. `results`, `soft_fallback` and `projection_cursor` of a fixed
//! fixture are pinned to the bytes the engine produced before the field
//! existed, and `degradations` is empty.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Deterministic, text-dependent vectors so the vector arm ranks real hits.
#[derive(Debug)]
struct ByteSumEmbedder;

impl Embedder for ByteSumEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("ep03-bytesum", "rev-a", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let sum: u32 = text.bytes().map(u32::from).sum();
        Ok((0..8u32).map(|i| ((sum * (i + 1)) % 97) as f32 / 97.0 + 0.01).collect())
    }
}

/// Captured from the engine before `degradations` existed.
const PINNED: &str = concat!(
    r#"(5, None, [SearchHit { id: IdSpace { space: Logical, value: "ep03-bytes-0" }, write_cursor: 1, kind: "note", body: "river stone carried downstream", score: 0.12903225806451613, branch: Vector, source_id: Some("test:ep03-bytes"), ce_score: None }"#,
    r#", SearchHit { id: IdSpace { space: Logical, value: "ep03-bytes-3" }, write_cursor: 4, kind: "note", body: "stone walls and river banks", score: 0.125, branch: Vector, source_id: Some("test:ep03-bytes"), ce_score: None }"#,
    r#", SearchHit { id: IdSpace { space: Logical, value: "ep03-bytes-1" }, write_cursor: 2, kind: "note", body: "a stone bridge over the river", score: 0.12032085561497327, branch: Vector, source_id: Some("test:ep03-bytes"), ce_score: None }"#,
    r#", SearchHit { id: IdSpace { space: Logical, value: "ep03-bytes-2" }, write_cursor: 3, kind: "note", body: "mountain lake at dawn", score: 0.030303030303030304, branch: Vector, source_id: Some("test:ep03-bytes"), ce_score: None }"#,
    r#", SearchHit { id: IdSpace { space: Logical, value: "ep03-bytes-4" }, write_cursor: 5, kind: "note", body: "quiet forest path", score: 0.02857142857142857, branch: Vector, source_id: Some("test:ep03-bytes"), ce_score: None }])"#,
);

#[test]
fn degradations_leave_results_and_soft_fallback_byte_identical() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ep03-bytes{SQLITE_SUFFIX}"));
    let engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(Arc::new(ByteSumEmbedder)),
        EngineConfig::default(),
    )
    .expect("open")
    .engine;
    engine.configure_vector_kind_for_test("note").expect("vector kind");
    let bodies = [
        "river stone carried downstream",
        "a stone bridge over the river",
        "mountain lake at dawn",
        "stone walls and river banks",
        "quiet forest path",
    ];
    let writes: Vec<PreparedWrite> = bodies
        .iter()
        .enumerate()
        .map(|(index, body)| PreparedWrite::Node {
            kind: "note".to_owned(),
            body: (*body).to_owned(),
            source_id: SourceId::new("test:ep03-bytes").expect("source id"),
            logical_id: Some(format!("ep03-bytes-{index}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&writes).expect("write");
    engine.drain(30_000).expect("drain");

    let result = engine.search("river stone").expect("search");
    let observed =
        format!("{:?}", (result.projection_cursor, &result.soft_fallback, &result.results));
    assert!(result.degradations.is_empty());
    assert_eq!(observed, PINNED);
    engine.close().expect("close");
}
