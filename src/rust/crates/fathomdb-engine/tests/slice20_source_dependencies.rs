//! 0.8.25 Slice 20 — caller-named dependency registration over Slice 15 provenance.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, CorruptionKind, DependencyDerivedLookupV1,
    DependencyErrorReason, DependencyId, DependencySourceLookupV1, Engine, EngineError,
    InitialState, PreparedWrite, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn canonical(revision: &str, logical: &str, source: &str, version: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("source {revision}"),
        source_id: SourceId::new(source).unwrap(),
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
    source: &str,
    version: &str,
    source_revision: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived {revision}"),
        source_id: SourceId::new(source).unwrap(),
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
            CanonicalHash::sha256(digest(&format!("source {source_revision}"))).unwrap(),
        ),
    })
}

fn request(dep: &str, source: &str, derived: &str) -> SourceDependencyRegistrationV1 {
    SourceDependencyRegistrationV1::new(dep, source, derived).unwrap()
}

fn generation(db: &std::path::Path) -> String {
    Connection::open(db)
        .unwrap()
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_dependency_generation'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn registration_replay_and_reciprocal_reads_are_pinned_and_cursor_independent() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "core");
    let opened = Engine::open(&db).unwrap();
    let receipt = opened
        .engine
        .write(&[
            canonical("source-r1", "source", "bucket", "v1"),
            derived("derived-r1", "derived", "bucket", "v1", "source-r1"),
        ])
        .unwrap();
    let boundary = receipt.cursor;

    let registered = opened
        .engine
        .register_source_dependency(request("dep-1", "source-r1", "derived-r1"))
        .unwrap();
    assert_eq!(registered.schema_version, 1);
    assert_eq!(registered.dependency_id.as_str(), "dep-1");
    assert_eq!(registered.source_revision_id.as_str(), "source-r1");
    assert_eq!(registered.derived_revision_id.as_str(), "derived-r1");
    assert_eq!(registered.registered_dependency_generation, 1);
    assert_eq!(generation(&db), "1");

    assert_eq!(
        opened
            .engine
            .register_source_dependency(request("dep-1", "source-r1", "derived-r1"))
            .unwrap(),
        registered,
        "exact replay returns the stored registration"
    );
    assert_eq!(generation(&db), "1", "replay must not advance generation");
    let source_list = opened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("source-r1").unwrap())
        .unwrap();
    assert_eq!(source_list.schema_version, 1);
    assert_eq!(source_list.items, vec![registered.clone()]);
    assert_eq!(
        opened
            .engine
            .dependency_for_derived(DependencyDerivedLookupV1::new("derived-r1").unwrap())
            .unwrap(),
        Some(registered)
    );
    assert_eq!(opened.engine.write(&[]).unwrap().cursor, boundary);
}

#[test]
fn validation_roles_conflicts_and_mismatch_are_typed_and_atomic() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "validation");
    let opened = Engine::open(&db).unwrap();
    opened
        .engine
        .write(&[
            canonical("source-r1", "source", "bucket", "v1"),
            canonical("source-r2", "source-2", "bucket-2", "v2"),
            derived("derived-r1", "derived", "bucket", "v1", "source-r1"),
        ])
        .unwrap();

    let cases = [
        (
            request("missing", "source-r1", "missing-derived"),
            DependencyErrorReason::DependencyReferenceMissing,
        ),
        (
            request("wrong-source-role", "derived-r1", "source-r1"),
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ),
        (
            request("wrong-derived-role", "source-r1", "source-r2"),
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ),
        (
            request("self", "source-r1", "source-r1"),
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ),
        (
            request("mismatch", "source-r2", "derived-r1"),
            DependencyErrorReason::DependencyProvenanceMismatch,
        ),
    ];
    for (request, reason) in cases {
        let before = generation(&db);
        let error = opened.engine.register_source_dependency(request).unwrap_err();
        assert!(matches!(error, EngineError::Dependency(ref e) if e.reason == reason));
        assert_eq!(generation(&db), before);
    }

    opened.engine.register_source_dependency(request("dep-1", "source-r1", "derived-r1")).unwrap();
    for conflicting in
        [request("dep-2", "source-r1", "derived-r1"), request("dep-1", "source-r2", "derived-r1")]
    {
        let error = opened.engine.register_source_dependency(conflicting).unwrap_err();
        assert!(matches!(
            error,
            EngineError::Dependency(ref e)
                if e.reason == DependencyErrorReason::DependencyConflict && e.field_path.is_empty()
        ));
    }
    assert_eq!(generation(&db), "1");
}

