//! Schema-39 `sqlite_master` completeness manifest and `ExportClass` contract,
//! derived directly from [`fathomdb_schema::TABLE_MANIFEST`].

use std::collections::BTreeMap;

use fathomdb_schema::{
    is_fts5_shadow_table, is_vec0_shadow_table, TableExportClass, TABLE_MANIFEST,
};
use rusqlite::Connection;

/// Classification of every non-`sqlite_%` table in a Schema-39 database for
/// export/audit completeness verification (re-export of
/// [`fathomdb_schema::TableExportClass`]).
pub type ExportClass = TableExportClass;

/// Classify a non-`sqlite_%` Schema-39 table into its authoritative [`ExportClass`].
///
/// Returns `None` for any table not covered by [`TABLE_MANIFEST`] or documented
/// FTS5/`vec0` shadow patterns so completeness audits fail closed on unclassified additions.
pub fn classify_schema_39_table(name: &str) -> Option<ExportClass> {
    if let Some(entry) = TABLE_MANIFEST.iter().find(|entry| entry.name == name) {
        return Some(entry.export_class);
    }
    if is_fts5_shadow_table(name) {
        Some(ExportClass::DerivedProjection)
    } else if is_vec0_shadow_table(name) {
        Some(ExportClass::InternalRuntime)
    } else {
        None
    }
}

/// Backward-compatible Schema-38 alias for [`classify_schema_39_table`].
pub fn classify_schema_38_table(name: &str) -> Option<ExportClass> {
    classify_schema_39_table(name)
}

/// Introspect every non-`sqlite_%` table in `sqlite_master` and classify it via
/// [`classify_schema_39_table`].
///
/// Returns `Ok(manifest)` when every table maps to a unique [`ExportClass`], or
/// `Err(unclassified_table_names)` listing every unrecognized table.
pub fn audit_schema_39_sqlite_master(
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
        match classify_schema_39_table(&name) {
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

/// Backward-compatible Schema-38 alias for [`audit_schema_39_sqlite_master`].
pub fn audit_schema_38_sqlite_master(
    conn: &Connection,
) -> Result<BTreeMap<String, ExportClass>, Vec<String>> {
    audit_schema_39_sqlite_master(conn)
}
