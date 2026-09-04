//! Slice 30 — dependency-aware lifecycle and erasure closure.

use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ArtifactRevisionId, Bm25fQueryPlan,
    CanonicalHash, ClosureCauseV1, ClosureLookupV1, ClosurePhaseV1, ClosureRootV1,
    DependencyErrorReason, Engine, EngineError, EngineOpenError, InitialState,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProvenanceErrorReason, ProvenancedEdgeV1, ProvenancedNodeV1, ReadView,
    SoftFallbackBranch, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, TraversalDirection, WriteProvenanceV1, TOP_K_BIT_CANDIDATES,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30", "fix2", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

fn seed_projection_race(engine: &Engine) {
    let mut derived = derived_node("race-derived-r1", "race-derived", "race-source-r1");
    let PreparedWrite::ProvenancedNode(node) = &mut derived else {
        unreachable!("derived_node returns a provenanced node");
    };
    node.kind = "doc".into();
    engine
        .write(&[canonical("race-source-r1", "v1", "race-source", "source body"), derived])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "race-dependency",
                "race-source-r1",
                "race-derived-r1",
            )
            .unwrap(),
        )
        .unwrap();
}

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(revision: &str, version: &str, logical: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
    })
}

fn canonical_with_validity(
    revision: &str,
    version: &str,
    logical: &str,
    body: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
    })
}

fn derived_node(revision: &str, logical: &str, source_revision: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived body {revision}"),
        source_id: SourceId::new("bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: derived_provenance(revision, source_revision),
    })
}

fn derived_node_with_body(revision: &str, logical: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("bucket").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: derived_provenance(revision, "source-r1"),
    })
}

fn plain_edge(from: &str, to: &str, logical_id: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("other-bucket").unwrap(),
        logical_id: Some(logical_id.into()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn install_soft_barrier(db: &std::path::Path, source_revision: &str, boundary: i64) {
    let connection = Connection::open(db).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='1' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,?1,'source_revision',?2,'soft_deleted',0,?3,1,1,?4,\
                      'proving',1,NULL,NULL,NULL)",
            params![
                format!("_fdb:c:{}", "e".repeat(64)),
                source_revision,
                boundary,
                "f".repeat(64)
            ],
        )
        .unwrap();
}

fn derived_edge(revision: &str, source_revision: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: "source".into(),
        to: "derived".into(),
        source_id: SourceId::new("bucket").unwrap(),
        logical_id: Some("derived-edge".into()),
        body: Some("derived edge body".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: derived_provenance(revision, source_revision),
    })
}

fn derived_provenance(revision: &str, source_revision: &str) -> WriteProvenanceV1 {
    WriteProvenanceV1::derived(
        ArtifactRevisionId::new(revision).unwrap(),
        SourceVersionId::new("v1").unwrap(),
        SourceRevisionId::new(source_revision).unwrap(),
        SourceLocator::whole_body(),
        CanonicalHash::sha256(digest("source body")).unwrap(),
    )
}

fn seed_registered(engine: &Engine) {
    engine
        .write(&[
            canonical("source-r1", "v1", "source", "source body"),
            derived_node("derived-r1", "derived", "source-r1"),
            derived_edge("edge-r1", "source-r1"),
        ])
        .unwrap();
    for (id, revision) in [("dep-node", "derived-r1"), ("dep-edge", "edge-r1")] {
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new(id, "source-r1", revision).unwrap(),
            )
            .unwrap();
    }
}

