use fathomdb_schema::{migrate, migrate_with_steps, MIGRATIONS, SCHEMA_VERSION};
use rusqlite::{Connection, OptionalExtension};
use std::collections::BTreeSet;

fn generation(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

fn state_nonce(connection: &Connection) -> String {
    connection
        .query_row(
            "SELECT state_nonce FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

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

#[test]
fn step33_has_exact_trigger_manifest_and_one_generation_cutover() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
    let before = generation(&connection);
    migrate(&connection).unwrap();
    assert_eq!(generation(&connection), before + 1);

    let expected = ["oc", "os"]
        .into_iter()
        .flat_map(|table| {
            ["ai", "au", "ad"]
                .into_iter()
                .map(move |event| format!("_fathomdb_read_visibility_{table}_{event}"))
        })
        .collect::<BTreeSet<_>>();
    let actual = connection
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='trigger' \
             AND (name LIKE '_fathomdb_read_visibility_oc_%' \
                  OR name LIKE '_fathomdb_read_visibility_os_%') ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<BTreeSet<_>, _>>()
        .unwrap();
    assert_eq!(actual, expected);
    let cutover_guard: Option<String> = connection
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='trigger' \
             AND name='_fathomdb_read_visibility_step33_cutover_guard'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert!(cutover_guard.is_none(), "temporary cutover guard must not persist");
}

#[test]
fn step33_mints_branch_sensitive_state_for_every_terminal_mutation() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    let mut seen = BTreeSet::from([state_nonce(&connection)]);
    for sql in [
        "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) \
         VALUES(1,'up_to_date')",
        "UPDATE _fathomdb_projection_terminal SET state='failed' WHERE write_cursor=1",
        "DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=1",
    ] {
        connection.execute(sql, []).unwrap();
        let nonce = state_nonce(&connection);
        assert_eq!(nonce.len(), 64);
        assert!(nonce.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert!(seen.insert(nonce), "each mutation must mint new branch-sensitive state");
    }
    assert_eq!(seen.len(), 4);
}

#[test]
fn step33_generation_exhaustion_rolls_back_every_schema_change() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
            [i64::MAX],
        )
        .unwrap();
    migrate(&connection).unwrap_err();
    let version: u32 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, 32);
    assert_eq!(generation(&connection), i64::MAX);
    for object in [
        "canonical_nodes_kind_cursor_page_idx",
        "operational_state_collection_cursor_page_idx",
        "operational_state_write_cursor_idx",
        "_fathomdb_artifact_revisions_write_cursor_idx",
        "_fathomdb_read_visibility_oc_ai",
        "_fathomdb_read_visibility_os_ai",
    ] {
        let present: i64 = connection
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name=?1", [object], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(present, 0, "failed migration left {object}");
    }
}

#[test]
fn step33_rejects_duplicate_canonical_keys_without_partial_schema() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..32]).unwrap();
    connection
        .execute(
            "INSERT INTO canonical_nodes(write_cursor,kind,body,logical_id) \
             VALUES(7,'doc','{}','a'),(7,'doc','{}','b')",
            [],
        )
        .unwrap();
    assert!(migrate(&connection).is_err());
    let version: u32 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, 32);
}
