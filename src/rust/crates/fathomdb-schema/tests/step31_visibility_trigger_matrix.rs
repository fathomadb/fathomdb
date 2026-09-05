use std::collections::BTreeMap;

use fathomdb_schema::migrate;
use rusqlite::Connection;

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
fn every_serving_authority_table_advances_visibility_for_all_three_mutations() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();

    let expected_tables = [
        ("cn", "canonical_nodes"),
        ("ce", "canonical_edges"),
        ("ar", "_fathomdb_artifact_revisions"),
        ("sv", "_fathomdb_source_versions"),
        ("sl", "_fathomdb_source_links"),
        ("sd", "_fathomdb_source_dependencies"),
        ("dc", "_fathomdb_dependency_closures"),
        ("pr", "_fathomdb_projection_registry"),
        ("ca", "canonical_attributes"),
        ("ps", "_fathomdb_projection_state"),
        ("pt", "_fathomdb_projection_terminal"),
        ("vk", "_fathomdb_vector_kinds"),
        ("vr", "_fathomdb_vector_rows"),
        ("ep", "_fathomdb_embedder_profiles"),
    ];
    let expected_operations = [("ai", "INSERT"), ("au", "UPDATE"), ("ad", "DELETE")];

    let actual = connection
        .prepare(
            "SELECT name, tbl_name, sql FROM sqlite_master
             WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, (row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        })
        .unwrap()
        .collect::<Result<BTreeMap<_, _>, _>>()
        .unwrap();

    assert_eq!(actual.len(), expected_tables.len() * expected_operations.len());
    for (short, table) in expected_tables {
        for (suffix, operation) in expected_operations {
            let name = format!("_fathomdb_read_visibility_{short}_{suffix}");
            let (actual_table, sql) = actual.get(&name).unwrap_or_else(|| panic!("missing {name}"));
            assert_eq!(actual_table, table, "{name} targets the wrong table");
            assert!(
                sql.contains(&format!("AFTER {operation} ON {table}")),
                "{name} does not cover {operation} on {table}: {sql}"
            );
            assert!(
                sql.contains("UPDATE _fathomdb_read_visibility_state"),
                "{name} does not advance the visibility generation"
            );
            assert!(
                sql.contains("read visibility generation exhausted"),
                "{name} does not fail closed at generation exhaustion"
            );
        }
    }
}