#[test]
fn source_supersession_atomically_soft_closes_node_and_edge() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "soft");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);

    opened.engine.write(&[canonical("source-r2", "v2", "source", "replacement body")]).unwrap();

    let connection = Connection::open(&db).unwrap();
    let node_state: String = connection
        .query_row(
            "SELECT n.state FROM canonical_nodes n JOIN _fathomdb_artifact_revisions r \
             ON r.write_cursor=n.write_cursor WHERE r.revision_id='derived-r1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let edge_retired: bool = connection
        .query_row(
            "SELECT e.superseded_at IS NOT NULL FROM canonical_edges e \
             JOIN _fathomdb_artifact_revisions r ON r.write_cursor=e.write_cursor \
             WHERE r.revision_id='edge-r1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let closure: (String, String, i64) = connection
        .query_row(
            "SELECT cause,phase,affected_count FROM _fathomdb_dependency_closures \
             WHERE root_value='source-r1' ORDER BY closure_sequence DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(node_state, "deleted");
    assert!(edge_retired);
    assert_eq!(closure, ("superseded".into(), "complete".into(), 2));
}

#[test]
fn actuation_returns_closure_id_and_keyed_complete_proof() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "actuation")).unwrap();
    seed_registered(&opened.engine);
    let request = ActuationBatchV1::new(
        "soft-delete-source",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "source",
                ArtifactRevisionId::new("source-r1").unwrap(),
                LifecycleState::Deleted,
                None,
            )
            .unwrap(),
        )],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::CommittedClosurePending);
    assert_eq!(receipt.closure_operation_ids.len(), 1);
    let status = opened
        .engine
        .read_dependency_closure(
            ClosureLookupV1::new(receipt.closure_operation_ids[0].clone()).unwrap(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(status.phase, ClosurePhaseV1::Complete);
    assert_eq!(status.cause, ClosureCauseV1::SoftDeleted);
    assert!(matches!(status.root, ClosureRootV1::SourceRevision { .. }));
    let proof = status.proof.unwrap();
    assert_eq!(proof.current_active_dependent_nodes, 0);
    assert_eq!(proof.current_derived_edges, 0);
    assert_eq!(proof.view_eligible_dependents, 0);
    assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
}

#[test]
fn active_barrier_precedes_source_lifecycle_eligibility() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "barrier");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-r1", "v1", "source", "source body"),
            derived_node("derived-r1", "derived", "source-r1"),
        ])
        .unwrap();
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='1' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,?1,'source_revision','source-r1','soft_deleted',0,2,0,1,?2,\
                      'proving',1,NULL,NULL,NULL)",
            params![format!("_fdb:c:{}", "a".repeat(64)), "b".repeat(64)],
        )
        .unwrap();
    drop(connection);

    let error = opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Dependency(ref error)
            if error.reason == DependencyErrorReason::DependencyClosureActive
    ));

    let write_error =
        opened.engine.write(&[derived_node("derived-r2", "derived-2", "source-r1")]).unwrap_err();
    assert!(matches!(
        write_error,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::SourceClosureActive
    ));
}

#[test]
fn physical_source_purge_erases_registered_dependents_and_keeps_proof() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "physical");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
    opened.engine.purge("source").unwrap();

    let connection = Connection::open(&db).unwrap();
    for revision in ["source-r1", "derived-r1", "edge-r1"] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
                [revision],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{revision} must be physically erased");
    }
    let physical: (String, i64, i64) = connection
        .query_row(
            "SELECT phase,structural_proof_write_boundary,\
                    json_extract(proof_json,'$.remaining_dependency_rows') \
             FROM _fathomdb_dependency_closures WHERE cause='purged' \
             ORDER BY closure_sequence DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(physical.0, "complete");
    assert!(physical.1 > 0);
    assert_eq!(physical.2, 0);
}

