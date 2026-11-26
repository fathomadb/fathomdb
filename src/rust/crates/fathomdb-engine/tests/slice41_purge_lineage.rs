//! 0.8.32 Slice 41 (`WP-41A`, `#268`, `AC32-PURGE-LINEAGE-01..02`) —
//! Purge-lineage cursor monotonicity and shared-bucket actuation receipt hardening.

use std::path::{Path, PathBuf};

use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, DataPlaneIntegrityCheckV1,
    DataPlaneIntegrityRequestV1, Engine, EngineError, InitialState, LifecycleActuationV1,
    LifecycleState, ProvenanceErrorReason, ProvenancedEdgeV1, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn sha256_hex(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical_node(
    revision: &str,
    source_version: &str,
    logical_id: &str,
    source_id: &str,
    body: &str,
) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).expect("valid source_id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).expect("valid revision"),
            SourceVersionId::new(source_version).expect("valid source version"),
        ),
        properties: None,
    }
}

fn derived_node(
    revision: &str,
    logical_id: &str,
    source_id: &str,
    source_version: &str,
    source_revision: &str,
    source_body: &str,
    body: &str,
) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).expect("valid source_id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).expect("valid revision"),
            SourceVersionId::new(source_version).expect("valid source version"),
            SourceRevisionId::new(source_revision).expect("valid source revision"),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body)).expect("valid hash"),
        ),
        properties: None,
    }
}

struct SourceRef<'a> {
    source_id: &'a str,
    version: &'a str,
    revision: &'a str,
    body: &'a str,
}

fn derived_edge(
    revision: &str,
    logical_id: &str,
    endpoints: (&str, &str),
    source: SourceRef<'_>,
    body: &str,
) -> ProvenancedEdgeV1 {
    ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: endpoints.0.into(),
        to: endpoints.1.into(),
        source_id: SourceId::new(source.source_id).expect("valid source_id"),
        logical_id: Some(logical_id.into()),
        body: Some(body.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).expect("valid revision"),
            SourceVersionId::new(source.version).expect("valid source version"),
            SourceRevisionId::new(source.revision).expect("valid source revision"),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source.body)).expect("valid hash"),
        ),
        properties: None,
    }
}

fn assert_data_plane_integrity_clean(engine: &Engine) {
    let request = DataPlaneIntegrityRequestV1::new(
        vec![
            DataPlaneIntegrityCheckV1::DependencyChain,
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
            DataPlaneIntegrityCheckV1::ProjectionGeneration,
            DataPlaneIntegrityCheckV1::MutationReadiness,
            DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity,
        ],
        10_000,
        100,
    )
    .expect("valid dpi request");
    let report = engine.check_data_plane_integrity(request).expect("dpi check succeeds");
    assert!(
        report.findings.is_empty(),
        "expected zero data-plane-integrity findings, got: {:?}",
        report.findings
    );
}

fn assert_database_and_wal_do_not_contain(path: &Path, secrets: &[&str]) {
    let mut bytes = std::fs::read(path).expect("read db file");
    let wal_path = format!("{}-wal", path.display());
    if let Ok(wal) = std::fs::read(wal_path) {
        bytes.extend_from_slice(&wal);
    }
    for secret in secrets {
        assert!(
            !bytes.windows(secret.len()).any(|window| window == secret.as_bytes()),
            "erased bytes remained at rest: {secret}"
        );
    }
}

fn assert_operation_erased(engine: &Engine, batch: ActuationBatchV1) {
    let err = engine.actuate(batch).expect_err("expected OperationIdErased on replay");
    assert!(
        matches!(
            err,
            EngineError::Actuation(ref act)
                if act.reason == ActuationErrorReason::OperationIdErased
        ),
        "expected OperationIdErased, got: {err:?}"
    );
}

// =========================================================================
// AC32-PURGE-LINEAGE-01: Post-purge cursor monotonicity & DPI
// =========================================================================

