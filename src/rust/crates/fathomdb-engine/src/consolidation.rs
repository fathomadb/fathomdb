use super::*;

/// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — one (subject-entity, relation) axis to
/// consolidate via [`Engine::consolidate_with_provider`]. FathomDB assembles the
/// competing fact-edge cluster for this axis DETERMINISTICALLY (CPU-only, no
/// LLM) by querying active `canonical_edges` where `from_id = subject_logical_id`
/// AND `kind = relation`.
#[derive(Clone, Debug)]
pub struct ConsolidateAxis {
    /// Stable `logical_id` of the subject entity (edge `from_id`).
    pub subject_logical_id: String,
    /// The relation/edge `kind` whose competing fact-edges form the cluster.
    pub relation: String,
}

/// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — one competing fact-edge in a candidate
/// cluster sent to the consolidation harness. Assembled deterministically from
/// `canonical_edges`; sent to the harness as the request payload; the harness's
/// verdict references edges back by `edge_ref` (the edge's stable `logical_id`).
#[derive(Clone, Debug)]
pub struct ConsolidateCandidateEdge {
    /// The edge's stable `logical_id` — the ref the harness uses in its verdict.
    pub edge_ref: String,
    /// The fact/relationship text (never rewritten by consolidation — §2.1).
    pub body: Option<String>,
    /// Event valid-time as INTEGER epoch seconds (UTC), if known.
    ///
    /// TC-33: epoch seconds, NOT ISO-8601. ISO-8601 lives only on the BYO-LLM
    /// extractor wire; `normalize_extractor_timestamp` is the one boundary.
    pub t_valid: Option<i64>,
    /// Event invalid-time as INTEGER epoch seconds (UTC), if already
    /// invalidated. `None` = still valid.
    pub t_invalid: Option<i64>,
    /// Extraction confidence ∈ [0.0, 1.0], if known.
    pub confidence: Option<f64>,
    /// Provenance: originating document id.
    pub source_doc_id: Option<String>,
    /// Provenance: extractor model id from the original BYO-LLM ingest.
    pub extractor_model_id: Option<String>,
}

/// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — receipt returned by
/// [`Engine::consolidate_with_provider`]. Consolidation records supersession /
/// recency METADATA only (§2.1): edge bodies are never rewritten and no row is
/// ever deleted, so these counts describe metadata transitions, not content
/// changes.
#[derive(Clone, Debug, Default)]
pub struct ConsolidateReceipt {
    /// Number of (subject, relation) axes with a non-empty cluster that were
    /// dispatched to the harness.
    pub clusters_processed: u64,
    /// Number of candidate edges presented across all clusters.
    pub edges_examined: u64,
    /// Number of edges the harness ruled `keep` (no metadata change).
    pub edges_kept: u64,
    /// Number of edges the harness ruled `invalidate` (t_invalid set; row + body
    /// preserved).
    pub edges_invalidated: u64,
    /// Number of edges the harness ruled `supersede`/`merge` (marked superseded
    /// via the existing G0 tombstone column; row + body preserved).
    pub edges_superseded: u64,
}

impl Engine {
    /// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — BYO-LLM CONSOLIDATION / RECENCY.
    ///
    /// The SECOND consumer of the one `provider_session` transport (ADR-0.8.6):
    /// consolidation reuses the exact NDJSON-over-stdio transport, hello/ready
    /// handshake, `supported_tasks` negotiation, `request_id` framing, and
    /// bounded-recv timeout — only the protocol string
    /// (`fathomdb.consolidate.v1`) and the task-specific payload differ. There is
    /// NO second transport and NO second handshake.
    ///
    /// For each `(subject, relation)` axis, FathomDB assembles a candidate
    /// cluster of competing active fact-edges DETERMINISTICALLY (CPU-only, no
    /// LLM), sends it to the caller-supplied harness, and applies the returned
    /// verdicts. **CALLER-SIDE BYO-LLM**: the harness is the caller's subprocess;
    /// the library never embeds or calls an LLM and makes NO network egress.
    ///
    /// **Load-bearing semantic (ADR-0.8.12 §2.1):** consolidation records
    /// supersession / recency METADATA only — `invalidate` sets `t_invalid`,
    /// `supersede`/`merge` marks the row superseded via the existing G0 tombstone
    /// column. Edge BODIES are NEVER rewritten and NO row is ever deleted (the
    /// 0.8.3 lesson: blind content-merge HURT accuracy). The original rows
    /// survive; the engine stays deterministic.
    ///
    /// Returns [`EngineError::Consolidator`] on any transport/handshake/protocol
    /// fault or a malformed / out-of-cluster verdict.
    pub fn consolidate_with_provider(
        &self,
        cmd: &[&str],
        axes: &[ConsolidateAxis],
    ) -> Result<ConsolidateReceipt, EngineError> {
        // Reuse the shared transport verbatim; remap its (Extractor-flavoured)
        // transport error to the task-specific Consolidator leaf.
        let mut session = self
            .provider_session(ProviderTask::Consolidate, cmd)
            .map_err(|_| EngineError::Consolidator)?;
        self.run_consolidate_session(&mut session, axes)
    }

