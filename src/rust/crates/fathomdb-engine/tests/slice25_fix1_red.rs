use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::{Event, EventCategory, Phase, Subscriber};
use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState,
    LifecycleActuationV1, LifecycleState, ProjectionFts, ProjectionRole, ProjectionSpec,
    ProjectionVector, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::{Arc, Barrier, Mutex};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn hash(body: &str) -> CanonicalHash {
    let digest = Sha256::digest(body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    CanonicalHash::sha256(digest).unwrap()
}

fn canonical(revision: &str, logical: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "source body".into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{revision}")).unwrap(),
        ),
    }
}

fn derived(revision: &str, logical: &str, source_revision: &str) -> ProvenancedNodeV1 {
    derived_with_source_hash(revision, logical, source_revision, "source body")
}

fn derived_with_source_hash(
    revision: &str,
    logical: &str,
    source_revision: &str,
    source_body: &str,
) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: "derived body".into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{source_revision}")).unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            hash(source_body),
        ),
    }
}

#[derive(Debug)]
struct RollbackEmbedder;

impl Embedder for RollbackEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice25-rollback", "v1", 384)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 384])
    }
}

fn configure_rollback_projections(engine: &Engine) {
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "topic".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
}

fn projection_rollback_state(connection: &Connection) -> Vec<(String, Vec<String>)> {
    [
        (
            "property_search_index",
            "SELECT json_array(rowid,attr_value,attr_name,write_cursor) \
             FROM property_search_index ORDER BY rowid",
        ),
        (
            "_fathomdb_projection_registry",
            "SELECT json_array(name,roles,fts_tokenizer,vector_embedder,vector_declared,source) \
             FROM _fathomdb_projection_registry ORDER BY name",
        ),
        (
            "_fathomdb_projection_state",
            "SELECT json_array(kind,last_enqueued_cursor,updated_at) \
             FROM _fathomdb_projection_state ORDER BY kind",
        ),
        (
            "_fathomdb_vector_kinds",
            "SELECT json_array(kind,profile,created_at) FROM _fathomdb_vector_kinds ORDER BY kind",
        ),
        (
            "_fathomdb_vector_rows",
            "SELECT json_array(rowid,kind,write_cursor) FROM _fathomdb_vector_rows ORDER BY rowid",
        ),
    ]
    .into_iter()
    .map(|(name, sql)| {
        let rows = connection
            .prepare(sql)
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        (name.into(), rows)
    })
    .collect()
}

#[test]
fn create_depend_delete_is_refused_by_prospective_closure() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "create-depend-delete")).unwrap();
    let request = ActuationBatchV1::new(
        "create-depend-delete",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source")),
            ActuationOperationV1::PutDerivedNode(derived("derived-r1", "derived", "source-r1")),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-r1", "source-r1", "derived-r1").unwrap(),
            ),
            ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "source",
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    LifecycleState::Deleted,
                    Some("caller decision".into()),
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::DependencyClosureRequired]);
    assert_eq!(receipt.refused_operation_index, Some(3));
}

#[test]
fn mutated_nested_lifecycle_request_is_typed_not_a_panic() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "mutated-request")).unwrap();
    let mut lifecycle = LifecycleActuationV1::new(
        "source",
        ArtifactRevisionId::new("source-r1").unwrap(),
        LifecycleState::Deleted,
        None,
    )
    .unwrap();
    lifecycle.to_state = LifecycleState::Pending;
    let request = ActuationBatchV1::new(
        "mutated-request",
        vec![ActuationOperationV1::TransitionLifecycle(lifecycle)],
    )
    .unwrap();

    assert!(matches!(
        opened.engine.actuate(request),
        Err(EngineError::Actuation(error))
            if error.reason == ActuationErrorReason::LifecycleTargetInvalid
                && error.field_path == "/operations/0/toState"
    ));
}

#[test]
fn missing_dependency_endpoint_is_reference_unavailable_with_exact_path() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "missing-reference")).unwrap();
    let request = ActuationBatchV1::new(
        "missing-reference",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dep-r1", "source-r1", "derived-r1").unwrap(),
        )],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
    assert_eq!(
        receipt.refused_field_path.as_deref(),
        Some("/operations/0/dependency/derivedRevisionId")
    );
    assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
}

#[test]
fn nested_provenance_failure_preserves_its_exact_path() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "nested-path")).unwrap();
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "seed-source",
                vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
            )
            .unwrap(),
        )
        .unwrap();
    let bad = derived_with_source_hash("derived-r1", "derived", "source-r1", "wrong body");
    let request =
        ActuationBatchV1::new("bad-provenance", vec![ActuationOperationV1::PutDerivedNode(bad)])
            .unwrap();

    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(
        receipt.refused_field_path.as_deref(),
        Some("/operations/0/record/provenance/canonicalSourceHash")
    );
    assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
}

