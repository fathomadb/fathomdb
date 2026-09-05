use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS};
use rusqlite::Connection;

fn table_exists(connection: &Connection, name: &str) -> bool {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn step32_adds_generation_authority_and_receipt_correlation_without_moving_content() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..31]).unwrap();
    connection
        .execute(
            "INSERT INTO canonical_nodes(write_cursor,kind,body,source_id,row_kind,state) \
             VALUES(7,'doc','preserved','source:step32','leaf','active')",
            [],
        )
        .unwrap();

    migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();

    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 32);
    assert!(table_exists(&connection, "_fathomdb_projection_generations"));
    assert!(table_exists(&connection, "_fathomdb_projection_generation_current"));
    let body: String = connection
        .query_row("SELECT body FROM canonical_nodes WHERE write_cursor=7", [], |row| row.get(0))
        .unwrap();
    assert_eq!(body, "preserved", "step 32 is shape-only");

    let receipt_columns = connection
        .prepare("PRAGMA table_info('_fathomdb_actuation_receipts')")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(receipt_columns.iter().any(|name| name == "projection_generation_id"));

    let trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' \
             AND name LIKE '_fathomdb_read_visibility_pg_%' \
                OR type='trigger' AND name LIKE '_fathomdb_read_visibility_pc_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trigger_count, 6);
}

#[test]
fn generation_history_schema_rejects_multiple_serving_rows_and_deletion() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    let digest = "0".repeat(64);
    connection
        .execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,'pgen1:00000000000000000000000000000001',?1,0,'serving','fresh')",
            [digest.as_str()],
        )
        .unwrap();
    assert!(connection
        .execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,'pgen1:00000000000000000000000000000002',?1,0,'serving','fresh')",
            [digest.as_str()],
        )
        .is_err());
    assert!(connection
        .execute(
            "DELETE FROM _fathomdb_projection_generations \
             WHERE generation_id='pgen1:00000000000000000000000000000001'",
            [],
        )
        .is_err());
}