    /// 0.8.12 Slice 15 — consolidate-specific driver over a `ProviderSession`.
    /// Mirrors [`run_extract_session`][Engine::run_extract_session]: assemble the
    /// task payload, run the framed request over the shared session, apply the
    /// task-specific DB effect. The cluster assembly + verdict application are
    /// CPU-only/deterministic.
    fn run_consolidate_session(
        &self,
        session: &mut ProviderSession,
        axes: &[ConsolidateAxis],
    ) -> Result<ConsolidateReceipt, EngineError> {
        let mut receipt = ConsolidateReceipt::default();

        for (i, axis) in axes.iter().enumerate() {
            // 1. Deterministically assemble the candidate cluster (CPU-only, no LLM).
            let cluster = self.assemble_consolidate_cluster(axis)?;
            if cluster.is_empty() {
                continue;
            }
            receipt.clusters_processed = receipt.clusters_processed.saturating_add(1);
            receipt.edges_examined = receipt.edges_examined.saturating_add(cluster.len() as u64);

            // 2. Send the cluster; receive the verdict envelope. The session adds
            //    protocol/type/request_id and validates type=="result" + matching
            //    request_id. Any transport/protocol fault → Consolidator.
            let request_id = format!("req-{i}");
            // TC-33: storage and `ConsolidateCandidateEdge` are INTEGER epoch
            // seconds, but the harness WIRE is ISO-8601 — the same split as the
            // extractor boundary. Render on the way out; the verdict's
            // `t_invalid` is normalised back on the way in. Without this the
            // harness would receive epoch integers and (since the reference stub
            // echoes the winner's `t_valid` straight back as `t_invalid`) its
            // reply would be rejected by our own inbound normaliser.
            let edges_json: Vec<Value> = {
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                // TC-33 fix-1 backstop [DEFENSIVE — unreachable]. A stored
                // `Some(ts)` that fails to render must NOT become a silent
                // `null`: that is exactly the "still valid" resurrection vector.
                // With `reject_unrenderable_edge_epoch` guarding the write
                // boundary, no unrenderable epoch can reach storage — so this is
                // a hard-assert upholding that invariant STRUCTURALLY, not the
                // primary defence. `None` (unknown) still renders to JSON null;
                // only a NON-NULL stored epoch that fails to render is an error.
                let render = |field: &str, value: Option<i64>| -> Result<Value, EngineError> {
                    match value {
                        None => Ok(Value::Null),
                        Some(ts) => match epoch_seconds_to_iso8601(connection, ts) {
                            Some(iso) => Ok(Value::from(iso)),
                            None => Err(EngineError::InvalidArgument {
                                msg: format!(
                                    "INVARIANT VIOLATION (TC-33 fix-1): stored edge `{field}` = \
                                     {ts} is unrenderable to ISO-8601 and would have gone to the \
                                     consolidation wire as a silent null (\"still valid\"). The \
                                     write boundary should have made this unstorable."
                                ),
                            }),
                        },
                    }
                };
                cluster
                    .iter()
                    .map(|e| {
                        Ok::<Value, EngineError>(serde_json::json!({
                            "edge_ref": e.edge_ref,
                            "body": e.body,
                            "t_valid": render("t_valid", e.t_valid)?,
                            "t_invalid": render("t_invalid", e.t_invalid)?,
                            "confidence": e.confidence,
                            "source_doc_id": e.source_doc_id,
                            "extractor_model_id": e.extractor_model_id,
                        }))
                    })
                    .collect::<Result<Vec<Value>, EngineError>>()?
            };
            let cluster_json = serde_json::json!({
                "subject": axis.subject_logical_id,
                "relation": axis.relation,
                "edges": edges_json,
            });
            let result = session
                .request(&request_id, vec![("cluster".to_string(), cluster_json)])
                .map_err(|_| EngineError::Consolidator)?;

            // 3. Apply the verdicts (metadata-only; original rows + bodies survive).
            let verdicts = result
                .get("verdicts")
                .and_then(|v| v.as_array())
                .ok_or(EngineError::Consolidator)?
                .clone();
            self.apply_consolidate_verdicts(&cluster, &verdicts, &mut receipt)?;
        }

        Ok(receipt)
    }

