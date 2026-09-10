#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Once};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyDerivedLookupV1, DependencySourceLookupV1, Engine,
    EngineError, FrozenReadErrorReason, InitialState, LifecycleState, PreparedWrite,
    ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::{migrate_with_steps, Migration, MIGRATIONS, SCHEMA_VERSION, SQLITE_SUFFIX};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SCHEMA_26_MIGRATIONS: &[Migration] = {
    let (head, _) = MIGRATIONS.split_at(26);
    head
};

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice75-schema26", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0_f32; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        if vector.iter().all(|value| *value == 0.0) {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn node(logical_id: &str, body: &str, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice75-schema26").expect("source id"),
        logical_id: Some(logical_id.into()),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn register_sqlite_vec_once() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        let entrypoint: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(entrypoint));
    });
}

fn seed_schema26(path: &std::path::Path) -> Vec<(i64, String, String, String)> {
    register_sqlite_vec_once();
    let mut connection = Connection::open(path).expect("create schema-26 fixture");
    let report = migrate_with_steps(&connection, SCHEMA_26_MIGRATIONS)
        .expect("migrate fixture through schema 26");
    assert_eq!(report.schema_version_after, 26);
    let transaction = connection.transaction().expect("fixture transaction");
    transaction
        .execute_batch(
            "INSERT INTO canonical_nodes(
                 write_cursor,kind,body,source_id,logical_id,row_kind,state
             ) VALUES
                 (1,'doc','schema26needle active','slice75-schema26','legacy-active','leaf','active'),
                 (2,'doc','schema26needle pending','slice75-schema26','legacy-pending','leaf','pending');
             INSERT INTO canonical_edges(
                 write_cursor,kind,from_id,to_id,source_id,logical_id,body,confidence,extractor_model_id
             ) VALUES(
                 3,'related','legacy-active','legacy-pending','slice75-schema26','legacy-edge',
                 'schema26needle relationship',0.75,'schema26-fixture'
             );
             INSERT INTO search_index(rowid,body,kind,write_cursor) VALUES
                 (1,'schema26needle active','doc',1),
                 (2,'schema26needle pending','doc',2);
             INSERT INTO search_index_v2(rowid,kind,body,status,write_cursor) VALUES
                 (1,'doc','schema26needle active','',1),
                 (2,'doc','schema26needle pending','',2);
             INSERT INTO search_index_edges(rowid,body,kind,write_cursor)
                 VALUES(3,'schema26needle relationship','related',3);
             INSERT INTO _fathomdb_projection_terminal(write_cursor,state) VALUES
                 (1,'up_to_date'),(2,'up_to_date'),(3,'up_to_date');
             INSERT INTO _fathomdb_projection_state(kind,last_enqueued_cursor,updated_at)
                 VALUES('doc',3,0);",
        )
        .expect("populate schema-26 fixture and projections");
    transaction.commit().expect("commit schema-26 fixture");
    drop(connection);
    snapshot(path)
}

fn snapshot(path: &std::path::Path) -> Vec<(i64, String, String, String)> {
    let connection = Connection::open(path).expect("open fixture for census");
    let version: u32 =
        connection.query_row("PRAGMA user_version", [], |row| row.get(0)).expect("schema version");
    assert!(version == 26 || version == SCHEMA_VERSION);
    let mut statement = connection
        .prepare(
            "SELECT write_cursor, 'node', body, state FROM canonical_nodes
             UNION ALL
             SELECT write_cursor, 'edge', COALESCE(body, ''), '' FROM canonical_edges
             ORDER BY write_cursor",
        )
        .expect("prepare census");
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .expect("query census")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect census")
}

fn open_current(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).expect("open current schema")
}

