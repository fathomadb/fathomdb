use std::collections::{BTreeMap, BTreeSet};

use fathomdb_schema::{
    check_migration_accretion, check_migration_logical_id_pin, migrate, migrate_with_steps,
    MIGRATIONS, SCHEMA_VERSION,
};
use rusqlite::Connection;

#[derive(Debug, Eq, PartialEq)]
struct ColumnInfo {
    data_type: String,
    notnull: bool,
    dflt_value: Option<String>,
    pk: i64,
}

fn table_columns(conn: &Connection, table: &str) -> BTreeMap<String, ColumnInfo> {
    let sql = format!("PRAGMA table_info('{table}')");
    conn.prepare(&sql)
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                ColumnInfo {
                    data_type: row.get::<_, String>(2)?,
                    notnull: row.get::<_, i64>(3)? != 0,
                    dflt_value: row.get::<_, Option<String>>(4)?,
                    pk: row.get::<_, i64>(5)?,
                },
            ))
        })
        .unwrap()
        .collect::<Result<BTreeMap<_, _>, _>>()
        .unwrap()
}

fn index_columns(conn: &Connection, index_name: &str) -> Vec<String> {
    let sql = format!("PRAGMA index_info('{index_name}')");
    conn.prepare(&sql)
        .unwrap()
        .query_map([], |row| row.get::<_, String>(2))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn index_sql(conn: &Connection, table: &str, index_name: &str) -> String {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND name=?2",
        [table, index_name],
        |row| row.get(0),
    )
    .unwrap_or_else(|err| panic!("missing index {index_name} on {table}: {err}"))
}