    /// 0.8.12 Slice 15 — assemble the competing fact-edge cluster for one
    /// `(subject, relation)` axis, deterministically, from active `canonical_edges`
    /// (`from_id = subject AND kind = relation AND superseded_at IS NULL`), ordered
    /// by `write_cursor` (stable insertion order). CPU-only; no network, no LLM.
    fn assemble_consolidate_cluster(
        &self,
        axis: &ConsolidateAxis,
    ) -> Result<Vec<ConsolidateCandidateEdge>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection
            .prepare(
                "SELECT logical_id, body, t_valid, t_invalid, confidence, source_id, \
                        extractor_model_id \
                 FROM canonical_edges \
                 WHERE from_id = ?1 AND kind = ?2 AND superseded_at IS NULL \
                 ORDER BY write_cursor",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = stmt
            .query_map(params![axis.subject_logical_id, axis.relation], |r| {
                Ok(ConsolidateCandidateEdge {
                    edge_ref: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    body: r.get(1)?,
                    t_valid: r.get(2)?,
                    t_invalid: r.get(3)?,
                    confidence: r.get(4)?,
                    source_doc_id: r.get(5)?,
                    extractor_model_id: r.get(6)?,
                })
            })
            .map_err(|_| EngineError::Storage)?;
        let out: rusqlite::Result<Vec<ConsolidateCandidateEdge>> = rows.collect();
        // Skip any edge with a NULL/empty logical_id (no stable ref to round-trip).
        Ok(out
            .map_err(|_| EngineError::Storage)?
            .into_iter()
            .filter(|e| !e.edge_ref.is_empty())
            .collect())
    }

