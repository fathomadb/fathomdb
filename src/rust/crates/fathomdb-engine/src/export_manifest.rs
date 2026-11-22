//! Schema-38 `sqlite_master` completeness manifest and `ExportClass` contract.

use std::collections::BTreeMap;

use rusqlite::Connection;

/// Classification of every non-`sqlite_%` table in a Schema-38 database for
/// export/audit completeness verification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExportClass {
    /// Canonical, operational, provenance, and schema declaration authority.
    DurableAuthority,
    /// Opt-in dense vector index and embedder metadata tables.
    OptInVector,
    /// Opt-in actuation receipts and dependency closure audit records.
    OptInAudit,
    /// Recomputable lexical, attribute, and typed-property projections.
    DerivedProjection,
    /// Internal runtime watermarks, migration history, and virtual-table shadows.
    InternalRuntime,
}

fn is_fts5_shadow_of(name: &str, base: &str) -> bool {
    let Some(suffix) = name.strip_prefix(base) else {
        return false;
    };
    matches!(suffix, "_config" | "_content" | "_data" | "_docsize" | "_idx")
}

fn is_vec0_shadow(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("vector_default_") else {
        return false;
    };
    if matches!(rest, "chunks" | "info" | "rowids" | "auxiliary") {
        return true;
    }
    for numbered_prefix in ["vector_chunks", "metadatachunks", "metadatatext"] {
        if let Some(digits) = rest.strip_prefix(numbered_prefix) {
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

/// Classify a non-`sqlite_%` Schema-38 table into its authoritative [`ExportClass`].
///
/// Returns `None` for any table not covered by the Schema-38 manifest so
/// completeness audits fail closed on unclassified schema additions.
pub fn classify_schema_38_table(name: &str) -> Option<ExportClass> {
    match name {
        // DurableAuthority: canonical + operational + provenance + declarations.
        "canonical_nodes"
        | "canonical_edges"
        | "operational_collections"
        | "operational_mutations"
        | "operational_state"
        | "_fathomdb_artifact_revisions"
        | "_fathomdb_source_versions"
        | "_fathomdb_source_links"
        | "_fathomdb_source_dependencies"
        | "_fathomdb_projection_registry"
        | "_fathomdb_vector_kinds"
        | "_fathomdb_commit_timestamps" => Some(ExportClass::DurableAuthority),

        // OptInAudit: receipts and dependency closures.
        "_fathomdb_actuation_receipts"
        | "_fathomdb_actuation_receipt_source_refs"
        | "_fathomdb_dependency_closures" => Some(ExportClass::OptInAudit),

        // OptInVector: dense vector partition and embedder profiles/probe.
        "vector_default"
        | "_fathomdb_vector_rows"
        | "_fathomdb_vector_spans"
        | "_fathomdb_embedder_profiles"
        | "_fathomdb_embed_probe" => Some(ExportClass::OptInVector),

        // DerivedProjection: recomputable lexical/EAV/typed-property projections.
        "canonical_attributes"
        | "canonical_property_values"
        | "property_search_index"
        | "search_index"
        | "search_index_v2"
        | "search_index_edges" => Some(ExportClass::DerivedProjection),

        // InternalRuntime: migrations, open state, visibility, runtime watermarks.
        "_fathomdb_migrations"
        | "_fathomdb_schema_meta"
        | "_fathomdb_open_state"
        | "_fathomdb_read_visibility_state"
        | "_fathomdb_projection_state"
        | "_fathomdb_projection_terminal"
        | "_fathomdb_projection_generations"
        | "_fathomdb_projection_generation_current"
        | "_fathomdb_erasure_requests" => Some(ExportClass::InternalRuntime),

        other => {
            if is_fts5_shadow_of(other, "search_index")
                || is_fts5_shadow_of(other, "search_index_v2")
                || is_fts5_shadow_of(other, "search_index_edges")
                || is_fts5_shadow_of(other, "property_search_index")
            {
                Some(ExportClass::DerivedProjection)
            } else if is_vec0_shadow(other) {
                Some(ExportClass::InternalRuntime)
            } else {
                None
            }
        }
    }
}

/// Introspect every non-`sqlite_%` table in `sqlite_master` and classify it via
/// [`classify_schema_38_table`].
///
/// Returns `Ok(manifest)` when every table maps to a unique [`ExportClass`], or
/// `Err(unclassified_table_names)` listing every unrecognized table.
pub fn audit_schema_38_sqlite_master(
    conn: &Connection,
) -> Result<BTreeMap<String, ExportClass>, Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name ASC",
        )
        .expect("prepare sqlite_master audit query");
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query sqlite_master")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect sqlite_master table names");

    let mut manifest = BTreeMap::new();
    let mut unclassified = Vec::new();
    for name in names {
        match classify_schema_38_table(&name) {
            Some(class) => {
                manifest.insert(name, class);
            }
            None => unclassified.push(name),
        }
    }
    if unclassified.is_empty() {
        Ok(manifest)
    } else {
        Err(unclassified)
    }
}
