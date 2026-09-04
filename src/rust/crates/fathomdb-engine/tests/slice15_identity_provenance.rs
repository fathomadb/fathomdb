//! 0.8.25 Slice 15 — immutable revision identity and exact source provenance.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState, PreparedWrite,
    ProvenanceErrorReason, ProvenancedEdgeV1, ProvenancedNodeV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn node(
    logical_id: &str,
    body: &str,
    source_id: &str,
    provenance: WriteProvenanceV1,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance,
    })
}

#[test]
fn canonical_and_derived_revisions_persist_exact_utf8_provenance_across_restart() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "identity");
    let source_body = "AéB";
    let source_hash = digest(source_body);

    {
        let opened = Engine::open(&path).unwrap();
        let canonical = node(
            "source-logical",
            source_body,
            "source-1",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
            ),
        );
        let derived = PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            kind: "mentions".into(),
            from: "source-logical".into(),
            to: "entity-logical".into(),
            source_id: SourceId::new("source-1").unwrap(),
            logical_id: Some("derived-logical".into()),
            body: Some("é".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("derived-revision-1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-revision-1").unwrap(),
                SourceLocator::utf8_bytes(1, 3),
                CanonicalHash::sha256(&source_hash).unwrap(),
            ),
        });
        let receipt = opened.engine.write(&[canonical, derived]).unwrap();
        assert_eq!(receipt.row_cursors.len(), 2);
        opened.engine.close().unwrap();
    }

    let connection = Connection::open(&path).unwrap();
    let owners: Vec<(String, String, String)> = connection
        .prepare(
            "SELECT revision_id, artifact_role, completeness \
             FROM _fathomdb_artifact_revisions ORDER BY write_cursor",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        owners,
        vec![
            ("source-revision-1".into(), "canonical_source".into(), "complete".into()),
            ("derived-revision-1".into(), "derived_semantic".into(), "complete".into()),
        ]
    );
    let link: (String, i64, i64, String) = connection
        .query_row(
            "SELECT locator_kind, start_byte, end_byte, hash_digest \
             FROM _fathomdb_source_links WHERE artifact_revision_id='derived-revision-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(link, ("utf8_bytes".into(), 1, 3, source_hash));
}

#[test]
fn invalid_locator_or_hash_rejects_the_whole_batch_with_typed_reason_and_path() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "atomic");
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[node(
            "source",
            "AéB",
            "source-1",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
            ),
        )])
        .unwrap();

    let bad = PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: "mentions".into(),
        from: "source".into(),
        to: "other".into(),
        source_id: SourceId::new("source-1").unwrap(),
        logical_id: Some("bad".into()),
        body: Some("bad".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("bad-derived").unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
            SourceRevisionId::new("source-revision-1").unwrap(),
            SourceLocator::utf8_bytes(2, 3),
            CanonicalHash::sha256(digest("AéB")).unwrap(),
        ),
    });
    let error = opened
        .engine
        .write(&[
            node(
                "would-have-committed",
                "rollback me",
                "source-2",
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rollback-revision").unwrap(),
                    SourceVersionId::new("source-v2").unwrap(),
                ),
            ),
            bad,
        ])
        .unwrap_err();
    match error {
        EngineError::Provenance(error) => {
            assert_eq!(error.reason, ProvenanceErrorReason::LocatorInvalid);
            assert_eq!(error.field_path, "/provenance/sourceLocator");
        }
        other => panic!("wrong error: {other:?}"),
    }
    opened.engine.close().unwrap();

    let connection = Connection::open(&path).unwrap();
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='would-have-committed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn legacy_write_gets_runtime_revision_without_claiming_complete_provenance() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "legacy-runtime");
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "legacy shape".into(),
            source_id: SourceId::new("legacy-source").unwrap(),
            logical_id: Some("legacy-logical".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.close().unwrap();

    let connection = Connection::open(&path).unwrap();
    let (revision_id, role, completeness): (String, String, String) = connection
        .query_row(
            "SELECT revision_id, artifact_role, completeness \
             FROM _fathomdb_artifact_revisions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(revision_id.starts_with("_fdb:r:"));
    assert_eq!(revision_id.len(), "_fdb:r:".len() + 64);
    assert_eq!(role, "legacy");
    assert_eq!(completeness, "migrated_incomplete");
    let links: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_links", [], |row| row.get(0))
        .unwrap();
    assert_eq!(links, 0);
}

proptest! {
    #[test]
    fn caller_revision_and_source_version_ids_round_trip(value in "[A-Za-z0-9][A-Za-z0-9._:-]{0,127}") {
        prop_assume!(!value.starts_with("_fdb:"));
        let revision = ArtifactRevisionId::new(value.clone()).unwrap();
        let source_version = SourceVersionId::new(value.clone()).unwrap();
        prop_assert_eq!(revision.as_str(), value.as_str());
        prop_assert_eq!(source_version.as_str(), value.as_str());
    }
}