#[test]
fn registered_derived_read_uses_source_validity_from_the_same_read_view() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "source-validity")).unwrap();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    opened
        .engine
        .write(&[
            canonical_with_validity(
                "source-r1",
                "v1",
                "source",
                "source body",
                Some(now - 10),
                Some(now + 10),
            ),
            derived_node("derived-r1", "derived", "source-r1"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();

    let inside = ReadView { valid_as_of: Some(now), ..ReadView::default() };
    let at_end = ReadView { valid_as_of: Some(now + 10), ..ReadView::default() };
    let relaxed = ReadView {
        valid_as_of: Some(now + 10),
        include_out_of_window: true,
        ..ReadView::default()
    };

    assert!(opened.engine.read_get("derived", &inside).unwrap().is_some());
    assert!(opened.engine.read_get("derived", &at_end).unwrap().is_none());
    assert!(opened.engine.read_get("derived", &relaxed).unwrap().is_some());
}

#[test]
fn physical_closure_measures_surviving_canonical_rows_and_rolls_back() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "physical-proof-measured");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();

    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER preserve_derived_row_before_delete \
             BEFORE DELETE ON canonical_nodes \
             WHEN OLD.logical_id='derived' \
             BEGIN SELECT RAISE(IGNORE); END;",
        )
        .unwrap();
    drop(connection);

    assert!(matches!(opened.engine.purge("source"), Err(EngineError::Storage)));

    let connection = Connection::open(&db).unwrap();
    let roots: i64 = connection
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='source'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let derived: i64 = connection
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='derived'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!((roots, derived), (1, 1), "the failed proof must roll back the erase");
}

#[test]
fn physical_closure_measures_projection_and_dependency_residue() {
    for residue in
        ["projection", "dependency", "artifact_revision", "source_link", "source_version"]
    {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, residue);
        let opened = Engine::open(&db).unwrap();
        seed_registered(&opened.engine);
        opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
        let connection = Connection::open(&db).unwrap();
        match residue {
            "projection" => {
                let cursor: i64 = connection
                    .query_row(
                        "SELECT write_cursor FROM _fathomdb_artifact_revisions \
                         WHERE revision_id='derived-r1'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                connection
                    .execute(
                        "INSERT OR REPLACE INTO _fathomdb_projection_terminal(write_cursor,state) \
                         VALUES(?1,'up_to_date')",
                        [cursor],
                    )
                    .unwrap();
                connection
                    .execute_batch(
                        "CREATE TRIGGER preserve_projection_before_delete \
                         BEFORE DELETE ON _fathomdb_projection_terminal \
                         BEGIN SELECT RAISE(IGNORE); END;",
                    )
                    .unwrap();
            }
            "dependency" => connection
                .execute_batch(
                    "CREATE TRIGGER preserve_dependency_before_delete \
                     BEFORE DELETE ON _fathomdb_source_dependencies \
                     WHEN OLD.derived_revision_id='derived-r1' \
                     BEGIN SELECT RAISE(IGNORE); END;",
                )
                .unwrap(),
            "artifact_revision" => connection
                .execute_batch(
                    "CREATE TRIGGER preserve_artifact_revision_before_delete \
                     BEFORE DELETE ON _fathomdb_artifact_revisions \
                     WHEN OLD.revision_id='derived-r1' \
                     BEGIN SELECT RAISE(IGNORE); END;",
                )
                .unwrap(),
            "source_link" => connection
                .execute_batch(
                    "CREATE TRIGGER preserve_source_link_before_delete \
                     BEFORE DELETE ON _fathomdb_source_links \
                     WHEN OLD.artifact_revision_id='derived-r1' \
                     BEGIN SELECT RAISE(IGNORE); END;",
                )
                .unwrap(),
            "source_version" => connection
                .execute_batch(
                    "CREATE TRIGGER preserve_source_version_before_delete \
                     BEFORE DELETE ON _fathomdb_source_versions \
                     WHEN OLD.source_revision_id='source-r1' \
                     BEGIN SELECT RAISE(IGNORE); END;",
                )
                .unwrap(),
            _ => unreachable!(),
        }
        drop(connection);

        assert!(matches!(opened.engine.purge("source"), Err(EngineError::Storage)), "{residue}");
        let connection = Connection::open(&db).unwrap();
        let roots: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='source'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(roots, 1, "{residue} proof failure must roll back the erase");
    }
}

