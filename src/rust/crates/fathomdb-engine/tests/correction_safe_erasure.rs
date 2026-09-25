//! Correction/supersession followed by source erasure.

use std::path::{Path, PathBuf};

use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash, ClosureLookupV1,
    ClosurePhaseV1, Engine, EngineError, InitialState, LifecycleActuationV1, LifecycleState,
    PreparedWrite, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::{types::ValueRef, Connection};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const ORIGINAL_BODY: &str = "slice20-original-sentinel";
const DEPENDENT_BODY: &str = "slice20-dependent-sentinel";
const REPLACEMENT_BODY: &str = "slice20-replacement-sentinel";
const SURVIVOR_BODY: &str = "slice20-survivor-sentinel";
const PROJECTION_TABLES: &[(&str, &str)] = &[
    ("search_index", "write_cursor"),
    ("search_index_v2", "write_cursor"),
    ("search_index_edges", "write_cursor"),
    ("vector_default", "rowid"),
    ("_fathomdb_vector_rows", "write_cursor"),
    ("_fathomdb_projection_terminal", "write_cursor"),
    ("canonical_attributes", "write_cursor"),
    ("property_search_index", "write_cursor"),
];

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn wal_path(path: &Path) -> PathBuf {
    let mut raw = path.as_os_str().to_os_string();
    raw.push("-wal");
    PathBuf::from(raw)
}

fn file_contains_bytes(path: &Path, needle: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let needle = needle.as_bytes();
    !needle.is_empty()
        && bytes.len() >= needle.len()
        && bytes.windows(needle.len()).any(|window| window == needle)
}

fn byte_hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(
    revision: &str,
    version: &str,
    logical: &str,
    source_id: &str,
    body: &str,
) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
    }
}

fn derived(source_id: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: DEPENDENT_BODY.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some("slice20-dependent".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("slice20-dependent-r1").unwrap(),
            SourceVersionId::new("slice20-original-v1").unwrap(),
            SourceRevisionId::new("slice20-original-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(ORIGINAL_BODY)).unwrap(),
        ),
    }
}

struct Fixture {
    dir: TempDir,
    db: std::path::PathBuf,
    opened: fathomdb_engine::OpenedEngine,
    correction_closure_ids: Vec<String>,
}

fn corrected_fixture(name: &str, original_bucket: &str, replacement_bucket: &str) -> Fixture {
    corrected_fixture_with(name, original_bucket, replacement_bucket, false)
}

/// `bare_supersede` corrects with a lone `PutCanonicalNode` on the same
/// logical id instead of a revision-pinned lifecycle transition plus put.
fn corrected_fixture_with(
    name: &str,
    original_bucket: &str,
    replacement_bucket: &str,
    bare_supersede: bool,
) -> Fixture {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, name);
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(canonical(
                "slice20-original-r1",
                "slice20-original-v1",
                "slice20-source",
                original_bucket,
                ORIGINAL_BODY,
            )),
            PreparedWrite::ProvenancedNode(derived(original_bucket)),
            PreparedWrite::ProvenancedNode(canonical(
                "slice20-survivor-r1",
                "slice20-survivor-v1",
                "slice20-survivor",
                "slice20-survivor-bucket",
                SURVIVOR_BODY,
            )),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "slice20-dependency",
                "slice20-original-r1",
                "slice20-dependent-r1",
            )
            .unwrap(),
        )
        .unwrap();
    let replacement = ActuationOperationV1::PutCanonicalNode(canonical(
        "slice20-replacement-r2",
        "slice20-replacement-v2",
        "slice20-source",
        replacement_bucket,
        REPLACEMENT_BODY,
    ));
    let operations = if bare_supersede {
        vec![replacement]
    } else {
        vec![
            ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "slice20-source",
                    ArtifactRevisionId::new("slice20-original-r1").unwrap(),
                    LifecycleState::Deleted,
                    Some("corrected".into()),
                )
                .unwrap(),
            ),
            replacement,
        ]
    };
    let receipt = opened
        .engine
        .actuate(ActuationBatchV1::new("slice20-correction", operations).unwrap())
        .unwrap();
    if bare_supersede {
        assert!(!receipt.closure_operation_ids.is_empty(), "bare supersession must close");
    } else {
        assert_eq!(receipt.closure_operation_ids.len(), 2);
    }
    let correction_closure_ids =
        receipt.closure_operation_ids.iter().map(|id| id.as_str().to_string()).collect::<Vec<_>>();
    for closure_id in &correction_closure_ids {
        let closure = opened
            .engine
            .read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(closure.phase, ClosurePhaseV1::Complete);
    }
    Fixture { dir, db, opened, correction_closure_ids }
}