#[test]
fn later_invalid_operation_precedes_exhausted_dependency_generation() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "generation-precedence");
    let opened = Engine::open(&db_path).unwrap();
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "seed-chain",
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source")),
                    ActuationOperationV1::PutDerivedNode(derived(
                        "derived-r1",
                        "derived",
                        "source-r1",
                    )),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 \
             WHERE key='_fathomdb_dependency_generation'",
            [i64::MAX.to_string()],
        )
        .unwrap();
    let request = ActuationBatchV1::new(
        "generation-precedence",
        vec![
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-r1", "source-r1", "derived-r1").unwrap(),
            ),
            ActuationOperationV1::PutCanonicalNode(derived("role-mismatch", "other", "source-r1")),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::ProvenanceRoleMismatch]);
    assert_eq!(receipt.refused_operation_index, Some(1));
}

#[test]
fn keyed_conflict_records_typed_failure_telemetry() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "keyed-telemetry")).unwrap();
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "same-id",
                vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
            )
            .unwrap(),
        )
        .unwrap();
    let before = opened.engine.counters();
    let conflict = ActuationBatchV1::new(
        "same-id",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r2", "other"))],
    )
    .unwrap();
    assert!(matches!(opened.engine.actuate(conflict), Err(EngineError::Actuation(_))));
    let after = opened.engine.counters();

    assert_eq!(after.errors_by_code.get("ActuationError"), Some(&1));
    assert_eq!(after.writes, before.writes);
    assert_eq!(after.write_rows, before.write_rows);
}

#[test]
fn later_invalid_operation_precedes_exhausted_write_cursor() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "cursor-precedence");
    let opened = Engine::open(&db_path).unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "INSERT INTO _fathomdb_open_state(key,value) VALUES(\
               'tc33_reserved_write_cursor',?1\
             ) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [i64::MAX.to_string()],
        )
        .unwrap();
    let request = ActuationBatchV1::new(
        "cursor-precedence",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source")),
            ActuationOperationV1::PutCanonicalNode(derived(
                "role-mismatch",
                "derived-as-canonical",
                "source-r1",
            )),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::ProvenanceRoleMismatch]);
    assert_eq!(receipt.refused_operation_index, Some(1));
}

#[test]
fn prospective_closure_precedes_exhausted_dependency_generation() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "closure-precedence");
    let opened = Engine::open(&db_path).unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 \
             WHERE key='_fathomdb_dependency_generation'",
            [i64::MAX.to_string()],
        )
        .unwrap();
    let request = ActuationBatchV1::new(
        "closure-precedence",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source")),
            ActuationOperationV1::PutDerivedNode(derived("derived-r1", "derived", "source-r1")),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-r1", "source-r1", "derived-r1").unwrap(),
            ),
            ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "source",
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    LifecycleState::Deleted,
                    None,
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::DependencyClosureRequired]);
    assert_eq!(receipt.refused_operation_index, Some(3));
}

#[test]
fn same_id_race_counts_only_the_winning_mutation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "same-id-race")).unwrap();
    let engine = Arc::new(opened.engine);
    let barrier = Arc::new(Barrier::new(9));
    let request = Arc::new(
        ActuationBatchV1::new(
            "same-id-race",
            vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
        )
        .unwrap(),
    );

    let receipts = std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                let engine = Arc::clone(&engine);
                let barrier = Arc::clone(&barrier);
                let request = Arc::clone(&request);
                scope.spawn(move || {
                    barrier.wait();
                    engine.actuate((*request).clone()).unwrap()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>()
    });

    assert!(receipts.windows(2).all(|pair| pair[0] == pair[1]));
    let counters = engine.counters();
    assert_eq!(counters.writes, 1);
    assert_eq!(counters.write_rows, 1);
}

#[test]
fn source_erasure_redacts_more_than_one_receipt_page() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "paged-redaction");
    let opened = Engine::open(&db_path).unwrap();
    for index in 0..130 {
        let request = ActuationBatchV1::new(
            format!("shared-source-{index:03}"),
            vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
        )
        .unwrap();
        opened.engine.actuate(request).unwrap();
    }

    opened.engine.erase_source("source-a").unwrap();
    let connection = Connection::open(&db_path).unwrap();
    let erased: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_actuation_receipts WHERE outcome='erased'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let refs: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!((erased, refs), (130, 0));
}

#[derive(Default)]
struct EventSink {
    events: Mutex<Vec<Event>>,
}

