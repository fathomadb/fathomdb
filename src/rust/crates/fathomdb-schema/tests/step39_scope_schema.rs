use std::collections::{BTreeMap, BTreeSet};

use fathomdb_schema::{
    check_migration_accretion, check_migration_logical_id_pin, is_fts5_shadow_table,
    is_vec0_shadow_table, migrate, migrate_with_steps, TableExportClass, CANONICAL_TABLES,
    MIGRATIONS, SCHEMA_VERSION, TABLE_MANIFEST,
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

fn pk_columns(conn: &Connection, table: &str) -> Vec<String> {
    let cols = table_columns(conn, table);
    let mut keyed: Vec<(i64, String)> = cols
        .into_iter()
        .filter(|(_, info)| info.pk > 0)
        .map(|(name, info)| (info.pk, name))
        .collect();
    keyed.sort_by_key(|(pk, _)| *pk);
    keyed.into_iter().map(|(_, name)| name).collect()
}

fn table_indexes(conn: &Connection, table: &str) -> BTreeSet<String> {
    conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql IS NOT NULL",
    )
    .unwrap()
    .query_map([table], |row| row.get::<_, String>(0))
    .unwrap()
    .collect::<Result<BTreeSet<_>, _>>()
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

fn table_triggers(conn: &Connection, table: &str) -> BTreeSet<String> {
    conn.prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND tbl_name=?1")
        .unwrap()
        .query_map([table], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<BTreeSet<_>, _>>()
        .unwrap()
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
fn step39_schema_version_and_migration_lints() {
    assert_eq!(SCHEMA_VERSION, 39);
    assert_eq!(MIGRATIONS.len(), 39);
    let step39 = MIGRATIONS.last().expect("step 39 migration");
    assert_eq!(step39.step_id, 39);

    check_migration_logical_id_pin("039_scope_composite.sql", step39.sql)
        .expect("Schema 39 must obey TC-11 logical_id pin without canonical_nodes table drop");
    check_migration_accretion("039_scope_composite.sql", step39.sql)
        .expect("Schema 39 must pass accretion lint");
    assert!(
        step39.sql.contains("-- MIGRATION-ACCRETION-EXEMPTION: "),
        "Schema 39 SQL must document MIGRATION-ACCRETION-EXEMPTION"
    );
    let upper = step39.sql.to_ascii_uppercase();
    assert!(!upper.contains("DROP TABLE CANONICAL_NODES"), "step 39 must not drop canonical_nodes");
    assert!(!upper.contains("DROP TABLE CANONICAL_EDGES"), "step 39 must not drop canonical_edges");
}

#[test]
fn step39_table_manifest_completeness_and_shadow_helpers() {
    let mut names = BTreeSet::new();
    for entry in TABLE_MANIFEST {
        assert!(names.insert(entry.name), "duplicate TABLE_MANIFEST entry: {}", entry.name);
    }
    assert_eq!(TABLE_MANIFEST.len(), 36, "31 regular + 5 virtual Schema-39 tables");

    let expected_canonical: Vec<&str> =
        TABLE_MANIFEST.iter().filter(|e| e.is_canonical).map(|e| e.name).collect();
    assert_eq!(CANONICAL_TABLES, expected_canonical.as_slice());

    let vis_count = TABLE_MANIFEST.iter().filter(|e| e.visibility_trigger_prefix.is_some()).count();
    assert_eq!(vis_count, 20);
    assert_eq!(vis_count * 3, 60);

    // Every export class variant is represented.
    for cls in [
        TableExportClass::DurableAuthority,
        TableExportClass::OptInAudit,
        TableExportClass::OptInVector,
        TableExportClass::DerivedProjection,
        TableExportClass::InternalRuntime,
    ] {
        assert!(
            TABLE_MANIFEST.iter().any(|e| e.export_class == cls),
            "missing export class {cls:?} in TABLE_MANIFEST"
        );
    }

    // Shadow helpers accept documented suffixes and reject arbitrary names.
    assert!(is_fts5_shadow_table("search_index_v2_data"));
    assert!(is_fts5_shadow_table("search_index_edges_docsize"));
    assert!(!is_fts5_shadow_table("search_index_v2_bogus"));
    assert!(is_vec0_shadow_table("vector_default_chunks"));
    assert!(is_vec0_shadow_table("vector_default_metadatachunks00"));
    assert!(is_vec0_shadow_table("vector_default_metadatatext08"));
    assert!(!is_vec0_shadow_table("vector_default_bogus_dummy"));
    assert!(!is_vec0_shadow_table("vector_default_metadatatextXX"));

    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();

    // Every non-virtual, non-sqlite table created by Schema 39 migrations must be in TABLE_MANIFEST.
    let created_tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    for name in &created_tables {
        if is_fts5_shadow_table(name) || is_vec0_shadow_table(name) {
            continue;
        }
        assert!(
            names.contains(name.as_str()),
            "sqlite_master table {name} is missing from TABLE_MANIFEST"
        );
    }
}

#[test]
fn step39_composite_indexes_and_keys_enforce_scope_isolation() {
    let conn = Connection::open_in_memory().unwrap();
    let report = migrate(&conn).unwrap();
    assert_eq!(report.schema_version_after, 39);
    let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(user_version, 39);

    // canonical_nodes indexes
    let cn_indexes = table_indexes(&conn, "canonical_nodes");
    for old_idx in [
        "canonical_nodes_source_id_idx",
        "canonical_nodes_logical_active_idx",
        "canonical_nodes_kind_idx",
        "canonical_nodes_state_active_idx",
        "canonical_nodes_validity_idx",
        "canonical_nodes_kind_cursor_page_idx",
        "canonical_nodes_logical_sup_idx",
    ] {
        assert!(!cn_indexes.contains(old_idx), "old node index {old_idx} should be dropped");
    }
    for new_idx in [
        "canonical_nodes_write_cursor_idx",
        "canonical_nodes_scope_source_id_idx",
        "canonical_nodes_scope_logical_active_idx",
        "canonical_nodes_scope_kind_idx",
        "canonical_nodes_scope_state_active_idx",
        "canonical_nodes_scope_validity_idx",
        "canonical_nodes_scope_cursor_idx",
        "canonical_nodes_scope_kind_cursor_page_idx",
        "canonical_nodes_scope_logical_sup_idx",
    ] {
        assert!(cn_indexes.contains(new_idx), "missing expected node index {new_idx}");
    }
    assert_eq!(
        index_columns(&conn, "canonical_nodes_scope_kind_cursor_page_idx"),
        vec!["scope", "kind", "write_cursor"]
    );
    assert_eq!(index_columns(&conn, "canonical_nodes_write_cursor_idx"), vec!["write_cursor"]);

    // canonical_edges indexes
    let ce_indexes = table_indexes(&conn, "canonical_edges");
    for old_idx in [
        "canonical_edges_source_id_idx",
        "canonical_edges_logical_active_idx",
        "canonical_edges_from_id_idx",
        "canonical_edges_to_id_idx",
        "canonical_edges_from_active_idx",
        "canonical_edges_to_active_idx",
        "canonical_edges_from_sup_kind_idx",
        "canonical_edges_to_sup_kind_idx",
    ] {
        assert!(!ce_indexes.contains(old_idx), "old edge index {old_idx} should be dropped");
    }
    for new_idx in [
        "canonical_edges_write_cursor_idx",
        "canonical_edges_scope_source_id_idx",
        "canonical_edges_scope_logical_active_idx",
        "canonical_edges_scope_from_id_idx",
        "canonical_edges_scope_to_id_idx",
        "canonical_edges_scope_cursor_idx",
        "canonical_edges_scope_from_active_idx",
        "canonical_edges_scope_to_active_idx",
        "canonical_edges_scope_from_sup_kind_idx",
        "canonical_edges_scope_to_sup_kind_idx",
    ] {
        assert!(ce_indexes.contains(new_idx), "missing expected edge index {new_idx}");
    }

    // operational_state & operational_mutations
    assert_eq!(
        pk_columns(&conn, "operational_state"),
        vec!["scope", "collection_name", "record_key"]
    );
    let os_indexes = table_indexes(&conn, "operational_state");
    assert!(!os_indexes.contains("operational_state_collection_cursor_page_idx"));
    assert!(os_indexes.contains("operational_state_scope_collection_cursor_page_idx"));
    assert!(os_indexes.contains("operational_state_write_cursor_idx"));

    let om_indexes = table_indexes(&conn, "operational_mutations");
    assert!(!om_indexes.contains("operational_mutations_collection_id_idx"));
    assert!(om_indexes.contains("operational_mutations_scope_collection_id_idx"));
    assert_eq!(
        index_columns(&conn, "operational_mutations_scope_collection_id_idx"),
        vec!["scope", "collection_name", "id"]
    );

    // Composite PKs and scope column on all 8 recreated control-plane / provenance / actuation / queue tables
    assert_eq!(pk_columns(&conn, "_fathomdb_actuation_receipts"), vec!["scope", "operation_id"]);
    assert_eq!(
        pk_columns(&conn, "_fathomdb_actuation_receipt_source_refs"),
        vec!["scope", "operation_id", "ref_kind", "ref_value"]
    );
    assert_eq!(pk_columns(&conn, "_fathomdb_artifact_revisions"), vec!["scope", "revision_id"]);
    assert_eq!(
        pk_columns(&conn, "_fathomdb_source_versions"),
        vec!["scope", "source_id", "source_version_id"]
    );
    assert_eq!(pk_columns(&conn, "_fathomdb_source_links"), vec!["scope", "artifact_revision_id"]);
    assert_eq!(pk_columns(&conn, "_fathomdb_source_dependencies"), vec!["scope", "dependency_id"]);
    assert_eq!(
        pk_columns(&conn, "_fathomdb_dependency_closures"),
        vec!["scope", "closure_operation_id"]
    );
    assert!(table_columns(&conn, "_fathomdb_erasure_requests").contains_key("scope"));

    // All 60 visibility triggers + actuation ref trigger must be present
    let vis_triggers: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        vis_triggers as usize,
        TABLE_MANIFEST.iter().filter(|e| e.visibility_trigger_prefix.is_some()).count() * 3
    );

    for (tbl, prefix) in [
        ("operational_state", "os"),
        ("_fathomdb_artifact_revisions", "ar"),
        ("_fathomdb_source_versions", "sv"),
        ("_fathomdb_source_links", "sl"),
        ("_fathomdb_source_dependencies", "sd"),
        ("_fathomdb_dependency_closures", "dc"),
    ] {
        let trigs = table_triggers(&conn, tbl);
        for op in ["ai", "au", "ad"] {
            let name = format!("_fathomdb_read_visibility_{prefix}_{op}");
            assert!(trigs.contains(&name), "missing restored trigger {name} on {tbl}");
        }
    }

    // Two distinct scopes can use identical IDs everywhere, and visibility triggers fire on recreated tables.
    conn.execute_batch(
        "INSERT INTO canonical_nodes(write_cursor, kind, body, logical_id, scope)
         VALUES (1, 'note', 'a', 'shared:1', 'scope-a'),
                (2, 'note', 'b', 'shared:1', 'scope-b');
         INSERT INTO canonical_edges(write_cursor, kind, from_id, to_id, logical_id, scope)
         VALUES (3, 'links', 'shared:1', 'shared:1', 'edge:1', 'scope-a'),
                (4, 'links', 'shared:1', 'shared:1', 'edge:1', 'scope-b');
         INSERT INTO operational_state(collection_name, record_key, payload_json, write_cursor, scope)
         VALUES ('c', 'k', '{}', 5, 'scope-a'),
                ('c', 'k', '{}', 6, 'scope-b');
         INSERT INTO _fathomdb_artifact_revisions(schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness, scope)
         VALUES (1, 'rev:1', 'node', 1, 'canonical_source', 'complete', 'scope-a'),
                (1, 'rev:1', 'node', 2, 'canonical_source', 'complete', 'scope-b');
         INSERT INTO _fathomdb_source_versions(schema_version, source_id, source_version_id, source_revision_id, scope)
         VALUES (1, 'src:1', 'v1', 'srev:1', 'scope-a'),
                (1, 'src:1', 'v1', 'srev:1', 'scope-b');
         INSERT INTO _fathomdb_source_links(schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id, locator_kind, hash_algorithm, hash_digest, scope)
         VALUES (1, 'rev:1', 'src:1', 'v1', 'srev:1', 'whole_body', 'sha256', '0000', 'scope-a'),
                (1, 'rev:1', 'src:1', 'v1', 'srev:1', 'whole_body', 'sha256', '0000', 'scope-b');
         INSERT INTO _fathomdb_source_dependencies(schema_version, dependency_id, derived_revision_id, registered_dependency_generation, scope)
         VALUES (1, 'dep:1', 'drev:1', 1, 'scope-a'),
                (1, 'dep:1', 'drev:1', 1, 'scope-b');
         INSERT INTO _fathomdb_erasure_requests(ticket, idempotency_key, payload, scope)
         VALUES ('t-a', 'idem-shared', '{\"ticket\":\"t-a\",\"key\":\"idem-shared\",\"status\":{\"phase\":\"queued\"}}', 'scope-a'),
                ('t-b', 'idem-shared', '{\"ticket\":\"t-b\",\"key\":\"idem-shared\",\"status\":{\"phase\":\"queued\"}}', 'scope-b');",
    )
    .expect("distinct scopes share logical_id/record_key/provenance/erasure keys");

    // Duplicates within the SAME scope still fail UNIQUE / PK constraints.
    assert!(
        conn.execute(
            "INSERT INTO _fathomdb_erasure_requests(ticket, idempotency_key, payload, scope)
             VALUES ('t-a2', 'idem-shared', '{\"ticket\":\"t-a2\",\"key\":\"idem-shared\",\"status\":{\"phase\":\"queued\"}}', 'scope-a')",
            [],
        )
        .is_err(),
        "duplicate idempotency_key inside same scope must fail"
    );

    // Actuation receipt ref owner trigger checks (scope, operation_id) not just operation_id!
    let sha = "a".repeat(64);
    conn.execute(
        "INSERT INTO _fathomdb_actuation_receipts(
            operation_id, schema_version, request_sha256, operations_count, outcome,
            reason_codes_json, affected_revision_ids_json, resulting_write_boundary,
            resulting_dependency_generation, pending_projection_write_cursors_json,
            closure_operation_ids_json, scope
         ) VALUES ('op-shared', 1, ?1, 1, 'committed', '[]', '[]', 1, NULL, '[]', '[]', 'scope-a')",
        [&sha],
    )
    .unwrap();
    // Inserting ref under scope-b with same operation_id must abort in BEFORE INSERT trigger!
    assert!(
        conn.execute(
            "INSERT INTO _fathomdb_actuation_receipt_source_refs(
                operation_id, schema_version, ref_kind, ref_value, scope
             ) VALUES ('op-shared', 1, 'source_id', 'src:1', 'scope-b')",
            [],
        )
        .is_err(),
        "actuation ref trigger must enforce scope matching owner receipt"
    );
    conn.execute(
        "INSERT INTO _fathomdb_actuation_receipt_source_refs(
            operation_id, schema_version, ref_kind, ref_value, scope
         ) VALUES ('op-shared', 1, 'source_id', 'src:1', 'scope-a')",
        [],
    )
    .expect("matching scope owner ref succeeds");

    // Visibility triggers fire on recreated tables when coalescing=0.
    let (gen_before, _) = visibility_state(&conn);
    conn.execute(
        "INSERT INTO _fathomdb_dependency_closures(
            schema_version, closure_operation_id, root_kind, root_value, cause,
            effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation,
            closure_sequence, retry_fingerprint, phase, affected_count, scope
         ) VALUES (1, 'clo-1', 'source_bucket', 'src:1', 'superseded', 100, 1, 0, 1, ?1, 'proving', 1, 'scope-a')",
        [&sha],
    )
    .unwrap();
    let (gen_after, _) = visibility_state(&conn);
    assert_eq!(gen_after, gen_before + 1);
}