#[test]
fn logical_purge_preserves_unrelated_artifact_in_the_same_source_bucket() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "purge-shared-bucket");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened
        .engine
        .write(&[canonical("unrelated-r1", "unrelated-v1", "unrelated", "unrelated body")])
        .unwrap();
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();

    opened.engine.purge("source").unwrap();

    let connection = Connection::open(&db).unwrap();
    let unrelated: i64 = connection
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='unrelated'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(unrelated, 1);
}

#[test]
fn bm25f_excludes_dependency_behind_an_admitted_barrier() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "bm25f-barrier");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-r1", "v1", "source", "source body"),
            derived_node_with_body("derived-r1", "derived", "classifiedtoken"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    install_soft_barrier(&db, "source-r1", 3);

    let results =
        opened.engine.bm25f_search("classifiedtoken", &Bm25fQueryPlan::default()).unwrap();
    assert!(results.is_empty(), "barrier-fenced dependent leaked through BM25F: {results:?}");
}

#[test]
fn bm25f_excludes_dependency_of_an_expired_source() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "bm25f-expired-source");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-r1", "v1", "source", "source body"),
            derived_node_with_body("derived-r1", "derived", "classifiedtoken"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    Connection::open(&db)
        .unwrap()
        .execute("UPDATE canonical_nodes SET valid_until=?1 WHERE logical_id='source'", [now - 1])
        .unwrap();

    let results =
        opened.engine.bm25f_search("classifiedtoken", &Bm25fQueryPlan::default()).unwrap();
    assert!(results.is_empty(), "expired-source dependent leaked through BM25F: {results:?}");
}

#[test]
fn post_commit_closure_finalization_failure_does_not_reuse_write_boundary() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "post-commit-finalization");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER fail_closure_completion_before_update \
             BEFORE UPDATE OF phase ON _fathomdb_dependency_closures \
             WHEN NEW.phase='complete' \
             BEGIN SELECT RAISE(ABORT, 'forced closure finalization failure'); END;",
        )
        .unwrap();
    drop(connection);

    let request = ActuationBatchV1::new(
        "force-finalization-failure",
        vec![
            ActuationOperationV1::PutCanonicalNode(
                match canonical("batch-r1", "batch-v1", "batch-write", "batch body") {
                    PreparedWrite::ProvenancedNode(node) => node,
                    _ => unreachable!(),
                },
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
    assert!(matches!(opened.engine.actuate(request), Err(EngineError::Storage)));

    let connection = Connection::open(&db).unwrap();
    connection.execute("DROP TRIGGER fail_closure_completion_before_update", []).unwrap();
    let committed_boundary: i64 = connection
        .query_row(
            "SELECT admitted_write_boundary FROM _fathomdb_dependency_closures \
             WHERE root_value='source-r1' ORDER BY closure_sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    opened
        .engine
        .write(&[canonical("unrelated-r1", "unrelated-v1", "unrelated", "unrelated body")])
        .unwrap();
    let connection = Connection::open(&db).unwrap();
    let next_cursor: i64 = connection
        .query_row(
            "SELECT write_cursor FROM canonical_nodes WHERE logical_id='unrelated'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(next_cursor > committed_boundary);
}

#[test]
fn vector_top_k_ineligible_dependencies_degrade_before_candidate_truncation() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "vector-pre-truncation");
    let opened = Engine::open_with_embedder_for_test(&db, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.configure_vector_kind_for_test("fact").unwrap();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    let mut writes = vec![canonical_with_validity(
        "source-r1",
        "v1",
        "source",
        "source body",
        Some(now - 10),
        Some(now + 10),
    )];
    for index in 0..TOP_K_BIT_CANDIDATES {
        writes.push(derived_node_with_body(
            &format!("derived-r{index}"),
            &format!("derived-{index}"),
            "decoy body",
        ));
    }
    writes.push(PreparedWrite::Node {
        kind: "doc".into(),
        body: "eligible unique target".into(),
        source_id: SourceId::new("eligible-bucket").unwrap(),
        logical_id: Some("eligible".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    opened.engine.write(&writes).unwrap();
    for index in 0..TOP_K_BIT_CANDIDATES {
        opened
            .engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new(
                    format!("dependency-{index}"),
                    "source-r1",
                    format!("derived-r{index}"),
                )
                .unwrap(),
            )
            .unwrap();
    }
    opened.engine.drain(10_000).unwrap();

    let view = ReadView { valid_as_of: Some(now + 10), ..ReadView::default() };
    let result = opened.engine.search_view("eligible", &view).unwrap();
    assert!(result.results.iter().any(|hit| hit.body == "eligible unique target"));
    assert_eq!(
        result.soft_fallback.as_ref().map(|fallback| fallback.branch),
        Some(SoftFallbackBranch::Vector),
    );
}

#[test]
fn projection_worker_before_admission_cannot_publish_dependency_residue() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        path(&dir, "projection-worker-first"),
        Arc::new(FixedEmbedder),
    )
    .unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let (ready, release) = opened.engine.pause_projection_worker_after_wal_transaction_for_test();
    seed_projection_race(&opened.engine);
    ready.wait();

    let (sent, received) = mpsc::sync_channel(1);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            sent.send(opened.engine.transition("race-source", LifecycleState::Deleted, None))
                .unwrap();
        });
        assert!(received.recv_timeout(Duration::from_millis(50)).is_err());
        release.wait();
        received.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    });
    opened.engine.drain(5_000).unwrap();
    assert_eq!(opened.engine.vector_row_count_for_test().unwrap(), 0);
}