fn visibility_state(conn: &Connection) -> (i64, String) {
    conn.query_row(
        "SELECT generation, state_nonce FROM _fathomdb_read_visibility_state WHERE singleton=1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap()
}

#[test]
fn step38_schema_version_and_columns_match_contract() {
    assert_eq!(SCHEMA_VERSION, 38);
    assert_eq!(MIGRATIONS.len(), 38);
    assert_eq!(MIGRATIONS.last().map(|m| m.step_id), Some(38));

    let conn = Connection::open_in_memory().unwrap();
    let report = migrate(&conn).unwrap();
    assert_eq!(report.schema_version_after, 38);
    let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(user_version, 38);

    let cn = table_columns(&conn, "canonical_nodes");
    assert_eq!(
        cn.get("properties"),
        Some(&ColumnInfo { data_type: "TEXT".into(), notnull: false, dflt_value: None, pk: 0 })
    );
    assert_eq!(
        cn.get("scope"),
        Some(&ColumnInfo {
            data_type: "TEXT".into(),
            notnull: true,
            dflt_value: Some("''".into()),
            pk: 0,
        })
    );

    let ce = table_columns(&conn, "canonical_edges");
    assert_eq!(
        ce.get("properties"),
        Some(&ColumnInfo { data_type: "TEXT".into(), notnull: false, dflt_value: None, pk: 0 })
    );
    assert_eq!(
        ce.get("scope"),
        Some(&ColumnInfo {
            data_type: "TEXT".into(),
            notnull: true,
            dflt_value: Some("''".into()),
            pk: 0,
        })
    );

    for op_table in ["operational_mutations", "operational_state"] {
        let cols = table_columns(&conn, op_table);
        assert_eq!(
            cols.get("scope"),
            Some(&ColumnInfo {
                data_type: "TEXT".into(),
                notnull: true,
                dflt_value: Some("''".into()),
                pk: 0,
            }),
            "{op_table} missing scope column"
        );
    }

    let pr = table_columns(&conn, "_fathomdb_projection_registry");
    assert_eq!(
        pr.get("entity_class"),
        Some(&ColumnInfo {
            data_type: "TEXT".into(),
            notnull: true,
            dflt_value: Some("'node'".into()),
            pk: 0,
        })
    );
    assert_eq!(
        pr.get("vector_kinds"),
        Some(&ColumnInfo { data_type: "TEXT".into(), notnull: false, dflt_value: None, pk: 0 })
    );

    // Registry entity_class CHECK constraint rejects unknown values.
    conn.execute(
        "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) \
         VALUES('ok_node', '[\"filterable\"]', 'node')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) \
         VALUES('ok_edge', '[\"filterable\"]', 'edge')",
        [],
    )
    .unwrap();
    assert!(conn
        .execute(
            "INSERT INTO _fathomdb_projection_registry(name, roles, entity_class) \
             VALUES('bad_class', '[\"filterable\"]', 'graph')",
            [],
        )
        .is_err());
}

#[test]
fn step38_upgrades_step37_rows_with_additive_defaults() {
    let conn = Connection::open_in_memory().unwrap();
    migrate_with_steps(&conn, &MIGRATIONS[..37]).unwrap();

    conn.execute(
        "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id) \
         VALUES(1, 'doc', '{\"title\":\"hi\"}', 'src:1', 'n1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO canonical_edges(write_cursor, kind, from_id, to_id, source_id, logical_id) \
         VALUES(2, 'rel', 'n1', 'n1', 'src:1', 'e1')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO operational_mutations(collection_name, record_key, op_kind, payload_json, write_cursor) \
         VALUES('col', 'k1', 'append', '{}', 3)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO operational_state(collection_name, record_key, payload_json, write_cursor) \
         VALUES('col', 'k1', '{}', 4)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO _fathomdb_projection_registry(name, roles, vector_declared) \
         VALUES('owner', '[\"filterable\"]', 0)",
        [],
    )
    .unwrap();

    let report = migrate(&conn).unwrap();
    assert_eq!(report.schema_version_before, 37);
    assert_eq!(report.schema_version_after, 38);
    assert_eq!(report.migration_steps.len(), 1);
    assert_eq!(report.migration_steps[0].step_id, 38);

    let (node_props, node_scope): (Option<String>, String) = conn
        .query_row("SELECT properties, scope FROM canonical_nodes WHERE write_cursor=1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(node_props, None);
    assert_eq!(node_scope, "");

    let (edge_props, edge_scope): (Option<String>, String) = conn
        .query_row("SELECT properties, scope FROM canonical_edges WHERE write_cursor=2", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(edge_props, None);
    assert_eq!(edge_scope, "");

    let mut_scope: String = conn
        .query_row("SELECT scope FROM operational_mutations WHERE write_cursor=3", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mut_scope, "");

    let state_scope: String = conn
        .query_row("SELECT scope FROM operational_state WHERE write_cursor=4", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state_scope, "");

    let (entity_class, vector_kinds): (String, Option<String>) = conn
        .query_row(
            "SELECT entity_class, vector_kinds FROM _fathomdb_projection_registry WHERE name='owner'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(entity_class, "node");
    assert_eq!(vector_kinds, None);
}

#[test]
fn step38_canonical_property_values_checks_and_indexes() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let cpv = table_columns(&conn, "canonical_property_values");
    assert_eq!(cpv.get("entity_class").map(|c| c.pk), Some(1));
    assert_eq!(cpv.get("write_cursor").map(|c| c.pk), Some(2));
    assert_eq!(cpv.get("attr_name").map(|c| c.pk), Some(3));
    assert_eq!(cpv.get("num_val").map(|c| c.data_type.as_str()), Some("NUMERIC"));

    // Valid inserts for all 4 types including exact > 2^53 i64 precision.
    conn.execute(
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val) \
         VALUES('node', 1, 'title', 'string', 'hello', NULL, NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val) \
         VALUES('edge', 2, 'count', 'integer', NULL, ?1, NULL)",
        [(1_i64 << 53) + 1],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val) \
         VALUES('node', 3, 'score', 'float', NULL, 3.25, NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val) \
         VALUES('edge', 4, 'verified', 'boolean', NULL, NULL, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val, num_val, bool_val) \
         VALUES('node', 5, 'archived', 'boolean', NULL, NULL, 0)",
        [],
    )
    .unwrap();

    // Verify invalid entity_class, write_cursor, attr_name, value_type, and column mismatches fail CHECK.
    for (label, sql) in [
        (
            "bad entity_class",
            "INSERT INTO canonical_property_values VALUES('graph', 10, 'k', 'string', 'v', NULL, NULL)",
        ),
        (
            "zero write_cursor",
            "INSERT INTO canonical_property_values VALUES('node', 0, 'k', 'string', 'v', NULL, NULL)",
        ),
        (
            "negative write_cursor",
            "INSERT INTO canonical_property_values VALUES('node', -1, 'k', 'string', 'v', NULL, NULL)",
        ),
        (
            "empty attr_name",
            "INSERT INTO canonical_property_values VALUES('node', 10, '', 'string', 'v', NULL, NULL)",
        ),
        (
            "bad value_type",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'json', '{}', NULL, NULL)",
        ),
        (
            "string missing text_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'string', NULL, NULL, NULL)",
        ),
        (
            "string with extra num_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'string', 'v', 1, NULL)",
        ),
        (
            "string with extra bool_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'string', 'v', NULL, 1)",
        ),
        (
            "integer missing num_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'integer', NULL, NULL, NULL)",
        ),
        (
            "integer with extra text_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'integer', '1', 1, NULL)",
        ),
        (
            "float with extra bool_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'float', NULL, 1.5, 0)",
        ),
        (
            "boolean missing bool_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'boolean', NULL, NULL, NULL)",
        ),
        (
            "boolean out-of-range bool_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'boolean', NULL, NULL, 2)",
        ),
        (
            "boolean with extra num_val",
            "INSERT INTO canonical_property_values VALUES('node', 10, 'k', 'boolean', NULL, 1, 1)",
        ),
    ] {
        assert!(conn.execute(sql, []).is_err(), "{label} should fail CHECK constraint");
    }

    let cpv_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'canonical_property_values'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        cpv_sql.contains("WITHOUT ROWID"),
        "canonical_property_values must be a WITHOUT ROWID table: {cpv_sql}"
    );
    assert!(
        conn.prepare("SELECT rowid FROM canonical_property_values").is_err(),
        "WITHOUT ROWID table must not expose a rowid column"
    );
    let cursor_idx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'canonical_property_values_cursor_idx'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        cursor_idx_count, 0,
        "redundant canonical_property_values_cursor_idx must be absent"
    );

    let mut delete_plan_stmt = conn
        .prepare(
            "EXPLAIN QUERY PLAN DELETE FROM canonical_property_values \
             WHERE entity_class IN ('node', 'edge') AND write_cursor = ?1",
        )
        .unwrap();
    let delete_plan: Vec<String> =
        delete_plan_stmt.query_map([1_i64], |r| r.get(3)).unwrap().map(Result::unwrap).collect();
    assert!(
        delete_plan
            .iter()
            .any(|line| line.contains("SEARCH canonical_property_values USING PRIMARY KEY")),
        "expected PRIMARY KEY search on canonical_property_values delete; got {delete_plan:?}"
    );
    assert!(
        !delete_plan.iter().any(|line| line.contains("SCAN canonical_property_values")),
        "canonical_property_values delete must not full-table SCAN; got {delete_plan:?}"
    );

    assert_eq!(
        index_columns(&conn, "canonical_property_values_text_idx"),
        vec!["entity_class", "attr_name", "text_val", "write_cursor"]
    );
    assert!(index_sql(&conn, "canonical_property_values", "canonical_property_values_text_idx")
        .contains("WHERE text_val IS NOT NULL"));
    assert_eq!(
        index_columns(&conn, "canonical_property_values_num_idx"),
        vec!["entity_class", "attr_name", "num_val", "write_cursor"]
    );
    assert!(index_sql(&conn, "canonical_property_values", "canonical_property_values_num_idx")
        .contains("WHERE num_val IS NOT NULL"));
    assert_eq!(
        index_columns(&conn, "canonical_property_values_bool_idx"),
        vec!["entity_class", "attr_name", "bool_val", "write_cursor"]
    );
    assert!(index_sql(&conn, "canonical_property_values", "canonical_property_values_bool_idx")
        .contains("WHERE bool_val IS NOT NULL"));
}

