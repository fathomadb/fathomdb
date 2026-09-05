use std::collections::BTreeMap;

use fathomdb_schema::migrate;
use rusqlite::Connection;

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