#[test]
fn actuate_delete_purge_actuate_sole_node_in_process_and_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let shared_bucket = "shared-bucket-sole";

    // Part A: In-process sole-node actuate -> transition("deleted") + purge() -> actuate.
    let in_proc_path = db_path(&dir, "sole-in-process");
    {
        let opened = Engine::open(&in_proc_path).expect("open in-process db");
        let r1 = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-sole-1",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "sole-r1",
                        "sole-v1",
                        "sole-node-1",
                        shared_bucket,
                        "sole node first body",
                    ))],
                )
                .unwrap(),
            )
            .expect("first actuation commits");
        assert_eq!(r1.outcome, ActuationOutcomeV1::Committed);
        let b1 = r1.resulting_write_boundary.expect("boundary 1");

        opened
            .engine
            .transition("sole-node-1", LifecycleState::Deleted, Some("sole purge".into()))
            .expect("transition to deleted");
        opened.engine.purge("sole-node-1").expect("purge sole node");

        // Canonical table is now empty; next actuation must succeed with strictly greater boundary.
        let r2 = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-sole-2",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "sole-r2",
                        "sole-v2",
                        "sole-node-2",
                        shared_bucket,
                        "sole node second body",
                    ))],
                )
                .unwrap(),
            )
            .expect("post-purge in-process actuation must succeed without StorageError");
        assert_eq!(r2.outcome, ActuationOutcomeV1::Committed);
        let b2 = r2.resulting_write_boundary.expect("boundary 2");
        assert!(b2 > b1, "expected strictly monotonic boundary: b2={b2} <= b1={b1}");

        opened.engine.drain(5_000).expect("drain");
        assert_data_plane_integrity_clean(&opened.engine);
        opened.engine.close().expect("close in-process db");
    }

    // Part B: Across close() / open() sole-node actuate -> transition("deleted") + purge() -> reopen -> actuate.
    let reopen_path = db_path(&dir, "sole-reopen");
    let b1 = {
        let opened = Engine::open(&reopen_path).expect("open reopen db");
        let r1 = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-reopen-sole-1",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "reopen-sole-r1",
                        "reopen-sole-v1",
                        "reopen-sole-node-1",
                        shared_bucket,
                        "reopen sole first body",
                    ))],
                )
                .unwrap(),
            )
            .expect("first actuation before reopen");
        assert_eq!(r1.outcome, ActuationOutcomeV1::Committed);
        let b1 = r1.resulting_write_boundary.expect("boundary 1");

        opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-reopen-sole-del-1",
                    vec![ActuationOperationV1::TransitionLifecycle(
                        LifecycleActuationV1::new(
                            "reopen-sole-node-1",
                            ArtifactRevisionId::new("reopen-sole-r1").unwrap(),
                            LifecycleState::Deleted,
                            Some("delete via actuation".into()),
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .expect("actuate transition to deleted");
        opened.engine.purge("reopen-sole-node-1").expect("purge sole node before close");
        opened.engine.close().expect("close after sole purge");
        b1
    };

    {
        let reopened = Engine::open(&reopen_path).expect("reopen after sole purge");
        let r2 = reopened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-reopen-sole-2",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "reopen-sole-r2",
                        "reopen-sole-v2",
                        "reopen-sole-node-2",
                        shared_bucket,
                        "reopen sole replacement body",
                    ))],
                )
                .unwrap(),
            )
            .expect("post-purge reopened actuation must succeed without StorageError");
        assert_eq!(r2.outcome, ActuationOutcomeV1::Committed);
        let b2 = r2.resulting_write_boundary.expect("boundary 2");
        assert!(b2 > b1, "expected strictly monotonic reopened boundary: b2={b2} <= b1={b1}");

        reopened.engine.drain(5_000).expect("drain after reopen");
        let hits = reopened
            .engine
            .search_text_only("replacement")
            .expect("search replacement node after reopen");
        assert_eq!(hits.results.len(), 1);
        assert_eq!(hits.results[0].id.value.as_str(), "reopen-sole-node-2");
        assert_data_plane_integrity_clean(&reopened.engine);
        reopened.engine.close().expect("close reopened db");
    }
}