#[test]
fn lookups_are_ordered_bounded_and_do_not_disclose_absent_revisions() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "bounds");
    let opened = Engine::open(&db).unwrap();
    let mut writes = vec![canonical("source-r1", "source", "bucket", "v1")];
    for i in 0..101 {
        writes.push(derived(
            &format!("derived-{i:03}"),
            &format!("derived-l-{i:03}"),
            "bucket",
            "v1",
            "source-r1",
        ));
    }
    opened.engine.write(&writes).unwrap();
    for i in (0..101).rev() {
        opened
            .engine
            .register_source_dependency(request(
                &format!("dep-{i:03}"),
                "source-r1",
                &format!("derived-{i:03}"),
            ))
            .unwrap();
    }
    let error = opened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("source-r1").unwrap())
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Dependency(ref e)
            if e.reason == DependencyErrorReason::DependencyLookupBoundExceeded
                && e.field_path.is_empty()
    ));
    assert!(opened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("absent-source").unwrap())
        .unwrap()
        .items
        .is_empty());
    assert!(opened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("absent-derived").unwrap())
        .unwrap()
        .is_none());
}

#[test]
fn persisted_generation_corruption_fails_open_without_fallback() {
    for (name, value) in [
        ("missing", None),
        ("negative", Some("-1")),
        ("padded", Some("01")),
        ("overflow", Some("9223372036854775808")),
        ("malformed", Some("x")),
    ] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, name);
        let opened = Engine::open(&db).unwrap();
        opened.engine.close().unwrap();
        let connection = Connection::open(&db).unwrap();
        match value {
            Some(value) => connection
                .execute(
                    "UPDATE _fathomdb_open_state SET value=?1 WHERE key='_fathomdb_dependency_generation'",
                    [value],
                )
                .unwrap(),
            None => connection
                .execute(
                    "DELETE FROM _fathomdb_open_state WHERE key='_fathomdb_dependency_generation'",
                    [],
                )
                .unwrap(),
        };
        drop(connection);
        let error = Engine::open(&db).unwrap_err();
        assert!(matches!(
            error,
            fathomdb_engine::EngineOpenError::Corruption(ref detail)
                if detail.kind == CorruptionKind::SchemaInconsistent
        ));
    }
}

#[test]
fn every_raw_provenance_chain_break_fails_reads_and_exact_replay_closed() {
    let corruptions = [
        "DELETE FROM _fathomdb_artifact_revisions WHERE revision_id='derived-r1'",
        "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' WHERE revision_id='derived-r1'",
        "UPDATE _fathomdb_artifact_revisions SET artifact_role='legacy' WHERE revision_id='derived-r1'",
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='derived-r1'",
        "UPDATE _fathomdb_source_links SET source_revision_id='source-r2' WHERE artifact_revision_id='derived-r1'",
        "DELETE FROM _fathomdb_artifact_revisions WHERE revision_id='source-r1'",
        "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' WHERE revision_id='source-r1'",
        "DELETE FROM canonical_nodes WHERE write_cursor=(SELECT write_cursor FROM _fathomdb_artifact_revisions WHERE revision_id='source-r1')",
        "DELETE FROM _fathomdb_source_versions WHERE source_revision_id='source-r1'",
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='source-r1'",
    ];
    for (index, corruption) in corruptions.iter().enumerate() {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, &format!("corrupt-{index}"));
        let opened = Engine::open(&db).unwrap();
        opened
            .engine
            .write(&[
                canonical("source-r1", "source", "bucket", "v1"),
                canonical("source-r2", "source2", "bucket2", "v2"),
                derived("derived-r1", "derived", "bucket", "v1", "source-r1"),
            ])
            .unwrap();
        opened
            .engine
            .register_source_dependency(request("dep", "source-r1", "derived-r1"))
            .unwrap();
        let raw = Connection::open(&db).unwrap();
        raw.pragma_update(None, "ignore_check_constraints", "ON").unwrap();
        raw.execute(corruption, []).unwrap();
        drop(raw);
        for result in [
            opened
                .engine
                .register_source_dependency(request("dep", "source-r1", "derived-r1"))
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
            assert!(matches!(result, Err(EngineError::Storage)), "corruption {index} was accepted");
        }
    }
}

proptest! {
    #[test]
    fn dependency_id_uses_the_closed_caller_grammar(
        valid in "[A-Za-z0-9][A-Za-z0-9._:-]{0,40}",
    ) {
        let id = DependencyId::new(valid.clone()).unwrap();
        prop_assert_eq!(id.as_str(), valid);
    }
}
