use super::*;

impl Engine {
    /// Operator regenerate workflow per `dev/design/projections.md`
    /// § Regenerate workflow. Drains in-flight projection work, then
    /// truncates FTS5 + vec0 shadow rows, resets the projection cursor,
    /// and lets the scheduler re-enqueue every canonical row. Durable
    /// `projection_failures` audit rows are preserved per design. AC-044
    /// + AC-063c.
    #[cfg(feature = "operator")]
    pub fn rebuild_projections(&self) -> Result<RebuildReport, EngineError> {
        self.ensure_open()?;
        self.run_rebuild(true, RebuildKind::Projections)
    }

    /// Vec0-only variant of [`Engine::rebuild_projections`]. Leaves
    /// FTS5 shadow content untouched; per recovery design,
    /// `recover --rebuild-vec0` is the surface for vec0-only repair.
    #[cfg(feature = "operator")]
    pub fn rebuild_vec0(&self) -> Result<RebuildReport, EngineError> {
        self.ensure_open()?;
        self.run_rebuild(false, RebuildKind::Vec0)
    }

    #[cfg(feature = "operator")]
    fn run_rebuild(
        &self,
        include_fts: bool,
        kind: RebuildKind,
    ) -> Result<RebuildReport, EngineError> {
        self.projection_runtime.set_frozen(true);
        // Drain MUST succeed: rebuild_shadow_state truncates shadow rows,
        // and SQLite-WAL allows a worker that already dequeued a job to
        // commit its `INSERT OR IGNORE INTO _fathomdb_vector_rows / vec0`
        // after our truncate releases the writer lock, leaving stale
        // rows. Surfacing the timeout (instead of swallowing it) lets the
        // operator retry rather than silently corrupt the rebuild.
        let drain_result = self.drain(REBUILD_DRAIN_TIMEOUT_MS);
        let result = drain_result.and_then(|()| self.rebuild_shadow_state(include_fts, kind));
        self.projection_runtime.set_frozen(false);
        result
    }

    #[cfg(feature = "operator")]
    fn rebuild_shadow_state(
        &self,
        include_fts: bool,
        kind: RebuildKind,
    ) -> Result<RebuildReport, EngineError> {
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;
        // 0.8.20 Slice 5a (R-20-E1) — registry-driven invalidation. A full
        // rebuild truncates EVERY row-owned projection (the previous hand-rolled
        // list omitted `search_index_v2`, so a rebuild neither dropped stale v2
        // rows nor repopulated the table); a vec0-only rebuild truncates the
        // vector + readiness classes exactly as before. Kind-owned watermark
        // state (`_fathomdb_projection_state`) is deliberately NOT truncated —
        // readiness is reset by rewinding the projection cursor below.
        let rows_invalidated = if include_fts {
            truncate_all_row_projections(&tx).map_err(|_| EngineError::Storage)?
        } else {
            truncate_row_projections_in(&tx, &[ProjectionClass::Vector, ProjectionClass::Readiness])
                .map_err(|_| EngineError::Storage)?
        };
        store_projection_cursor(&tx, 0).map_err(|_| EngineError::Storage)?;
        // 0.8.20 Slice 5a (R-20-E1, work item 1) — the replay runs through the
        // SAME two projectors the write path uses, so the rebuilt projections
        // are identical to what a re-write would have produced. `include_fts`
        // selects the pass: a vec0-only rebuild must not write FTS rows.
        let pass = if include_fts { ProjectionPass::Write } else { ProjectionPass::VectorOnly };
        let mut rows_rebuilt: u64 = 0;
        for row in canonical_node_rows(&tx).map_err(|_| EngineError::Storage)? {
            project_canonical_node_row(
                &tx,
                row.cursor,
                &row.kind,
                &row.body,
                row.row_kind,
                pass,
                // fix-2 [P2]: the attribute half of the replay tracks the backfill's
                // active-and-non-superseded row set; FTS / vector shadows still
                // rebuild for every row (read-side lifecycle filter, unchanged).
                row.attr_projected,
            )
            .map_err(|_| EngineError::Storage)?;
            if include_fts {
                rows_rebuilt = rows_rebuilt.saturating_add(1);
            }
        }
        // fix-26 [P2]: rebuild the edge shadows from active canonical_edges
        // (G11 search_index_edges).
        // 0.8.12 Slice A (R-CON-2 named default-ON blocker; Slice-20 codex
        // §9 [P2]): mirror the graph-traversal recency filter
        // (`edge_validity_sql`) here too, so a full rebuild does not re-surface
        // an edge that recency consolidation already invalidated.
        // 0.8.20 Slice 5a: body-less structural edges are now included in the
        // replay. They project no FTS/vector row, but the write path DOES record
        // their readiness terminal — which this rebuild truncated and (before
        // this slice) never restored, stalling `advance_projection_cursor`.
        // TC-33: the filter is generated by `edge_validity_sql` and `:now` is
        // bound (?1) rather than inlined as `datetime('now')`.
        let edge_rows: Vec<(i64, String, Option<String>)> = {
            let edge_sql = format!(
                "SELECT write_cursor, kind, body FROM canonical_edges \
                 WHERE superseded_at IS NULL{}",
                edge_validity_sql("canonical_edges", 1)
            );
            let mut edge_stmt = tx.prepare(&edge_sql).map_err(|_| EngineError::Storage)?;
            let rows = edge_stmt
                .query_map(params![current_epoch_seconds()], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map_err(|_| EngineError::Storage)?
                .collect::<rusqlite::Result<_>>()
                .map_err(|_| EngineError::Storage)?;
            rows
        };
        for (cursor, kind, body) in edge_rows {
            let has_body = body.is_some();
            project_canonical_edge_row(&tx, cursor as u64, &kind, body.as_deref(), pass)
                .map_err(|_| EngineError::Storage)?;
            if include_fts && has_body {
                rows_rebuilt = rows_rebuilt.saturating_add(1);
            }
        }
        projection_generation::transition(&tx, ProjectionGenerationOriginV1::Rebuild)?;
        let projection_cursor_after =
            load_projection_cursor(&tx).map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(RebuildReport { kind, rows_invalidated, rows_rebuilt, projection_cursor_after })
    }
}