#[test]
fn actuate_delete_purge_actuate_trailing_cursor_and_dpi() {
    let dir = TempDir::new().expect("tempdir");
    let shared_bucket = "shared-bucket-trailing";
    let path = db_path(&dir, "trailing-cursor");

    // Seed surviving lower-cursor node + trailing higher-cursor node in same source_id bucket.
    let (b_trailing_in_proc, b_after_in_proc) = {
        let opened = Engine::open(&path).expect("open trailing db");
        let r_base = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-base",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "base-r1",
                        "base-v1",
                        "base-survivor",
                        shared_bucket,
                        "baseline survivor node alpha",
                    ))],
                )
                .unwrap(),
            )
            .expect("commit base survivor");
        let b_base = r_base.resulting_write_boundary.expect("base boundary");

        let r_trail_1 = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-trail-1",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "trail-r1",
                        "trail-v1",
                        "trailing-ephemeral-1",
                        shared_bucket,
                        "trailing ephemeral one",
                    ))],
                )
                .unwrap(),
            )
            .expect("commit trailing 1");
        let b_trail_1 = r_trail_1.resulting_write_boundary.expect("trail 1 boundary");
        assert!(b_trail_1 > b_base);

        // In-process delete + purge of trailing-cursor node, then immediate actuate.
        opened
            .engine
            .transition(
                "trailing-ephemeral-1",
                LifecycleState::Deleted,
                Some("trailing purge 1".into()),
            )
            .expect("delete trailing 1");
        opened.engine.purge("trailing-ephemeral-1").expect("purge trailing 1");

        let r_after_1 = opened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-trail-2",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "trail-r2",
                        "trail-v2",
                        "trailing-ephemeral-2",
                        shared_bucket,
                        "trailing ephemeral two",
                    ))],
                )
                .unwrap(),
            )
            .expect("in-process actuate after trailing purge must succeed");
        let b_after_1 = r_after_1.resulting_write_boundary.expect("trail 2 boundary");
        assert!(
            b_after_1 > b_trail_1,
            "expected strictly monotonic boundary after in-process trailing purge: {b_after_1} <= {b_trail_1}"
        );

        // Now delete + purge trailing-ephemeral-2 and close before next actuation.
        opened
            .engine
            .transition(
                "trailing-ephemeral-2",
                LifecycleState::Deleted,
                Some("trailing purge 2".into()),
            )
            .expect("delete trailing 2");
        opened.engine.purge("trailing-ephemeral-2").expect("purge trailing 2");
        assert_data_plane_integrity_clean(&opened.engine);
        opened.engine.close().expect("close after trailing purge 2");
        (b_trail_1, b_after_1)
    };

    // Reopen and actuate a third trailing node; boundary must stay strictly monotonic past b_after_in_proc.
    {
        let reopened = Engine::open(&path).expect("reopen after trailing purge");
        let r_after_reopen = reopened
            .engine
            .actuate(
                ActuationBatchV1::new(
                    "op-trail-3",
                    vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "trail-r3",
                        "trail-v3",
                        "trailing-final",
                        shared_bucket,
                        "trailing final survivor beta",
                    ))],
                )
                .unwrap(),
            )
            .expect("reopened actuate after trailing purge must succeed");
        let b_final = r_after_reopen.resulting_write_boundary.expect("final boundary");
        assert!(b_final > b_after_in_proc);
        assert!(b_final > b_trailing_in_proc);

        reopened.engine.drain(5_000).expect("drain");
        let survivor_hits =
            reopened.engine.search_text_only("alpha").expect("search base survivor");
        assert_eq!(survivor_hits.results.len(), 1);
        assert_eq!(survivor_hits.results[0].id.value.as_str(), "base-survivor");
        let final_hits = reopened.engine.search_text_only("beta").expect("search trailing final");
        assert_eq!(final_hits.results.len(), 1);
        assert_eq!(final_hits.results[0].id.value.as_str(), "trailing-final");
        assert_data_plane_integrity_clean(&reopened.engine);
        reopened.engine.close().expect("close reopened trailing db");
    }
}

// =========================================================================
// AC32-PURGE-LINEAGE-02: Shared-bucket receipt hardening & security matrix
// =========================================================================