#[test]
fn admission_before_projection_worker_terminalizes_without_publication() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "projection-admission-first");
    let opened = Engine::open_with_embedder_for_test(&db, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    seed_projection_race(&opened.engine);
    install_soft_barrier(&db, "race-source-r1", 3);
    opened.engine.set_projection_scheduler_frozen_for_test(false);
    opened.engine.drain(5_000).unwrap();
    assert_eq!(opened.engine.vector_row_count_for_test().unwrap(), 1);
}

#[test]
fn next_writer_recovers_a_proving_soft_closure_after_reopen() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "recover-soft");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    let request = ActuationBatchV1::new(
        "recover-soft",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "source",
                ArtifactRevisionId::new("source-r1").unwrap(),
                LifecycleState::Deleted,
                None,
            )
            .unwrap(),
        )],
    )
    .unwrap();
    let receipt = opened.engine.actuate(request).unwrap();
    let closure_id = receipt.closure_operation_ids[0].clone();
    opened.engine.close().unwrap();

    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures SET phase='proving', \
             structural_proof_write_boundary=NULL, proof_json=NULL \
             WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    let reopened = Engine::open(&db).unwrap();
    reopened
        .engine
        .write(&[canonical("unrelated-r1", "v-unrelated", "unrelated", "unrelated body")])
        .unwrap();
    let status = reopened
        .engine
        .read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(status.phase, ClosurePhaseV1::Complete);
}

