use super::*;

/// G11 (Slice 15) — a document sent to a BYO-LLM extraction harness via
/// [`Engine::ingest_with_extractor`].
#[derive(Clone, Debug)]
pub struct ExtractDocument {
    /// Stable opaque identifier for this document. Used as `source_id` on
    /// ingested edges and for provenance tracking.
    pub source_doc_id: String,
    /// Full text body of the document to extract entities and relationships from.
    pub body: String,
}

/// G11 (Slice 15) — receipt returned by [`Engine::ingest_with_extractor`].
#[derive(Clone, Debug, Default)]
pub struct IngestWithExtractorReceipt {
    /// Number of `canonical_nodes` rows written (new entity insertions; skipped
    /// for entities that already have a matching active logical_id).
    pub nodes_written: u64,
    /// Number of `canonical_edges` rows written (new fact-edge insertions;
    /// superseded prior edges are ALSO counted as rows written).
    pub edges_written: u64,
    /// Number of documents processed (including no-facts documents).
    pub docs_processed: u64,
}

impl Engine {
    /// G11 (Slice 15) — BYO-LLM ingest: spawn an external extraction harness
    /// speaking the `fathomdb.extract.v1` NDJSON-over-stdio protocol, send
    /// documents for extraction, and write the resulting entities
    /// (→ `canonical_nodes`) and fact-edges (→ `canonical_edges` with G11
    /// enrichment columns) to the store.
    ///
    /// `cmd` is argv (first element = program, rest = args). Documents are
    /// batched per the harness's `max_docs_per_request`. Entity `logical_id`
    /// is derived as `sha256("<type>:<name>")` (lowercase, hex-encoded) for
    /// stable cross-re-ingestion identity. Edge `logical_id` is derived as
    /// `sha256("<from_lid>:<to_lid>:<relation>")`. Both are consistent with
    /// G0 supersession: re-ingesting the same document yields the same ids,
    /// triggering tombstone-then-insert rather than accumulation.
    ///
    /// Returns [`EngineError::Extractor`] on protocol errors (bad handshake,
    /// subprocess spawn failure, JSON decode error). `no_facts` warnings from
    /// the harness are not errors and do not affect the receipt counts.
    pub fn ingest_with_extractor(
        &self,
        cmd: &[&str],
        documents: &[ExtractDocument],
    ) -> Result<IngestWithExtractorReceipt, EngineError> {
        // 0.8.6 Slice 5 (ADR-0.8.6): the spawn + hello/ready handshake +
        // request_id framing + error mapping now live in the reusable
        // `provider_session` transport seam, parameterized by `ProviderTask`.
        // `ingest_with_extractor` is the thin extract caller: it opens a session
        // for `ProviderTask::Extract` then runs the extract-specific payload
        // build + DB writes. The session owns child reaping via Drop.
        let mut session = self.provider_session(ProviderTask::Extract, cmd)?;
        self.run_extract_session(&mut session, documents)
    }

