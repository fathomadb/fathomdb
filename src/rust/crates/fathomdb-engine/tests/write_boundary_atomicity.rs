//! 0.8.27 Slice 60 — write-boundary atomicity characterization.
//!
//! Every failure boundary of `Engine::write`, the extractor provider, and the
//! consolidation provider must leave every durable state plane byte-identical
//! to its pre-call snapshot. The snapshot is derived from `sqlite_master`, so a
//! new state plane cannot be silently omitted. After each refused `write`, a
//! valid write must receive the next unconsumed cursor.

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, ConsolidateAxis, Engine, EngineError, ExtractDocument, InitialState,
    OpenedEngine, PreparedWrite, ProjectionRole, ProjectionSpec, ProjectionVector,
    ProvenanceErrorReason, ProvenancedNodeV1, SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestRunner};
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

const DRAIN_TIMEOUT_MS: u64 = 10_000;
/// Vector-committable kind that the fixture never enrols, so every refused
/// batch carrying it has a pending late enrolment.
const LATE_KIND: &str = "note";
/// Kind no case's trigger matches; used only by the cursor probe.
const PROBE_KIND: &str = "probe";
const SEED_REVISION: &str = "seed-revision-1";

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice60-boundary", "1", 384)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 384];
        vector[0] = 1.0;
        Ok(vector)
    }
}

type TableRows = Vec<Vec<String>>;

#[derive(Debug, PartialEq, Eq)]
struct FullSnapshot {
    schema: TableRows,
    tables: Vec<(String, TableRows)>,
}

struct Fixture {
    _dir: TempDir,
    path: PathBuf,
    opened: OpenedEngine,
    /// The cursor of the last committed setup write.
    cursor: u64,
}

impl Fixture {
    fn engine(&self) -> &Engine {
        &self.opened.engine
    }

    fn write(&mut self, batch: &[PreparedWrite]) {
        let receipt = self.engine().write(batch).expect("fixture write");
        self.cursor = receipt.cursor;
    }

    /// Drain the projection worker, assert no nonterminal dependency closure is
    /// pending, and capture the full committed database state.
    fn settled_snapshot(&self) -> FullSnapshot {
        self.engine().drain(DRAIN_TIMEOUT_MS).expect("projection worker must drain");
        let connection = Connection::open(&self.path).unwrap();
        let pending: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_dependency_closures \
                 WHERE phase IN ('proving','incomplete')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0, "closure precondition: no nonterminal closure may be pending");
        full_snapshot(&connection)
    }

    fn assert_unchanged(&self, before: &FullSnapshot) {
        let after = self.settled_snapshot();
        assert_eq!(
            &after, before,
            "a refused call must leave every durable state plane byte-identical"
        );
    }

    /// A following valid write must receive the next unconsumed cursor.
    fn assert_next_cursor_unconsumed(&self) {
        let receipt = self
            .engine()
            .write(&[plain_node(PROBE_KIND, "cursor-probe", "probe body", None, None)])
            .expect("cursor probe write");
        assert_eq!(
            receipt.row_cursors,
            vec![self.cursor + 1],
            "a refused write must neither publish nor consume a cursor"
        );
    }
}

