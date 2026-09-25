use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WritePlan {
    Node,
    Edge,
    AppendOnlyLog,
    LatestState,
    AdminSchema,
}

pub(crate) fn validate_batch(
    connection: &Connection,
    batch: &[PreparedWrite],
) -> Result<Vec<WritePlan>, EngineError> {
    batch.iter().map(|write| validate_write(connection, write)).collect()
}

pub(crate) fn collect_projection_jobs(
    connection: &Connection,
    batch: &[PreparedWrite],
) -> Result<Vec<ProjectionJob>, EngineError> {
    let mut jobs = Vec::new();
    let generation_id = projection_generation::current_generation_id(connection)?;
    for write in batch {
        let write = storage_write_shape(write);
        if let PreparedWrite::Node { kind, body, .. } = write.as_ref() {
            // 0.8.20 Slice 20c — this read-only probe finds already enrolled
            // kinds. `write_inner` separately treats every prospective enrolment
            // as pending, then notifies only after the batch transaction commits.
            if kind_is_vector_indexed(connection, kind)? {
                jobs.push(ProjectionJob {
                    cursor: 0,
                    kind: kind.clone(),
                    body: body.clone(),
                    generation_id: generation_id.clone(),
                });
            }
        }
    }
    Ok(jobs)
}

pub(crate) fn validate_write(
    connection: &Connection,
    write: &PreparedWrite,
) -> Result<WritePlan, EngineError> {
    match write {
        PreparedWrite::ProvenancedNode(node) => {
            validate_write(
                connection,
                &PreparedWrite::Node {
                    kind: node.kind.clone(),
                    body: node.body.clone(),
                    source_id: node.source_id.clone(),
                    logical_id: node.logical_id.clone(),
                    state: node.state,
                    reason: node.reason.clone(),
                    valid_from: node.valid_from,
                    valid_until: node.valid_until,
                },
            )?;
            if node.provenance.schema_version != 1 {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::UnsupportedSchemaVersion,
                    "/provenance/schemaVersion",
                )
                .into());
            }
            Ok(WritePlan::Node)
        }
        PreparedWrite::ProvenancedEdge(edge) => {
            if edge.provenance.role != ProvenanceRole::Derived {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::RoleInvalid,
                    "/provenance/role",
                )
                .into());
            }
            validate_write(
                connection,
                &PreparedWrite::Edge {
                    kind: edge.kind.clone(),
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    source_id: edge.source_id.clone(),
                    logical_id: edge.logical_id.clone(),
                    body: edge.body.clone(),
                    t_valid: edge.t_valid,
                    t_invalid: edge.t_invalid,
                    confidence: edge.confidence,
                    extractor_model_id: edge.extractor_model_id.clone(),
                    temporal_fallback: edge.temporal_fallback,
                },
            )?;
            if edge.provenance.schema_version != 1 {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::UnsupportedSchemaVersion,
                    "/provenance/schemaVersion",
                )
                .into());
            }
            Ok(WritePlan::Edge)
        }
        PreparedWrite::Node { kind, body, logical_id, valid_from, valid_until, .. } => {
            if kind.trim().is_empty() || body.trim().is_empty() {
                return Err(EngineError::WriteValidation);
            }
            // 0.8.20 Slice 15b (TC-34) — the validity window is HALF-OPEN
            // `[valid_from, valid_until)`, so a pair with `from >= until` selects
            // no instant at all: the row would be written but no default read
            // could ever return it. Silently accepting that is a trap, so it is a
            // typed refusal.
            //
            // 0.8.20 Slice 22 (R-20-VC) — **decision #18, SETTLED: one family.**
            // This site used to return `EngineError::InvalidArgument { msg }`
            // carrying both bounds, which made `validate_write` — ONE function —
            // reject across TWO error families, so the same `write` call raised
            // `InvalidArgumentError` for an inverted window and
            // `WriteValidationError` for a non-integer bound. `dev/design/errors.md`
            // (status: locked) defines `WriteValidationError` as "malformed typed
            // write shape" / "the submitted typed write is malformed **before**
            // schema-sensitive payload checks run" — which is exactly this
            // boundary — so the code now agrees with the taxonomy of record.
            // `InvalidArgument` stays the family for caller-argument rejections
            // OUTSIDE this boundary (see the errors.md 2026-07-28 amendment).
            //
            // **The cost, stated:** `WriteValidation` is a UNIT variant and both
            // bindings map it to a fixed message-less string, so the offending
            // bounds are no longer recoverable from the error. That is a breaking
            // behaviour change on a published surface (CHANGELOG 0.8.20) and it is
            // the diagnostic the prior split existed to preserve. Restoring it
            // needs a message-carrying `WriteValidation { msg }`, which is a
            // cross-cutting change across every engine + binding raise site and
            // both binding payload shapes — its own slice, not this one.
            //
            // Only the PAIR can be empty. A one-sided window is unbounded on the
            // missing side and can never be empty, so it is never refused.
            if let (Some(from), Some(until)) = (valid_from, valid_until) {
                if from >= until {
                    return Err(EngineError::WriteValidation);
                }
            }
            // R-20-E3: `source_id` needs no emptiness check here — `SourceId`
            // cannot hold an empty or reserved id, so the check has moved from
            // this branch into the type's constructor.
            // G0 — an explicit logical_id must be non-empty (NULL/None is the
            // legacy default; an empty string is never a valid identity).
            // Also reject char(30) = \x1e (ASCII RS), which is the BFS cycle-guard
            // delimiter; allowing it would corrupt the visited-path substring test.
            if let Some(logical_id) = logical_id {
                if logical_id.is_empty() || logical_id.contains('\x1e') {
                    return Err(EngineError::WriteValidation);
                }
            }
            Ok(WritePlan::Node)
        }
        PreparedWrite::Edge { kind, from, to, logical_id, t_valid, t_invalid, .. } => {
            if kind.trim().is_empty() || from.trim().is_empty() || to.trim().is_empty() {
                return Err(EngineError::WriteValidation);
            }
            // Reject char(30) in from/to: these become from_id/to_id in canonical_edges
            // and appear in BFS visited strings — an \x1e there would corrupt the guard.
            if from.contains('\x1e') || to.contains('\x1e') {
                return Err(EngineError::WriteValidation);
            }
            // R-20-E3: see the Node branch — emptiness is a `SourceId` invariant.
            if let Some(logical_id) = logical_id {
                if logical_id.is_empty() || logical_id.contains('\x1e') {
                    return Err(EngineError::WriteValidation);
                }
            }
            // TC-33 fix-1 (codex §9 P2) — an epoch SQLite cannot render to
            // ISO-8601 must be UNSTORABLE. The governed integer surface is the
            // only way to reach one (inbound ISO normalisation maxes at year
            // 9999), so this write boundary is where it is stopped, before it
            // can render to a silent `null` on the consolidation wire and
            // resurrect an invalidated edge. Structural primary layer; the
            // render site keeps a defensive hard-assert as the backstop.
            reject_unrenderable_edge_epoch("t_valid", *t_valid)?;
            reject_unrenderable_edge_epoch("t_invalid", *t_invalid)?;
            Ok(WritePlan::Edge)
        }
        PreparedWrite::AdminSchema { name, kind, schema_json, retention_json } => {
            if name.trim().is_empty()
                || !matches!(kind.as_str(), "append_only_log" | "latest_state")
                || serde_json::from_str::<Value>(schema_json).is_err()
                || serde_json::from_str::<Value>(retention_json).is_err()
                || contains_external_ref(schema_json)
            {
                return Err(EngineError::SchemaValidation);
            }
            Ok(WritePlan::AdminSchema)
        }
        PreparedWrite::OpStore { collection, record_key, schema_id, body } => {
            if collection.trim().is_empty() || record_key.trim().is_empty() {
                return Err(EngineError::WriteValidation);
            }
            let (kind, schema_json) = collection_metadata(connection, collection)?;
            if let Some(schema_id) = schema_id {
                if schema_id != collection {
                    return Err(EngineError::SchemaValidation);
                }
                validate_payload(&schema_json, body)?;
            } else if serde_json::from_str::<Value>(body).is_err() {
                return Err(EngineError::SchemaValidation);
            }

            match kind.as_str() {
                "append_only_log" => Ok(WritePlan::AppendOnlyLog),
                "latest_state" => Ok(WritePlan::LatestState),
                _ => Err(EngineError::OpStore),
            }
        }
    }
}