#[test]
fn unrelated_writer_is_fenced_while_physical_closure_is_nonterminal() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "physical-fence");
    let opened = Engine::open(&db).unwrap();
    opened.engine.write(&[canonical("victim-r1", "v1", "victim", "victim body")]).unwrap();
    let connection = Connection::open(&db).unwrap();
    connection
        .execute("UPDATE canonical_nodes SET state='deleted' WHERE logical_id='victim'", [])
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='1' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();
    let proof = serde_json::json!({
        "schema_version": 1,
        "proof_write_boundary": 0,
        "current_active_dependent_nodes": 0,
        "current_derived_edges": 0,
        "view_eligible_dependents": 0,
        "ownerless_projection_rows": 0,
        "post_admission_registrations": 0,
        "remaining_dependency_rows": 0,
        "remaining_canonical_rows": 0,
        "remaining_projection_rows": 0,
        "remaining_receipt_reference_rows": 0,
    });
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,?1,'source_bucket','bucket','source_erased',0,0,0,1,?2,\
                      'at_rest_pending',1,NULL,0,?3)",
            params![format!("_fdb:c:{}", "c".repeat(64)), "d".repeat(64), proof.to_string()],
        )
        .unwrap();
    drop(connection);

    let error = opened
        .engine
        .write(&[canonical("unrelated-r1", "v-unrelated", "unrelated", "unrelated body")])
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::ErasureIncomplete { ref stage, .. } if stage == "dependency_closure"
    ));
    let configure_error = opened.engine.configure_projections(&[], &[]).unwrap_err();
    assert!(matches!(
        configure_error,
        EngineError::ErasureIncomplete { ref stage, .. } if stage == "dependency_closure"
    ));
    let purge_error = opened.engine.purge("victim").unwrap_err();
    assert!(matches!(
        purge_error,
        EngineError::ErasureIncomplete { ref stage, .. } if stage == "dependency_closure"
    ));
}

#[test]
fn nonterminal_barrier_hides_derived_search_hits() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "read-barrier");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-r1", "v1", "source", "source body"),
            derived_node("derived-r1", "derived", "source-r1"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    assert!(!opened.engine.search("derived body").unwrap().results.is_empty());

    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='1' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,?1,'source_revision','source-r1','soft_deleted',0,2,1,1,?2,\
                      'proving',1,NULL,NULL,NULL)",
            params![format!("_fdb:c:{}", "e".repeat(64)), "f".repeat(64)],
        )
        .unwrap();
    drop(connection);

    assert!(opened
        .engine
        .search("derived body")
        .unwrap()
        .results
        .iter()
        .all(|hit| !hit.body.starts_with("derived body")));
}