    /// 0.8.12 Slice 15 — apply the harness verdicts as METADATA-ONLY transitions
    /// (ADR-0.8.12 §2.1). NEVER rewrites a body, NEVER deletes a row. A verdict
    /// referencing an edge not in the presented cluster, or an unknown verdict
    /// kind, is a protocol fault → [`EngineError::Consolidator`].
    fn apply_consolidate_verdicts(
        &self,
        cluster: &[ConsolidateCandidateEdge],
        verdicts: &[Value],
        receipt: &mut ConsolidateReceipt,
    ) -> Result<(), EngineError> {
        let known: std::collections::HashSet<&str> =
            cluster.iter().map(|e| e.edge_ref.as_str()).collect();
        // fix-1 [P2] bijection: the verdict set must cover the presented cluster
        // EXACTLY — every presented edge ruled on, none ruled on twice.
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;

        for v in verdicts {
            let edge_ref =
                v.get("edge_ref").and_then(|x| x.as_str()).ok_or(EngineError::Consolidator)?;
            // The harness may only rule on edges FathomDB presented in the cluster.
            if !known.contains(edge_ref) {
                return Err(EngineError::Consolidator);
            }
            // fix-1 [P2]: a repeated edge_ref is a protocol fault (not a bijection).
            if !seen.insert(edge_ref) {
                return Err(EngineError::Consolidator);
            }
            let verdict =
                v.get("verdict").and_then(|x| x.as_str()).ok_or(EngineError::Consolidator)?;
            // Look up the active edge's projection cursor BEFORE any UPDATE so a
            // supersede (which clears `superseded_at IS NULL`) can still find it.
            let active_cursor = Self::active_edge_write_cursor(&tx, edge_ref)?;
            match verdict {
                "keep" => {
                    receipt.edges_kept = receipt.edges_kept.saturating_add(1);
                }
                "invalidate" => {
                    // Recency metadata: set t_invalid; the row and its body are
                    // left intact (this is NOT a destructive content rewrite).
                    //
                    // TC-33: the CONSOLIDATION harness is the same class of
                    // BYO-LLM boundary as the extractor, so it carries ISO-8601
                    // on the wire and is normalised here with the SAME hard
                    // rejection. Previously the raw string went straight into the
                    // UPDATE with no validation whatsoever.
                    // fix-3 [P2]: consolidation is a BYO-LLM PROVIDER boundary, so
                    // a malformed / non-string `t_invalid` is a PROVIDER protocol
                    // fault → `Consolidator`, NOT the extractor/user `InvalidArgument`
                    // that `normalize_extractor_timestamp` emits. Remap it to match
                    // the two sibling failure modes on this same value (missing key
                    // and null/unparseable-to-None, both `Consolidator`). Consistent,
                    // not a diagnostic loss: `Consolidator` is a unit variant and the
                    // adjacent `.ok_or(EngineError::Consolidator)` cases already
                    // discard any message.
                    let ts = normalize_extractor_timestamp(
                        &tx,
                        "t_invalid",
                        Some(v.get("t_invalid").ok_or(EngineError::Consolidator)?),
                    )
                    .map_err(|_| EngineError::Consolidator)?
                    .ok_or(EngineError::Consolidator)?;
                    tx.execute(
                        "UPDATE canonical_edges SET t_invalid = ?1 \
                         WHERE logical_id = ?2 AND superseded_at IS NULL",
                        params![ts, edge_ref],
                    )
                    .map_err(|_| EngineError::Storage)?;
                    // fix-1 [P1]: prune the STATIC projection shadow rows so the
                    // consolidated-away edge stops surfacing in FTS/vector — but
                    // ONLY when the edge is ended as of the engine's "now",
                    // mirroring the graph-traversal filter `edge_validity_sql`.
                    // A future-dated t_invalid keeps the edge valid ⇒ keep the
                    // projection. NON-DESTRUCTIVE: the canonical_edges row + body
                    // survive (ADR-0.8.12 §2.1).
                    //
                    // TC-33: this used to be `SELECT datetime(?1) <= datetime('now')`
                    // — an inline clock, AND a misleading error class: junk made
                    // the SELECT yield SQL NULL, so `r.get::<bool>` failed as
                    // `EngineError::Storage`. Both timestamps are integers now, so
                    // the comparison is plain Rust against the bound `:now` seam.
                    if let Some(cursor) = active_cursor {
                        let ended = ts <= current_epoch_seconds();
                        if ended {
                            // fix-2 [P2]: KEEP the projection terminal row. The
                            // canonical_edges row stays NON-superseded (invalidate
                            // is metadata-only), and `database_has_pending_projection_work`
                            // flags any non-superseded edge that has a body but no
                            // terminal as pending; since `next_pending_projection_jobs`
                            // only scans cursors ABOVE the stored projection cursor, an
                            // already-projected invalidated edge would never be requeued
                            // and `drain()`/`wait_for_idle` would hang forever. Dropping
                            // the FTS/vec shadows (below) hides it from active retrieval;
                            // retaining the terminal keeps the scheduler idle.
                            Self::prune_edge_projection_shadows(&tx, cursor, true)?;
                        }
                    }
                    receipt.edges_invalidated = receipt.edges_invalidated.saturating_add(1);
                }
                // `merge` maps cleanly to supersede + metadata (ADR-0.8.12 §3):
                // the loser is marked superseded; the winner ("by"/"into") is the
                // surviving active row. No body is merged.
                "supersede" | "merge" => {
                    // Mark superseded via the existing G0 tombstone column; the row
                    // survives (invalidate-not-delete). Use a fresh monotonic cursor.
                    let cursor = self.next_cursor.fetch_add(1, Ordering::SeqCst).saturating_add(1);
                    tx.execute(
                        "UPDATE canonical_edges SET superseded_at = ?1 \
                         WHERE logical_id = ?2 AND superseded_at IS NULL",
                        params![cursor, edge_ref],
                    )
                    .map_err(|_| EngineError::Storage)?;
                    // fix-1 [P1]: a superseded edge is unconditionally out of the
                    // active set (graph traversal filters `superseded_at IS NULL`),
                    // so prune its FTS/vector projection shadow rows to match.
                    if let Some(active_cursor) = active_cursor {
                        // A superseded row is excluded from the pending-work check
                        // (`superseded_at IS NOT NULL`), so dropping its terminal too
                        // is safe (matches the excise pattern) and cannot phantom-pend.
                        Self::prune_edge_projection_shadows(&tx, active_cursor, false)?;
                    }
                    receipt.edges_superseded = receipt.edges_superseded.saturating_add(1);
                }
                _ => return Err(EngineError::Consolidator),
            }
        }

        // fix-1 [P2]: bijection completeness — every presented cluster edge must
        // have received exactly one verdict.
        if seen.len() != known.len() {
            return Err(EngineError::Consolidator);
        }

        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(())
    }