    /// 0.8.6 Slice 5 — extract-specific driver over a `ProviderSession`. The
    /// payload build (documents → entities/edges) and DB writes are byte-identical
    /// to the pre-0.8.6 inner loop; only the spawn/handshake/framing moved into
    /// the shared session.
    fn run_extract_session(
        &self,
        session: &mut ProviderSession,
        documents: &[ExtractDocument],
    ) -> Result<IngestWithExtractorReceipt, EngineError> {
        let extractor_model_id = session.model.clone();
        let max_docs = session.max_docs_per_request;

        // --- per-batch extract → write loop ---
        let mut nodes_written: u64 = 0;
        let mut edges_written: u64 = 0;
        let docs_processed = documents.len() as u64;

        for (batch_idx, batch) in documents.chunks(max_docs).enumerate() {
            let request_id = format!("req-{batch_idx}");
            let docs_json: Vec<Value> = batch
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "source_doc_id": d.source_doc_id,
                        "body": d.body,
                    })
                })
                .collect();

            // Send the framed extract request and receive its matching `result`.
            // The session adds protocol/type/request_id and validates the
            // type=="result" + matching request_id envelope (fix-24 [P2]).
            let result = session
                .request(&request_id, vec![("documents".to_string(), Value::Array(docs_json))])?;

            // R-20-E2 (0.8.20 Slice 5c, design §4 item 10) — every row this batch
            // produces takes its provenance from the CALLER's
            // `ExtractDocument.source_doc_id`, NEVER from the model's echo of that
            // field. The echo is attacker-/error-controlled: a harness that omits
            // it used to yield rows with NULL `source_id`, which no
            // `excise_source` call can reach — the model could make a row
            // permanently un-erasable simply by dropping a key.
            //
            // `resolve_provenance` therefore admits the echo only as a SELECTOR
            // among ids the caller already supplied in THIS batch, and never as a
            // value:
            //
            //   * single-document batch — attribution is unambiguous, so the
            //     caller's id is used and the echo is ignored outright;
            //   * multi-document batch — the echo must name one of the batch's
            //     caller-supplied ids (the caller's own copy of the string is
            //     then stored). An absent or unrecognised echo is a protocol
            //     violation and fails the ingest LOUDLY with
            //     `EngineError::Extractor`, because the alternative — guessing an
            //     attribution — would silently mis-file the row under a document
            //     whose erasure would then not remove it.
            let batch_provenance = batch
                .iter()
                .map(|d| SourceId::new(d.source_doc_id.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            let resolve_provenance = |echo: Option<&str>| -> Result<SourceId, EngineError> {
                if let [only] = batch_provenance.as_slice() {
                    return Ok(only.clone());
                }
                let echo = echo.ok_or(EngineError::Extractor)?;
                batch_provenance
                    .iter()
                    .find(|caller_id| caller_id.as_str() == echo)
                    .cloned()
                    .ok_or(EngineError::Extractor)
            };

            // --- map entities → PreparedWrite::Node with stable logical_id ---
            let entities =
                result.get("entities").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let raw_edges =
                result.get("edges").and_then(|v| v.as_array()).cloned().unwrap_or_default();

            // R3 (SCHEMA-GATE-1): collect substituted_t_valid values from
            // temporal_fallback warnings. An edge whose t_valid matches one of
            // these values had its event time defaulted to created_at (not
            // text-grounded) and must be flagged so BFS can exclude it.
            //
            // TC-33: kept as RAW `Value`s here and normalised below, together
            // with the edge side, through the SAME function. See the
            // normalisation block for why that is load-bearing.
            let raw_fallback_dates: Vec<&Value> = result
                .get("warnings")
                .and_then(|v| v.as_array())
                .map(|ws| {
                    ws.iter()
                        .filter(|w| {
                            w.get("kind").and_then(|k| k.as_str()) == Some("temporal_fallback")
                        })
                        .filter_map(|w| w.get("substituted_t_valid"))
                        .collect()
                })
                .unwrap_or_default();

            if !entities.is_empty() {
                let node_batch: Vec<PreparedWrite> = entities
                    .iter()
                    .map(|entity| -> Result<PreparedWrite, EngineError> {
                        let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let kind = entity.get("type").and_then(|v| v.as_str()).unwrap_or("entity");
                        // R-20-E2: caller-grounded, echo used only as a selector.
                        let source_doc_id = resolve_provenance(
                            entity.get("source_doc_id").and_then(|v| v.as_str()),
                        )?;
                        // fix-34 [P1]: derive_logical_id now rejects an empty name
                        // or a ':' in kind — inputs that would collide distinct
                        // entities onto one identity and silently drop one.
                        let logical_id = derive_logical_id(kind, name)?;
                        Ok(PreparedWrite::Node {
                            kind: kind.to_string(),
                            body: name.to_string(),
                            source_id: source_doc_id,
                            logical_id: Some(logical_id),
                            state: InitialState::Active,
                            reason: None,
                            valid_from: None,
                            valid_until: None,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // fix-29/fix-34 [P2]: deduplicate within the batch by logical_id so
                // a harness that returns the same entity twice does not write a row
                // that immediately supersedes its sibling (shared with the edge arm).
                let node_batch = dedup_prepared_by_logical_id(node_batch);

                // fix-23 [P2]: skip entities whose logical_id is already active
                // to avoid needless supersede churn on re-ingest.
                let ids: Vec<String> = node_batch
                    .iter()
                    .filter_map(|w| {
                        if let PreparedWrite::Node { logical_id: Some(id), .. } = w {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let existing: std::collections::HashSet<String> = self
                    // Internal existence probe: STRICT view — this must see
                    // exactly the rows the pre-slice code saw.
                    .read_get_many(&ids, &ReadView::default())?
                    .into_iter()
                    .zip(ids)
                    .filter_map(|(opt, id)| opt.map(|_| id))
                    .collect();
                let new_nodes: Vec<PreparedWrite> = node_batch
                    .into_iter()
                    .filter(|w| {
                        if let PreparedWrite::Node { logical_id: Some(id), .. } = w {
                            !existing.contains(id)
                        } else {
                            true
                        }
                    })
                    .collect();
                if !new_nodes.is_empty() {
                    let n = new_nodes.len() as u64;
                    self.write(&new_nodes)?;
                    nodes_written = nodes_written.saturating_add(n);
                }
            }

            // --- map edges → PreparedWrite::Edge with G11 columns ---
            if !raw_edges.is_empty() {
                // fix-33 [P1]: the protocol gives edges NO endpoint types —
                // `from_entity`/`to_entity` reference entities BY NAME (or alias).
                // Build a name+alias → (canonical name, type) index from the same
                // result's `entities[]` so each endpoint's logical_id matches the
                // node's. (Nodes derive id from the entity's real type; defaulting
                // the edge endpoint kind to "entity" orphaned every contract-faithful
                // edge from its nodes and tripped the G8 dangling probe.)
                //
                // Two passes so a canonical NAME always wins over a (different
                // entity's) ALIAS regardless of `entities[]` order: pass 1 inserts
                // all canonical names, pass 2 fills aliases only where no name
                // already claims that key. (Name↔name clashes remain first-wins —
                // contradictory input; no principled resolution exists.)
                let mut entity_index: std::collections::HashMap<String, (String, String)> =
                    std::collections::HashMap::new();
                for entity in &entities {
                    let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let kind =
                        entity.get("type").and_then(|v| v.as_str()).unwrap_or("entity").to_string();
                    entity_index
                        .entry(name.to_lowercase())
                        .or_insert_with(|| (name.to_string(), kind));
                }
                for entity in &entities {
                    let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let kind =
                        entity.get("type").and_then(|v| v.as_str()).unwrap_or("entity").to_string();
                    if let Some(aliases) = entity.get("aliases").and_then(|v| v.as_array()) {
                        for alias in aliases.iter().filter_map(|a| a.as_str()) {
                            if !alias.is_empty() {
                                entity_index
                                    .entry(alias.to_lowercase())
                                    .or_insert_with(|| (name.to_string(), kind.clone()));
                            }
                        }
                    }
                }

                // TC-33 — normalise EVERY extractor timestamp here, in ONE pass,
                // under ONE connection lock, BEFORE any edge is built. Both the
                // edge side (`t_valid`/`t_invalid`) and the temporal_fallback
                // warning side (`substituted_t_valid`) go through the SAME
                // function, and any value that cannot be normalised HARD-REJECTS
                // the whole ingest.
                //
                // **Normalising both sides is load-bearing, and nothing would
                // have caught it.** `temporal_fallback` is decided by comparing
                // the edge's t_valid against the warnings' substituted_t_valid.
                // That was a RAW BYTE-FOR-BYTE STRING MATCH with
                // `.unwrap_or(false)` on the miss path, and `substituted_t_valid`
                // is a FREE-FORM JSON key on the ELPS warnings envelope, not a
                // Rust struct field. So normalising only the edge side would
                // leave the set never matching, `.unwrap_or(false)` firing, and
                // EVERY fallback edge silently becoming a TRUSTED edge — with no
                // compile error anywhere. That flag is the only thing excluding
                // untrustworthy-time edges from graph BFS and graph seeding.
                //
                // Normalising both sides also FIXES a pre-existing brittleness:
                // `2025-03-20T09:30:00Z` and `2025-03-20T09:30:00+00:00` are the
                // same instant but MISS each other under a byte comparison. They
                // now compare equal as epochs.
                //
                // A malformed `substituted_t_valid` rejects rather than being
                // skipped: skipping it would leave the edge unflagged, i.e.
                // treated as TRUSTED — the same fail-open in a different place.
                //
                // The lock is taken and released HERE; `self.write(...)` below
                // re-acquires it, so no lock is held across the write.
                // (t_valid, t_invalid) epoch pair per edge, in `raw_edges` order.
                type EdgeTimes = Vec<(Option<i64>, Option<i64>)>;
                let (edge_times, fallback_epochs): (EdgeTimes, std::collections::HashSet<i64>) = {
                    let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                    let connection = connection.as_ref().ok_or(EngineError::Closing)?;

                    let mut times = Vec::with_capacity(raw_edges.len());
                    for edge in &raw_edges {
                        times.push((
                            normalize_extractor_timestamp(
                                connection,
                                "t_valid",
                                edge.get("t_valid"),
                            )?,
                            normalize_extractor_timestamp(
                                connection,
                                "t_invalid",
                                edge.get("t_invalid"),
                            )?,
                        ));
                    }

                    let mut epochs = std::collections::HashSet::new();
                    for raw in &raw_fallback_dates {
                        if let Some(epoch) = normalize_extractor_timestamp(
                            connection,
                            "substituted_t_valid",
                            Some(raw),
                        )? {
                            epochs.insert(epoch);
                        }
                    }
                    (times, epochs)
                };

                let edge_batch: Vec<PreparedWrite> = raw_edges
                    .iter()
                    .zip(&edge_times)
                    .map(|(edge, &(t_valid, t_invalid))| -> Result<PreparedWrite, EngineError> {
                        let from_entity =
                            edge.get("from_entity").and_then(|v| v.as_str()).unwrap_or("");
                        let to_entity =
                            edge.get("to_entity").and_then(|v| v.as_str()).unwrap_or("");
                        let relation =
                            edge.get("relation").and_then(|v| v.as_str()).unwrap_or("related_to");
                        let body = edge.get("body").and_then(|v| v.as_str()).map(str::to_string);
                        // TC-33: `t_valid`/`t_invalid` were normalised (and any
                        // malformed or non-string value hard-rejected) in the
                        // pass above; they arrive here as epoch seconds.
                        // fix-26 [P2]: validate confidence is in [0.0, 1.0] at the
                        // protocol boundary; reject out-of-range values.
                        let confidence = match edge.get("confidence").and_then(|v| v.as_f64()) {
                            Some(c) if !(0.0..=1.0).contains(&c) => {
                                return Err(EngineError::Extractor);
                            }
                            c => c,
                        };
                        // R-20-E2: caller-grounded, echo used only as a selector.
                        let source_doc_id =
                            resolve_provenance(edge.get("source_doc_id").and_then(|v| v.as_str()))?;

                        // fix-33 [P1]: resolve each endpoint via the entities[]
                        // index (by name or alias) → the entity's canonical
                        // (name, type); fall back to kind "entity" only for a truly
                        // unlisted name (synthesized dangling endpoints ARE listed,
                        // so this is the defensive path). derive_logical_id (fix-34)
                        // still rejects an empty name / ':' in kind.
                        let (from_name, from_kind) = entity_index
                            .get(&from_entity.to_lowercase())
                            .cloned()
                            .unwrap_or_else(|| (from_entity.to_string(), "entity".to_string()));
                        let (to_name, to_kind) = entity_index
                            .get(&to_entity.to_lowercase())
                            .cloned()
                            .unwrap_or_else(|| (to_entity.to_string(), "entity".to_string()));
                        let from_lid = derive_logical_id(&from_kind, &from_name)?;
                        let to_lid = derive_logical_id(&to_kind, &to_name)?;
                        let edge_key = format!("{from_lid}:{to_lid}:{relation}");
                        let edge_lid = derive_logical_id("edge", &edge_key)?;

                        // TC-33: BOTH sides are now epochs from the SAME
                        // normalisation, so this compares instants rather than
                        // byte strings.
                        let is_temporal_fallback =
                            t_valid.is_some_and(|tv| fallback_epochs.contains(&tv));
                        Ok(PreparedWrite::Edge {
                            kind: relation.to_string(),
                            from: from_lid,
                            to: to_lid,
                            source_id: source_doc_id,
                            logical_id: Some(edge_lid),
                            body,
                            t_valid,
                            t_invalid,
                            confidence,
                            extractor_model_id: extractor_model_id.clone(),
                            temporal_fallback: if is_temporal_fallback { Some(true) } else { None },
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                // fix-34 [P2]: dedup edges by logical_id, mirroring the node arm
                // (fix-29) — a duplicate edge in one harness response would
                // otherwise write a row that immediately supersedes its sibling.
                let edge_batch = dedup_prepared_by_logical_id(edge_batch);
                let n = edge_batch.len() as u64;
                self.write(&edge_batch)?;
                edges_written = edges_written.saturating_add(n);
            }
        }

        // The `ProviderSession` (and its writer/child) is dropped by the caller
        // when `ingest_with_extractor` returns: Drop sends stdin EOF and reaps
        // the child, matching the prior explicit drop(writer)+kill/wait.
        Ok(IngestWithExtractorReceipt { nodes_written, edges_written, docs_processed })
    }
}

/// fix-34 [P2]: dedup a batch of [`PreparedWrite`]s by `logical_id`, keeping the
/// first occurrence. Shared by the entity and edge arms of the BYO-LLM ingest
/// path so a harness that returns the same node/edge twice in one response does
/// not write a row that immediately supersedes its sibling.
///
/// **TC-32 (0.8.20) — single-provenance entity dedupe is INTENTIONAL and
/// ACCEPTED.** Because dedupe keeps the FIRST occurrence, same-name entities
/// collapse onto one `logical_id` row that carries only the FIRST document's
/// `source_id`; erasing a later document therefore does not remove the shared
/// entity row. The HITL has ruled this acceptable for now and explicitly
/// declined a multi-source-provenance model. Tracked as TC-32 — do not "fix"
/// this by changing dedupe behaviour without a fresh decision.
fn dedup_prepared_by_logical_id(batch: Vec<PreparedWrite>) -> Vec<PreparedWrite> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    batch
        .into_iter()
        .filter(|w| match w {
            PreparedWrite::Node { logical_id: Some(id), .. }
            | PreparedWrite::Edge { logical_id: Some(id), .. } => seen.insert(id.clone()),
            _ => true,
        })
        .collect()
}