#[test]
fn barrier_hides_derived_nodes_from_expansion_and_property_search() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "all-read-barriers");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "summary".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    opened.engine.write(&[canonical("source-r1", "v1", "source", "source body")]).unwrap();
    opened
        .engine
        .write(&[
            derived_node_with_body(
                "derived-r1",
                "derived",
                r#"{"summary":"derived projected needle"}"#,
            ),
            PreparedWrite::Node {
                kind: "doc".into(),
                body: r#"{"summary":"root needle"}"#.into(),
                source_id: SourceId::new("other-bucket").unwrap(),
                logical_id: Some("root".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            plain_edge("root", "derived", "root-derived"),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();

    assert_eq!(
        opened
            .engine
            .query_i64_col_for_test(
                "SELECT COUNT(*) FROM property_search_index \
                 WHERE property_search_index MATCH 'needle' AND attr_name='summary'",
            )
            .unwrap(),
        vec![2]
    );
    let baseline = opened
        .engine
        .search_projected_text("needle", "summary", None, &ReadView::default())
        .unwrap();
    assert!(
        baseline.results.iter().any(|hit| hit.body.contains("derived projected needle")),
        "baseline projected hits: {:?}",
        baseline.results
    );
    let derived_cursor = Connection::open(&db)
        .unwrap()
        .query_row(
            "SELECT write_cursor FROM canonical_nodes WHERE logical_id='derived'",
            [],
            |row| row.get::<_, u64>(0),
        )
        .unwrap();
    opened.engine.write_node_importance(derived_cursor, 0.75).unwrap();
    assert_eq!(opened.engine.node_importance(derived_cursor).unwrap(), Some(0.75));

    install_soft_barrier(&db, "source-r1", 4);

    let relaxed = ReadView {
        include_superseded: true,
        include_inactive: true,
        include_out_of_window: true,
        valid_as_of: None,
    };
    assert!(opened.engine.read_get("derived", &relaxed).unwrap().is_none());
    assert!(opened
        .engine
        .read_list("fact", &[], 10, &relaxed)
        .unwrap()
        .iter()
        .all(|node| node.logical_id != "derived"));
    assert!(opened
        .engine
        .graph_neighbors("root", 1, TraversalDirection::Both, &relaxed)
        .unwrap()
        .iter()
        .all(|node| node.logical_id != "derived"));
    let expanded = opened.engine.search_expand("root needle", None, 1).unwrap();
    assert!(expanded.expanded.iter().all(|(node, _)| node.logical_id != "derived"));
    let projected = opened
        .engine
        .search_projected_text("needle", "summary", None, &ReadView::default())
        .unwrap();
    assert!(projected.results.iter().all(|hit| !hit.body.starts_with("derived body")));
    assert_eq!(opened.engine.node_importance(derived_cursor).unwrap(), None);
}

#[test]
fn source_erasure_records_blocker_and_exact_retry_completes_closure() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "physical-retry");
    let sink = dir.path().join("telemetry.jsonl");
    let rotated = dir.path().join("telemetry.jsonl.1");
    let opened = Engine::open(&db).unwrap();
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    seed_registered(&opened.engine);
    opened.engine.search("derived body").unwrap();
    assert!(std::fs::read_to_string(&sink).unwrap().contains("l:derived"));
    std::fs::rename(&sink, &rotated).unwrap();

    let error = opened.engine.erase_source("bucket").unwrap_err();
    assert!(matches!(
        error,
        EngineError::ErasureIncomplete { ref stage, .. } if stage == "telemetry_redaction"
    ));
    let connection = Connection::open(&db).unwrap();
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
             WHERE root_kind='source_bucket' AND root_value='bucket'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);
    let status = opened
        .engine
        .read_dependency_closure(ClosureLookupV1::new(&closure_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(status.phase, ClosurePhaseV1::Incomplete);
    assert_eq!(status.blocker_code.as_deref(), Some("telemetry_redaction"));
    assert!(matches!(
        opened.engine.write(&[canonical(
            "unrelated-r1",
            "v-unrelated",
            "unrelated",
            "unrelated body",
        )]),
        Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "dependency_closure"
    ));

    std::fs::rename(&rotated, &sink).unwrap();
    let retry = opened.engine.erase_source("bucket").unwrap();
    assert_eq!(retry.nodes_excised, 0);
    let status = opened
        .engine
        .read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(status.phase, ClosurePhaseV1::Complete);
    assert_eq!(status.blocker_code, None);
    let contents = std::fs::read_to_string(&sink).unwrap();
    assert!(!contents.contains("l:source"));
    assert!(!contents.contains("l:derived"));
}

#[test]
fn open_rejects_missing_malformed_or_regressed_closure_sequence() {
    for mutation in [
        "DELETE FROM _fathomdb_open_state WHERE key='_fathomdb_closure_sequence'",
        "UPDATE _fathomdb_open_state SET value='01' WHERE key='_fathomdb_closure_sequence'",
        "UPDATE _fathomdb_open_state SET value='0' WHERE key='_fathomdb_closure_sequence'",
    ] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, "sequence-corruption");
        let opened = Engine::open(&db).unwrap();
        seed_registered(&opened.engine);
        opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
        opened.engine.close().unwrap();
        let connection = Connection::open(&db).unwrap();
        connection.execute(mutation, []).unwrap();
        drop(connection);

        assert!(matches!(Engine::open(&db), Err(EngineOpenError::Corruption(_))), "{mutation}");
    }
}

#[test]
fn point_status_rejects_corrupt_proof_shape() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "proof-corruption");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
    let connection = Connection::open(&db).unwrap();
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
             SET proof_json=json_remove(proof_json,'$.view_eligible_dependents') \
             WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    assert!(matches!(
        opened.engine.read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap()),
        Err(EngineError::Storage)
    ));
}