#[test]
fn every_serving_authority_trigger_executes_for_insert_update_and_delete() {
    let connection = Connection::open_in_memory().unwrap();
    migrate(&connection).unwrap();
    let cases = [
        (
            "canonical_nodes",
            "INSERT INTO canonical_nodes(write_cursor,kind,body) VALUES(101,'doc','a')",
            "UPDATE canonical_nodes SET body='b' WHERE write_cursor=101",
            "DELETE FROM canonical_nodes WHERE write_cursor=101",
        ),
        (
            "canonical_edges",
            "INSERT INTO canonical_edges(write_cursor,kind,from_id,to_id) VALUES(102,'link','a','b')",
            "UPDATE canonical_edges SET to_id='c' WHERE write_cursor=102",
            "DELETE FROM canonical_edges WHERE write_cursor=102",
        ),
        (
            "_fathomdb_artifact_revisions",
            "INSERT INTO _fathomdb_artifact_revisions VALUES(1,'rev','node',103,'legacy','migrated_incomplete')",
            "UPDATE _fathomdb_artifact_revisions SET write_cursor=104 WHERE revision_id='rev'",
            "DELETE FROM _fathomdb_artifact_revisions WHERE revision_id='rev'",
        ),
        (
            "_fathomdb_source_versions",
            "INSERT INTO _fathomdb_source_versions VALUES(1,'source','version','source-rev')",
            "UPDATE _fathomdb_source_versions SET source_version_id='version-2' WHERE source_revision_id='source-rev'",
            "DELETE FROM _fathomdb_source_versions WHERE source_revision_id='source-rev'",
        ),
        (
            "_fathomdb_source_links",
            "INSERT INTO _fathomdb_source_links VALUES(1,'artifact','source','version','source-rev','whole_body',NULL,NULL,'sha256','0000000000000000000000000000000000000000000000000000000000000000')",
            "UPDATE _fathomdb_source_links SET source_version_id='version-2' WHERE artifact_revision_id='artifact'",
            "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='artifact'",
        ),
        (
            "_fathomdb_source_dependencies",
            "INSERT INTO _fathomdb_source_dependencies VALUES(1,'dependency','derived',1)",
            "UPDATE _fathomdb_source_dependencies SET registered_dependency_generation=2 WHERE dependency_id='dependency'",
            "DELETE FROM _fathomdb_source_dependencies WHERE dependency_id='dependency'",
        ),
        (
            "_fathomdb_dependency_closures",
            "INSERT INTO _fathomdb_dependency_closures VALUES(1,'closure','source_revision','root','superseded',0,0,0,1,'0000000000000000000000000000000000000000000000000000000000000000','complete',1,NULL,0,'{}')",
            "UPDATE _fathomdb_dependency_closures SET root_value='root-2' WHERE closure_operation_id='closure'",
            "DELETE FROM _fathomdb_dependency_closures WHERE closure_operation_id='closure'",
        ),
        (
            "_fathomdb_projection_registry",
            "INSERT INTO _fathomdb_projection_registry(name,roles,vector_declared) VALUES('owner','[]',0)",
            "UPDATE _fathomdb_projection_registry SET roles='[\"filterable\"]' WHERE name='owner'",
            "DELETE FROM _fathomdb_projection_registry WHERE name='owner'",
        ),
        (
            "canonical_attributes",
            "INSERT INTO canonical_attributes VALUES(105,'owner','alice')",
            "UPDATE canonical_attributes SET attr_value='bob' WHERE write_cursor=105",
            "DELETE FROM canonical_attributes WHERE write_cursor=105",
        ),
        (
            "_fathomdb_projection_state",
            "INSERT INTO _fathomdb_projection_state VALUES('doc',1,0)",
            "UPDATE _fathomdb_projection_state SET last_enqueued_cursor=2 WHERE kind='doc'",
            "DELETE FROM _fathomdb_projection_state WHERE kind='doc'",
        ),
        (
            "_fathomdb_projection_terminal",
            "INSERT INTO _fathomdb_projection_terminal VALUES(106,'up_to_date')",
            "UPDATE _fathomdb_projection_terminal SET state='failed' WHERE write_cursor=106",
            "DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=106",
        ),
        (
            "_fathomdb_vector_kinds",
            "INSERT INTO _fathomdb_vector_kinds VALUES('doc','default',0)",
            "UPDATE _fathomdb_vector_kinds SET created_at=1 WHERE kind='doc'",
            "DELETE FROM _fathomdb_vector_kinds WHERE kind='doc'",
        ),
        (
            "_fathomdb_vector_rows",
            "INSERT INTO _fathomdb_vector_rows VALUES(107,'doc',107)",
            "UPDATE _fathomdb_vector_rows SET kind='note' WHERE rowid=107",
            "DELETE FROM _fathomdb_vector_rows WHERE rowid=107",
        ),
        (
            "_fathomdb_embedder_profiles",
            "INSERT INTO _fathomdb_embedder_profiles(profile,name,revision,dimension) VALUES('default','test','v1',8)",
            "UPDATE _fathomdb_embedder_profiles SET revision='v2' WHERE profile='default'",
            "DELETE FROM _fathomdb_embedder_profiles WHERE profile='default'",
        ),
    ];

    for (table, insert, update, delete) in cases {
        for (operation, sql) in [("insert", insert), ("update", update), ("delete", delete)] {
            let before = generation(&connection);
            let changed = connection.execute(sql, []).unwrap_or_else(|error| {
                panic!("{table} {operation} did not execute: {error}: {sql}")
            });
            assert_eq!(changed, 1, "{table} {operation} did not affect exactly one row");
            assert_eq!(
                generation(&connection),
                before + 1,
                "{table} {operation} did not advance visibility exactly once"
            );
        }
    }
}
