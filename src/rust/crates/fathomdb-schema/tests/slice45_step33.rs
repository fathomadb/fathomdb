use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;

#[test]
fn step33_installs_unique_page_indexes_and_state_visibility_triggers() {
    assert_eq!(SCHEMA_VERSION, 33);
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    for name in
        ["canonical_nodes_kind_cursor_page_idx", "operational_state_collection_cursor_page_idx"]
    {
        let unique: i64 = connection
            .query_row(
                "SELECT [unique] FROM pragma_index_list(CASE WHEN ?1 LIKE 'canonical%' THEN 'canonical_nodes' ELSE 'operational_state' END) WHERE name=?1",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unique, 1, "{name}");
    }
    let state_cursor_index: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_index_list('operational_state') \
             WHERE name='operational_state_write_cursor_idx' AND [unique]=0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state_cursor_index, 1, "frozen boundary lookup must not scan latest state");
    let revision_cursor_index: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_index_list('_fathomdb_artifact_revisions') \
             WHERE name='_fathomdb_artifact_revisions_write_cursor_idx' AND [unique]=0",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revision_cursor_index, 1, "page eligibility must not scan all revisions per row");
    for short in ["oc", "os"] {
        for suffix in ["ai", "au", "ad"] {
            let name = format!("_fathomdb_read_visibility_{short}_{suffix}");
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1);
        }
    }
}

#[test]
fn step33_refuses_duplicate_legacy_page_keys_atomically() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
    connection
        .execute(
            "INSERT INTO operational_state(collection_name,record_key,payload_json,schema_id,write_cursor) VALUES('state','a','{}',NULL,7),('state','b','{}',NULL,7)",
            [],
        )
        .unwrap();
    assert!(migrate(&connection).is_err());
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 32);
    let index_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='operational_state_collection_cursor_page_idx'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 0);
}