fn byte_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn encoded_rows(connection: &Connection, sql: &str) -> TableRows {
    let mut statement = connection.prepare(sql).unwrap();
    let width = statement.column_count();
    let mut rows = statement
        .query_map([], |row| {
            (0..width)
                .map(|column| {
                    Ok(match row.get_ref(column)? {
                        ValueRef::Null => "null".to_string(),
                        ValueRef::Integer(value) => format!("integer:{value}"),
                        ValueRef::Real(value) => format!("real:{:016x}", value.to_bits()),
                        ValueRef::Text(value) => format!("text:{}", byte_hex(value)),
                        ValueRef::Blob(value) => format!("blob:{}", byte_hex(value)),
                    })
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    rows.sort();
    rows
}

/// Every row of every ordinary table (virtual-table shadow tables included) plus
/// the `sqlite_master` rows themselves.
fn full_snapshot(connection: &Connection) -> FullSnapshot {
    let schema =
        encoded_rows(connection, "SELECT type, name, tbl_name, rootpage, sql FROM sqlite_master");
    let names = connection
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' \
             AND COALESCE(sql,'') NOT LIKE 'CREATE VIRTUAL TABLE%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(
        names.iter().any(|name| name == "canonical_nodes"),
        "snapshot must see canonical tables"
    );
    let tables = names
        .into_iter()
        .map(|name| {
            let rows = encoded_rows(connection, &format!("SELECT * FROM \"{name}\""));
            (name, rows)
        })
        .collect();
    FullSnapshot { schema, tables }
}

fn source(id: &str) -> SourceId {
    SourceId::new(id).unwrap()
}

fn plain_node(
    kind: &str,
    logical_id: &str,
    body: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: source("slice60-source"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
    }
}

fn late_node(logical_id: &str) -> PreparedWrite {
    plain_node(LATE_KIND, logical_id, &format!("late body {logical_id}"), None, None)
}

fn plain_edge(
    kind: &str,
    from: &str,
    to: &str,
    logical_id: &str,
    body: Option<&str>,
    t_valid: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: source("slice60-source"),
        logical_id: Some(logical_id.into()),
        body: body.map(str::to_string),
        t_valid,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn provenanced_seed(logical_id: &str, version: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("{{\"summary\":\"canonical {logical_id}\"}}"),
        source_id: source("slice60-provenanced"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(SEED_REVISION).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
    })
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

/// Active nodes and an edge, a provenanced node with a source version, a
/// declared vector projection, and a live embedder so late enrolment is live.
fn seeded(name: &str) -> Fixture {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let mut fixture = Fixture { _dir: dir, path, opened, cursor: 0 };
    // One item per seed write, so the seed is independent of batch composition.
    for seed in [
        plain_node("doc", "alice", "{\"summary\":\"alice\"}", None, None),
        plain_node("doc", "bob", "{\"summary\":\"bob\"}", None, None),
        plain_edge("knows", "alice", "bob", "alice-knows-bob", Some("Alice knows Bob"), None),
        provenanced_seed("seed-source", "seed-version-1"),
    ] {
        fixture.write(&[seed]);
    }
    fixture.engine().configure_projections(&[vector_spec()], &[]).unwrap();
    fixture.engine().drain(DRAIN_TIMEOUT_MS).unwrap();
    fixture.write(&[plain_node("doc", "anchor", "{\"summary\":\"anchor\"}", None, None)]);
    fixture.engine().drain(DRAIN_TIMEOUT_MS).unwrap();
    let connection = Connection::open(&fixture.path).unwrap();
    let late_enrolled: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_vector_kinds WHERE kind=?1",
            [LATE_KIND],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(late_enrolled, 0, "the late kind must start unenrolled");
    fixture
}

#[cfg(any(debug_assertions, feature = "test-hooks"))]
fn drop_temp_trigger(fixture: &Fixture, name: &str) {
    fixture.engine().execute_for_test(&format!("DROP TRIGGER temp.{name}")).unwrap();
}

#[cfg(feature = "test-hooks")]
fn arm_next_write_commit_abort(fixture: &Fixture) {
    fixture
        .engine()
        .execute_for_test(
            "CREATE TEMP TABLE _fathomdb_test_abort_next_write_commit(marker INTEGER)",
        )
        .unwrap();
}

#[test]
fn structural_validation_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("structural");
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[
        late_node("structural-1"),
        late_node("structural-2"),
        plain_node(LATE_KIND, "structural-bad", "inverted window", Some(20), Some(10)),
    ]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::WriteValidation);
    fixture.assert_next_cursor_unconsumed();
}

#[test]
fn db_dependent_schema_refusal_leaves_full_state_unchanged() {
    let mut fixture = seeded("db-dependent");
    fixture.write(&[PreparedWrite::AdminSchema {
        name: "validated".into(),
        kind: "append_only_log".into(),
        schema_json: r#"{"type":"object","required":["n"],"properties":{"n":{"type":"integer"}}}"#
            .into(),
        retention_json: "{}".into(),
    }]);
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[
        late_node("db-dependent-1"),
        PreparedWrite::OpStore {
            collection: "validated".into(),
            record_key: "bad".into(),
            schema_id: Some("validated".into()),
            body: r#"{"n":"not an integer"}"#.into(),
        },
    ]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::SchemaValidation);
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(any(debug_assertions, feature = "test-hooks"))]
#[test]
fn enrolment_raise_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("enrolment-raise");
    fixture
        .engine()
        .execute_for_test(&format!(
            "CREATE TEMP TRIGGER slice60_enrolment_raise \
             BEFORE INSERT ON main._fathomdb_vector_kinds WHEN NEW.kind = '{LATE_KIND}' \
             BEGIN SELECT RAISE(ABORT, 'slice60 enrolment fault'); END"
        ))
        .unwrap();
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[late_node("enrolment-1")]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    drop_temp_trigger(&fixture, "slice60_enrolment_raise");
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(debug_assertions)]
#[test]
fn pre_transaction_hook_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("pre-tx-hook");
    let before = fixture.settled_snapshot();
    fixture.engine().force_next_commit_failure_for_test();
    let outcome = fixture.engine().write(&[late_node("hook-1"), late_node("hook-2")]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    fixture.assert_next_cursor_unconsumed();
}

#[test]
fn late_provenance_refusal_rolls_back_pending_enrolment() {
    let fixture = seeded("late-provenance");
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[
        late_node("late-provenance-1"),
        provenanced_seed("seed-replay", "seed-version-2"),
    ]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert!(
        matches!(
            error,
            EngineError::Provenance(ref error)
                if error.reason == ProvenanceErrorReason::RevisionIdConflict
        ),
        "expected RevisionIdConflict, got {error:?}"
    );
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(any(debug_assertions, feature = "test-hooks"))]
#[test]
fn execution_raise_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("execution-raise");
    fixture
        .engine()
        .execute_for_test(
            "CREATE TEMP TRIGGER slice60_execution_raise \
             BEFORE INSERT ON main.canonical_edges WHEN NEW.kind = 'slice60_sentinel' \
             BEGIN SELECT RAISE(ABORT, 'slice60 execution fault'); END",
        )
        .unwrap();
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[
        late_node("execution-1"),
        late_node("execution-2"),
        plain_edge("slice60_sentinel", "execution-1", "execution-2", "sentinel", None, None),
    ]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    drop_temp_trigger(&fixture, "slice60_execution_raise");
    fixture.assert_next_cursor_unconsumed();
}