fn requested_inventory(connection: &Connection, source_id: &str) -> (u64, u64, u64) {
    let cursors = |table: &str| {
        let mut statement = connection
            .prepare(&format!("SELECT write_cursor FROM {table} WHERE source_id=?1"))
            .unwrap();
        statement
            .query_map([source_id], |row| row.get::<_, i64>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    let nodes = cursors("canonical_nodes");
    let edges = cursors("canonical_edges");
    let mut projections = 0_u64;
    for cursor in nodes.iter().chain(edges.iter()) {
        for (table, column) in PROJECTION_TABLES {
            let count: i64 = connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE {column}=?1"),
                    [cursor],
                    |row| row.get(0),
                )
                .unwrap();
            projections += u64::try_from(count).unwrap();
        }
    }
    (nodes.len() as u64, edges.len() as u64, projections)
}

fn requested_cursors(connection: &Connection, source_id: &str) -> Vec<i64> {
    let mut cursors = Vec::new();
    for table in ["canonical_nodes", "canonical_edges"] {
        let mut statement = connection
            .prepare(&format!("SELECT write_cursor FROM {table} WHERE source_id=?1"))
            .unwrap();
        cursors.extend(
            statement
                .query_map([source_id], |row| row.get::<_, i64>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap(),
        );
    }
    cursors
}

fn assert_cursors_physically_absent(connection: &Connection, cursors: &[i64]) {
    for cursor in cursors {
        for (table, column) in PROJECTION_TABLES {
            let count: i64 = connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE {column}=?1"),
                    [cursor],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "cursor {cursor} survived in {table}");
        }
    }
}

const ROLLBACK_TABLES: &[&str] = &[
    "canonical_nodes",
    "canonical_edges",
    "search_index",
    "search_index_v2",
    "search_index_edges",
    "vector_default",
    "_fathomdb_vector_rows",
    "_fathomdb_projection_terminal",
    "canonical_attributes",
    "property_search_index",
    "_fathomdb_artifact_revisions",
    "_fathomdb_source_dependencies",
    "_fathomdb_source_links",
    "_fathomdb_source_versions",
    "_fathomdb_dependency_closures",
    "_fathomdb_open_state",
    "_fathomdb_actuation_receipts",
    "_fathomdb_actuation_receipt_source_refs",
    "operational_mutations",
];

fn database_plane_snapshot(connection: &Connection) -> Vec<(String, Vec<Vec<String>>)> {
    ROLLBACK_TABLES
        .iter()
        .map(|table| {
            let mut statement = connection.prepare(&format!("SELECT * FROM {table}")).unwrap();
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
            ((*table).to_string(), rows)
        })
        .collect()
}

fn assert_audit(connection: &Connection, source_id: &str, expected: (u64, u64, u64)) {
    let rows = connection
        .prepare(
            "SELECT record_key,payload_json FROM operational_mutations \
             WHERE collection_name='excise_source_audit' AND record_key=?1",
        )
        .unwrap()
        .query_map([source_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(rows.len(), 1, "one audit row per committed primary erase");
    assert_eq!(rows[0].0, source_id);
    let payload: serde_json::Value = serde_json::from_str(&rows[0].1).unwrap();
    assert_eq!(payload["source_id"], source_id);
    assert_eq!(payload["nodes_excised"], expected.0);
    assert_eq!(payload["edges_excised"], expected.1);
    assert_eq!(payload["projections_invalidated"], expected.2);
}

fn assert_revisions(connection: &Connection, expected: &[&str]) {
    let mut actual = connection
        .prepare("SELECT revision_id FROM _fathomdb_artifact_revisions ORDER BY revision_id")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let mut expected = expected.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn correction_safe_erasure_matrix_preserves_requested_counts_and_exact_survivors() {
    struct Case {
        name: &'static str,
        original_bucket: &'static str,
        replacement_bucket: &'static str,
        erase_order: &'static [&'static str],
        bare_supersede: bool,
    }
    let cases = [
        Case {
            name: "same-bucket",
            original_bucket: "slice20-target",
            replacement_bucket: "slice20-target",
            erase_order: &["slice20-target"],
            bare_supersede: false,
        },
        Case {
            name: "bare-supersede-same-bucket",
            original_bucket: "slice20-target",
            replacement_bucket: "slice20-target",
            erase_order: &["slice20-target"],
            bare_supersede: true,
        },
        Case {
            name: "bare-supersede-cross-original-first",
            original_bucket: "slice20-original-bucket",
            replacement_bucket: "slice20-replacement-bucket",
            erase_order: &["slice20-original-bucket", "slice20-replacement-bucket"],
            bare_supersede: true,
        },
        Case {
            name: "cross-original-first",
            original_bucket: "slice20-original-bucket",
            replacement_bucket: "slice20-replacement-bucket",
            erase_order: &["slice20-original-bucket", "slice20-replacement-bucket"],
            bare_supersede: false,
        },
        Case {
            name: "cross-replacement-first",
            original_bucket: "slice20-original-bucket",
            replacement_bucket: "slice20-replacement-bucket",
            erase_order: &["slice20-replacement-bucket", "slice20-original-bucket"],
            bare_supersede: false,
        },
    ];

    for case in cases {
        let fixture = corrected_fixture_with(
            case.name,
            case.original_bucket,
            case.replacement_bucket,
            case.bare_supersede,
        );
        let mut expected_revisions = vec![
            "slice20-original-r1",
            "slice20-dependent-r1",
            "slice20-replacement-r2",
            "slice20-survivor-r1",
        ];
        for source_id in case.erase_order {
            let before = Connection::open(&fixture.db).unwrap();
            let expected = requested_inventory(&before, source_id);
            let erased_cursors = requested_cursors(&before, source_id);
            let erased_bodies = before
                .prepare("SELECT body FROM canonical_nodes WHERE source_id=?1 ORDER BY body")
                .unwrap()
                .query_map([source_id], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            assert!(!erased_bodies.is_empty(), "{} fixture must be non-vacuous", case.name);
            drop(before);
            let report = fixture.opened.engine.erase_source(source_id).unwrap();
            assert_eq!(report.source_ref, *source_id, "{}", case.name);
            assert_eq!(
                (report.nodes_excised, report.edges_excised, report.projections_invalidated),
                expected,
                "{} requested-bucket report contract",
                case.name
            );
            expected_revisions.retain(|revision| match *source_id {
                "slice20-target" => !matches!(
                    *revision,
                    "slice20-original-r1" | "slice20-dependent-r1" | "slice20-replacement-r2"
                ),
                "slice20-original-bucket" => {
                    !matches!(*revision, "slice20-original-r1" | "slice20-dependent-r1")
                }
                "slice20-replacement-bucket" => *revision != "slice20-replacement-r2",
                _ => true,
            });
            let connection = Connection::open(&fixture.db).unwrap();
            assert_revisions(&connection, &expected_revisions);
            assert_audit(&connection, source_id, expected);
            assert_cursors_physically_absent(&connection, &erased_cursors);
            for body in &erased_bodies {
                assert!(
                    !file_contains_bytes(&fixture.db, body)
                        && !file_contains_bytes(&wal_path(&fixture.db), body),
                    "{} left erased body bytes at rest: {body}",
                    case.name
                );
            }
            assert!(
                file_contains_bytes(&fixture.db, SURVIVOR_BODY)
                    || file_contains_bytes(&wal_path(&fixture.db), SURVIVOR_BODY),
                "{} must preserve the unrelated survivor bytes",
                case.name
            );
        }

        fixture.opened.engine.close().unwrap();
        let reopened = Engine::open(&fixture.db).unwrap();
        let connection = Connection::open(&fixture.db).unwrap();
        assert_revisions(&connection, &["slice20-survivor-r1"]);
        let survivor: String = connection
            .query_row(
                "SELECT body FROM canonical_nodes WHERE logical_id='slice20-survivor'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(survivor, SURVIVOR_BODY);
        let residues: i64 = connection
            .query_row(
                "SELECT \
                   (SELECT COUNT(*) FROM _fathomdb_source_dependencies) + \
                   (SELECT COUNT(*) FROM _fathomdb_source_links \
                    WHERE artifact_revision_id IN ('slice20-original-r1','slice20-dependent-r1','slice20-replacement-r2')) + \
                   (SELECT COUNT(*) FROM _fathomdb_source_versions \
                    WHERE source_revision_id IN ('slice20-original-r1','slice20-replacement-r2'))",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(residues, 0, "{} leaves no live authority", case.name);
        for closure_id in &fixture.correction_closure_ids {
            let soft_closure: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM _fathomdb_dependency_closures \
                     WHERE closure_operation_id=?1",
                    [closure_id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(soft_closure, 0, "obsolete correction closure must be removed");
        }
        let proof_rows = connection
            .prepare(
                "SELECT root_kind,root_value,cause,phase FROM _fathomdb_dependency_closures \
                 WHERE cause='source_erased' ORDER BY root_value",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(proof_rows.len(), 1, "one dependent-bearing bucket proof survives");
        assert_eq!(proof_rows[0].0, "source_bucket");
        assert_eq!(proof_rows[0].2, "source_erased");
        assert_eq!(proof_rows[0].3, "complete");
        assert_eq!(
            proof_rows[0].1, case.original_bucket,
            "only the original dependent-bearing bucket retains proof identity"
        );
        let redacted_receipt_refs: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs \
                 WHERE operation_id='slice20-correction'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(redacted_receipt_refs, 0);
        let redacted_receipt: (String, Option<String>, String) = connection
            .query_row(
                "SELECT outcome,request_sha256,closure_operation_ids_json \
                 FROM _fathomdb_actuation_receipts WHERE operation_id='slice20-correction'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(redacted_receipt, ("erased".into(), None, "[]".into()));
        drop(connection);
        reopened.engine.close().unwrap();
        drop(fixture.dir);
    }
}

#[test]
fn correction_erasure_precommit_proof_failure_rolls_back_every_primary_plane() {
    let fixture =
        corrected_fixture("rollback", "slice20-original-bucket", "slice20-replacement-bucket");
    let connection = Connection::open(&fixture.db).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER preserve_slice20_dependent \
             BEFORE DELETE ON canonical_nodes \
             WHEN OLD.logical_id='slice20-dependent' \
             BEGIN SELECT RAISE(IGNORE); END;",
        )
        .unwrap();
    drop(connection);

    let sink = fixture.dir.path().join("rollback-telemetry.jsonl");
    fixture.opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    fixture.opened.engine.search("original sentinel").unwrap();
    fixture.opened.engine.search("survivor sentinel").unwrap();
    let telemetry_before = std::fs::read(&sink).unwrap();
    let database_before = database_plane_snapshot(&Connection::open(&fixture.db).unwrap());

    assert!(matches!(
        fixture.opened.engine.erase_source("slice20-original-bucket"),
        Err(EngineError::Storage)
    ));
    let connection = Connection::open(&fixture.db).unwrap();
    assert_eq!(
        database_plane_snapshot(&connection),
        database_before,
        "pre-commit refusal must restore every protected database plane exactly"
    );
    assert_eq!(
        std::fs::read(&sink).unwrap(),
        telemetry_before,
        "pre-commit refusal must not redact or rewrite telemetry"
    );
    assert_revisions(
        &connection,
        &[
            "slice20-original-r1",
            "slice20-dependent-r1",
            "slice20-replacement-r2",
            "slice20-survivor-r1",
        ],
    );
    let audit_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM operational_mutations \
             WHERE collection_name='excise_source_audit'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(audit_rows, 0);
    let physical_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_dependency_closures WHERE cause='source_erased'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(physical_rows, 0);
}

#[test]
fn correction_erasure_postcommit_incomplete_retry_is_truthful_and_audit_idempotent() {
    let fixture = corrected_fixture("retry", "slice20-target", "slice20-target");
    let sink = fixture.dir.path().join("telemetry.jsonl");
    let rotated = fixture.dir.path().join("telemetry.jsonl.1");
    fixture.opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    fixture.opened.engine.search("replacement sentinel").unwrap();
    assert!(std::fs::read_to_string(&sink).unwrap().contains("l:slice20-source"));
    std::fs::rename(&sink, &rotated).unwrap();

    let expected = requested_inventory(&Connection::open(&fixture.db).unwrap(), "slice20-target");
    let error = fixture.opened.engine.erase_source("slice20-target").unwrap_err();
    assert!(matches!(
        error,
        EngineError::ErasureIncomplete { ref stage, .. } if stage == "telemetry_redaction"
    ));
    let connection = Connection::open(&fixture.db).unwrap();
    let primary_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice20-target'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(primary_rows, 0, "primary deletion committed before at-rest failure");
    assert_audit(&connection, "slice20-target", expected);
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
             WHERE root_kind='source_bucket' AND root_value='slice20-target'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    std::fs::rename(&rotated, &sink).unwrap();
    let retry = fixture.opened.engine.erase_source("slice20-target").unwrap();
    assert_eq!(
        (retry.nodes_excised, retry.edges_excised, retry.projections_invalidated),
        (0, 0, 0)
    );
    let status = fixture
        .opened
        .engine
        .read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(status.phase, ClosurePhaseV1::Complete);
    let connection = Connection::open(&fixture.db).unwrap();
    assert_audit(&connection, "slice20-target", expected);
    assert!(!std::fs::read_to_string(&sink).unwrap().contains("l:slice20-source"));
}

#[test]
fn correction_then_purge_of_corrected_logical_id_succeeds_and_erases_exact_rows() {
    let fixture = corrected_fixture("purge-after-correction", "slice20-target", "slice20-target");
    let engine = &fixture.opened.engine;
    engine
        .actuate(
            ActuationBatchV1::new(
                "slice20-retire-replacement",
                vec![ActuationOperationV1::TransitionLifecycle(
                    LifecycleActuationV1::new(
                        "slice20-source",
                        ArtifactRevisionId::new("slice20-replacement-r2").unwrap(),
                        LifecycleState::Deleted,
                        Some("retired".into()),
                    )
                    .unwrap(),
                )],
            )
            .unwrap(),
        )
        .unwrap();
    let before = Connection::open(&fixture.db).unwrap();
    let purged_cursors = requested_cursors(&before, "slice20-target");
    assert!(!purged_cursors.is_empty(), "purge fixture must be non-vacuous");
    drop(before);

    engine.purge("slice20-source").expect("purge after a dependent-bearing correction");

    let connection = Connection::open(&fixture.db).unwrap();
    assert_revisions(&connection, &["slice20-survivor-r1"]);
    assert_cursors_physically_absent(&connection, &purged_cursors);
    for closure_id in &fixture.correction_closure_ids {
        let remaining: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1",
                [closure_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0, "completed correction closure {closure_id} survived purge");
    }
    for body in [ORIGINAL_BODY, DEPENDENT_BODY, REPLACEMENT_BODY] {
        assert!(
            !file_contains_bytes(&fixture.db, body)
                && !file_contains_bytes(&wal_path(&fixture.db), body),
            "purge left body bytes at rest: {body}"
        );
    }
    assert!(
        file_contains_bytes(&fixture.db, SURVIVOR_BODY)
            || file_contains_bytes(&wal_path(&fixture.db), SURVIVOR_BODY),
        "purge must preserve the unrelated survivor"
    );
    let redacted_receipt: (String, Option<String>, String) = connection
        .query_row(
            "SELECT outcome,request_sha256,closure_operation_ids_json \
             FROM _fathomdb_actuation_receipts WHERE operation_id='slice20-correction'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(redacted_receipt, ("erased".into(), None, "[]".into()));
    let proofs = connection
        .prepare(
            "SELECT closure_operation_id,root_kind,root_value,phase \
             FROM _fathomdb_dependency_closures WHERE cause='purged'",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(proofs.len(), 1, "purge must keep exactly its own proof row: {proofs:?}");
    assert_eq!(
        (proofs[0].1.as_str(), proofs[0].2.as_str(), proofs[0].3.as_str()),
        ("source_revision", "slice20-original-r1", "complete")
    );
    drop(connection);
    fixture.opened.engine.close().unwrap();
    let reopened = Engine::open(&fixture.db).unwrap();
    let proof = reopened
        .engine
        .read_dependency_closure(ClosureLookupV1::new(&proofs[0].0).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(proof.phase, ClosurePhaseV1::Complete);
    let connection = Connection::open(&fixture.db).unwrap();
    assert_revisions(&connection, &["slice20-survivor-r1"]);
    assert_cursors_physically_absent(&connection, &purged_cursors);
    drop(connection);
    for body in [ORIGINAL_BODY, DEPENDENT_BODY, REPLACEMENT_BODY] {
        assert!(
            !file_contains_bytes(&fixture.db, body)
                && !file_contains_bytes(&wal_path(&fixture.db), body),
            "reopen resurfaced purged body bytes: {body}"
        );
    }
    reopened.engine.close().unwrap();
    drop(fixture.dir);
}

fn retire(engine: &Engine, operation_id: &str, logical_id: &str, revision: &str) {
    engine
        .actuate(
            ActuationBatchV1::new(
                operation_id,
                vec![ActuationOperationV1::TransitionLifecycle(
                    LifecycleActuationV1::new(
                        logical_id,
                        ArtifactRevisionId::new(revision).unwrap(),
                        LifecycleState::Deleted,
                        Some("retired".into()),
                    )
                    .unwrap(),
                )],
            )
            .unwrap(),
        )
        .unwrap();
}

#[test]
fn purge_after_dependents_are_gone_leaves_no_closure_for_an_erased_revision() {
    let fixture = corrected_fixture("purge-dependent-first", "slice20-target", "slice20-target");
    let engine = &fixture.opened.engine;
    retire(engine, "slice20-retire-dependent", "slice20-dependent", "slice20-dependent-r1");
    engine.purge("slice20-dependent").unwrap();
    retire(engine, "slice20-retire-replacement", "slice20-source", "slice20-replacement-r2");
    engine.purge("slice20-source").unwrap();

    let connection = Connection::open(&fixture.db).unwrap();
    assert_revisions(&connection, &["slice20-survivor-r1"]);
    let orphaned = connection
        .prepare(
            "SELECT root_value,cause,phase FROM _fathomdb_dependency_closures \
             WHERE root_kind='source_revision' AND root_value NOT IN \
               (SELECT revision_id FROM _fathomdb_artifact_revisions)",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let non_proof = orphaned.iter().filter(|(_, cause, _)| cause != "purged").collect::<Vec<_>>();
    assert!(non_proof.is_empty(), "erased revision identifiers survive in closures: {non_proof:?}");
    drop(connection);
    fixture.opened.engine.close().unwrap();
    drop(fixture.dir);
}

#[derive(Clone, Copy, Debug)]
enum HardErasure {
    EraseSource,
    Purge,
}

impl HardErasure {
    fn all() -> [Self; 2] {
        [Self::EraseSource, Self::Purge]
    }

    fn prepare(self, engine: &Engine) {
        if matches!(self, Self::Purge) {
            retire(
                engine,
                "slice50-retire-replacement",
                "slice20-source",
                "slice20-replacement-r2",
            );
        }
    }

    fn apply(self, engine: &Engine, source_bucket: &str) -> Result<(), EngineError> {
        match self {
            Self::EraseSource => engine.erase_source(source_bucket).map(|_| ()),
            Self::Purge => engine.purge("slice20-source"),
        }
    }

    fn expected_proof(self, source_bucket: &str) -> (&'static str, &str, &'static str) {
        match self {
            Self::EraseSource => ("source_bucket", source_bucket, "source_erased"),
            Self::Purge => ("source_revision", "slice20-original-r1", "purged"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DependencyShape {
    SameBucket,
    CrossBucket,
}

impl DependencyShape {
    fn all() -> [Self; 2] {
        [Self::SameBucket, Self::CrossBucket]
    }

    fn buckets(self) -> (&'static str, &'static str) {
        match self {
            Self::SameBucket => ("slice20-target", "slice20-target"),
            Self::CrossBucket => ("slice20-original-bucket", "slice20-replacement-bucket"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SoftClosurePhase {
    Proving,
    Incomplete,
}

impl SoftClosurePhase {
    fn all() -> [Self; 2] {
        [Self::Proving, Self::Incomplete]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Proving => "proving",
            Self::Incomplete => "incomplete",
        }
    }

    fn blocker(self) -> Option<&'static str> {
        match self {
            Self::Proving => None,
            Self::Incomplete => Some("proof_unavailable"),
        }
    }
}

fn make_soft_closure_nonterminal(db: &Path, cause: &str, phase: SoftClosurePhase) -> String {
    let connection = Connection::open(db).unwrap();
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
             WHERE root_kind='source_revision' AND root_value='slice20-original-r1' \
               AND cause=?1",
            [cause],
            |row| row.get(0),
        )
        .unwrap();
    let changed = connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
             SET phase=?2,affected_count=affected_count+1000,blocker_code=?3,\
                 structural_proof_write_boundary=NULL,proof_json=NULL \
             WHERE closure_operation_id=?1",
            rusqlite::params![closure_id, phase.as_str(), phase.blocker()],
        )
        .unwrap();
    assert_eq!(changed, 1, "fixture must mutate exactly one soft closure");
    if matches!(phase, SoftClosurePhase::Proving) {
        keep_proving_through_bounded_maintenance(&connection, &closure_id);
    }
    closure_id
}

fn keep_proving_through_bounded_maintenance(connection: &Connection, closure_id: &str) {
    const MAINTENANCE_WINDOW: i64 = 32;

    let maximum: i64 = connection
        .query_row("SELECT MAX(closure_sequence) FROM _fathomdb_dependency_closures", [], |row| {
            row.get(0)
        })
        .unwrap();
    let target_sequence = maximum + MAINTENANCE_WINDOW + 1;
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures SET closure_sequence=?2 \
             WHERE closure_operation_id=?1",
            rusqlite::params![closure_id, target_sequence],
        )
        .unwrap();
    for offset in 1..=MAINTENANCE_WINDOW {
        let decoy_id = format!("_fdb:c:{}", digest(&format!("{closure_id}:decoy:{offset}")));
        let retry = digest(&format!("{closure_id}:retry:{offset}"));
        connection
            .execute(
                "INSERT INTO _fathomdb_dependency_closures(\
                   schema_version,closure_operation_id,root_kind,root_value,cause,\
                   effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
                   closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
                   structural_proof_write_boundary,proof_json\
                 ) SELECT schema_version,?2,root_kind,root_value,cause,effective_at_epoch_s,\
                          admitted_write_boundary,admitted_dependency_generation,?3,?4,\
                          'incomplete',affected_count,'proof_unavailable',NULL,NULL \
                   FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1",
                rusqlite::params![closure_id, decoy_id, maximum + offset, retry],
            )
            .unwrap();
    }
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 \
             WHERE key='_fathomdb_closure_sequence'",
            [target_sequence.to_string()],
        )
        .unwrap();
    connection
        .execute_batch(&format!(
            "CREATE TABLE slice50_deletion_witness(\
                 closure_operation_id TEXT NOT NULL, phase TEXT NOT NULL\
             ); \
             CREATE TRIGGER witness_slice50_proving_deletion \
             BEFORE DELETE ON _fathomdb_dependency_closures \
             WHEN OLD.closure_operation_id='{closure_id}' \
             BEGIN \
                 SELECT CASE WHEN OLD.phase!='proving' \
                     THEN RAISE(ABORT, 'target closure left proving before deletion') END; \
                 INSERT INTO slice50_deletion_witness VALUES(\
                     OLD.closure_operation_id,OLD.phase\
                 ); \
             END;"
        ))
        .unwrap();
}

fn assert_soft_closure_absent(db: &Path, closure_id: &str, phase: SoftClosurePhase) {
    let connection = Connection::open(db).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_dependency_closures \
             WHERE closure_operation_id=?1",
            [closure_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "nonphysical closure retained erased revision identity");
    if matches!(phase, SoftClosurePhase::Proving) {
        let witnessed = connection
            .prepare(
                "SELECT phase FROM slice50_deletion_witness \
                 WHERE closure_operation_id=?1",
            )
            .unwrap()
            .query_map([closure_id], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            witnessed,
            ["proving"],
            "the target must still be proving immediately before hard-erasure deletion"
        );
    }
}

fn assert_exact_physical_proof(db: &Path, erasure: HardErasure, source_bucket: &str) {
    let expected = erasure.expected_proof(source_bucket);
    let connection = Connection::open(db).unwrap();
    let proofs = connection
        .prepare(
            "SELECT root_kind,root_value,cause,phase \
             FROM _fathomdb_dependency_closures \
             WHERE cause IN ('purged','source_erased') ORDER BY closure_sequence",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(proofs.len(), 1, "hard erasure must retain exactly one physical proof");
    assert_eq!(
        proofs[0],
        (expected.0.into(), expected.1.into(), expected.2.into(), "complete".into())
    );
}

#[test]
fn nonterminal_soft_closure_state_machine_erases_identity_and_preserves_physical_proof() {
    for erasure in HardErasure::all() {
        for shape in DependencyShape::all() {
            for phase in SoftClosurePhase::all() {
                for cause in ["superseded", "soft_deleted"] {
                    let (original_bucket, replacement_bucket) = shape.buckets();
                    let name = format!("slice50-{erasure:?}-{shape:?}-{}-{cause}", phase.as_str());
                    let fixture = corrected_fixture(&name, original_bucket, replacement_bucket);
                    erasure.prepare(&fixture.opened.engine);
                    let closure_id = make_soft_closure_nonterminal(&fixture.db, cause, phase);

                    let before = Connection::open(&fixture.db).unwrap();
                    let erased_cursors = match erasure {
                        HardErasure::EraseSource => requested_cursors(&before, original_bucket),
                        HardErasure::Purge => {
                            let mut cursors = requested_cursors(&before, original_bucket);
                            if replacement_bucket != original_bucket {
                                cursors.extend(requested_cursors(&before, replacement_bucket));
                            }
                            cursors
                        }
                    };
                    assert!(!erased_cursors.is_empty(), "state-machine fixture is non-vacuous");
                    drop(before);

                    erasure
                        .apply(&fixture.opened.engine, original_bucket)
                        .unwrap_or_else(|error| panic!("{name}: hard erasure failed: {error}"));
                    assert_soft_closure_absent(&fixture.db, &closure_id, phase);
                    assert_exact_physical_proof(&fixture.db, erasure, original_bucket);

                    let connection = Connection::open(&fixture.db).unwrap();
                    let survivor: String = connection
                        .query_row(
                            "SELECT body FROM canonical_nodes \
                             WHERE logical_id='slice20-survivor'",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap();
                    assert_eq!(survivor, SURVIVOR_BODY, "unrelated row must survive");
                    assert_cursors_physically_absent(&connection, &erased_cursors);
                }
            }
        }
    }
}

#[test]
fn nonterminal_soft_closure_refusal_rolls_back_every_primary_plane() {
    for erasure in HardErasure::all() {
        for shape in DependencyShape::all() {
            let (original_bucket, replacement_bucket) = shape.buckets();
            let name = format!("slice50-rollback-{erasure:?}-{shape:?}");
            let fixture = corrected_fixture(&name, original_bucket, replacement_bucket);
            erasure.prepare(&fixture.opened.engine);
            make_soft_closure_nonterminal(&fixture.db, "superseded", SoftClosurePhase::Incomplete);
            let connection = Connection::open(&fixture.db).unwrap();
            connection
                .execute_batch(
                    "CREATE TRIGGER preserve_slice50_dependent \
                     BEFORE DELETE ON canonical_nodes \
                     WHEN OLD.logical_id='slice20-dependent' \
                     BEGIN SELECT RAISE(IGNORE); END;",
                )
                .unwrap();
            let before = database_plane_snapshot(&connection);
            drop(connection);

            assert!(matches!(
                erasure.apply(&fixture.opened.engine, original_bucket),
                Err(EngineError::Storage)
            ));
            let connection = Connection::open(&fixture.db).unwrap();
            assert_eq!(
                database_plane_snapshot(&connection),
                before,
                "{name}: refusal must restore every protected database plane exactly"
            );
            let physical_rows: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM _fathomdb_dependency_closures \
                     WHERE cause IN ('purged','source_erased')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(physical_rows, 0, "refusal must not retain a physical proof");
        }
    }
}
