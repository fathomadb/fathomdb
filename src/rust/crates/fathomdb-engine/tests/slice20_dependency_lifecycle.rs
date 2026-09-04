//! Slice 20 dependency generation and hard-erasure integration.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyDerivedLookupV1, DependencyErrorReason, Engine,
    EngineError, InitialState, LifecycleState, PreparedWrite, ProvenanceErrorReason,
    ProvenancedNodeV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write_chain(engine: &Engine, source_id: &str) {
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: "canonical body".into(),
                source_id: SourceId::new(source_id).unwrap(),
                logical_id: Some("source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "fact".into(),
                body: "derived body".into(),
                source_id: SourceId::new(source_id).unwrap(),
                logical_id: Some("derived".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest("canonical body")).unwrap(),
                ),
            }),
        ])
        .unwrap();
    engine
        .register_source_dependency(
            fathomdb_engine::SourceDependencyRegistrationV1::new(
                "dep-1",
                "source-r1",
                "derived-r1",
            )
            .unwrap(),
        )
        .unwrap();
}

fn generation(db: &std::path::Path) -> String {
    Connection::open(db)
        .unwrap()
        .query_row(
            "SELECT value FROM _fathomdb_open_state \
             WHERE key='_fathomdb_dependency_generation'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn purge_of_derived_owner_deletes_registration_and_advances_once() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "purge");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");

    opened.engine.transition("derived", LifecycleState::Deleted, None).unwrap();
    opened.engine.purge("derived").unwrap();

    assert_eq!(generation(&db), "2");
    assert!(opened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn source_erasure_deletes_registration_and_advances_once() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "erase-source");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");

    opened.engine.erase_source("bucket").unwrap();

    assert_eq!(generation(&db), "2");
    let count: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    opened.engine.close().unwrap();
    let reopened = Engine::open(&db).unwrap();
    assert_eq!(generation(&db), "2", "erased highest generation survives restart");
    reopened.engine.close().unwrap();
}

#[test]
fn purge_of_source_with_surviving_dependent_refuses_without_mutation() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "source-in-use");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");

    opened.engine.transition("source", LifecycleState::Deleted, None).unwrap();
    let error = opened.engine.purge("source").unwrap_err();

    assert!(matches!(
        error,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::ProvenanceInUse
    ));
    assert_eq!(generation(&db), "1");
    assert!(opened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
        .unwrap()
        .is_some());
}

#[test]
fn exhausted_generation_refuses_registration_atomically() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "exhausted");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute("DELETE FROM _fathomdb_source_dependencies WHERE dependency_id='dep-1'", [])
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value='9223372036854775807' \
             WHERE key='_fathomdb_dependency_generation'",
            [],
        )
        .unwrap();
    drop(connection);

    let error = opened
        .engine
        .register_source_dependency(
            fathomdb_engine::SourceDependencyRegistrationV1::new(
                "dep-2",
                "source-r1",
                "derived-r1",
            )
            .unwrap(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Dependency(ref error)
            if error.reason == DependencyErrorReason::DependencyGenerationExhausted
                && error.field_path.is_empty()
    ));
    assert_eq!(generation(&db), "9223372036854775807");
    let count: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn raw_dependency_row_corruption_fails_every_read_closed() {
    for statement in [
        "UPDATE _fathomdb_source_dependencies SET schema_version=2",
        "UPDATE _fathomdb_source_dependencies SET dependency_id='_fdb:reserved'",
        "UPDATE _fathomdb_source_dependencies SET registered_dependency_generation=2",
    ] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, "raw-row");
        let opened = Engine::open(&db).unwrap();
        write_chain(&opened.engine, "bucket");
        let connection = Connection::open(&db).unwrap();
        connection.pragma_update(None, "ignore_check_constraints", "ON").unwrap();
        connection.execute(statement, []).unwrap();
        drop(connection);

        assert!(matches!(
            opened
                .engine
                .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap()),
            Err(EngineError::Storage)
        ));
    }
}

#[test]
fn consistently_invalid_source_identity_refuses_purge_with_rollback() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "grammar-purge");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");
    opened.engine.transition("derived", LifecycleState::Deleted, None).unwrap();

    let connection = Connection::open(&db).unwrap();
    connection.execute("UPDATE canonical_nodes SET source_id='bad source'", []).unwrap();
    connection.execute("UPDATE _fathomdb_source_versions SET source_id='bad source'", []).unwrap();
    connection.execute("UPDATE _fathomdb_source_links SET source_id='bad source'", []).unwrap();
    drop(connection);

    assert!(matches!(opened.engine.purge("derived"), Err(EngineError::Storage)));
    assert_eq!(generation(&db), "1");
    let count: i64 = Connection::open(&db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "failed purge must retain dependency membership");
}

#[cfg(feature = "operator")]
#[test]
fn projection_rebuilds_do_not_remint_dependency_identity_or_generation() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "rebuild");
    let opened = Engine::open(&db).unwrap();
    write_chain(&opened.engine, "bucket");
    let before = opened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
        .unwrap();

    opened.engine.rebuild_projections().unwrap();
    opened.engine.rebuild_vec0().unwrap();

    assert_eq!(generation(&db), "1");
    assert_eq!(
        opened
            .engine
            .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
            .unwrap(),
        before
    );
}