#[test]
fn visibility_exhaustion_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("visibility-last");
    let original: i64 = {
        let connection = Connection::open(&fixture.path).unwrap();
        let original = connection
            .query_row(
                "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection
            .execute(
                "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
                [i64::MAX],
            )
            .unwrap();
        original
    };
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().write(&[late_node("visibility-1")]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    Connection::open(&fixture.path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
            [original],
        )
        .unwrap();
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(feature = "test-hooks")]
#[test]
fn trigger_suppressed_commit_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("commit-suppressed");
    let before = fixture.settled_snapshot();
    arm_next_write_commit_abort(&fixture);
    let outcome = fixture.engine().write(&[late_node("commit-suppressed-1")]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(feature = "test-hooks")]
#[test]
fn row_trigger_commit_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("commit-row-trigger");
    fixture
        .engine()
        .execute_for_test(
            "CREATE TEMP TRIGGER slice60_commit_row_trigger \
             AFTER INSERT ON main.canonical_nodes \
             WHEN NEW.logical_id = 'commit-row-trigger-1' \
             BEGIN SELECT 1; END",
        )
        .unwrap();
    let before = fixture.settled_snapshot();
    arm_next_write_commit_abort(&fixture);
    let outcome = fixture.engine().write(&[late_node("commit-row-trigger-1")]);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Storage);
    drop_temp_trigger(&fixture, "slice60_commit_row_trigger");
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(feature = "test-hooks")]
#[test]
fn armed_commit_abort_does_not_survive_validation_refusal() {
    let fixture = seeded("commit-abort-validation");
    arm_next_write_commit_abort(&fixture);
    let outcome = fixture.engine().write(&[plain_node(
        LATE_KIND,
        "commit-abort-validation-bad",
        "inverted window",
        Some(20),
        Some(10),
    )]);
    assert_eq!(outcome.unwrap_err(), EngineError::WriteValidation);
    fixture.assert_next_cursor_unconsumed();
}

