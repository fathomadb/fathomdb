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
    for version in [33_u32, 35, 37] {
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
fn schema_38_admission_rejects_missing_schema_38_tables_and_columns() {
    assert_eq!(fathomdb_schema::SCHEMA_VERSION, 38);
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
        ("drop-node-scope", "ALTER TABLE canonical_nodes DROP COLUMN scope;"),
        ("drop-edge-props", "ALTER TABLE canonical_edges DROP COLUMN properties;"),
        ("drop-edge-scope", "ALTER TABLE canonical_edges DROP COLUMN scope;"),
        ("drop-mut-scope", "ALTER TABLE operational_mutations DROP COLUMN scope;"),
        ("drop-state-scope", "ALTER TABLE operational_state DROP COLUMN scope;"),
        ("drop-proj-class", "ALTER TABLE _fathomdb_projection_registry DROP COLUMN entity_class;"),
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
            "missing Schema 38 layout element must fail closed: {name}"
        );
        assert_eq!(bytes(&target), before, "refused database bytes must be untouched: {name}");
    }
}