/// Covers design.md §1.3 Scenarios 1 & 4:
/// - Derived node `D1` purged (with touching edge, dependency registration, committed
///   `TransitionLifecycle`, and a refused `TransitionLifecycle` referencing `rev-D1`).
/// - Surviving sibling `D2` and canonical source `S1` share `"bucket-a"`.
/// - `D1` receipts (and touching edge + refused lifecycle referencing `rev-D1`) are redacted
///   to `OperationIdErased`, while `op-S1` and `op-D2` replay their exact cached receipts cleanly.
#[test]
fn purge_single_node_preserves_shared_bucket_sibling_receipt_replay() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "shared-bucket-single-node");
    let bucket = "bucket-a";

    let s1_body = "shared-bucket-canonical-source-s1-body";
    let d1_body = "secret-derived-d1-body-4c9a12ee";
    let d1_rev = "secret-rev-d1-7b3109fa";
    let d1_edge_body = "secret-edge-d1-body-a1f055cc";
    let d1_edge_rev = "secret-rev-edge-d1-88de21bb";
    let d2_body = "surviving-sibling-d2-body";

    let op_s1 = ActuationBatchV1::new(
        "op-s1",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "rev-s1", "ver-s1", "node-s1", bucket, s1_body,
        ))],
    )
    .unwrap();
    let op_d1 = ActuationBatchV1::new(
        "op-d1",
        vec![ActuationOperationV1::PutDerivedNode(derived_node(
            d1_rev, "node-d1", bucket, "ver-s1", "rev-s1", s1_body, d1_body,
        ))],
    )
    .unwrap();
    let op_d2 = ActuationBatchV1::new(
        "op-d2",
        vec![ActuationOperationV1::PutDerivedNode(derived_node(
            "rev-d2", "node-d2", bucket, "ver-s1", "rev-s1", s1_body, d2_body,
        ))],
    )
    .unwrap();
    let op_edge_d1 = ActuationBatchV1::new(
        "op-edge-d1",
        vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
            d1_edge_rev,
            "edge-d1-to-s1",
            ("node-d1", "node-s1"),
            SourceRef { source_id: bucket, version: "ver-s1", revision: "rev-s1", body: s1_body },
            d1_edge_body,
        ))],
    )
    .unwrap();
    let op_dep_d1 = ActuationBatchV1::new(
        "op-dep-d1",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dep-s1-d1", "rev-s1", d1_rev).unwrap(),
        )],
    )
    .unwrap();
    // Scenario 4: refused TransitionLifecycle referencing expected_current_revision_id = rev-D1
    // (attempting Active -> Active is an illegal lifecycle transition, producing a terminal refusal receipt).
    let op_refused_life_d1 = ActuationBatchV1::new(
        "op-refused-life-d1",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "node-d1",
                ArtifactRevisionId::new(d1_rev).unwrap(),
                LifecycleState::Active,
                Some("illegal self transition".into()),
            )
            .unwrap(),
        )],
    )
    .unwrap();
    let op_life_d1 = ActuationBatchV1::new(
        "op-life-d1",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "node-d1",
                ArtifactRevisionId::new(d1_rev).unwrap(),
                LifecycleState::Deleted,
                Some("retire d1".into()),
            )
            .unwrap(),
        )],
    )
    .unwrap();

    let (receipt_s1, receipt_d2) = {
        let opened = Engine::open(&path).expect("open");
        let r_s1 = opened.engine.actuate(op_s1.clone()).expect("commit s1");
        let r_d1 = opened.engine.actuate(op_d1.clone()).expect("commit d1");
        let r_d2 = opened.engine.actuate(op_d2.clone()).expect("commit d2");
        let r_edge = opened.engine.actuate(op_edge_d1.clone()).expect("commit edge d1");
        let r_dep = opened.engine.actuate(op_dep_d1.clone()).expect("commit dep d1");
        assert_eq!(r_s1.outcome, ActuationOutcomeV1::Committed);
        assert_eq!(r_d1.outcome, ActuationOutcomeV1::Committed);
        assert_eq!(r_d2.outcome, ActuationOutcomeV1::Committed);
        assert_eq!(r_edge.outcome, ActuationOutcomeV1::Committed);
        assert_eq!(r_dep.outcome, ActuationOutcomeV1::Committed);

        let refused =
            opened.engine.actuate(op_refused_life_d1.clone()).expect("refused lifecycle receipt");
        assert_eq!(refused.outcome, ActuationOutcomeV1::Refused);
        assert_eq!(refused.reason_codes, vec![ActuationRefusalReasonV1::LifecycleRefused]);
        assert_eq!(
            opened.engine.actuate(op_refused_life_d1.clone()).expect("replay refused before purge"),
            refused
        );

        let r_life = opened.engine.actuate(op_life_d1.clone()).expect("commit delete d1");
        assert_eq!(r_life.outcome, ActuationOutcomeV1::Committed);

        // Purge ONLY `node-d1` in shared `"bucket-a"`.
        opened.engine.purge("node-d1").expect("purge node-d1");

        // In-process checks: D1's receipts are redacted; surviving siblings S1 & D2 replay cleanly!
        assert_operation_erased(&opened.engine, op_d1.clone());
        assert_operation_erased(&opened.engine, op_edge_d1.clone());
        assert_operation_erased(&opened.engine, op_dep_d1.clone());
        assert_operation_erased(&opened.engine, op_refused_life_d1.clone());
        assert_operation_erased(&opened.engine, op_life_d1.clone());

        assert_eq!(
            opened.engine.actuate(op_s1.clone()).expect("surviving S1 receipt must replay"),
            r_s1
        );
        assert_eq!(
            opened.engine.actuate(op_d2.clone()).expect("surviving sibling D2 receipt must replay"),
            r_d2
        );

        assert_data_plane_integrity_clean(&opened.engine);
        opened.engine.close().expect("close");
        (r_s1, r_d2)
    };

    // At-rest check: zero bytes of D1 body/rev or touching edge body/rev remain in DB or -wal.
    assert_database_and_wal_do_not_contain(&path, &[d1_body, d1_rev, d1_edge_body, d1_edge_rev]);

    // Reopen checks: replay parity persists cleanly across restart.
    {
        let reopened = Engine::open(&path).expect("reopen");
        assert_operation_erased(&reopened.engine, op_d1);
        assert_operation_erased(&reopened.engine, op_edge_d1);
        assert_operation_erased(&reopened.engine, op_dep_d1);
        assert_operation_erased(&reopened.engine, op_refused_life_d1);
        assert_operation_erased(&reopened.engine, op_life_d1);

        assert_eq!(
            reopened.engine.actuate(op_s1).expect("S1 receipt replays after reopen"),
            receipt_s1
        );
        assert_eq!(
            reopened.engine.actuate(op_d2).expect("sibling D2 receipt replays after reopen"),
            receipt_d2
        );
        assert_data_plane_integrity_clean(&reopened.engine);
        reopened.engine.close().expect("close reopened");
    }
}

