use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, FrozenReadErrorReason, InitialState, PreparedWrite, ReadContextV1,
    ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice35-fixed", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        if vector.iter().all(|value| *value == 0.0) {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn open(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).unwrap()
}

fn node(logical_id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(logical_id.to_string()),
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice35").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn context(kind: &str) -> ReadContextV1 {
    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some(kind.to_string());
    ReadContextV1::new(ReadView::default(), eligibility).unwrap()
}

fn search(
    engine: &Engine,
    context: &fathomdb_engine::FrozenReadContextV1,
) -> Result<fathomdb_engine::SearchResult, EngineError> {
    engine.search_frozen("needle", context, 0, false, 0.3, 0, false, 10)
}

#[test]
fn frozen_context_survives_restart_and_binds_eligibility_without_leaking_it_in_token() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("frozen{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_vector_kind_for_test("note").unwrap();
    opened.engine.configure_vector_kind_for_test("task").unwrap();
    opened
        .engine
        .write(&[node("N1", "note", "needle filter-secret"), node("T1", "task", "needle task")])
        .unwrap();
    opened.engine.drain(10_000).unwrap();

    let frozen = opened.engine.freeze_read_context(&context("note")).unwrap();
    assert_eq!(frozen.schema_version, 1);
    assert_eq!(frozen.context.view.valid_as_of, Some(frozen.effective_valid_at));
    assert!(frozen.token.len() <= 1024);
    let payload_hex = frozen.token.split('.').nth(1).unwrap();
    let payload = (0..payload_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&payload_hex[index..index + 2], 16).unwrap())
        .collect::<Vec<_>>();
    assert!(!String::from_utf8_lossy(&payload).contains("filter-secret"));

    let first = search(&opened.engine, &frozen).unwrap();
    assert!(first.results.iter().any(|hit| hit.kind == "note"));
    assert!(first.results.iter().all(|hit| hit.kind == "note"));
    opened.engine.close().unwrap();

    let reopened = open(&path);
    let after_restart = search(&reopened.engine, &frozen).unwrap();
    assert_eq!(
        first.results.iter().map(|hit| &hit.id).collect::<Vec<_>>(),
        after_restart.results.iter().map(|hit| &hit.id).collect::<Vec<_>>()
    );
    reopened.engine.close().unwrap();
}

#[test]
fn frozen_context_rejects_tamper_database_mismatch_context_change_and_state_drift() {
    let left_dir = TempDir::new().unwrap();
    let right_dir = TempDir::new().unwrap();
    let left_path = left_dir.path().join(format!("left{SQLITE_SUFFIX}"));
    let right_path = right_dir.path().join(format!("right{SQLITE_SUFFIX}"));
    let left = open(&left_path);
    left.engine.configure_vector_kind_for_test("note").unwrap();
    left.engine.write(&[node("N1", "note", "needle one")]).unwrap();
    left.engine.drain(10_000).unwrap();
    let frozen = left.engine.freeze_read_context(&context("note")).unwrap();

    let mut tampered = frozen.clone();
    let replacement = if tampered.token.ends_with('0') { '1' } else { '0' };
    tampered.token.pop();
    tampered.token.push(replacement);
    assert!(matches!(
        search(&left.engine, &tampered),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::TokenAuthenticationFailed
    ));

    let mut changed_context = frozen.clone();
    changed_context.context.eligibility.kind = Some("task".to_string());
    assert!(matches!(
        search(&left.engine, &changed_context),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::ContextInvalid
    ));

    let right = open(&right_path);
    assert!(matches!(
        search(&right.engine, &frozen),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::DatabaseMismatch
    ));
    right.engine.close().unwrap();

    left.engine.write(&[node("N2", "note", "needle two")]).unwrap();
    assert!(matches!(
        search(&left.engine, &frozen),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    left.engine.close().unwrap();
}

#[test]
fn frozen_context_bounds_attributes_but_legacy_duplicate_conjunctions_remain_valid() {
    let mut legacy = SearchFilter::default();
    legacy.attributes = vec![
        ("scope".to_string(), "personal".to_string()),
        ("scope".to_string(), "personal".to_string()),
    ];
    assert!(ReadContextV1::new(ReadView::default(), legacy).is_ok());

    let mut too_many = SearchFilter::default();
    too_many.attributes = (0..65).map(|index| (format!("f{index}"), "v".to_string())).collect();
    assert!(matches!(
        ReadContextV1::new(ReadView::default(), too_many),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::ContextInvalid
    ));
}
