//! Slice 20 FIX-2: persisted revision identities use the public grammar.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyDerivedLookupV1, Engine, EngineError,
    InitialState, PreparedWrite, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn seed_and_register(engine: &Engine) {
    let source_body = "source bytes";
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: source_body.into(),
                source_id: SourceId::new("bucket").unwrap(),
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
                body: "derived".into(),
                source_id: SourceId::new("bucket").unwrap(),
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
                    CanonicalHash::sha256(
                        Sha256::digest(source_body.as_bytes())
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>(),
                    )
                    .unwrap(),
                ),
            }),
        ])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
}

fn assert_derived_read_and_replay_are_storage(engine: &Engine) {
    assert!(matches!(
        engine.dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap()),
        Err(EngineError::Storage)
    ));
    assert!(matches!(
        engine.register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap()
        ),
        Err(EngineError::Storage)
    ));
}

/// Post-hoc regression only: FIX-1's RED oracle used `bad source`, which the
/// public source-ID grammar accepts. This corrected corruption already passes.
#[test]
fn post_hoc_consistent_invalid_source_id_fails_closed() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "source-id");
    let opened = Engine::open(&db).unwrap();
    seed_and_register(&opened.engine);
    let raw = Connection::open(&db).unwrap();
    raw.execute("UPDATE canonical_nodes SET source_id='_bad-source'", []).unwrap();
    raw.execute("UPDATE _fathomdb_source_versions SET source_id='_bad-source'", []).unwrap();
    raw.execute("UPDATE _fathomdb_source_links SET source_id='_bad-source'", []).unwrap();
    drop(raw);

    assert_derived_read_and_replay_are_storage(&opened.engine);
}

/// Genuine FIX-2 RED: every reciprocal reference agrees, but the canonical
/// artifact/source revision begins with `_` and is outside the public grammar.
#[test]
fn consistent_invalid_canonical_source_revision_fails_closed() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "source-revision");
    let opened = Engine::open(&db).unwrap();
    seed_and_register(&opened.engine);
    let raw = Connection::open(&db).unwrap();
    raw.execute(
        "UPDATE _fathomdb_artifact_revisions SET revision_id='_bad-rev' \
         WHERE revision_id='source-r1'",
        [],
    )
    .unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_versions SET source_revision_id='_bad-rev' \
         WHERE source_revision_id='source-r1'",
        [],
    )
    .unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_links \
         SET artifact_revision_id='_bad-rev', source_revision_id='_bad-rev' \
         WHERE artifact_revision_id='source-r1'",
        [],
    )
    .unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_links SET source_revision_id='_bad-rev' \
         WHERE artifact_revision_id='derived-r1'",
        [],
    )
    .unwrap();
    drop(raw);

    assert_derived_read_and_replay_are_storage(&opened.engine);
}