#[cfg(feature = "test-hooks")]
#[test]
fn commit_error_before_abort_hook_does_not_poison_next_write() {
    let fixture = seeded("commit-abort-error");
    fixture
        .engine()
        .execute_for_test(
            "CREATE TABLE slice60_commit_parent(id INTEGER PRIMARY KEY); \
             CREATE TABLE slice60_commit_child( \
                 parent_id INTEGER REFERENCES slice60_commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED \
             ); \
             CREATE TEMP TRIGGER slice60_commit_deferred_fk \
             AFTER INSERT ON main.canonical_nodes \
             WHEN NEW.logical_id = 'commit-abort-error-bad' \
             BEGIN INSERT INTO main.slice60_commit_child(parent_id) VALUES(1); END",
        )
        .unwrap();
    arm_next_write_commit_abort(&fixture);
    let outcome = fixture.engine().write(&[late_node("commit-abort-error-bad")]);
    assert_eq!(outcome.unwrap_err(), EngineError::Storage);
    drop_temp_trigger(&fixture, "slice60_commit_deferred_fk");
    fixture.assert_next_cursor_unconsumed();
}

fn python_harness(script: &str) -> Vec<String> {
    vec!["python3".to_string(), "-c".to_string(), script.to_string()]
}

fn extract_documents() -> Vec<ExtractDocument> {
    vec![
        ExtractDocument { source_doc_id: "doc-1".into(), body: "Carol owns Delta".into() },
        ExtractDocument { source_doc_id: "doc-2".into(), body: "Erin owns Foxtrot".into() },
    ]
}

#[test]
fn provider_handshake_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("provider-handshake");
    let harness = python_harness(
        r#"
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("type") == "hello":
        print(json.dumps({"protocol": "fathomdb.wrong.v1", "type": "ready",
                          "schema_version": 1, "model": "slice60",
                          "max_docs_per_request": 10}), flush=True)
"#,
    );
    let cmd: Vec<&str> = harness.iter().map(String::as_str).collect();
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().ingest_with_extractor(&cmd, &extract_documents());
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Extractor);
    fixture.assert_next_cursor_unconsumed();
}

#[test]
fn provider_request_id_refusal_leaves_full_state_unchanged() {
    let fixture = seeded("provider-request-id");
    let harness = python_harness(
        r#"
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("type") == "hello":
        print(json.dumps({"protocol": "fathomdb.extract.v1", "type": "ready",
                          "schema_version": 1, "model": "slice60",
                          "max_docs_per_request": 10}), flush=True)
    elif msg.get("type") == "extract":
        print(json.dumps({"protocol": "fathomdb.extract.v1", "type": "result",
                          "request_id": "not-" + str(msg.get("request_id")),
                          "entities": [{"name": "Carol", "type": "person", "aliases": []}],
                          "edges": []}), flush=True)
"#,
    );
    let cmd: Vec<&str> = harness.iter().map(String::as_str).collect();
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().ingest_with_extractor(&cmd, &extract_documents());
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Extractor);
    fixture.assert_next_cursor_unconsumed();
}