#[test]
fn point_status_rejects_nonzero_complete_proof() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "proof-semantic-point-corruption");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
    let connection = Connection::open(&db).unwrap();
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
             SET proof_json=json_set(proof_json,'$.current_active_dependent_nodes',1) \
             WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    assert!(matches!(
        opened.engine.read_dependency_closure(ClosureLookupV1::new(closure_id).unwrap()),
        Err(EngineError::Storage)
    ));
}

#[test]
fn reopen_rejects_nonzero_complete_proof() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "proof-semantic-open-corruption");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
    opened.engine.close().unwrap();
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
             SET proof_json=json_set(proof_json,'$.current_active_dependent_nodes',1)",
            [],
        )
        .unwrap();
    drop(connection);

    assert!(matches!(Engine::open(&db), Err(EngineOpenError::Corruption(_))));
}

#[test]
fn closure_sequence_exhaustion_rolls_back_root_supersession() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "sequence-exhaustion");
    let opened = Engine::open(&db).unwrap();
    seed_registered(&opened.engine);
    Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_open_state SET value='9223372036854775807' \
             WHERE key='_fathomdb_closure_sequence'",
            [],
        )
        .unwrap();

    assert!(matches!(
        opened.engine.write(&[canonical("source-r2", "v2", "source", "replacement")]),
        Err(EngineError::Storage)
    ));
    let connection = Connection::open(&db).unwrap();
    let current: (String, String) = connection
        .query_row(
            "SELECT r.revision_id,n.body FROM canonical_nodes n \
             JOIN _fathomdb_artifact_revisions r ON r.write_cursor=n.write_cursor \
             WHERE n.logical_id='source' AND n.superseded_at IS NULL",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(current, ("source-r1".into(), "source body".into()));
    let closures: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_dependency_closures", [], |row| row.get(0))
        .unwrap();
    assert_eq!(closures, 0);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn mutated_proof_semantics_fail_closed_identically_at_point_read_and_reopen(
        physical in any::<bool>(),
        field_index in 0_usize..9,
        nonzero in 1_u64..=u16::MAX as u64,
    ) {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, "proof-semantic-property");
        let opened = Engine::open(&db).unwrap();
        seed_registered(&opened.engine);
        opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
        if physical {
            opened.engine.purge("source").unwrap();
        }
        let connection = Connection::open(&db).unwrap();
        let closure_id: String = connection
            .query_row(
                "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
                 WHERE cause=?1 ORDER BY closure_sequence DESC LIMIT 1",
                [if physical { "purged" } else { "soft_deleted" }],
                |row| row.get(0),
            )
            .unwrap();
        let common_fields = [
            "current_active_dependent_nodes",
            "current_derived_edges",
            "view_eligible_dependents",
            "ownerless_projection_rows",
            "post_admission_registrations",
        ];
        let physical_fields = [
            "remaining_dependency_rows",
            "remaining_canonical_rows",
            "remaining_projection_rows",
            "remaining_receipt_reference_rows",
        ];
        let (field, value): (&str, Option<u64>) = if field_index < common_fields.len() {
            (common_fields[field_index], Some(nonzero))
        } else if physical {
            (physical_fields[field_index - common_fields.len()], None)
        } else {
            (physical_fields[field_index - common_fields.len()], Some(0))
        };
        let json_path = format!("$.{field}");
        connection
            .execute(
                "UPDATE _fathomdb_dependency_closures \
                 SET proof_json=json_set(proof_json,?1,?2) \
                 WHERE closure_operation_id=?3",
                params![json_path, value, closure_id],
            )
            .unwrap();
        drop(connection);

        prop_assert!(matches!(
            opened.engine.read_dependency_closure(
                ClosureLookupV1::new(closure_id.clone()).unwrap()
            ),
            Err(EngineError::Storage)
        ));
        opened.engine.close().unwrap();
        prop_assert!(matches!(Engine::open(&db), Err(EngineOpenError::Corruption(_))));
    }
}