fn body_hash(body: &str) -> CanonicalHash {
    let digest = Sha256::digest(body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    CanonicalHash::sha256(digest).expect("canonical hash")
}

fn assert_legacy_projection_state(engine: &Engine) {
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT COUNT(*) FROM _fathomdb_projection_terminal \
                 WHERE write_cursor IN (1,2,3) AND state='up_to_date'",
            )
            .expect("legacy projection terminal census"),
        vec![3]
    );
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT last_enqueued_cursor FROM _fathomdb_projection_state WHERE kind='doc'",
            )
            .expect("legacy projection state"),
        vec![3]
    );
}

#[test]
fn populated_schema26_upgrades_through_every_step_then_reopens_and_projects() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "content");
    let before = seed_schema26(&path);
    assert_eq!(before.len(), 3);

    let upgraded = open_current(&path);
    assert_eq!(upgraded.report.schema_version_before, 26);
    assert_eq!(upgraded.report.schema_version_after, SCHEMA_VERSION);
    assert_eq!(
        upgraded.report.migration_steps.iter().map(|step| step.step_id).collect::<Vec<_>>(),
        (27..=SCHEMA_VERSION).collect::<Vec<_>>()
    );
    assert_legacy_projection_state(&upgraded.engine);
    println!(
        "SLICE75_SCHEMA26_RESULT before=26 after={SCHEMA_VERSION} steps=27,28,29,30,31,32,33 reopen_steps=0"
    );
    let visible = upgraded.engine.search("schema26needle").expect("upgraded search");
    assert!(visible.results.iter().any(|hit| hit.body == "schema26needle active"));
    assert!(visible.results.iter().all(|hit| hit.body != "schema26needle pending"));
    upgraded.engine.close().expect("close upgraded fixture");
    assert_eq!(snapshot(&path), before, "shape-only migrations changed prior-release rows");

    let reopened = open_current(&path);
    assert_eq!(reopened.report.schema_version_before, SCHEMA_VERSION);
    assert_eq!(reopened.report.schema_version_after, SCHEMA_VERSION);
    assert!(reopened.report.migration_steps.is_empty());
    assert_legacy_projection_state(&reopened.engine);
    reopened.engine.configure_vector_kind_for_test("doc").expect("configure projection");
    reopened
        .engine
        .write(&[node("current-vector", "schema26needle projected", InitialState::Active)])
        .expect("post-upgrade projected write");
    reopened.engine.drain(10_000).expect("projection drain");
    assert_eq!(reopened.engine.vector_row_count_for_test().expect("vector census"), 1);
    assert_eq!(
        reopened
            .engine
            .search_with_limit("schema26needle", 10)
            .expect("post-upgrade search")
            .results
            .len(),
        3
    );
}

