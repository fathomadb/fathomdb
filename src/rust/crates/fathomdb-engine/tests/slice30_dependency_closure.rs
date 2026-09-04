//! Slice 30 — dependency-aware lifecycle and erasure closure.

use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ArtifactRevisionId, CanonicalHash,
    ClosureCauseV1, ClosureLookupV1, ClosurePhaseV1, ClosureRootV1, DependencyErrorReason, Engine,
    EngineError, InitialState, LifecycleActuationV1, LifecycleState, PreparedWrite,
    ProvenanceErrorReason, ProvenancedEdgeV1, ProvenancedNodeV1, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

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
    assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
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
    let connection = Connection::open(&db).unwrap();
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

    assert!(opened.engine.search("derived body").unwrap().results.is_empty());
}
