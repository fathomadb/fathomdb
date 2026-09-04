//! 0.8.25 Slice 15 — additive identity/provenance registry migration.

use fathomdb_schema::{migrate, migrate_with_steps, Migration, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;
use std::sync::Once;

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

fn steps_through(limit: u32) -> Vec<Migration> {
    MIGRATIONS.iter().filter(|step| step.step_id <= limit).cloned().collect()
}

#[test]
fn step27_adds_closed_versioned_registries_without_backfill() {
    register_sqlite_vec_once();
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &steps_through(26)).unwrap();
    connection
        .execute(
            "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id) \
             VALUES(1, 'doc', 'legacy', 'legacy-source', 'legacy-logical')",
            [],
        )
        .unwrap();

    let report = migrate(&connection).unwrap();

    assert_eq!(SCHEMA_VERSION, 27);
    assert_eq!(report.schema_version_before, 26);
    assert_eq!(report.schema_version_after, 27);
    assert_eq!(report.migration_steps.iter().map(|s| s.step_id).collect::<Vec<_>>(), vec![27]);
    for table in
        ["_fathomdb_artifact_revisions", "_fathomdb_source_versions", "_fathomdb_source_links"]
    {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "step 27 must not backfill {table}");
    }

    assert!(connection
        .execute(
            "INSERT INTO _fathomdb_artifact_revisions(\
               schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
             ) VALUES(2, 'revision-1', 'node', 1, 'canonical_source', 'complete')",
            [],
        )
        .is_err());
    assert!(connection
        .execute(
            "INSERT INTO _fathomdb_source_links(\
               schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
               locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
             ) VALUES(1, 'revision-1', 'source', 'v1', 'revision-1',\
                      'future_locator', NULL, NULL, 'sha256', printf('%064d', 0))",
            [],
        )
        .is_err());
}