fn collection_metadata(
    connection: &Connection,
    collection: &str,
) -> Result<(String, String), EngineError> {
    connection
        .query_row(
            "SELECT kind, schema_json FROM operational_collections WHERE name = ?1",
            [collection],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| EngineError::OpStore)
}

fn validate_payload(schema_json: &str, body: &str) -> Result<(), EngineError> {
    let schema =
        serde_json::from_str::<Value>(schema_json).map_err(|_| EngineError::SchemaValidation)?;
    let payload = serde_json::from_str::<Value>(body).map_err(|_| EngineError::SchemaValidation)?;

    let compiled = JSONSchema::compile(&schema).map_err(|_| EngineError::SchemaValidation)?;
    compiled.validate(&payload).map_err(|_| EngineError::SchemaValidation)?;

    Ok(())
}

fn contains_external_ref(schema_json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(schema_json) else {
        return false;
    };
    value_contains_external_ref(&value)
}

fn value_contains_external_ref(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            if key == "$ref" {
                return value.as_str().is_some_and(|uri| !uri.starts_with('#'));
            }
            value_contains_external_ref(value)
        }),
        Value::Array(values) => values.iter().any(value_contains_external_ref),
        _ => false,
    }
}

// fix-30 [P2]: helpers to collect active edge write_cursors BEFORE a supersession
// UPDATE so the callers can prune stale vector_default rows.
pub(crate) fn prior_edge_cursors_by_logical_id(
    tx: &Connection,
    logical_id: &str,
) -> rusqlite::Result<Vec<i64>> {
    let mut s = tx.prepare_cached(
        "SELECT write_cursor FROM canonical_edges \
         WHERE logical_id = ?1 AND superseded_at IS NULL",
    )?;
    let rows = s.query_map(params![logical_id], |r| r.get(0))?;
    rows.collect()
}

