use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, FrozenReadErrorReason, InitialState, PageErrorReason, PageRequestV1,
    PreparedWrite, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice45-fixed", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        Ok(vector)
    }
}

fn open(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).unwrap()
}

fn node(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(logical_id.to_string()),
        kind: "slice45_doc".to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice45").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn page(limit: usize, cursor: Option<fathomdb_engine::PageCursor>) -> PageRequestV1 {
    PageRequestV1 { schema_version: 1, limit, cursor }
}

#[test]
fn canonical_pages_are_frozen_complete_and_cursor_bound() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("canonical{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[
            node("n-1", r#"{"owner":"alice","n":1}"#),
            node("n-2", r#"{"owner":"alice","n":2}"#),
            node("n-3", r#"{"owner":"alice","n":3}"#),
            node("n-4", r#"{"owner":"alice","n":4}"#),
            node("n-5", r#"{"owner":"alice","n":5}"#),
        ])
        .unwrap();

    let mut filter = SearchFilter::default();
    filter.kind = Some("slice45_doc".to_string());
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).unwrap())
        .unwrap();

    let first = opened.engine.read_canonical_page("slice45_doc", &frozen, &page(2, None)).unwrap();
    assert_eq!(
        first.items.iter().map(|row| row.logical_id.as_str()).collect::<Vec<_>>(),
        ["n-1", "n-2"]
    );
    let first_cursor = first.next_cursor.clone().expect("continuation");

    let repeated =
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(2, None)).unwrap();
    assert_eq!(first, repeated);

    let second = opened
        .engine
        .read_canonical_page("slice45_doc", &frozen, &page(2, Some(first_cursor.clone())))
        .unwrap();
    let third = opened
        .engine
        .read_canonical_page("slice45_doc", &frozen, &page(2, second.next_cursor.clone()))
        .unwrap();
    let ids = first
        .items
        .iter()
        .chain(&second.items)
        .chain(&third.items)
        .map(|row| row.logical_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["n-1", "n-2", "n-3", "n-4", "n-5"]);
    assert!(third.next_cursor.is_none());

    let mut tampered = first_cursor;
    tampered.0.push('0');
    assert!(matches!(
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(2, Some(tampered))),
        Err(EngineError::Page(error))
            if error.reason == PageErrorReason::CursorAuthenticationFailed
    ));

    opened.engine.write(&[node("n-6", r#"{"owner":"alice","n":6}"#)]).unwrap();
    assert!(matches!(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &frozen,
            &page(2, second.next_cursor),
        ),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    opened.engine.close().unwrap();
}

#[test]
fn operational_state_point_and_page_agree_without_reading_mutation_history() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("state{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "agent_state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    for (key, body) in [("a", r#"{"n":1}"#), ("b", r#"{"n":2}"#), ("c", r#"{"n":3}"#)] {
        opened
            .engine
            .write(&[PreparedWrite::OpStore {
                collection: "agent_state".to_string(),
                record_key: key.to_string(),
                schema_id: None,
                body: body.to_string(),
            }])
            .unwrap();
    }
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let first =
        opened.engine.read_operational_state_page("agent_state", &frozen, &page(2, None)).unwrap();
    let point =
        opened.engine.read_operational_state("agent_state", "a", Some(&frozen)).unwrap().unwrap();
    assert_eq!(point, first.items[0]);
    assert_eq!(point.payload, r#"{"n":1}"#);
    let second = opened
        .engine
        .read_operational_state_page("agent_state", &frozen, &page(2, first.next_cursor))
        .unwrap();
    assert_eq!(first.items.len() + second.items.len(), 3);
    assert!(second.next_cursor.is_none());
    opened.engine.close().unwrap();
}

#[test]
fn page_request_limit_and_operational_collection_are_typed_refusals() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("errors{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "events".to_string(),
            kind: "append_only_log".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(0, None)),
        Err(EngineError::Page(error)) if error.reason == PageErrorReason::InvalidPageLimit
    ));
    assert!(matches!(
        opened.engine.read_operational_state("events", "a", Some(&frozen)),
        Err(EngineError::Page(error)) if error.reason == PageErrorReason::CollectionKindMismatch
    ));
    opened.engine.close().unwrap();
}
