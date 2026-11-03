use super::*;

pub(crate) fn refused() -> EngineError {
    EngineError::ErasureIncomplete {
        stage: "physical_admission".into(),
        detail: "physical cleanup currently refuses this mutation or resource envelope".into(),
    }
}

pub(crate) fn envelope(c: &Connection) -> Result<(), EngineError> {
    let pages: u64 =
        c.query_row("PRAGMA page_count", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
    let size: u64 =
        c.query_row("PRAGMA page_size", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
    if pages.saturating_mul(size) > 1 << 30 {
        return Err(refused());
    }
    let (rows, bytes, largest): (u64, u64, u64) = c.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(body AS BLOB))),0),COALESCE(MAX(length(CAST(body AS BLOB))),0) FROM (SELECT body FROM canonical_nodes UNION ALL SELECT body FROM canonical_edges)", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|_| EngineError::Storage)?;
    let vectors: u64 = c
        .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0))
        .map_err(|_| EngineError::Storage)?;
    // The file-size cap bounds the scans before any target-set collection.
    let mut metadata = 0_u64;
    for table in [
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
        "_fathomdb_source_dependencies",
        "_fathomdb_dependency_closures",
        "_fathomdb_actuation_receipts",
        "_fathomdb_actuation_receipt_source_refs",
        "operational_mutations",
        "operational_state",
    ] {
        let mut statement =
            c.prepare(&format!("PRAGMA table_info({table})")).map_err(|_| EngineError::Storage)?;
        let columns = statement
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        if columns.is_empty() {
            return Err(EngineError::Storage);
        }
        let lengths = columns
            .iter()
            .map(|column| {
                format!("COALESCE(length(CAST(\"{}\" AS BLOB)),0)", column.replace('"', "\"\""))
            })
            .collect::<Vec<_>>()
            .join("+");
        let bytes: u64 = c
            .query_row(&format!("SELECT COALESCE(SUM({lengths}),0) FROM {table}"), [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        metadata = metadata.saturating_add(bytes);
        if metadata > 64 << 20 {
            return Err(refused());
        }
    }
    if rows > 100_000
        || bytes > 256 << 20
        || largest > 1 << 20
        || vectors > 1_000_000
        || metadata > 64 << 20
    {
        return Err(refused());
    }
    Ok(())
}

pub(super) fn deny_hash(domain: &str, value: &str) -> String {
    fingerprint(domain, value)
}

pub(super) fn capture_denial(c: &Connection, debt: &mut RootDebt) -> Result<(), EngineError> {
    let cursors = serde_json::to_string(&debt.cursors).map_err(|_| EngineError::Storage)?;
    for table in ["canonical_nodes", "canonical_edges"] {
        let mut statement = c.prepare(&format!("SELECT source_id,logical_id,body FROM {table} WHERE write_cursor IN (SELECT value FROM json_each(?1))")).map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([&cursors], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (source, logical, body) = row.map_err(|_| EngineError::Storage)?;
            if let Some(source) = source {
                debt.denied_sources.push(deny_hash("source", &source));
            }
            debt.denied_identities.push(deny_hash(
                "identity",
                &derive_stable_id(logical.as_deref(), body.as_deref().unwrap_or("")).to_prefixed(),
            ));
        }
    }
    debt.denied_sources.sort_unstable();
    debt.denied_sources.dedup();
    debt.denied_identities.sort_unstable();
    debt.denied_identities.dedup();
    Ok(())
}

pub(super) fn set_prospective_policy(c: &Connection, enabled: bool) -> Result<(), EngineError> {
    for table in families() {
        c.execute(
            &format!("INSERT INTO {table}({table},rank) VALUES('secure-delete',?1)"),
            [i32::from(enabled)],
        )
        .map_err(|_| EngineError::Storage)?;
    }
    Ok(())
}

fn check_identity(
    debt: &RootDebt,
    source: &str,
    logical: Option<&str>,
    body: &str,
) -> Result<(), EngineError> {
    if debt.denied_sources.contains(&deny_hash("source", source))
        || debt
            .denied_identities
            .contains(&deny_hash("identity", &derive_stable_id(logical, body).to_prefixed()))
    {
        return Err(refused());
    }
    Ok(())
}

pub(crate) fn guard_plain_write(
    c: &Connection,
    batch: &[PreparedWrite],
) -> Result<bool, EngineError> {
    if !active(c)? {
        return Ok(false);
    }
    let debt = root(c)?.ok_or_else(refused)?;
    if debt.version != 2
        || debt.redacted
        || !matches!(state(c)?.phase.as_str(), "cleaning" | "finalizing")
    {
        return Err(refused());
    }
    if load_dependency_generation(c)? != debt.generation {
        return Err(EngineError::Storage);
    }
    for write in batch {
        let (table, source, logical, body, prior_sql, extra) = match write {
            PreparedWrite::Node { source_id, logical_id, body, .. } => (
                "canonical_nodes",
                source_id.as_str(),
                logical_id.as_deref(),
                body.as_str(),
                "logical_id=?1",
                None,
            ),
            PreparedWrite::Edge { source_id, logical_id, body, from, to, kind, .. } => {
                for endpoint in [from, to] {
                    if debt
                        .denied_identities
                        .contains(&deny_hash("identity", &format!("l:{endpoint}")))
                    {
                        return Err(refused());
                    }
                }
                (
                    "canonical_edges",
                    source_id.as_str(),
                    logical_id.as_deref(),
                    body.as_deref().unwrap_or(""),
                    "logical_id=?1 OR (from_id=?2 AND to_id=?3 AND kind=?4)",
                    Some((from, to, kind)),
                )
            }
            _ => return Err(refused()),
        };
        check_identity(&debt, source, logical, body)?;
        let endpoints = if table == "canonical_edges" { "n.from_id,n.to_id" } else { "NULL,NULL" };
        let sql = format!("SELECT n.source_id,n.logical_id,n.body,r.revision_id,r.artifact_role,{endpoints} FROM {table} n LEFT JOIN _fathomdb_artifact_revisions r ON r.write_cursor=n.write_cursor WHERE {prior_sql}");
        let mut statement = c.prepare(&sql).map_err(|_| EngineError::Storage)?;
        let read = |r: &rusqlite::Row<'_>| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
            ))
        };
        let rows = if let Some((from, to, kind)) = extra {
            statement.query_map(params![logical, from, to, kind], read)
        } else {
            statement.query_map(params![logical], read)
        }
        .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (source, logical, body, revision, role, from, to) =
                row.map_err(|_| EngineError::Storage)?;
            for endpoint in [from, to].into_iter().flatten() {
                if debt.denied_identities.contains(&deny_hash("identity", &format!("l:{endpoint}")))
                {
                    return Err(refused());
                }
            }
            check_identity(
                &debt,
                source.as_deref().unwrap_or(""),
                logical.as_deref(),
                body.as_deref().unwrap_or(""),
            )?;
            if role.as_deref() != Some("legacy") {
                return Err(refused());
            }
            let protected: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM _fathomdb_source_links WHERE artifact_revision_id=?1 OR source_revision_id=?1) OR EXISTS(SELECT 1 FROM _fathomdb_source_dependencies WHERE derived_revision_id=?1) OR EXISTS(SELECT 1 FROM _fathomdb_dependency_closures WHERE root_value=?1) OR EXISTS(SELECT 1 FROM _fathomdb_actuation_receipt_source_refs WHERE ref_value=?1 OR (ref_kind='source_id' AND ref_value=?2))", params![revision,source], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
            if protected {
                return Err(refused());
            }
        }
    }
    Ok(true)
}

pub(crate) fn certify_plain_write(c: &Connection, boundary: u64) -> Result<(), EngineError> {
    if let Some(mut debt) = root(c)? {
        if debt.version != 2 || debt.redacted {
            return Err(refused());
        }
        envelope(c)?;
        debt.safe_boundary = debt.safe_boundary.max(boundary);
        store(c, ROOT_KEY, &debt)?;
        validate_zero(c, &debt)?;
    }
    Ok(())
}

pub(crate) fn closure_safe_boundary(
    c: &Connection,
    id: &ClosureOperationId,
    boundary: u64,
    generation: u64,
) -> Result<u64, EngineError> {
    if let Some(debt) = root(c)? {
        if debt.version == 2
            && debt.closures.iter().any(|value| value == id.as_str())
            && debt.boundary == boundary
            && debt.generation == generation
        {
            return Ok(debt.safe_boundary);
        }
    }
    Ok(boundary)
}
