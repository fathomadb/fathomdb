//! Coupling audit supplementing the whole-crate closed scanner in
//! `experiments/slice35_virtual_mutation_audit.py`.

const SOURCE: &str = include_str!("../src/lib.rs");

fn function_body(name: &str) -> &'static str {
    let start = SOURCE.find(&format!("fn {name}(")).unwrap_or_else(|| panic!("missing {name}"));
    let tail = &SOURCE[start + 1..];
    let end = ["\nfn ", "\n    fn ", "\n    pub fn ", "\n    pub async fn "]
        .iter()
        .filter_map(|marker| tail.find(marker))
        .min()
        .map_or(SOURCE.len(), |offset| start + 1 + offset);
    &SOURCE[start..end]
}

fn contains_all(name: &str, needles: &[&str]) {
    let body = function_body(name);
    for needle in needles {
        assert!(body.contains(needle), "{name} lost required coupling {needle:?}");
    }
}

#[test]
fn production_virtual_mutation_sites_remain_closed_and_owner_coupled() {
    // New literal mutation sites must be classified here rather than silently
    // bypassing frozen-read invalidation.
    for (needle, count) in [
        ("\"INSERT INTO search_index(", 1),
        ("\"INSERT INTO search_index_v2(", 1),
        ("\"INSERT INTO search_index_edges(", 1),
        ("\"INSERT INTO property_search_index(", 1),
        ("\"DELETE FROM property_search_index", 1),
        ("\"INSERT INTO vector_default(", 6),
        ("\"INSERT OR IGNORE INTO vector_default(", 0),
        ("\"UPDATE {DEFAULT_VECTOR_PARTITION}", 1),
        ("\"INSERT INTO {DEFAULT_VECTOR_PARTITION}", 1),
        ("\"CREATE VIRTUAL TABLE {guard}{DEFAULT_VECTOR_PARTITION}", 1),
        ("DROP TABLE {DEFAULT_VECTOR_PARTITION}", 1),
        ("DROP TABLE vector_default", 2),
        ("\"DELETE FROM {DEFAULT_VECTOR_PARTITION}", 1),
        ("\"DELETE FROM {} WHERE {}", 1),
        ("\"DELETE FROM {}\"", 1),
    ] {
        assert_eq!(SOURCE.matches(needle).count(), count, "unclassified mutation: {needle}");
    }

    // Same-transaction projection helpers pair virtual rows with a triggered
    // authoritative row. Canonical node/edge inserts precede their projectors
    // in commit_batch; rebuild uses the same projectors and triggered readiness.
    contains_all(
        "apply_batch_in_transaction",
        &[
            "INSERT INTO canonical_nodes",
            "project_canonical_node_row",
            "INSERT INTO canonical_edges",
            "project_canonical_edge_row",
        ],
    );
    contains_all(
        "project_canonical_node_row",
        &[
            "INSERT INTO search_index(",
            "INSERT INTO search_index_v2(",
            "_fathomdb_projection_state",
        ],
    );
    contains_all(
        "project_canonical_edge_row",
        &["INSERT INTO search_index_edges(", "_fathomdb_projection_state"],
    );
    contains_all(
        "project_one_attribute",
        &["INSERT INTO canonical_attributes", "INSERT INTO property_search_index"],
    );
    contains_all(
        "clear_attribute_projection",
        &["DELETE FROM property_search_index", "DELETE FROM canonical_attributes"],
    );
    contains_all(
        "prune_edge_projection_shadows",
        &["DELETE FROM search_index_edges", "delete_vector_partition_row", "_fathomdb_vector_rows"],
    );
    contains_all(
        "write_vector_for_test",
        &[
            "INSERT INTO _fathomdb_vector_rows",
            "INSERT INTO vector_default(",
            "_fathomdb_embedder_profiles",
        ],
    );
    contains_all(
        "run_pin_and_requantize_pass",
        &["delete_vector_partition_row", "INSERT INTO vector_default("],
    );
    contains_all(
        "commit_projection_outcomes",
        &[
            "INSERT INTO _fathomdb_vector_rows",
            "INSERT INTO vector_default(",
            "record_projection_terminal",
        ],
    );
    contains_all(
        "refresh_vector_attr_values_for_row",
        &["UPDATE {DEFAULT_VECTOR_PARTITION}", "vector_attr_insert_fragments"],
    );
    contains_all(
        "reshape_vector_partition_nondestructive",
        &[
            "DROP TABLE {DEFAULT_VECTOR_PARTITION}",
            "vector_partition_create_sql",
            "INSERT INTO {DEFAULT_VECTOR_PARTITION}",
        ],
    );
    contains_all(
        "erase_row_projections",
        &["ROW_OWNED_PROJECTIONS", "delete_row_owned_projection"],
    );
    contains_all("truncate_row_projections_in", &["ROW_OWNED_PROJECTIONS", "DELETE FROM {}"]);

    // The remaining vec0 INSERTs are schema/open reshapes. They execute before
    // a Slice-35 token can be minted and therefore cannot evade a live token.
    contains_all("migrate_vector_partition_pack1_to_pack2", &["INSERT INTO vector_default("]);
    contains_all("migrate_vector_partition_to_pack1", &["INSERT INTO vector_default("]);
}
