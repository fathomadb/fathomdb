use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState, LifecycleActuationV1,
    LifecycleState, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(revision: &str, logical: &str, body: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("source-bucket").unwrap(),
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

fn derived(revision: &str, logical: &str, body: &str, source_revision: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: body.into(),
        source_id: SourceId::new("source-bucket").unwrap(),
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
            CanonicalHash::sha256(digest("source body")).unwrap(),
        ),
    }
}

#[test]
fn constructors_bound_requests_and_normalize_lifecycle_logical_ids() {
    let empty = ActuationBatchV1::new("operation-1", vec![]).unwrap_err();
    assert_eq!(empty.reason, ActuationErrorReason::OperationCountInvalid);
    assert_eq!(empty.field_path, "/operations");

    let revision = ArtifactRevisionId::new("source-r1").unwrap();
    let bare =
        LifecycleActuationV1::new("subject", revision.clone(), LifecycleState::Deleted, None)
            .unwrap();
    let prefixed =
        LifecycleActuationV1::new("l:subject", revision, LifecycleState::Deleted, None).unwrap();
    assert_eq!(bare, prefixed);
    for invalid in ["", "l:", "h:subject", "p:subject", "bad\u{1e}id"] {
        let err = LifecycleActuationV1::new(
            invalid,
            ArtifactRevisionId::new("source-r1").unwrap(),
            LifecycleState::Deleted,
            None,
        )
        .unwrap_err();
        assert_eq!(err.reason, ActuationErrorReason::LogicalIdInvalid);
        assert_eq!(err.field_path, "/logicalId");
    }
}

#[test]
fn mixed_batch_commits_atomically_and_exact_replay_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "mixed")).unwrap();
    let batch = ActuationBatchV1::new(
        "operation-1",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source", "source body")),
            ActuationOperationV1::PutDerivedNode(derived(
                "derived-r1",
                "derived",
                "derived body",
                "source-r1",
            )),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
            ),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(batch.clone()).unwrap();
    assert_eq!(receipt.schema_version, 1);
    assert_eq!(receipt.operation_id, "operation-1");
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
    assert_eq!(receipt.reason_codes, vec![]);
    assert_eq!(receipt.affected_revision_ids.len(), 2);
    assert_eq!(receipt.resulting_dependency_generation, Some(1));
    assert_eq!(opened.engine.actuate(batch).unwrap(), receipt);

    let conflict = ActuationBatchV1::new(
        "operation-1",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r2", "other", "other body"))],
    )
    .unwrap();
    assert!(matches!(
        opened.engine.actuate(conflict),
        Err(EngineError::Actuation(error))
            if error.reason == ActuationErrorReason::OperationIdConflict
                && error.field_path == "/operationId"
    ));
}

#[test]
fn dependency_protected_source_loss_returns_replayable_terminal_refusal() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "closure")).unwrap();
    let seed = ActuationBatchV1::new(
        "seed",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source", "source body")),
            ActuationOperationV1::PutDerivedNode(derived(
                "derived-r1",
                "derived",
                "derived body",
                "source-r1",
            )),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
            ),
        ],
    )
    .unwrap();
    opened.engine.actuate(seed).unwrap();

    let replacement = ActuationBatchV1::new(
        "replace",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r2", "source", "new body"))],
    )
    .unwrap();
    let receipt = opened.engine.actuate(replacement.clone()).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(receipt.reason_codes[0].as_str(), "dependency_closure_required");
    assert_eq!(receipt.refused_operation_index, Some(0));
    assert_eq!(receipt.refused_field_path.as_deref(), Some("/operations/0"));
    assert_eq!(opened.engine.actuate(replacement).unwrap(), receipt);
}