    /// fix-1 [P1] — the active (non-superseded) row's projection `write_cursor`
    /// for a fact-edge `logical_id`, or `None` if there is no active row. The
    /// cursor keys the STATIC projection shadow rows (FTS `search_index_edges`,
    /// vec0 `vector_default` by rowid, `_fathomdb_vector_rows`,
    /// `_fathomdb_projection_terminal`).
    fn active_edge_write_cursor(
        tx: &rusqlite::Transaction<'_>,
        edge_ref: &str,
    ) -> Result<Option<i64>, EngineError> {
        tx.query_row(
            "SELECT write_cursor FROM canonical_edges \
             WHERE logical_id = ?1 AND superseded_at IS NULL",
            params![edge_ref],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)
    }

    /// fix-1 [P1] — prune the STATIC projection shadow rows for a canonical
    /// row's `write_cursor` so a consolidated-away edge stops surfacing in
    /// FTS/vector retrieval. Mirrors the excision pattern at
    /// `excise_source_inner` (invalidate-not-delete: the canonical row + body
    /// are NEVER touched here).
    ///
    /// `keep_terminal` retains the `_fathomdb_projection_terminal` marker — set it
    /// when the canonical row stays NON-superseded (an `invalidate` verdict), so the
    /// projection scheduler still treats the cursor as done (fix-2 [P2]); clear it
    /// when the row is superseded (excluded from the pending-work scan anyway).
    ///
    /// FIXED (0.8.12 Slice A, R-CON-2 named default-ON blocker; Slice-20 codex
    /// §9 [P2]): a full `rebuild_projections` re-projects every non-superseded
    /// edge with a body from `canonical_edges` — this used to re-materialise an
    /// invalidated edge's FTS/vec shadows even though graph traversal excludes
    /// it via the `t_invalid > now` filter. The FTS rebuild SELECT
    /// (`rebuild_shadow_state`), the vec projection queue
    /// (`next_pending_projection_jobs`), and the pending-work probe
    /// (`database_has_pending_projection_work`) now all carry the same
    /// `edge_validity_sql` filter as graph traversal (TC-33: INTEGER compare
    /// against the bound `:now`, formerly `datetime(t_invalid) > datetime('now')`),
    /// so a rebuild is durable across the recency exclusion.
    fn prune_edge_projection_shadows(
        tx: &rusqlite::Transaction<'_>,
        cursor: i64,
        keep_terminal: bool,
    ) -> Result<(), EngineError> {
        tx.execute("DELETE FROM search_index_edges WHERE write_cursor = ?1", [cursor])
            .map_err(|_| EngineError::Storage)?;
        // vec0 rowid is the canonical row's write_cursor. TC-76: via the one
        // vec0-delete primitive ([`delete_vector_partition_row`]).
        delete_vector_partition_row(tx, cursor).map_err(|_| EngineError::Storage)?;
        tx.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor = ?1", [cursor])
            .map_err(|_| EngineError::Storage)?;
        if !keep_terminal {
            tx.execute(
                "DELETE FROM _fathomdb_projection_terminal WHERE write_cursor = ?1",
                [cursor],
            )
            .map_err(|_| EngineError::Storage)?;
        }
        Ok(())
    }
}