/// 0.8.20 Slice 15d fix-1 finding 2 [P2] — the active (non-superseded) NODE
/// cursors for a `logical_id`, collected BEFORE the tombstone-then-insert
/// supersession UPDATE so the caller can purge the about-to-be-superseded row's
/// row-owned attribute projections. Mirrors [`prior_edge_cursors_by_logical_id`].
/// The partial-unique-active index means this is at most one cursor; a `Vec`
/// keeps it robust and symmetric with the edge path.
pub(crate) fn prior_node_cursors_by_logical_id(
    tx: &Connection,
    logical_id: &str,
) -> rusqlite::Result<Vec<i64>> {
    let mut s = tx.prepare_cached(
        "SELECT write_cursor FROM canonical_nodes \
         WHERE logical_id = ?1 AND superseded_at IS NULL",
    )?;
    let rows = s.query_map(params![logical_id], |r| r.get(0))?;
    rows.collect()
}

pub(crate) fn prior_edge_cursors_by_triple(
    tx: &Connection,
    from: &str,
    to: &str,
    kind: &str,
) -> rusqlite::Result<Vec<i64>> {
    let mut s = tx.prepare_cached(
        "SELECT write_cursor FROM canonical_edges \
         WHERE from_id = ?1 AND to_id = ?2 AND kind = ?3 AND superseded_at IS NULL",
    )?;
    let rows = s.query_map(params![from, to, kind], |r| r.get(0))?;
    rows.collect()
}