impl Subscriber for EventSink {
    fn on_event(&self, event: &Event) {
        self.events.lock().unwrap().push(event.clone());
    }
}

impl EventSink {
    fn take_writer_phases(&self) -> Vec<Phase> {
        let mut events = self.events.lock().unwrap();
        let phases = events
            .iter()
            .filter(|event| event.category == EventCategory::Writer)
            .map(|event| event.phase)
            .collect();
        events.clear();
        phases
    }
}

fn assert_causal_terminal(phases: &[Phase], terminal: Phase) {
    assert_eq!(phases.first(), Some(&Phase::Started));
    assert_eq!(phases.last(), Some(&terminal));
    assert_eq!(phases.iter().filter(|phase| **phase == Phase::Started).count(), 1);
    assert_eq!(phases.iter().filter(|phase| **phase == terminal).count(), 1);
    assert!(phases[1..phases.len() - 1].iter().all(|phase| *phase == Phase::Slow));
}

#[test]
fn actuation_event_order_is_causal_for_commit_refusal_and_failure() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "event-order")).unwrap();
    let sink = Arc::new(EventSink::default());
    let _subscription = opened.engine.subscribe(Arc::clone(&sink) as Arc<dyn Subscriber>);
    opened.engine.set_slow_threshold_ms(0).unwrap();

    let committed = ActuationBatchV1::new(
        "event-commit",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
    )
    .unwrap();
    opened.engine.actuate(committed.clone()).unwrap();
    assert_causal_terminal(&sink.take_writer_phases(), Phase::Finished);

    let refused = ActuationBatchV1::new(
        "event-refusal",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dep-r1", "missing", "missing-derived").unwrap(),
        )],
    )
    .unwrap();
    opened.engine.actuate(refused).unwrap();
    assert_causal_terminal(&sink.take_writer_phases(), Phase::Finished);

    let conflict = ActuationBatchV1::new(
        "event-commit",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r2", "other"))],
    )
    .unwrap();
    assert!(opened.engine.actuate(conflict).is_err());
    assert_causal_terminal(&sink.take_writer_phases(), Phase::Failed);

    opened.engine.actuate(committed).unwrap();
    assert!(sink.take_writer_phases().is_empty());
}

#[test]
fn forced_slow_inner_race_replays_emit_no_extra_events() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "inner-race-events")).unwrap();
    let engine = Arc::new(opened.engine);
    let sink = Arc::new(EventSink::default());
    let _subscription = engine.subscribe(Arc::clone(&sink) as Arc<dyn Subscriber>);
    engine.set_slow_threshold_ms(0).unwrap();
    engine.set_actuation_after_initial_lookup_delay_ms_for_test(25);
    let barrier = Arc::new(Barrier::new(9));
    let request = Arc::new(
        ActuationBatchV1::new(
            "inner-race-events",
            vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
        )
        .unwrap(),
    );

    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                let engine = Arc::clone(&engine);
                let barrier = Arc::clone(&barrier);
                let request = Arc::clone(&request);
                scope.spawn(move || {
                    barrier.wait();
                    engine.actuate((*request).clone()).unwrap()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        for handle in handles {
            handle.join().unwrap();
        }
    });

    let phases = sink.take_writer_phases();
    assert_causal_terminal(&phases, Phase::Finished);
    assert!(phases.iter().filter(|phase| **phase == Phase::Slow).count() <= 1);
}

#[test]
fn cursor_refusal_precommit_failure_rolls_back_receipt_and_is_retryable() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "cursor-refusal-fault");
    let opened = Engine::open(&db_path).unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "INSERT INTO _fathomdb_open_state(key,value) VALUES(\
               'tc33_reserved_write_cursor',?1\
             ) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [i64::MAX.to_string()],
        )
        .unwrap();
    let request = ActuationBatchV1::new(
        "cursor-refusal-fault",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source"))],
    )
    .unwrap();

    opened.engine.force_next_commit_failure_for_test();
    assert!(matches!(opened.engine.actuate(request.clone()), Err(EngineError::Storage)));
    let receipt_count: i64 = Connection::open(&db_path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_actuation_receipts \
             WHERE operation_id='cursor-refusal-fault'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(receipt_count, 0);

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::WriteCursorExhausted]);
}

