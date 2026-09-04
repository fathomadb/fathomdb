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

#[test]
fn step31_adds_identity_key_generation_and_exact_trigger_manifest() {
    let connection = Connection::open_in_memory().unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..30]).unwrap();
    assert_eq!(
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0)).unwrap(),
        30
    );

    migrate(&connection).unwrap();
    assert_eq!(SCHEMA_VERSION, 31);
    assert_eq!(
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0)).unwrap(),
        31
    );

    let database_id: String = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_database_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let key: String = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_read_context_key'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(database_id.len(), 32);
    assert_eq!(key.len(), 64);
    assert!(database_id.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    assert!(key.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
    assert_eq!(generation(&connection), 0);

    let trigger_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(trigger_count, 42, "14 real serving-authority tables x three mutations");

    let expected_tables =
        ["cn", "ce", "ar", "sv", "sl", "sd", "dc", "pr", "ca", "ps", "pt", "vk", "vr", "ep"];
    let expected_names = expected_tables
        .into_iter()
        .flat_map(|short| {
            ["ai", "au", "ad"]
                .into_iter()
                .map(move |suffix| format!("_fathomdb_read_visibility_{short}_{suffix}"))
        })
        .collect::<BTreeSet<_>>();
    let actual_names = connection
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<BTreeSet<_>, _>>()
        .unwrap();
    assert_eq!(actual_names, expected_names, "the trigger manifest is exact, not count-only");

    connection
        .execute("INSERT INTO canonical_nodes(write_cursor,kind,body) VALUES(1,'doc','alpha')", [])
        .unwrap();
    assert_eq!(generation(&connection), 1);
    connection.execute("UPDATE canonical_nodes SET body='beta' WHERE write_cursor=1", []).unwrap();
    assert_eq!(generation(&connection), 2);
    connection.execute("DELETE FROM canonical_nodes WHERE write_cursor=1", []).unwrap();
    assert_eq!(generation(&connection), 3);

    connection
        .execute(
            "INSERT INTO _fathomdb_embedder_profiles(profile,name,revision,dimension)
             VALUES('default','test','v1',1)",
            [],
        )
        .unwrap();
    assert_eq!(generation(&connection), 4);
    connection
        .execute(
            "UPDATE _fathomdb_embedder_profiles SET mean_vec=X'00000000'
             WHERE profile='default'",
            [],
        )
        .unwrap();
    assert_eq!(generation(&connection), 5, "centering state participates in frozen visibility");
}

#[test]
fn step31_generation_fails_closed_at_sqlite_integer_maximum() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
            [i64::MAX],
        )
        .unwrap();

    let error = connection
        .execute("INSERT INTO canonical_nodes(write_cursor,kind,body) VALUES(1,'doc','alpha')", [])
        .unwrap_err();
    assert!(error.to_string().contains("read visibility generation exhausted"));
    assert_eq!(generation(&connection), i64::MAX);

    let row: Option<String> = connection
        .query_row("SELECT body FROM canonical_nodes WHERE write_cursor=1", [], |row| row.get(0))
        .optional()
        .unwrap();
    assert_eq!(row, None, "failed increment rolls back the visibility-changing write");
}
