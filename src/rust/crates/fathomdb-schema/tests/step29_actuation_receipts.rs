use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;

fn user_version(connection: &Connection) -> u32 {
    connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap()
}

#[test]
fn step29_adds_only_bounded_terminal_actuation_receipts() {
    assert_eq!(SCHEMA_VERSION, 29);
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..28]).unwrap();

    migrate(&connection).unwrap();
    assert_eq!(user_version(&connection), 29);
    for name in ["_fathomdb_actuation_receipts", "_fathomdb_actuation_receipt_source_refs"] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing {name}");
    }
    let reverse_index: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND \
             name='_fathomdb_actuation_receipt_refs_reverse'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(reverse_index.contains("ref_kind, ref_value, operation_id"));
    let trigger: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND \
             name='_fathomdb_actuation_ref_owner_before_insert'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(trigger.contains("outcome != 'erased'"));

    let domain_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_actuation_receipts", [], |row| row.get(0))
        .unwrap();
    assert_eq!(domain_count, 0, "migration must not synthesize receipts");
}

#[test]
fn step29_receipt_shape_rejects_nonterminal_or_unbounded_rows() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    let digest = "0".repeat(64);
    let insert = |outcome: &str, operations: i64, reasons: &str| {
        connection.execute(
            "INSERT INTO _fathomdb_actuation_receipts(\
               operation_id,schema_version,request_sha256,operations_count,outcome,\
               refused_operation_index,refused_field_path,reason_codes_json,\
               affected_revision_ids_json,resulting_write_boundary,\
               resulting_dependency_generation,pending_projection_write_cursors_json,\
               closure_operation_ids_json\
             ) VALUES(?1,1,?2,?3,?4,NULL,NULL,?5,'[]',0,NULL,'[]','[]')",
            rusqlite::params![
                format!("op-{outcome}-{operations}"),
                digest,
                operations,
                outcome,
                reasons
            ],
        )
    };
    assert!(insert("prepared", 1, "[]").is_err());
    assert!(insert("committed", 129, "[]").is_err());
    assert!(insert("committed", 1, "[\"write_refused\"]").is_err());
}