#[test]
fn step39_upgrade_from_38_preserves_existing_rows_and_triggers() {
    let conn = Connection::open_in_memory().unwrap();
    migrate_with_steps(&conn, &MIGRATIONS[..38]).unwrap();
    assert_eq!(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0)).unwrap(), 38);

    let sha = "b".repeat(64);
    conn.execute_batch(&format!(
        "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id)
         VALUES (1, 'note', 'body-1', 'src:1', 'n:1'), (2, 'note', 'body-2', 'src:1', 'n:2');
         INSERT INTO canonical_edges(write_cursor, kind, from_id, to_id, source_id, logical_id)
         VALUES (3, 'links', 'n:1', 'n:2', 'src:1', 'e:1');
         INSERT INTO operational_state(collection_name, record_key, payload_json, write_cursor)
         VALUES ('col', 'k1', '{{\"x\":1}}', 4);
         INSERT INTO operational_mutations(collection_name, record_key, op_kind, payload_json, write_cursor)
         VALUES ('log', 'm1', 'append', '{{\"m\":1}}', 5);
         INSERT INTO _fathomdb_actuation_receipts(
            operation_id, schema_version, request_sha256, operations_count, outcome,
            reason_codes_json, affected_revision_ids_json, resulting_write_boundary,
            pending_projection_write_cursors_json, closure_operation_ids_json
         ) VALUES ('op-38', 1, '{sha}', 1, 'committed', '[]', '[]', 5, '[]', '[]');
         INSERT INTO _fathomdb_actuation_receipt_source_refs(operation_id, schema_version, ref_kind, ref_value)
         VALUES ('op-38', 1, 'source_id', 'src:1');
         INSERT INTO _fathomdb_artifact_revisions(schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness)
         VALUES (1, 'rev-38', 'node', 1, 'canonical_source', 'complete');
         INSERT INTO _fathomdb_source_versions(schema_version, source_id, source_version_id, source_revision_id)
         VALUES (1, 'src:1', 'v1', 'srev-38');
         INSERT INTO _fathomdb_source_links(schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id, locator_kind, hash_algorithm, hash_digest)
         VALUES (1, 'rev-38', 'src:1', 'v1', 'srev-38', 'whole_body', 'sha256', '{sha}');
         INSERT INTO _fathomdb_source_dependencies(schema_version, dependency_id, derived_revision_id, registered_dependency_generation)
         VALUES (1, 'dep-38', 'drev-38', 1);
         INSERT INTO _fathomdb_dependency_closures(
            schema_version, closure_operation_id, root_kind, root_value, cause,
            effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation,
            closure_sequence, retry_fingerprint, phase, affected_count
         ) VALUES (1, 'clo-38', 'source_bucket', 'src:1', 'superseded', 100, 1, 0, 1, '{sha}', 'proving', 1);
         INSERT INTO _fathomdb_erasure_requests(ticket, idempotency_key, payload)
         VALUES ('t-38', 'k-38', '{{\"ticket\":\"t-38\",\"key\":\"k-38\",\"status\":{{\"phase\":\"queued\"}}}}');"
    ))
    .unwrap();

    migrate_with_steps(&conn, &MIGRATIONS[38..]).expect("upgrade 38 -> 39 must succeed");
    assert_eq!(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0)).unwrap(), 39);

    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM canonical_nodes WHERE scope=''", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM canonical_edges WHERE scope=''", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM operational_state WHERE scope=''", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM operational_mutations WHERE scope=''", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM _fathomdb_actuation_receipts WHERE scope=''",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs WHERE scope=''",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM _fathomdb_artifact_revisions WHERE scope=''",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM _fathomdb_source_versions WHERE scope=''", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM _fathomdb_source_links WHERE scope=''", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE scope=''",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM _fathomdb_dependency_closures WHERE scope=''",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT phase FROM _fathomdb_erasure_requests WHERE ticket='t-38' AND scope=''",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "queued"
    );

    // Restored visibility triggers fire on operational_state post-migration
    let (gen_before, _) = visibility_state(&conn);
    conn.execute(
        "UPDATE operational_state SET payload_json='{\"x\":2}' WHERE collection_name='col' AND record_key='k1'",
        [],
    )
    .unwrap();
    let (gen_after, _) = visibility_state(&conn);
    assert_eq!(gen_after, gen_before + 1);
}
