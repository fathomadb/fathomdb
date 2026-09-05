use std::collections::BTreeSet;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_page_after_validation_hook_for_test, Engine, EngineError, FrozenReadErrorReason,
    InitialState, PageCursor, PageErrorReason, PageRequestV1, PreparedWrite, ProjectionRole,
    ProjectionSpec, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

static RACE_HOOK_LOCK: Mutex<()> = Mutex::new(());

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

fn strict_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

fn assert_page_reason(
    result: Result<fathomdb_engine::PageV1<fathomdb_engine::NodeRecord>, EngineError>,
    reason: PageErrorReason,
) {
    assert!(matches!(result, Err(EngineError::Page(error)) if error.reason == reason));
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
fn canonical_pages_support_explicit_superseded_history_without_relaxing_search() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("history{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.write(&[node("n-1", r#"{"version":1}"#)]).unwrap();
    opened.engine.write(&[node("n-1", r#"{"version":2}"#)]).unwrap();
    let context = ReadContextV1::new(
        ReadView { include_superseded: true, ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let first = opened.engine.read_canonical_page("slice45_doc", &frozen, &page(1, None)).unwrap();
    let second = opened
        .engine
        .read_canonical_page("slice45_doc", &frozen, &page(1, first.next_cursor))
        .unwrap();
    assert_eq!(
        first.items.into_iter().chain(second.items).map(|row| row.body).collect::<Vec<_>>(),
        [r#"{"version":1}"#, r#"{"version":2}"#]
    );
    assert!(matches!(
        opened.engine.search_frozen("version", &frozen, 0, false, 0.3, 0, false, 10),
        Err(EngineError::InvalidArgument { .. })
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

#[test]
fn cursor_is_opaque_and_bound_with_documented_error_precedence() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("cursor{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[node("secret-logical-id", r#"{"owner":"secret-owner"}"#), node("n-2", "{}")])
        .unwrap();
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    let frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();
    let first = opened.engine.read_canonical_page("slice45_doc", &frozen, &page(1, None)).unwrap();
    let cursor = first.next_cursor.unwrap();
    let payload_hex = cursor.0.split('.').nth(1).unwrap();
    let payload = (0..payload_hex.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&payload_hex[offset..offset + 2], 16).unwrap())
        .collect::<Vec<_>>();
    for forbidden in ["secret-logical-id", "secret-owner", "slice45_doc", "state"] {
        assert!(!cursor.0.contains(forbidden), "cursor leaked {forbidden}");
        assert!(
            !payload.windows(forbidden.len()).any(|window| window == forbidden.as_bytes()),
            "decoded cursor payload leaked {forbidden}"
        );
    }

    assert_page_reason(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &frozen,
            &PageRequestV1 { schema_version: 2, limit: 0, cursor: Some(PageCursor("bad".into())) },
        ),
        PageErrorReason::UnsupportedSchemaVersion,
    );
    assert_page_reason(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &frozen,
            &PageRequestV1 { schema_version: 1, limit: 0, cursor: Some(PageCursor("bad".into())) },
        ),
        PageErrorReason::InvalidPageLimit,
    );
    assert_page_reason(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &frozen,
            &page(1, Some(PageCursor("bad".into()))),
        ),
        PageErrorReason::CursorMalformed,
    );
    assert_page_reason(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &frozen,
            &page(1, Some(PageCursor("x".repeat(2049)))),
        ),
        PageErrorReason::CursorTooLarge,
    );
    let pieces = cursor.0.split('.').collect::<Vec<_>>();
    let noncanonical =
        PageCursor(format!("{}.{}.{}", pieces[0], pieces[1].to_uppercase(), pieces[2]));
    assert_page_reason(
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(1, Some(noncanonical))),
        PageErrorReason::CursorMalformed,
    );
    let mut tampered = cursor.clone();
    tampered.0.push('0');
    assert_page_reason(
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(1, Some(tampered))),
        PageErrorReason::CursorAuthenticationFailed,
    );
    assert_page_reason(
        opened.engine.read_canonical_page("other", &frozen, &page(1, Some(cursor.clone()))),
        PageErrorReason::CursorMismatch,
    );
    assert_page_reason(
        opened.engine.read_canonical_page("slice45_doc", &frozen, &page(2, Some(cursor.clone()))),
        PageErrorReason::CursorMismatch,
    );
    let other_context = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    assert_page_reason(
        opened.engine.read_canonical_page(
            "slice45_doc",
            &other_context,
            &page(1, Some(cursor.clone())),
        ),
        PageErrorReason::CursorMismatch,
    );
    assert!(matches!(
        opened.engine.read_operational_state_page("state", &frozen, &page(1, Some(cursor.clone()))),
        Err(EngineError::Page(error)) if error.reason == PageErrorReason::CursorMismatch
    ));

    let second_path = dir.path().join(format!("other-database{SQLITE_SUFFIX}"));
    let second = open(&second_path);
    let key: String = Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_read_context_key'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    Connection::open(&second_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 WHERE key='_fathomdb_read_context_key'",
            [key],
        )
        .unwrap();
    let second_frozen = second.engine.freeze_read_context(&strict_context()).unwrap();
    assert_page_reason(
        second.engine.read_canonical_page("slice45_doc", &second_frozen, &page(1, Some(cursor))),
        PageErrorReason::DatabaseMismatch,
    );
    second.engine.close().unwrap();
    opened.engine.close().unwrap();
}

#[test]
fn every_eligibility_term_executes_before_page_truncation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("eligibility{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".to_string(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let mut writes = (0..300)
        .map(|index| PreparedWrite::Node {
            logical_id: Some(format!("excluded-{index:03}")),
            kind: "doc".to_string(),
            body: r#"{"owner":"bob"}"#.to_string(),
            source_id: SourceId::new("test:slice45-filter").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    writes.push(PreparedWrite::Node {
        logical_id: Some("eligible".to_string()),
        kind: "doc".to_string(),
        body: r#"{"owner":"alice"}"#.to_string(),
        source_id: SourceId::new("test:slice45-filter").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    writes.push(PreparedWrite::Node {
        logical_id: None,
        kind: "doc".to_string(),
        body: r#"{"owner":"alice"}"#.to_string(),
        source_id: SourceId::new("test:slice45-filter").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    opened.engine.write(&writes).unwrap();
    opened.engine.drain(10_000).unwrap();

    let mut filter = SearchFilter::default();
    filter.source_type = Some("article".to_string());
    filter.kind = Some("doc".to_string());
    filter.created_after = Some(0);
    filter.status = Some(String::new());
    filter.attributes = vec![("owner".to_string(), "alice".to_string())];
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).unwrap())
        .unwrap();
    let result = opened.engine.read_canonical_page("doc", &frozen, &page(1, None)).unwrap();
    assert_eq!(
        result.items.iter().map(|row| row.logical_id.as_str()).collect::<Vec<_>>(),
        ["eligible"]
    );
    assert!(result.next_cursor.is_none());
    opened.engine.close().unwrap();
}

#[test]
fn operational_governance_and_replacement_are_frozen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("governance{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();
    assert!(matches!(
        opened.engine.read_operational_state("missing", "key", None),
        Err(EngineError::Page(error)) if error.reason == PageErrorReason::CollectionNotFound
    ));
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    opened
        .engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: "  {\"exact\":true}  ".to_string(),
        }])
        .unwrap();
    let state_frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();
    let point =
        opened.engine.read_operational_state("state", "key", Some(&state_frozen)).unwrap().unwrap();
    assert_eq!(point.payload, "  {\"exact\":true}  ");
    assert!(opened.engine.read_operational_state("state", "absent", None).unwrap().is_none());
    opened
        .engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: "{\"exact\":false}".to_string(),
        }])
        .unwrap();
    assert!(matches!(
        opened.engine.read_operational_state("state", "key", Some(&state_frozen)),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    assert!(matches!(
        opened.engine.read_operational_state_page("state", &frozen, &page(1, None)),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    let mut canonical_only = SearchFilter::default();
    canonical_only.kind = Some("doc".to_string());
    let relaxed = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), canonical_only).unwrap())
        .unwrap();
    assert!(matches!(
        opened.engine.read_operational_state_page("state", &relaxed, &page(1, None)),
        Err(EngineError::Page(error)) if error.reason == PageErrorReason::ContextNotApplicable
    ));
    opened.engine.close().unwrap();
}

#[test]
fn page_reader_snapshot_linearizes_before_concurrent_write() {
    let _race_guard = RACE_HOOK_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("race{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let engine = Arc::new(opened.engine);
    engine.write(&[node("n-1", "{}"), node("n-2", "{}")]).unwrap();
    let frozen = engine.freeze_read_context(&strict_context()).unwrap();
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_page_after_validation_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let worker = {
        let engine = Arc::clone(&engine);
        let frozen = frozen.clone();
        thread::spawn(move || engine.read_canonical_page("slice45_doc", &frozen, &page(1, None)))
    };
    ready.wait();
    engine.write(&[node("n-3", "{}")]).unwrap();
    release.wait();
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.items[0].logical_id, "n-1");
    assert!(matches!(
        engine.read_canonical_page("slice45_doc", &frozen, &page(1, None)),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    engine.close().unwrap();
}

#[test]
fn operational_page_snapshot_linearizes_before_concurrent_replacement() {
    let _race_guard = RACE_HOOK_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("state-race{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let engine = Arc::new(opened.engine);
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: r#"{"value":1}"#.to_string(),
        }])
        .unwrap();
    let frozen = engine.freeze_read_context(&strict_context()).unwrap();
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_page_after_validation_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let worker = {
        let engine = Arc::clone(&engine);
        let frozen = frozen.clone();
        thread::spawn(move || engine.read_operational_state_page("state", &frozen, &page(1, None)))
    };
    ready.wait();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: r#"{"value":2}"#.to_string(),
        }])
        .unwrap();
    release.wait();
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.items[0].payload, r#"{"value":1}"#);
    assert!(matches!(
        engine.read_operational_state_page("state", &frozen, &page(1, None)),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    engine.close().unwrap();
}

#[test]
fn operational_point_snapshot_linearizes_before_concurrent_replacement() {
    let _race_guard = RACE_HOOK_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("state-point-race{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let engine = Arc::new(opened.engine);
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: r#"{"value":1}"#.to_string(),
        }])
        .unwrap();
    let frozen = engine.freeze_read_context(&strict_context()).unwrap();
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_page_after_validation_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let worker = {
        let engine = Arc::clone(&engine);
        let frozen = frozen.clone();
        thread::spawn(move || engine.read_operational_state("state", "key", Some(&frozen)))
    };
    ready.wait();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "state".to_string(),
            record_key: "key".to_string(),
            schema_id: None,
            body: r#"{"value":2}"#.to_string(),
        }])
        .unwrap();
    release.wait();
    let result = worker.join().unwrap().unwrap().unwrap();
    assert_eq!(result.payload, r#"{"value":1}"#);
    assert!(matches!(
        engine.read_operational_state("state", "key", Some(&frozen)),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    engine.close().unwrap();
}

#[test]
fn page_query_plans_use_governed_indexes_without_mutation_log_or_temp_sort() {
    let dir = TempDir::new().unwrap();
    let opened = open(&dir.path().join(format!("plans{SQLITE_SUFFIX}")));
    let plans = opened.engine.slice45_page_query_plans_for_test("doc").unwrap();
    let canonical = plans.iter().find(|(name, _)| name == "canonical_page").unwrap().1.join(" ");
    let point = plans.iter().find(|(name, _)| name == "operational_point").unwrap().1.join(" ");
    let state_page = plans.iter().find(|(name, _)| name == "operational_page").unwrap().1.join(" ");
    assert!(canonical.contains("canonical_nodes_kind_cursor_page_idx"), "{canonical}");
    assert!(point.contains("sqlite_autoindex_operational_state_1"), "{point}");
    assert!(state_page.contains("operational_state_collection_cursor_page_idx"), "{state_page}");
    for plan in [canonical, point, state_page] {
        assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
        assert!(!plan.contains("operational_mutations"), "{plan}");
    }
    opened.engine.close().unwrap();
}

#[test]
fn continuation_survives_restart_only_while_bound_state_is_unchanged() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("restart{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.write(&[node("n-1", "{}"), node("n-2", "{}")]).unwrap();
    let frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();
    let first = opened.engine.read_canonical_page("slice45_doc", &frozen, &page(1, None)).unwrap();
    opened.engine.close().unwrap();

    let reopened = open(&path);
    let second = reopened
        .engine
        .read_canonical_page("slice45_doc", &frozen, &page(1, first.next_cursor))
        .unwrap();
    assert_eq!(second.items[0].logical_id, "n-2");
    assert!(second.next_cursor.is_none());
    reopened.engine.close().unwrap();
}

#[test]
fn open_refuses_missing_page_schema_and_duplicate_post_migration_keys() {
    for (name, removal) in [
        ("missing-index", "DROP INDEX canonical_nodes_kind_cursor_page_idx"),
        ("missing-trigger", "DROP TRIGGER _fathomdb_read_visibility_os_au"),
    ] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
        let opened = open(&path);
        opened.engine.close().unwrap();
        Connection::open(&path).unwrap().execute_batch(removal).unwrap();
        assert!(Engine::open(&path).is_err(), "open accepted {name}");
    }

    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("duplicate{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.close().unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP INDEX canonical_nodes_kind_cursor_page_idx;
             INSERT INTO canonical_nodes(write_cursor,kind,body,logical_id)
             VALUES(7,'doc','{}','a'),(7,'doc','{}','b');",
        )
        .unwrap();
    drop(connection);
    assert!(Engine::open(&path).is_err(), "open accepted duplicate page keys");
}

#[test]
fn unsupported_operational_format_is_a_typed_refusal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("format{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "state".to_string(),
            kind: "latest_state".to_string(),
            schema_json: "{}".to_string(),
            retention_json: "{}".to_string(),
        }])
        .unwrap();
    Connection::open(&path)
        .unwrap()
        .execute("UPDATE operational_collections SET format_version=2 WHERE name='state'", [])
        .unwrap();
    assert!(matches!(
        opened.engine.read_operational_state("state", "key", None),
        Err(EngineError::Page(error))
            if error.reason == PageErrorReason::CollectionFormatUnsupported
    ));
    opened.engine.close().unwrap();
}
