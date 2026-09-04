//! 0.8.25 Slice 15 — immutable revision identity and exact source provenance.

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState, LifecycleState,
    PreparedWrite, ProjectionRole, ProjectionSpec, ProjectionVector, ProvenanceErrorReason,
    ProvenancedEdgeV1, ProvenancedNodeV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice15-fix1", "1", 384)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 384];
        vector[0] = 1.0;
        Ok(vector)
    }
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

fn registry_snapshot(path: &std::path::Path) -> (i64, i64, i64, i64, String, String, i64) {
    let connection = Connection::open(path).unwrap();
    (
        connection.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM _fathomdb_artifact_revisions", [], |r| r.get(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM _fathomdb_source_versions", [], |r| r.get(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM _fathomdb_source_links", [], |r| r.get(0))
            .unwrap(),
        connection
            .query_row(
                "SELECT COALESCE(group_concat(kind, ','), '') FROM \
                 (SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind)",
                [],
                |r| r.get(0),
            )
            .unwrap(),
        connection
            .query_row(
                "SELECT COALESCE(group_concat(write_cursor || ':' || state, ','), '') FROM \
                 (SELECT write_cursor, state FROM _fathomdb_projection_terminal \
                  ORDER BY write_cursor)",
                [],
                |r| r.get(0),
            )
            .unwrap(),
        connection
            .query_row(
                "SELECT COUNT(*) FROM canonical_nodes n \
                 JOIN _fathomdb_vector_kinds k ON k.kind = n.kind \
                 LEFT JOIN _fathomdb_projection_terminal t ON t.write_cursor = n.write_cursor \
                 WHERE t.write_cursor IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap(),
    )
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
fn derived_source_reference_hash_and_locator_failures_use_closed_reason_paths() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "derived-validation-matrix");
    let source_body = "AéB";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[node(
            "source",
            source_body,
            "source-1",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-matrix").unwrap(),
                SourceVersionId::new("source-version-matrix").unwrap(),
            ),
        )])
        .unwrap();

    let cases = [
        (
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("missing-source-case").unwrap(),
                SourceVersionId::new("source-version-matrix").unwrap(),
                SourceRevisionId::new("missing-source-revision").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(source_body)).unwrap(),
            ),
            ProvenanceErrorReason::SourceRevisionMissing,
            "/provenance/sourceRevisionId",
        ),
        (
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("source-mismatch-case").unwrap(),
                SourceVersionId::new("different-version").unwrap(),
                SourceRevisionId::new("source-revision-matrix").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(source_body)).unwrap(),
            ),
            ProvenanceErrorReason::SourceMismatch,
            "/provenance/sourceRevisionId",
        ),
        (
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("hash-mismatch-case").unwrap(),
                SourceVersionId::new("source-version-matrix").unwrap(),
                SourceRevisionId::new("source-revision-matrix").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256("0".repeat(64)).unwrap(),
            ),
            ProvenanceErrorReason::HashMismatch,
            "/provenance/canonicalSourceHash",
        ),
        (
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("unicode-boundary-case").unwrap(),
                SourceVersionId::new("source-version-matrix").unwrap(),
                SourceRevisionId::new("source-revision-matrix").unwrap(),
                SourceLocator::utf8_bytes(2, 3),
                CanonicalHash::sha256(digest(source_body)).unwrap(),
            ),
            ProvenanceErrorReason::LocatorInvalid,
            "/provenance/sourceLocator",
        ),
        (
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new("signed-limit-case").unwrap(),
                SourceVersionId::new("source-version-matrix").unwrap(),
                SourceRevisionId::new("source-revision-matrix").unwrap(),
                SourceLocator::utf8_bytes(0, i64::MAX as u64),
                CanonicalHash::sha256(digest(source_body)).unwrap(),
            ),
            ProvenanceErrorReason::LocatorInvalid,
            "/provenance/sourceLocator",
        ),
    ];

    for (index, (provenance, reason, path)) in cases.into_iter().enumerate() {
        let error = opened
            .engine
            .write(&[node(&format!("derived-{index}"), "derived", "source-1", provenance)])
            .unwrap_err();
        assert!(matches!(
            error,
            EngineError::Provenance(ref error)
                if error.reason == reason && error.field_path == path
        ));
    }
    opened.engine.close().unwrap();
    assert_eq!(registry_snapshot(&path).0, 1, "invalid derived cases must not commit rows");
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