/// The cursor probe does not apply: an applied `supersede`/`merge` verdict
/// consumes the in-memory cursor inside its transaction, so the preceding valid
/// verdict is pinned to `invalidate`.
#[test]
fn consolidate_out_of_cluster_verdict_rolls_back_applied_invalidate() {
    let mut fixture = seeded("consolidate-verdict");
    fixture.write(&[
        plain_edge(
            "works_for",
            "carol",
            "acme",
            "edge-acme",
            Some("Carol works for Acme"),
            Some(1_546_300_800),
        ),
        plain_edge(
            "works_for",
            "carol",
            "globex",
            "edge-globex",
            Some("Carol works for Globex"),
            Some(1_640_995_200),
        ),
    ]);
    let harness = python_harness(
        r#"
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("type") == "hello":
        print(json.dumps({"protocol": "fathomdb.consolidate.v1", "type": "ready",
                          "schema_version": 1, "model": "slice60",
                          "max_docs_per_request": 10}), flush=True)
    elif msg.get("type") == "consolidate":
        edges = msg["cluster"]["edges"]
        verdicts = [{"edge_ref": edges[0]["edge_ref"], "verdict": "invalidate",
                     "t_invalid": "2020-06-01T00:00:00Z"},
                    {"edge_ref": "not-in-cluster", "verdict": "keep"}]
        print(json.dumps({"protocol": "fathomdb.consolidate.v1", "type": "result",
                          "request_id": msg.get("request_id"),
                          "verdicts": verdicts}), flush=True)
"#,
    );
    let cmd: Vec<&str> = harness.iter().map(String::as_str).collect();
    let axes =
        [ConsolidateAxis { subject_logical_id: "carol".into(), relation: "works_for".into() }];
    let before = fixture.settled_snapshot();
    let outcome = fixture.engine().consolidate_with_provider(&cmd, &axes);
    fixture.assert_unchanged(&before);
    let error = outcome.unwrap_err();
    assert_eq!(error, EngineError::Consolidator);
}

#[derive(Clone, Debug)]
enum ValidItem {
    Node,
    Edge,
}

#[derive(Clone, Copy, Debug)]
enum StructuralFault {
    EmptyNodeBody,
    InvertedNodeWindow,
    EmptyNodeLogicalId,
    EmptyEdgeKind,
    EdgeEndpointRecordSeparator,
}

fn invalid_item(fault: StructuralFault) -> PreparedWrite {
    match fault {
        StructuralFault::EmptyNodeBody => plain_node(LATE_KIND, "prop-invalid", " ", None, None),
        StructuralFault::InvertedNodeWindow => {
            plain_node(LATE_KIND, "prop-invalid", "window", Some(5), Some(5))
        }
        StructuralFault::EmptyNodeLogicalId => plain_node(LATE_KIND, "", "body", None, None),
        StructuralFault::EmptyEdgeKind => plain_edge(" ", "a", "b", "prop-invalid", None, None),
        StructuralFault::EdgeEndpointRecordSeparator => {
            plain_edge("links", "a\u{1e}", "b", "prop-invalid", None, None)
        }
    }
}

fn structural_fault() -> impl Strategy<Value = StructuralFault> {
    prop_oneof![
        Just(StructuralFault::EmptyNodeBody),
        Just(StructuralFault::InvertedNodeWindow),
        Just(StructuralFault::EmptyNodeLogicalId),
        Just(StructuralFault::EmptyEdgeKind),
        Just(StructuralFault::EdgeEndpointRecordSeparator),
    ]
}

/// Validation precedes every mutation, for any batch composition: one invalid
/// item at a generated position among generated valid nodes and edges is
/// refused with `WriteValidation` and leaves the full snapshot unchanged.
#[test]
fn validation_precedes_mutation_for_generated_batches() {
    let fixture = seeded("property");
    let strategy = (
        proptest::collection::vec(prop_oneof![Just(ValidItem::Node), Just(ValidItem::Edge)], 1..=6),
        structural_fault(),
        any::<prop::sample::Index>(),
    );
    let mut runner = TestRunner::new(ProptestConfig {
        cases: 32,
        failure_persistence: None,
        ..ProptestConfig::default()
    });
    runner
        .run(&strategy, |(items, fault, position)| {
            let mut batch: Vec<PreparedWrite> = items
                .iter()
                .enumerate()
                .map(|(index, item)| match item {
                    ValidItem::Node => late_node(&format!("prop-node-{index}")),
                    ValidItem::Edge => plain_edge(
                        "links",
                        &format!("prop-from-{index}"),
                        &format!("prop-to-{index}"),
                        &format!("prop-edge-{index}"),
                        Some(&format!("prop edge body {index}")),
                        None,
                    ),
                })
                .collect();
            batch.insert(position.index(batch.len() + 1), invalid_item(fault));
            let before = fixture.settled_snapshot();
            let outcome = fixture.engine().write(&batch);
            let after = fixture.settled_snapshot();
            prop_assert_eq!(after, before);
            prop_assert_eq!(outcome.unwrap_err(), EngineError::WriteValidation);
            Ok(())
        })
        .unwrap();
}
