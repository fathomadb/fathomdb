use fathomdb_schema::{migrate, migrate_with_steps, Migration, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;

fn user_version(connection: &Connection) -> u32 {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

#[test]
fn step28_adds_dependency_shape_generation_and_join_index_without_backfill() {
    assert_eq!(SCHEMA_VERSION, 29);
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..27]).unwrap();
    connection
        .execute(
            "INSERT INTO _fathomdb_artifact_revisions VALUES(1, 'derived-r1', 'node', 1, 'derived_semantic', 'complete')",
            [],
        )
        .unwrap();

    migrate(&connection).unwrap();
    assert_eq!(user_version(&connection), 29);
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0,
        "step 28 must not infer registrations from provenance"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_dependency_generation'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "0"
    );
    let index_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='_fathomdb_source_links_source_derived_idx'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(index_sql.contains("source_revision_id, artifact_revision_id"));

    for sql in [
        "INSERT INTO _fathomdb_source_dependencies VALUES(2, 'dep', 'derived-r1', 1)",
        "INSERT INTO _fathomdb_source_dependencies VALUES(1, 'dep', 'derived-r1', 0)",
    ] {
        assert!(connection.execute(sql, []).is_err());
    }
}

#[test]
fn step28_is_atomic_and_idempotent() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..27]).unwrap();
    let broken = [MIGRATIONS[27], Migration { step_id: 29, sql: "INVALID SQL" }];
    let report = migrate_with_steps(&connection, &broken).unwrap_err();
    assert!(format!("{report}").contains("schema migration failed at step 29"));
    assert_eq!(user_version(&connection), 28);
    migrate(&connection).unwrap();
    assert_eq!(user_version(&connection), 29);
}