#[test]
fn schema26_upgrade_supports_lifecycle_dependency_erasure_recreation_and_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "lifecycle");
    seed_schema26(&path);
    let upgraded = open_current(&path);
    assert_legacy_projection_state(&upgraded.engine);
    upgraded.engine.configure_vector_kind_for_test("doc").expect("configure vector projection");
    upgraded.engine.rebuild_projections().expect("rebuild legacy projections");
    upgraded.engine.drain(10_000).expect("legacy projection drain");
    assert_eq!(
        upgraded.engine.projection_status_for_test("doc").unwrap(),
        ProjectionStatus::UpToDate
    );
    assert_eq!(upgraded.engine.vector_row_count_for_test().expect("legacy vector census"), 3);
    assert!(upgraded.engine.has_vector_row_for_cursor_for_test(1).unwrap());
    assert!(upgraded.engine.has_vector_row_for_cursor_for_test(2).unwrap());
    upgraded
        .engine
        .transition("legacy-pending", LifecycleState::Active, None)
        .expect("promote legacy pending row");
    upgraded.engine.drain(10_000).expect("promoted projection drain");
    assert_eq!(upgraded.engine.vector_row_count_for_test().expect("promoted vector census"), 3);
    assert!(upgraded
        .engine
        .search("pending")
        .expect("promoted search")
        .results
        .iter()
        .any(|hit| hit.body == "schema26needle pending"));

    let source_body = "upgraded dependency source";
    upgraded
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: source_body.into(),
                source_id: SourceId::new("slice75-upgraded-source").expect("source id"),
                logical_id: Some("upgraded-source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("upgraded-source-r1").expect("revision"),
                    SourceVersionId::new("v1").expect("version"),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: "upgraded dependency derived".into(),
                source_id: SourceId::new("slice75-upgraded-source").expect("source id"),
                logical_id: Some("upgraded-derived".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("upgraded-derived-r1").expect("revision"),
                    SourceVersionId::new("v1").expect("version"),
                    SourceRevisionId::new("upgraded-source-r1").expect("source revision"),
                    SourceLocator::whole_body(),
                    body_hash(source_body),
                ),
            }),
        ])
        .expect("write post-upgrade provenance");
    upgraded
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "upgraded-dependency",
                "upgraded-source-r1",
                "upgraded-derived-r1",
            )
            .expect("dependency registration"),
        )
        .expect("register dependency");
    upgraded.engine.drain(10_000).expect("dependency projection drain");
    assert_eq!(
        upgraded.engine.projection_status_for_test("doc").unwrap(),
        ProjectionStatus::UpToDate
    );
    assert_eq!(upgraded.engine.vector_row_count_for_test().expect("dependency vector census"), 5);
    let frozen = upgraded
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    upgraded.engine.erase_source("slice75-upgraded-source").expect("erase upgraded source");
    upgraded.engine.drain(10_000).expect("erasure projection drain");
    let post_erasure = upgraded.engine.search("upgraded dependency").expect("post-erasure search");
    assert!(post_erasure.results.iter().all(|hit| {
        hit.body != "upgraded dependency source" && hit.body != "upgraded dependency derived"
    }));
    assert!(upgraded
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("upgraded-source-r1").unwrap())
        .expect("source dependencies after erasure")
        .items
        .is_empty());
    assert!(upgraded
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("upgraded-derived-r1").unwrap())
        .expect("derived dependency after erasure")
        .is_none());
    assert_eq!(upgraded.engine.vector_row_count_for_test().expect("post-erasure vectors"), 3);
    assert!(matches!(
        upgraded.engine.search_frozen(
            "upgraded dependency",
            &frozen,
            0,
            false,
            0.3,
            0,
            false,
            10,
        ),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    upgraded
        .engine
        .write(&[node("upgraded-recreated", "upgraded dependency recreated", InitialState::Active)])
        .expect("safe recreation");
    upgraded.engine.drain(10_000).expect("recreation projection drain");
    assert_eq!(
        upgraded.engine.projection_status_for_test("doc").unwrap(),
        ProjectionStatus::UpToDate
    );
    assert_eq!(upgraded.engine.vector_row_count_for_test().expect("recreated vectors"), 4);
    upgraded.engine.close().expect("close lifecycle fixture");

    let reopened = open_current(&path);
    assert!(reopened.report.migration_steps.is_empty());
    assert_eq!(
        reopened.engine.projection_status_for_test("doc").unwrap(),
        ProjectionStatus::UpToDate
    );
    assert_eq!(reopened.engine.vector_row_count_for_test().expect("reopened vectors"), 4);
    let results = reopened.engine.search("upgraded dependency").expect("reopen search").results;
    assert_eq!(results.iter().filter(|hit| hit.body == "upgraded dependency recreated").count(), 1);
    assert!(results.iter().all(|hit| {
        hit.body != "upgraded dependency source" && hit.body != "upgraded dependency derived"
    }));
    assert!(reopened
        .engine
        .dependencies_for_source(DependencySourceLookupV1::new("upgraded-source-r1").unwrap())
        .expect("source dependencies after reopen")
        .items
        .is_empty());
    assert!(reopened
        .engine
        .dependency_for_derived(DependencyDerivedLookupV1::new("upgraded-derived-r1").unwrap())
        .expect("derived dependency after reopen")
        .is_none());
}