#[test]
fn failing_provenance_write_cannot_commit_late_vector_enrolment_or_queue_changes() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "atomic-enrolment");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let source_body = r#"{"summary":"canonical source"}"#;
    opened
        .engine
        .write(&[node(
            "source",
            source_body,
            "source-1",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-atomic").unwrap(),
                SourceVersionId::new("source-v-atomic").unwrap(),
            ),
        )])
        .unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.drain(5_000).unwrap();
    let before = registry_snapshot(&path);

    let failing = PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "note".into(),
        body: r#"{"summary":"derived"}"#.into(),
        source_id: SourceId::new("source-1").unwrap(),
        logical_id: Some("derived-atomic".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("derived-revision-atomic").unwrap(),
            SourceVersionId::new("source-v-atomic").unwrap(),
            SourceRevisionId::new("source-revision-atomic").unwrap(),
            SourceLocator::utf8_bytes(0, 10_000),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
    });
    let error = opened.engine.write(&[failing]).unwrap_err();
    assert!(matches!(
        error,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::LocatorInvalid
                && error.field_path == "/provenance/sourceLocator"
    ));

    assert_eq!(
        registry_snapshot(&path),
        before,
        "a refused provenance write must leave canonical rows, provenance registries, \
         vector-kind enrollment, projection terminals, and pending queue state unchanged"
    );
}

#[test]
fn collisions_are_global_while_source_versions_are_scoped_by_source_id() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "identity-scope");
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[node(
            "source-a",
            "A",
            "source-a",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("revision-a1").unwrap(),
                SourceVersionId::new("version-1").unwrap(),
            ),
        )])
        .unwrap();

    let collision = opened
        .engine
        .write(&[node(
            "source-a-replay",
            "A",
            "source-a",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("revision-a1").unwrap(),
                SourceVersionId::new("version-2").unwrap(),
            ),
        )])
        .unwrap_err();
    assert!(matches!(
        collision,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::RevisionIdConflict
                && error.field_path.is_empty()
    ));

    let version_conflict = opened
        .engine
        .write(&[node(
            "source-a-conflict",
            "different",
            "source-a",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("revision-a2").unwrap(),
                SourceVersionId::new("version-1").unwrap(),
            ),
        )])
        .unwrap_err();
    assert!(matches!(
        version_conflict,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::SourceVersionConflict
                && error.field_path == "/provenance/sourceVersionId"
    ));

    opened
        .engine
        .write(&[node(
            "source-b",
            "B",
            "source-b",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("revision-b1").unwrap(),
                SourceVersionId::new("version-1").unwrap(),
            ),
        )])
        .unwrap();
}

#[test]
fn runtime_edge_revision_distinguishes_null_from_empty_body_at_the_same_cursor() {
    fn first_revision(body: Option<&str>) -> String {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir, "edge-digest");
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .write(&[PreparedWrite::Edge {
                kind: "rel".into(),
                from: "a".into(),
                to: "b".into(),
                source_id: SourceId::new("source").unwrap(),
                logical_id: Some("edge".into()),
                body: body.map(str::to_string),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            }])
            .unwrap();
        opened.engine.close().unwrap();
        Connection::open(path)
            .unwrap()
            .query_row("SELECT revision_id FROM _fathomdb_artifact_revisions", [], |row| row.get(0))
            .unwrap()
    }

    assert_ne!(first_revision(None), first_revision(Some("")));
}

#[test]
fn referenced_canonical_purge_refuses_then_source_erasure_leaves_no_registry_orphans() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "erasure");
    let opened = Engine::open(&path).unwrap();
    let source_body = "source bytes";
    opened
        .engine
        .write(&[
            node(
                "source-logical",
                source_body,
                "source-bucket",
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-revision-erasure").unwrap(),
                    SourceVersionId::new("source-version-erasure").unwrap(),
                ),
            ),
            node(
                "derived-logical",
                "derived",
                "source-bucket",
                WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-revision-erasure").unwrap(),
                    SourceVersionId::new("source-version-erasure").unwrap(),
                    SourceRevisionId::new("source-revision-erasure").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(source_body)).unwrap(),
                ),
            ),
        ])
        .unwrap();
    opened.engine.transition("source-logical", LifecycleState::Deleted, None).unwrap();
    let error = opened.engine.purge("source-logical").unwrap_err();
    assert!(matches!(
        error,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::ProvenanceInUse
                && error.field_path.is_empty()
    ));
    assert_eq!(registry_snapshot(&path).0, 2);

    opened.engine.erase_source("source-bucket").unwrap();
    opened.engine.close().unwrap();
    let connection = Connection::open(path).unwrap();
    for table in [
        "canonical_nodes",
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "source erasure left an orphan in {table}");
    }
}

