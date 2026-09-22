//! Correction/supersession followed by source erasure.

use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash, ClosureLookupV1,
    ClosurePhaseV1, Engine, EngineError, InitialState, LifecycleActuationV1, LifecycleState,
    PreparedWrite, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
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
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice20-correction",
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
                    ActuationOperationV1::PutCanonicalNode(canonical(
                        "slice20-replacement-r2",
                        "slice20-replacement-v2",
                        "slice20-source",
                        replacement_bucket,
                        REPLACEMENT_BODY,
                    )),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.closure_operation_ids.len(), 2);
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
    }
    let cases = [
        Case {
            name: "same-bucket",
            original_bucket: "slice20-target",
            replacement_bucket: "slice20-target",
            erase_order: &["slice20-target"],
        },
        Case {
            name: "cross-original-first",
            original_bucket: "slice20-original-bucket",
            replacement_bucket: "slice20-replacement-bucket",
            erase_order: &["slice20-original-bucket", "slice20-replacement-bucket"],
        },
        Case {
            name: "cross-replacement-first",
            original_bucket: "slice20-original-bucket",
            replacement_bucket: "slice20-replacement-bucket",
            erase_order: &["slice20-replacement-bucket", "slice20-original-bucket"],
        },
    ];

    for case in cases {
        let fixture = corrected_fixture(case.name, case.original_bucket, case.replacement_bucket);
        let mut expected_revisions = vec![
            "slice20-original-r1",
            "slice20-dependent-r1",
            "slice20-replacement-r2",
            "slice20-survivor-r1",
        ];
        for source_id in case.erase_order {
            let expected = requested_inventory(&Connection::open(&fixture.db).unwrap(), source_id);
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

    assert!(matches!(
        fixture.opened.engine.erase_source("slice20-original-bucket"),
        Err(EngineError::Storage)
    ));
    let connection = Connection::open(&fixture.db).unwrap();
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
