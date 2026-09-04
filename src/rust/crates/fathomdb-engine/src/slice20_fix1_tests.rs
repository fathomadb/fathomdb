use super::{
    apply_validated_source_dependency, load_dependency_generation, reserve_dependency_generation,
    store_dependency_generation, validate_source_dependency_registration, ArtifactRevisionId,
    CanonicalHash, DependencyProspectiveState, InitialState, PreparedWrite, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical() -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "same-batch source".into(),
        source_id: SourceId::new("batch-source").unwrap(),
        logical_id: Some("source".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("batch-source-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
        ),
    })
}

fn derived(revision: &str, logical: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived-{revision}"),
        source_id: SourceId::new("batch-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("batch-source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("same-batch source")).unwrap(),
        ),
    })
}

#[test]
fn prospective_validation_and_apply_reuse_one_enclosing_transaction_generation() {
    let connection = Connection::open_in_memory().unwrap();
    fathomdb_schema::migrate(&connection).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();

    let writes = [
        canonical(),
        derived("batch-derived-r1", "derived-1"),
        derived("batch-derived-r2", "derived-2"),
    ];
    let mut prospective =
        DependencyProspectiveState::from_prepared_writes(&connection, &writes).unwrap();
    let first = validate_source_dependency_registration(
        &connection,
        SourceDependencyRegistrationV1::new("batch-dep-1", "batch-source-r1", "batch-derived-r1")
            .unwrap(),
        &prospective,
    )
    .unwrap();
    prospective.record_registration(&first).unwrap();
    let second = validate_source_dependency_registration(
        &connection,
        SourceDependencyRegistrationV1::new("batch-dep-2", "batch-source-r1", "batch-derived-r2")
            .unwrap(),
        &prospective,
    )
    .unwrap();
    prospective.record_registration(&second).unwrap();

    let generation = reserve_dependency_generation(&connection).unwrap();
    apply_validated_source_dependency(&connection, &first, generation).unwrap();
    apply_validated_source_dependency(&connection, &second, generation).unwrap();
    assert_eq!(load_dependency_generation(&connection).unwrap(), 0);
    store_dependency_generation(&connection, generation).unwrap();

    let rows: Vec<(String, i64)> = connection
        .prepare(
            "SELECT dependency_id, registered_dependency_generation \
             FROM _fathomdb_source_dependencies ORDER BY dependency_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(rows, vec![("batch-dep-1".into(), 1), ("batch-dep-2".into(), 1)]);
    connection.execute_batch("ROLLBACK").unwrap();
}