#[test]
fn step38_commit_timestamps_and_canonical_indexes_exist() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let ts_cols = table_columns(&conn, "_fathomdb_commit_timestamps");
    assert_eq!(
        ts_cols.get("write_cursor"),
        Some(&ColumnInfo { data_type: "INTEGER".into(), notnull: false, dflt_value: None, pk: 1 })
    );
    assert_eq!(
        ts_cols.get("committed_at_ms"),
        Some(&ColumnInfo { data_type: "INTEGER".into(), notnull: true, dflt_value: None, pk: 0 })
    );
    conn.execute(
        "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(1, 1700000000000)",
        [],
    )
    .unwrap();
    assert!(conn
        .execute(
            "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(0, 1700000000000)",
            [],
        )
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(-1, 1700000000000)",
            [],
        )
        .is_err());
    assert_eq!(
        index_columns(&conn, "_fathomdb_commit_timestamps_ms_idx"),
        vec!["committed_at_ms", "write_cursor"]
    );

    assert_eq!(
        index_columns(&conn, "canonical_nodes_logical_sup_idx"),
        vec!["logical_id", "superseded_at", "write_cursor"]
    );
    assert!(index_sql(&conn, "canonical_nodes", "canonical_nodes_logical_sup_idx")
        .contains("WHERE superseded_at IS NOT NULL"));
    assert_eq!(index_columns(&conn, "canonical_edges_from_active_idx"), vec!["from_id", "kind"]);
    assert!(index_sql(&conn, "canonical_edges", "canonical_edges_from_active_idx")
        .contains("WHERE superseded_at IS NULL"));
    assert_eq!(index_columns(&conn, "canonical_edges_to_active_idx"), vec!["to_id", "kind"]);
    assert!(index_sql(&conn, "canonical_edges", "canonical_edges_to_active_idx")
        .contains("WHERE superseded_at IS NULL"));
    assert_eq!(
        index_columns(&conn, "canonical_edges_from_sup_kind_idx"),
        vec!["from_id", "superseded_at", "kind"]
    );
    assert!(index_sql(&conn, "canonical_edges", "canonical_edges_from_sup_kind_idx")
        .contains("WHERE superseded_at IS NOT NULL"));
    assert_eq!(
        index_columns(&conn, "canonical_edges_to_sup_kind_idx"),
        vec!["to_id", "superseded_at", "kind"]
    );
    assert!(index_sql(&conn, "canonical_edges", "canonical_edges_to_sup_kind_idx")
        .contains("WHERE superseded_at IS NOT NULL"));
}

