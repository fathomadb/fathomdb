use erasure_availability::*;
#[test]
fn campaign_fixture_counts_revision_owners_targets_and_real_property_membership() {
    for family in ["node", "edge", "property"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.sqlite");
        build_campaign(&path, family, false, 10).expect("governed measurement fixture");
        let c = reader(&path).unwrap();
        let scalar = |sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
        let (nodes, edges, properties) = match family {
            "edge" => (11, 50, 0),
            "property" => (50, 0, 20),
            _ => (50, 0, 0),
        };
        assert_eq!(scalar("SELECT count(*) FROM canonical_nodes"), nodes);
        assert_eq!(scalar("SELECT count(*) FROM canonical_edges"), edges);
        assert_eq!(scalar("SELECT count(*) FROM property_search_index"), properties);
        let table = if family == "edge" { "canonical_edges" } else { "canonical_nodes" };
        assert_eq!(
            scalar(&format!(
                "SELECT count(DISTINCT logical_id) FROM {table} WHERE source_id='target'"
            )),
            1
        );
        assert_eq!(scalar(&format!("SELECT count(*) FROM {table} WHERE source_id='target'")), 5);
        let primary = if family == "property" {
            "property_search_index"
        } else if family == "edge" {
            "search_index_edges"
        } else {
            "search_index"
        };
        let column = if family == "property" { "attr_value" } else { "body" };
        assert_eq!(
            scalar(&format!("SELECT min(length(CAST({column} AS BLOB))) FROM {primary}")),
            1024
        );
        assert_eq!(
            scalar(&format!("SELECT max(length(CAST({column} AS BLOB))) FROM {primary}")),
            1024
        );
        let inv = inventory(&path, MARKER).unwrap();
        let selected = inv.iter().find(|r| r.table == primary).unwrap();
        assert_eq!(selected.matches, if family == "property" { 2 } else { 5 });
        assert!(selected.shadow_markers > 0);
    }
}