fn insert_raw_source_side_link(
    path: &std::path::Path,
    artifact_revision_id: &str,
    source_id: &str,
    source_version_id: &str,
    source_revision_id: &str,
    hash: &str,
) {
    Connection::open(path)
        .unwrap()
        .execute(
            "INSERT INTO _fathomdb_source_links(\
               schema_version, artifact_revision_id, source_id, source_version_id,\
               source_revision_id, locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
             ) VALUES(1, ?1, ?2, ?3, ?4, 'whole_body', NULL, NULL, 'sha256', ?5)",
            rusqlite::params![
                artifact_revision_id,
                source_id,
                source_version_id,
                source_revision_id,
                hash
            ],
        )
        .unwrap();
}

#[test]
fn purge_refuses_a_raw_source_side_link_without_an_artifact_owner_and_rolls_back() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "purge-raw-source-link");
    let source_body = "source bytes";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[node(
            "source-logical",
            source_body,
            "source-bucket",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-raw-purge").unwrap(),
                SourceVersionId::new("source-version-raw-purge").unwrap(),
            ),
        )])
        .unwrap();
    opened.engine.transition("source-logical", LifecycleState::Deleted, None).unwrap();
    opened.engine.close().unwrap();
    insert_raw_source_side_link(
        &path,
        "orphan-derived-revision",
        "source-bucket",
        "source-version-raw-purge",
        "source-revision-raw-purge",
        &digest(source_body),
    );

    let opened = Engine::open(&path).unwrap();
    let error = opened.engine.purge("source-logical").unwrap_err();
    assert!(matches!(
        error,
        EngineError::Provenance(ref error)
            if error.reason == ProvenanceErrorReason::ProvenanceInUse
                && error.field_path.is_empty()
    ));
    opened.engine.close().unwrap();
    let connection = Connection::open(path).unwrap();
    let state: String = connection
        .query_row(
            "SELECT state FROM canonical_nodes WHERE logical_id='source-logical'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let raw_link: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_links \
             WHERE artifact_revision_id='orphan-derived-revision'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "deleted", "failed purge must roll back the canonical delete");
    assert_eq!(raw_link, 1, "failed purge must not partially delete the raw link");
}

#[test]
fn source_erasure_removes_raw_source_side_links_without_artifact_owners() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "erase-raw-source-link");
    let source_body = "source bytes";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[node(
            "source-logical",
            source_body,
            "source-bucket",
            WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-revision-raw-erase").unwrap(),
                SourceVersionId::new("source-version-raw-erase").unwrap(),
            ),
        )])
        .unwrap();
    opened.engine.close().unwrap();
    insert_raw_source_side_link(
        &path,
        "orphan-derived-revision",
        "source-bucket",
        "source-version-raw-erase",
        "source-revision-raw-erase",
        &digest(source_body),
    );

    let opened = Engine::open(&path).unwrap();
    opened.engine.erase_source("source-bucket").unwrap();
    opened.engine.close().unwrap();
    let connection = Connection::open(path).unwrap();
    for table in [
        "canonical_nodes",
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "source erasure left raw provenance in {table}");
    }
}

#[cfg(feature = "operator")]
#[test]
fn supersession_and_projection_rebuild_preserve_every_revision_owner() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "supersession-rebuild");
    let opened = Engine::open(&path).unwrap();
    for (body, revision, version) in
        [("first", "revision-1", "version-1"), ("second", "revision-2", "version-2")]
    {
        opened
            .engine
            .write(&[node(
                "same-logical",
                body,
                "source",
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(revision).unwrap(),
                    SourceVersionId::new(version).unwrap(),
                ),
            )])
            .unwrap();
    }
    let before: Vec<String> = Connection::open(&path)
        .unwrap()
        .prepare("SELECT revision_id FROM _fathomdb_artifact_revisions ORDER BY revision_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    opened.engine.rebuild_projections().unwrap();
    let after: Vec<String> = Connection::open(&path)
        .unwrap()
        .prepare("SELECT revision_id FROM _fathomdb_artifact_revisions ORDER BY revision_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(before, vec!["revision-1", "revision-2"]);
    assert_eq!(after, before);
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
