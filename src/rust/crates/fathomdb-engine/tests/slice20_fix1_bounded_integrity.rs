//! Slice 20 FIX-1: bounded relevant-row integrity and stored identity grammar.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyDerivedLookupV1, DependencyErrorReason,
    DependencySourceLookupV1, Engine, EngineError, InitialState, PreparedWrite, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
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

fn canonical(revision: &str, logical: &str, source_id: &str, version: &str) -> PreparedWrite {
    let body = format!("body-{revision}");
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body,
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
    })
}

fn derived(
    revision: &str,
    logical: &str,
    source_id: &str,
    version: &str,
    source_revision: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived-{revision}"),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(&format!("body-{source_revision}"))).unwrap(),
        ),
    })
}

fn register(engine: &Engine, dependency: &str, source: &str, derived: &str) {
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(dependency, source, derived).unwrap(),
        )
        .unwrap();
}

#[test]
fn relevant_overflow_wins_without_scanning_an_unrelated_corrupt_row() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "bounded-overflow");
    let opened = Engine::open(&db).unwrap();
    let mut writes = vec![
        canonical("source-a-r1", "source-a", "source-a", "v1"),
        canonical("source-b-r1", "source-b", "source-b", "v1"),
        derived("unrelated-r1", "unrelated", "source-b", "v1", "source-b-r1"),
    ];
    for index in 0..101 {
        writes.push(derived(
            &format!("derived-a-{index:03}"),
            &format!("logical-a-{index:03}"),
            "source-a",
            "v1",
            "source-a-r1",
        ));
    }
    opened.engine.write(&writes).unwrap();
    register(&opened.engine, "unrelated-dep", "source-b-r1", "unrelated-r1");
    for index in 0..101 {
        register(
            &opened.engine,
            &format!("dep-a-{index:03}"),
            "source-a-r1",
            &format!("derived-a-{index:03}"),
        );
    }
    let raw = Connection::open(&db).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_dependencies SET dependency_id='bad id' \
         WHERE derived_revision_id='unrelated-r1'",
        [],
    )
    .unwrap();
    drop(raw);

    let error = opened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("source-a-r1").unwrap())
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Dependency(ref error)
            if error.reason == DependencyErrorReason::DependencyLookupBoundExceeded
    ));
}

#[test]
fn bounded_healthy_reads_ignore_unrelated_corruption_but_validate_returned_rows() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "bounded-healthy");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-a-r1", "source-a", "source-a", "v1"),
            derived("derived-a-r1", "derived-a", "source-a", "v1", "source-a-r1"),
            canonical("source-b-r1", "source-b", "source-b", "v1"),
            derived("derived-b-r1", "derived-b", "source-b", "v1", "source-b-r1"),
        ])
        .unwrap();
    register(&opened.engine, "dep-a", "source-a-r1", "derived-a-r1");
    register(&opened.engine, "dep-b", "source-b-r1", "derived-b-r1");
    let raw = Connection::open(&db).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_dependencies SET dependency_id='bad id' \
         WHERE derived_revision_id='derived-b-r1'",
        [],
    )
    .unwrap();
    drop(raw);

    let by_source = opened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("source-a-r1").unwrap())
        .unwrap();
    assert_eq!(by_source.items.len(), 1);
    let by_derived = opened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("derived-a-r1").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(by_derived.dependency_id.as_str(), "dep-a");

    let raw = Connection::open(&db).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_dependencies SET dependency_id='bad target' \
         WHERE derived_revision_id='derived-a-r1'",
        [],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        opened
            .engine
            .dependency_for_derived(DependencyDerivedLookupV1::new("derived-a-r1").unwrap()),
        Err(EngineError::Storage)
    ));
}

#[test]
fn consistently_invalid_stored_source_and_version_ids_fail_closed() {
    for statements in [
        [
            "UPDATE canonical_nodes SET source_id='_bad-source'",
            "UPDATE _fathomdb_source_versions SET source_id='_bad-source'",
            "UPDATE _fathomdb_source_links SET source_id='_bad-source'",
        ],
        [
            "UPDATE _fathomdb_source_versions SET source_version_id='bad version'",
            "UPDATE _fathomdb_source_links SET source_version_id='bad version'",
            "UPDATE _fathomdb_source_links SET schema_version=schema_version",
        ],
    ] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, "stored-grammar");
        let opened = Engine::open(&db).unwrap();
        opened
            .engine
            .write(&[
                canonical("source-r1", "source", "source", "v1"),
                derived("derived-r1", "derived", "source", "v1", "source-r1"),
            ])
            .unwrap();
        register(&opened.engine, "dep", "source-r1", "derived-r1");
        let raw = Connection::open(&db).unwrap();
        for statement in statements {
            raw.execute(statement, []).unwrap();
        }
        drop(raw);

        for result in [
            opened
                .engine
                .register_source_dependency(
                    SourceDependencyRegistrationV1::new("dep", "source-r1", "derived-r1").unwrap(),
                )
                .map(|_| ()),
            opened
                .engine
                .dependencies_for_source(DependencySourceLookupV1::new("source-r1").unwrap())
                .map(|_| ()),
            opened
                .engine
                .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
                .map(|_| ()),
        ] {
            assert!(matches!(result, Err(EngineError::Storage)));
        }
    }
}