/// Covers design.md §1.3 Scenario 2:
/// - `canonical_source` node `S1` purged with registered dependent `D1` (in `physical_plans`),
///   while unrelated `S2` and its derived `D2` share `"bucket-a"`.
/// - `op-S1`, `op-D1`, `op-dep-S1-D1`, and `op-life-S1` are redacted (`OperationIdErased`).
/// - `op-S2` and `op-D2` remain completely untouched and replay their exact cached receipts.
#[test]
fn purge_canonical_source_redacts_source_and_dependents_but_not_unrelated_sibling() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "shared-bucket-canonical-source");
    let bucket = "bucket-a";

    let s1_rev = "secret-rev-s1-1122aabb";
    let s1_body = "secret-source-s1-body-3344ccdd";
    let d1_rev = "secret-rev-d1-5566eeff";
    let d1_body = "secret-dependent-d1-body-77889900";

    let s2_rev = "surviving-rev-s2";
    let s2_body = "surviving-source-s2-body";
    let d2_rev = "surviving-rev-d2";
    let d2_body = "surviving-derived-d2-body";

    let op_s1 = ActuationBatchV1::new(
        "op-s1",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            s1_rev, "ver-s1", "node-s1", bucket, s1_body,
        ))],
    )
    .unwrap();
    let op_d1 = ActuationBatchV1::new(
        "op-d1",
        vec![ActuationOperationV1::PutDerivedNode(derived_node(
            d1_rev, "node-d1", bucket, "ver-s1", s1_rev, s1_body, d1_body,
        ))],
    )
    .unwrap();
    let op_dep_1 = ActuationBatchV1::new(
        "op-dep-1",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dep-s1-to-d1", s1_rev, d1_rev).unwrap(),
        )],
    )
    .unwrap();
    let op_s2 = ActuationBatchV1::new(
        "op-s2",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            s2_rev, "ver-s2", "node-s2", bucket, s2_body,
        ))],
    )
    .unwrap();
    let op_d2 = ActuationBatchV1::new(
        "op-d2",
        vec![ActuationOperationV1::PutDerivedNode(derived_node(
            d2_rev, "node-d2", bucket, "ver-s2", s2_rev, s2_body, d2_body,
        ))],
    )
    .unwrap();
    let op_dep_2 = ActuationBatchV1::new(
        "op-dep-2",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dep-s2-to-d2", s2_rev, d2_rev).unwrap(),
        )],
    )
    .unwrap();
    let op_life_s1 = ActuationBatchV1::new(
        "op-life-s1",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "node-s1",
                ArtifactRevisionId::new(s1_rev).unwrap(),
                LifecycleState::Deleted,
                Some("delete canonical source s1".into()),
            )
            .unwrap(),
        )],
    )
    .unwrap();

    let (receipt_s2, receipt_d2, receipt_dep_2) = {
        let opened = Engine::open(&path).expect("open");
        opened.engine.actuate(op_s1.clone()).expect("commit s1");
        opened.engine.actuate(op_d1.clone()).expect("commit d1");
        opened.engine.actuate(op_dep_1.clone()).expect("commit dep 1");
        let r_s2 = opened.engine.actuate(op_s2.clone()).expect("commit s2");
        let r_d2 = opened.engine.actuate(op_d2.clone()).expect("commit d2");
        let r_dep_2 = opened.engine.actuate(op_dep_2.clone()).expect("commit dep 2");
        opened.engine.actuate(op_life_s1.clone()).expect("delete s1");

        // Purge canonical source S1 (cascades to registered dependent D1 via physical_plans).
        opened.engine.purge("node-s1").expect("purge canonical source s1");

        assert_operation_erased(&opened.engine, op_s1.clone());
        assert_operation_erased(&opened.engine, op_d1.clone());
        assert_operation_erased(&opened.engine, op_dep_1.clone());
        assert_operation_erased(&opened.engine, op_life_s1.clone());

        assert_eq!(
            opened.engine.actuate(op_s2.clone()).expect("unrelated S2 in bucket-a must replay"),
            r_s2
        );
        assert_eq!(
            opened.engine.actuate(op_d2.clone()).expect("unrelated D2 in bucket-a must replay"),
            r_d2
        );
        assert_eq!(
            opened
                .engine
                .actuate(op_dep_2.clone())
                .expect("unrelated dep2 in bucket-a must replay"),
            r_dep_2
        );

        assert_data_plane_integrity_clean(&opened.engine);
        opened.engine.close().expect("close");
        (r_s2, r_d2, r_dep_2)
    };

    // Erased content bodies (`S1`, `D1`) and dependent revision `d1_rev` have zero bytes on disk
    // or in `-wal`. `s1_rev` is gone from all canonical, provenance, dependency, and receipt
    // tables — retained solely as the durable physical-closure proof root in
    // `_fathomdb_dependency_closures` (`cause = 'purged', phase = 'complete'`).
    assert_database_and_wal_do_not_contain(&path, &[s1_body, d1_body, d1_rev]);
    {
        let conn = rusqlite::Connection::open(&path).expect("open sqlite");
        for (table, sql) in [
            (
                "_fathomdb_artifact_revisions",
                "SELECT COUNT(*) FROM _fathomdb_artifact_revisions WHERE revision_id = ?1",
            ),
            (
                "_fathomdb_source_versions",
                "SELECT COUNT(*) FROM _fathomdb_source_versions WHERE source_revision_id = ?1",
            ),
            (
                "_fathomdb_source_links",
                "SELECT COUNT(*) FROM _fathomdb_source_links WHERE artifact_revision_id = ?1 OR source_revision_id = ?1",
            ),
            (
                "_fathomdb_source_dependencies",
                "SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE derived_revision_id = ?1",
            ),
            (
                "_fathomdb_actuation_receipt_source_refs",
                "SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs WHERE ref_value = ?1",
            ),
            (
                "_fathomdb_actuation_receipts",
                "SELECT COUNT(*) FROM _fathomdb_actuation_receipts WHERE affected_revision_ids_json LIKE '%' || ?1 || '%'",
            ),
        ] {
            let count: i64 = conn.query_row(sql, [s1_rev], |row| row.get(0)).expect("count query");
            assert_eq!(count, 0, "erased source revision {s1_rev} remained in {table}");
        }
        let closure_proofs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_dependency_closures \
                 WHERE root_kind = 'source_revision' AND root_value = ?1 \
                   AND cause = 'purged' AND phase = 'complete'",
                [s1_rev],
                |row| row.get(0),
            )
            .expect("closure proof count");
        assert_eq!(closure_proofs, 1, "expected durable completed physical-closure proof for S1");
    }

    {
        let reopened = Engine::open(&path).expect("reopen");
        assert_operation_erased(&reopened.engine, op_s1);
        assert_operation_erased(&reopened.engine, op_d1);
        assert_operation_erased(&reopened.engine, op_dep_1);
        assert_operation_erased(&reopened.engine, op_life_s1);

        assert_eq!(reopened.engine.actuate(op_s2).expect("S2 replay after reopen"), receipt_s2);
        assert_eq!(reopened.engine.actuate(op_d2).expect("D2 replay after reopen"), receipt_d2);
        assert_eq!(
            reopened.engine.actuate(op_dep_2).expect("dep2 replay after reopen"),
            receipt_dep_2
        );
        assert_data_plane_integrity_clean(&reopened.engine);
        reopened.engine.close().expect("close reopened");
    }
}