#[test]
fn infrastructure_failure_after_each_operation_rolls_back_the_whole_batch() {
    for fault_index in 0..4 {
        let dir = TempDir::new().unwrap();
        let db_path = path(&dir, &format!("operation-fault-{fault_index}"));
        let opened =
            Engine::open_with_embedder_for_test(&db_path, Arc::new(RollbackEmbedder)).unwrap();
        configure_rollback_projections(&opened.engine);
        let rollback_tables = [
            "canonical_nodes",
            "canonical_edges",
            "canonical_attributes",
            "property_search_index",
            "search_index",
            "search_index_v2",
            "search_index_edges",
            "_fathomdb_artifact_revisions",
            "_fathomdb_source_versions",
            "_fathomdb_source_links",
            "_fathomdb_source_dependencies",
            "_fathomdb_projection_registry",
            "_fathomdb_projection_state",
            "_fathomdb_projection_terminal",
            "_fathomdb_vector_kinds",
            "_fathomdb_vector_rows",
            "_fathomdb_actuation_receipts",
            "_fathomdb_actuation_receipt_source_refs",
        ];
        let before = Connection::open(&db_path).unwrap();
        let before_counts = rollback_tables
            .iter()
            .map(|table| {
                before
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let before_state = before
            .prepare("SELECT key,value FROM _fathomdb_open_state ORDER BY key")
            .unwrap()
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let before_projection_state = projection_rollback_state(&before);
        assert!(!before_projection_state[1].1.is_empty(), "projection registry fixture is live");
        drop(before);
        let mut source = canonical("source-r1", "source");
        source.kind = "s25src".into();
        source.body = r#"{"topic":"source"}"#.into();
        let mut derived =
            derived_with_source_hash("derived-r1", "derived", "source-r1", &source.body);
        derived.kind = "s25drv".into();
        derived.body = r#"{"topic":"derived"}"#.into();
        let request = ActuationBatchV1::new(
            format!("operation-fault-{fault_index}"),
            vec![
                ActuationOperationV1::PutCanonicalNode(source),
                ActuationOperationV1::PutDerivedNode(derived),
                ActuationOperationV1::RegisterSourceDependency(
                    SourceDependencyRegistrationV1::new("dep-r1", "source-r1", "derived-r1")
                        .unwrap(),
                ),
                ActuationOperationV1::TransitionLifecycle(
                    LifecycleActuationV1::new(
                        "derived",
                        ArtifactRevisionId::new("derived-r1").unwrap(),
                        LifecycleState::Deleted,
                        Some("caller decision".into()),
                    )
                    .unwrap(),
                ),
            ],
        )
        .unwrap();

        opened.engine.force_actuation_failure_after_operation_for_test(fault_index);
        assert!(matches!(opened.engine.actuate(request.clone()), Err(EngineError::Storage)));
        let connection = Connection::open(&db_path).unwrap();
        for (table, expected) in rollback_tables.iter().zip(&before_counts) {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
                .unwrap();
            assert_eq!(
                count, *expected,
                "fault after operation {fault_index} changed rows in {table}"
            );
        }
        let after_state = connection
            .prepare("SELECT key,value FROM _fathomdb_open_state ORDER BY key")
            .unwrap()
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(after_state, before_state, "fault changed cursor or generation state");
        assert_eq!(
            projection_rollback_state(&connection),
            before_projection_state,
            "fault after operation {fault_index} changed exact projection/vector state"
        );
        drop(connection);

        let receipt = opened.engine.actuate(request).unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
        opened.engine.drain(5_000).unwrap();
        let control = Connection::open(&db_path).unwrap();
        let control_state = projection_rollback_state(&control);
        assert_eq!(control_state[1], before_projection_state[1]);
        for (index, table) in rollback_tables.iter().enumerate() {
            let count: i64 = control
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
                .unwrap();
            if matches!(
                *table,
                "canonical_nodes"
                    | "canonical_attributes"
                    | "property_search_index"
                    | "search_index"
                    | "search_index_v2"
                    | "_fathomdb_artifact_revisions"
                    | "_fathomdb_source_versions"
                    | "_fathomdb_source_links"
                    | "_fathomdb_source_dependencies"
                    | "_fathomdb_projection_state"
                    | "_fathomdb_projection_terminal"
                    | "_fathomdb_vector_kinds"
                    | "_fathomdb_vector_rows"
                    | "_fathomdb_actuation_receipts"
                    | "_fathomdb_actuation_receipt_source_refs"
            ) {
                assert!(
                    count > before_counts[index],
                    "successful control did not increase rows in {table}: {} -> {count}",
                    before_counts[index]
                );
            }
        }
        for table in [
            "property_search_index",
            "_fathomdb_projection_state",
            "_fathomdb_vector_kinds",
            "_fathomdb_vector_rows",
        ] {
            let before_rows =
                before_projection_state.iter().find(|(name, _)| name == table).unwrap();
            let after_rows = control_state.iter().find(|(name, _)| name == table).unwrap();
            assert_ne!(
                after_rows.1, before_rows.1,
                "successful control did not change exact rows in {table}"
            );
        }
    }
}
