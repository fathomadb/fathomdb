use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState,
    LifecycleActuationV1, LifecycleState, ProvenancedNodeV1, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
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

    let receipt = opened.engine.actuate(request).unwrap();
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

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
    assert_eq!(
        receipt.refused_field_path.as_deref(),
        Some("/operations/0/dependency/derivedRevisionId")
    );
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

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(
        receipt.refused_field_path.as_deref(),
        Some("/operations/0/record/provenance/canonicalSourceHash/digestHex")
    );
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

    assert_eq!(after.errors_by_code.get("FDB_ACTUATION"), Some(&1));
    assert_eq!(after.writes, before.writes);
    assert_eq!(after.write_rows, before.write_rows);
}