/// Covers design.md §1.3 Scenario 3:
/// - `canonical_source` node `S1` purged while an unregistered derived node `D_unreg` still
///   links to `rev-S1` -> refuses atomically with `ProvenanceErrorReason::ProvenanceInUse`,
///   leaving zero partial purge and all shared-bucket receipts replayable untouched.
#[test]
fn purge_canonical_source_with_unregistered_dependent_refuses_atomically() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "shared-bucket-unregistered-dependent");
    let bucket = "bucket-a";

    let s1_body = "unregistered-dep-source-s1-body";
    let d_unreg_body = "unregistered-derived-body";
    let s2_body = "unrelated-sibling-s2-body";

    let op_s1 = ActuationBatchV1::new(
        "op-s1-unreg",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "rev-s1-unreg",
            "ver-s1-unreg",
            "node-s1-unreg",
            bucket,
            s1_body,
        ))],
    )
    .unwrap();
    let op_d_unreg = ActuationBatchV1::new(
        "op-d-unreg",
        vec![ActuationOperationV1::PutDerivedNode(derived_node(
            "rev-d-unreg",
            "node-d-unreg",
            bucket,
            "ver-s1-unreg",
            "rev-s1-unreg",
            s1_body,
            d_unreg_body,
        ))],
    )
    .unwrap();
    let op_s2 = ActuationBatchV1::new(
        "op-s2-unreg",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "rev-s2-unreg",
            "ver-s2-unreg",
            "node-s2-unreg",
            bucket,
            s2_body,
        ))],
    )
    .unwrap();

    let opened = Engine::open(&path).expect("open");
    let r_s1 = opened.engine.actuate(op_s1.clone()).expect("commit s1");
    let r_d = opened.engine.actuate(op_d_unreg.clone()).expect("commit unregistered derived");
    let r_s2 = opened.engine.actuate(op_s2.clone()).expect("commit sibling s2");

    opened
        .engine
        .transition("node-s1-unreg", LifecycleState::Deleted, Some("attempt purge".into()))
        .expect("delete s1");

    // Purge must refuse with ProvenanceInUse and roll back atomically.
    let err = opened.engine.purge("node-s1-unreg").expect_err("must refuse ProvenanceInUse");
    assert!(
        matches!(
            err,
            EngineError::Provenance(ref p) if p.reason == ProvenanceErrorReason::ProvenanceInUse
        ),
        "expected ProvenanceInUse, got: {err:?}"
    );

    // Zero partial purge: all receipts remain untouched and replayable, dependency & receipt
    // provenance remain clean, and the rolled-back node can still be reactivated cleanly.
    assert_eq!(opened.engine.actuate(op_s1).expect("S1 receipt intact after refusal"), r_s1);
    assert_eq!(
        opened.engine.actuate(op_d_unreg).expect("D_unreg receipt intact after refusal"),
        r_d
    );
    assert_eq!(opened.engine.actuate(op_s2).expect("S2 receipt intact after refusal"), r_s2);
    let dep_report = opened
        .engine
        .check_data_plane_integrity(
            DataPlaneIntegrityRequestV1::new(
                vec![
                    DataPlaneIntegrityCheckV1::DependencyChain,
                    DataPlaneIntegrityCheckV1::MutationReadiness,
                    DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity,
                ],
                10_000,
                100,
            )
            .unwrap(),
        )
        .expect("dpi check after refused purge");
    assert!(
        dep_report.findings.is_empty(),
        "expected zero provenance/readiness findings after refused purge, got: {:?}",
        dep_report.findings
    );
    opened
        .engine
        .transition("node-s1-unreg", LifecycleState::Active, None)
        .expect("rolled-back node remains intact and reactivatable");
    assert_data_plane_integrity_clean(&opened.engine);
    opened.engine.close().expect("close");
}