#[test]
fn step38_installs_60_coalescing_visibility_triggers_including_cpv() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    let triggers: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT name, tbl_name, sql FROM sqlite_master \
             WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%' ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(triggers.len(), 60, "expected 20 tables * 3 triggers = 60");
    let cpv_names: BTreeSet<String> = triggers
        .iter()
        .filter(|(_, tbl, _)| tbl == "canonical_property_values")
        .map(|(name, _, _)| name.clone())
        .collect();
    assert_eq!(
        cpv_names,
        BTreeSet::from([
            "_fathomdb_read_visibility_cpv_ad".to_string(),
            "_fathomdb_read_visibility_cpv_ai".to_string(),
            "_fathomdb_read_visibility_cpv_au".to_string(),
        ])
    );

    let mut seen_nonces = BTreeSet::new();
    let (mut gen_before, nonce0) = visibility_state(&conn);
    seen_nonces.insert(nonce0);

    for sql in [
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, text_val) \
         VALUES('node', 1, 'k', 'string', 'v1')",
        "UPDATE canonical_property_values SET text_val='v2' \
         WHERE entity_class='node' AND write_cursor=1 AND attr_name='k'",
        "DELETE FROM canonical_property_values \
         WHERE entity_class='node' AND write_cursor=1 AND attr_name='k'",
    ] {
        conn.execute(sql, []).unwrap();
        let (gen_after, nonce_after) = visibility_state(&conn);
        assert_eq!(gen_after, gen_before + 1);
        assert!(seen_nonces.insert(nonce_after));
        gen_before = gen_after;
    }

    // Coalescing = 1 suppresses trigger updates on canonical_property_values.
    conn.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1", [])
        .unwrap();
    let (coalesced_gen, coalesced_nonce) = visibility_state(&conn);
    for sql in [
        "INSERT INTO canonical_property_values(entity_class, write_cursor, attr_name, value_type, num_val) \
         VALUES('edge', 2, 'w', 'integer', 7)",
        "UPDATE canonical_property_values SET num_val=8 \
         WHERE entity_class='edge' AND write_cursor=2 AND attr_name='w'",
        "DELETE FROM canonical_property_values \
         WHERE entity_class='edge' AND write_cursor=2 AND attr_name='w'",
    ] {
        conn.execute(sql, []).unwrap();
        assert_eq!(visibility_state(&conn), (coalesced_gen, coalesced_nonce.clone()));
    }
    conn.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=0 WHERE singleton=1", [])
        .unwrap();
}

#[test]
fn step38_passes_accretion_and_logical_id_pin_guards() {
    let step38 = MIGRATIONS.iter().find(|m| m.step_id == 38).expect("step 38 must exist");
    assert!(
        step38.sql.contains("-- MIGRATION-ACCRETION-EXEMPTION: "),
        "step 38 must carry the MIGRATION-ACCRETION-EXEMPTION marker"
    );
    check_migration_accretion("038_property_schema.sql", step38.sql).unwrap();

    let without_exemption = step38
        .sql
        .lines()
        .filter(|line| !line.contains("MIGRATION-ACCRETION-EXEMPTION"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(check_migration_accretion("038_property_schema.sql", &without_exemption).is_err());

    check_migration_logical_id_pin("038_property_schema.sql", step38.sql).unwrap();
}
