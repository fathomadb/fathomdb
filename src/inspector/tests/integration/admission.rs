use crate::{admission::preflight, fixtures::database};
use std::fs;
use std::path::Path;
fn bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap()
}

#[test]
fn current_existing_admission_and_refusals_preserve_product() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let original = bytes(&path);
    assert_eq!(preflight(&path).unwrap().path, path);
    assert_eq!(bytes(&path), original);
    assert_eq!(preflight(&dir.path().join("missing.sqlite")).unwrap_err(), "missing");
    let empty = dir.path().join("empty.sqlite");
    fs::write(&empty, b"").unwrap();
    assert_eq!(preflight(&empty).unwrap_err(), "empty");
    assert!(bytes(&empty).is_empty());
    for suffix in ["-wal", "-journal"] {
        let sidecar = format!("{}{suffix}", path.display());
        fs::write(&sidecar, b"preserve sidecar").unwrap();
        assert_eq!(preflight(&path).unwrap_err(), "initial_sidecar");
        assert_eq!(bytes(&path), original);
        assert_eq!(bytes(Path::new(&sidecar)), b"preserve sidecar");
        fs::write(&sidecar, b"").unwrap();
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("{}.lock", path.display()))
        .unwrap();
    lock.lock().unwrap();
    assert_eq!(preflight(&path).unwrap_err(), "locked");
    drop(lock);
    for version in [33_u32, 35, 37, 38] {
        let target = dir.path().join(format!("schema{version}.sqlite"));
        let mut contents = original.clone();
        contents[60..64].copy_from_slice(&version.to_be_bytes());
        fs::write(&target, &contents).unwrap();
        fs::write(format!("{}.lock", target.display()), b"lock sentinel").unwrap();
        assert_eq!(preflight(&target).unwrap_err(), "unsupported_schema");
        assert_eq!(bytes(&target), contents);
    }
    let no_lock = dir.path().join("no-lock.sqlite");
    fs::write(&no_lock, &original).unwrap();
    assert_eq!(preflight(&no_lock).unwrap_err(), "missing_lock");
    assert_eq!(bytes(&no_lock), original);
}

#[test]
fn schema_39_admission_rejects_missing_schema_39_tables_and_columns() {
    assert_eq!(fathomdb_schema::SCHEMA_VERSION, 39);
    let dir = tempfile::tempdir().unwrap();
    let base = database(dir.path());
    let clone = |name: &str| {
        let target = dir.path().join(format!("{name}.sqlite"));
        fs::copy(&base, &target).unwrap();
        fs::write(format!("{}.lock", target.display()), b"lock sentinel").unwrap();
        target
    };
    for (name, sql) in [
        ("drop-cpv", "DROP TABLE canonical_property_values;"),
        ("drop-commit-ts", "DROP TABLE _fathomdb_commit_timestamps;"),
        ("drop-vec-kinds", "DROP TABLE _fathomdb_vector_kinds;"),
        ("drop-node-props", "ALTER TABLE canonical_nodes DROP COLUMN properties;"),
        (
            "drop-node-scope",
            "DROP INDEX canonical_nodes_scope_source_id_idx;\
             DROP INDEX canonical_nodes_scope_logical_active_idx;\
             DROP INDEX canonical_nodes_scope_kind_idx;\
             DROP INDEX canonical_nodes_scope_state_active_idx;\
             DROP INDEX canonical_nodes_scope_validity_idx;\
             DROP INDEX canonical_nodes_scope_cursor_idx;\
             DROP INDEX canonical_nodes_scope_kind_cursor_page_idx;\
             DROP INDEX canonical_nodes_scope_logical_sup_idx;\
             ALTER TABLE canonical_nodes DROP COLUMN scope;",
        ),
        ("drop-edge-props", "ALTER TABLE canonical_edges DROP COLUMN properties;"),
        (
            "drop-edge-scope",
            "DROP INDEX canonical_edges_scope_source_id_idx;\
             DROP INDEX canonical_edges_scope_logical_active_idx;\
             DROP INDEX canonical_edges_scope_from_id_idx;\
             DROP INDEX canonical_edges_scope_to_id_idx;\
             DROP INDEX canonical_edges_scope_cursor_idx;\
             DROP INDEX canonical_edges_scope_from_active_idx;\
             DROP INDEX canonical_edges_scope_to_active_idx;\
             DROP INDEX canonical_edges_scope_from_sup_kind_idx;\
             DROP INDEX canonical_edges_scope_to_sup_kind_idx;\
             ALTER TABLE canonical_edges DROP COLUMN scope;",
        ),
        (
            "drop-mut-scope",
            "DROP INDEX operational_mutations_scope_collection_id_idx;\
             ALTER TABLE operational_mutations DROP COLUMN scope;",
        ),
        (
            "drop-state-scope",
            "DROP TABLE operational_state;\
             CREATE TABLE operational_state(collection_name TEXT NOT NULL, record_key TEXT NOT NULL, payload_json TEXT NOT NULL, schema_id TEXT, write_cursor INTEGER NOT NULL, PRIMARY KEY(collection_name, record_key));",
        ),
        (
            "drop-ar-scope",
            "DROP TABLE _fathomdb_artifact_revisions;\
             CREATE TABLE _fathomdb_artifact_revisions(schema_version INTEGER NOT NULL, revision_id TEXT PRIMARY KEY NOT NULL, artifact_class TEXT NOT NULL, write_cursor INTEGER NOT NULL, artifact_role TEXT NOT NULL, completeness TEXT NOT NULL);",
        ),
        (
            "drop-proj-class",
            "DROP TABLE _fathomdb_projection_registry;\
             CREATE TABLE _fathomdb_projection_registry(name TEXT PRIMARY KEY NOT NULL, roles TEXT NOT NULL, fts_tokenizer TEXT, vector_embedder TEXT, vector_declared INTEGER NOT NULL DEFAULT 0, source TEXT, vector_kinds TEXT);",
        ),
        ("drop-proj-vkinds", "ALTER TABLE _fathomdb_projection_registry DROP COLUMN vector_kinds;"),
    ] {
        let target = clone(name);
        let c = rusqlite::Connection::open(&target).unwrap();
        c.execute_batch(sql).unwrap();
        drop(c);
        let before = bytes(&target);
        assert_eq!(
            preflight(&target).unwrap_err(),
            "malformed_layout",
            "missing Schema 39 layout element must fail closed: {name}"
        );
        assert_eq!(bytes(&target), before, "refused database bytes must be untouched: {name}");
    }
}