/// Covers design.md §1.3 Scenario 5:
/// - `erase_source("bucket-a")` continues to redact EVERY receipt bound to `"bucket-a"`
///   (all committed nodes/edges AND refused multi-source receipts whose revisions never landed
///   in `_fathomdb_artifact_revisions`), while leaving unrelated `"bucket-b"` receipts intact.
#[test]
fn erase_source_still_redacts_entire_shared_bucket_including_refused() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "erase-source-shared-bucket");
    let bucket_a = "bucket-a";
    let bucket_b = "bucket-b";

    let s1_rev = "secret-erase-s1-rev-1010abcd";
    let s1_body = "secret-erase-s1-body-2020ef01";
    let s2_rev = "secret-erase-s2-rev-3030abcd";
    let s2_body = "secret-erase-s2-body-4040ef01";
    let refused_rev = "secret-erase-refused-rev-5050abcd";
    let refused_body_a = "secret-erase-refused-body-a-6060ef01";
    let refused_body_b = "secret-erase-refused-body-b-7070ef01";

    let op_a1 = ActuationBatchV1::new(
        "op-a1",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            s1_rev, "ver-a1", "node-a1", bucket_a, s1_body,
        ))],
    )
    .unwrap();
    let op_a2 = ActuationBatchV1::new(
        "op-a2",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            s2_rev, "ver-a2", "node-a2", bucket_a, s2_body,
        ))],
    )
    .unwrap();
    // Refused multi-source receipt touching bucket-a (duplicate artifact_revision_id causes refusal
    // before any row lands in _fathomdb_artifact_revisions).
    let op_refused_a = ActuationBatchV1::new(
        "op-refused-multi-source",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical_node(
                refused_rev,
                "ver-ref-1",
                "node-ref-1",
                bucket_a,
                refused_body_a,
            )),
            ActuationOperationV1::PutCanonicalNode(canonical_node(
                refused_rev,
                "ver-ref-2",
                "node-ref-2",
                "bucket-other-refused",
                refused_body_b,
            )),
        ],
    )
    .unwrap();
    let op_b1 = ActuationBatchV1::new(
        "op-b1",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "rev-b1",
            "ver-b1",
            "node-b1",
            bucket_b,
            "surviving bucket-b body",
        ))],
    )
    .unwrap();

    let receipt_b1 = {
        let opened = Engine::open(&path).expect("open");
        assert_eq!(
            opened.engine.actuate(op_a1.clone()).unwrap().outcome,
            ActuationOutcomeV1::Committed
        );
        assert_eq!(
            opened.engine.actuate(op_a2.clone()).unwrap().outcome,
            ActuationOutcomeV1::Committed
        );
        assert_eq!(
            opened.engine.actuate(op_refused_a.clone()).unwrap().outcome,
            ActuationOutcomeV1::Refused
        );
        let r_b1 = opened.engine.actuate(op_b1.clone()).expect("commit b1");
        assert_eq!(r_b1.outcome, ActuationOutcomeV1::Committed);

        opened.engine.erase_source(bucket_a).expect("erase_source bucket-a");

        assert_operation_erased(&opened.engine, op_a1.clone());
        assert_operation_erased(&opened.engine, op_a2.clone());
        assert_operation_erased(&opened.engine, op_refused_a.clone());
        assert_eq!(
            opened.engine.actuate(op_b1.clone()).expect("bucket-b receipt must survive"),
            r_b1
        );
        assert_data_plane_integrity_clean(&opened.engine);
        opened.engine.close().expect("close");
        r_b1
    };

    assert_database_and_wal_do_not_contain(
        &path,
        &[s1_rev, s1_body, s2_rev, s2_body, refused_rev, refused_body_a, refused_body_b],
    );

    {
        let reopened = Engine::open(&path).expect("reopen");
        assert_operation_erased(&reopened.engine, op_a1);
        assert_operation_erased(&reopened.engine, op_a2);
        assert_operation_erased(&reopened.engine, op_refused_a);
        assert_eq!(
            reopened.engine.actuate(op_b1).expect("bucket-b receipt survives reopen"),
            receipt_b1
        );
        assert_data_plane_integrity_clean(&reopened.engine);
        reopened.engine.close().expect("close reopened");
    }
}
