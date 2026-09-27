use super::*;

pub(crate) fn structural_dependency_state(
    tx: &Connection,
    write_cursor: u64,
    effective_at: i64,
) -> rusqlite::Result<StructuralDependencyStateV1> {
    let owner: Option<(String, String)> = tx
        .query_row(
            "SELECT artifact_role, completeness FROM _fathomdb_artifact_revisions \
             WHERE write_cursor=?1 AND schema_version=1",
            [i64::try_from(write_cursor).map_err(|_| rusqlite::Error::InvalidQuery)?],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((role, completeness)) = owner else {
        return Ok(StructuralDependencyStateV1::NotApplicable);
    };
    if role != "derived_semantic" || completeness != "complete" {
        return Ok(StructuralDependencyStateV1::NotApplicable);
    }
    let registered =
        dependency_trace::registered_dependency_for_cursor(tx, write_cursor, effective_at)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(if registered {
        StructuralDependencyStateV1::Registered
    } else {
        StructuralDependencyStateV1::NotRegistered
    })
}

pub(crate) fn structural_lifecycle_state(
    tx: &Connection,
    write_cursor: u64,
) -> rusqlite::Result<StructuralLifecycleStateV1> {
    let cursor = i64::try_from(write_cursor).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let edge_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
        [cursor],
        |row| row.get(0),
    )?;
    if edge_exists {
        return Ok(StructuralLifecycleStateV1::EdgeValid);
    }
    let state: String =
        tx.query_row("SELECT state FROM canonical_nodes WHERE write_cursor=?1", [cursor], |row| {
            row.get(0)
        })?;
    match state.as_str() {
        "pending" => Ok(StructuralLifecycleStateV1::NodePending),
        "active" => Ok(StructuralLifecycleStateV1::NodeActive),
        "deleted" => Ok(StructuralLifecycleStateV1::NodeDeleted),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

/// Read projection cursor and matching body rows inside one read tx.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_projected_text_in_tx(
    reader: &mut Connection,
    query: &str,
    name: &str,
    filter: Option<&SearchFilter>,
    limit: usize,
    view: ReadView,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> ProjectedTextReaderResponse {
    let compiled = compile_text_query(query);
    let frozen = view.freeze();
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    let registry = load_projection_registry(&tx)?;
    let declared = registry.get(name).ok_or_else(|| {
        SearchReaderError::InvalidFilter(format!("projected text field {name:?} is not declared"))
    })?;
    if !declared.wants_property_fts() {
        return Err(SearchReaderError::InvalidFilter(format!(
            "projected text field {name:?} is not a declared `searchable` projection with property FTS"
        )));
    }
    if let Some(filter) = filter {
        validate_filter_attributes_on_snapshot(&tx, filter)?;
    }
    let node_predicate = frozen.node_sql("n", 3);
    let mut params = vec![
        rusqlite::types::Value::Text(name.to_string()),
        rusqlite::types::Value::Text(compiled.match_expression),
    ];
    if let Some(now) = frozen.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
    }
    let filter_predicate = append_node_eligibility_sql(filter, "n", &mut params);
    let sql = property_fts_rank_sql(&node_predicate, &filter_predicate);
    let mut stmt = tx.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        let (cursor, bm25, kind, body, logical_id, source_id) = row?;
        results.push(SearchHit {
            id: derive_stable_id(logical_id.as_deref(), &body),
            write_cursor: cursor as u64,
            kind,
            body,
            score: -bm25,
            branch: SoftFallbackBranch::Text,
            source_id,
            ce_score: None,
        });
        if results.len() >= limit {
            break;
        }
    }
    let projection_cursor = load_projection_cursor(&tx)?;
    Ok(SearchResult { projection_cursor, soft_fallback: None, results, explanation: None })
}

// The 8th parameter (`vector_stage_only`) is the additive GA-2 / ◆ B-1
// measurement seam; the reader-worker call site threads each field through
// explicitly (mirroring the existing `recency_enabled` plumbing), so a wrapper
// struct would only obscure that 1:1 mapping for a test-only flag.
#[derive(Clone, Debug)]
pub(crate) enum CapturedGraphOrigin {
    EntitySeed,
    EdgeSeed { edge_cursor: u64 },
    Traversal { edge_cursor: u64, hop_count: u32 },
}

pub(crate) trait SearchOriginCapture: Sized {
    type Output;

    fn record_graph_origin(&mut self, _node_cursor: u64, _origin: CapturedGraphOrigin) {}

    #[allow(clippy::too_many_arguments)]
    fn finish(
        self,
        connection: &Connection,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<SearchHit>,
        graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        expanded: Option<SearchExpandResult>,
    ) -> Result<Self::Output, SearchReaderError>;
}

pub(crate) struct NoEvidenceCapture;

// Ordinary search retains a zero-sized capture strategy: evidence state and
// provenance collection are absent unless the explicit evidence operation is used.
const _: () = assert!(std::mem::size_of::<NoEvidenceCapture>() == 0);

pub(crate) struct EvidenceCapture {
    pub(crate) frozen: FrozenReadContextV1,
    pub(crate) include_explanation: bool,
    pub(crate) graph_origins: HashMap<u64, CapturedGraphOrigin>,
}

pub(crate) fn read_search_work_in_tx<C: SearchOriginCapture>(
    reader: &mut Connection,
    work: SearchReaderWork,
    capture: C,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<C::Output, SearchReaderError> {
    read_search_in_tx(
        reader,
        work.compiled.as_ref(),
        work.query_vector.as_deref(),
        work.query_vector_bin.as_deref(),
        work.result_limit,
        work.candidate_limit,
        work.direct_text_candidate_limit,
        work.filter.as_deref(),
        work.recency_enabled,
        work.importance_enabled,
        work.vector_stage_only,
        &work.raw_query,
        work.rerank_depth,
        work.use_graph_arm,
        work.alpha,
        work.pool_n,
        work.explain,
        work.projection_runtime_state,
        work.view,
        work.frozen_binding.as_deref(),
        work.frozen_query_runtime.as_deref(),
        work.expand_depth,
        attribution,
        worker_idx,
        capture,
    )
}

pub(crate) struct SearchStatement<'connection> {
    statement: CachedStatement<'connection>,
}

pub(crate) fn prepare_search_statement<'connection>(
    connection: &'connection Connection,
    sql: &str,
) -> rusqlite::Result<SearchStatement<'connection>> {
    connection.prepare_cached(sql).map(|statement| SearchStatement { statement })
}

pub(crate) fn load_projection_cursor_for_search(connection: &Connection) -> rusqlite::Result<u64> {
    prepare_search_statement(connection, "SELECT value FROM _fathomdb_open_state WHERE key = ?1")?
        .query_row([PROJECTION_CURSOR_KEY], |row| row.get::<_, String>(0))
        .map(|value| value.parse::<u64>().unwrap_or(0))
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(0),
            _ => Err(err),
        })
}

pub(crate) fn rank_search_hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
    let body = row.get::<_, String>(0)?;
    let logical_id = row.get::<_, Option<String>>(4)?;
    Ok(SearchHit {
        id: derive_stable_id(logical_id.as_deref(), &body),
        body,
        kind: row.get::<_, String>(1)?,
        write_cursor: row.get::<_, i64>(2)? as u64,
        score: row.get::<_, f64>(3)?,
        branch: SoftFallbackBranch::Text,
        source_id: row.get::<_, Option<String>>(5)?,
        ce_score: None,
    })
}

pub(crate) fn collect_complete_rank_boundary(
    rows: &mut rusqlite::Rows<'_>,
    limit: usize,
) -> rusqlite::Result<(Vec<SearchHit>, bool)> {
    let mut candidates = Vec::with_capacity(limit.saturating_add(1));
    let mut boundary_score = None;

    while let Some(row) = rows.next()? {
        let candidate = rank_search_hit_from_row(row)?;
        if candidates.len() < limit {
            if candidates.len().saturating_add(1) == limit {
                boundary_score = Some(candidate.score);
            }
            candidates.push(candidate);
            continue;
        }
        if boundary_score.is_some_and(|score: f64| {
            score.total_cmp(&candidate.score) == std::cmp::Ordering::Equal
        }) {
            candidates.push(candidate);
            continue;
        }
        break;
    }

    let crossed_boundary_tie = candidates.len() > limit;
    candidates = retain_complete_rank_boundary_candidates(candidates, limit);
    Ok((candidates, crossed_boundary_tie))
}

pub(crate) fn retain_complete_rank_boundary_candidates(
    mut candidates: Vec<SearchHit>,
    limit: usize,
) -> Vec<SearchHit> {
    candidates.sort_by(|left, right| {
        left.score.total_cmp(&right.score).then_with(|| left.write_cursor.cmp(&right.write_cursor))
    });
    candidates.truncate(limit);
    candidates
}

#[cfg(feature = "test-hooks")]
pub(crate) fn append_json_witness_for_test(variable: &str, value: &serde_json::Value) {
    let Some(path) = std::env::var_os(variable) else {
        return;
    };
    static WITNESS_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();
    let Ok(_guard) = WITNESS_LOCK.get_or_init(|| Mutex::new(())).lock() else {
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{value}");
    }
}

#[cfg(feature = "test-hooks")]
pub(crate) fn record_fts_route_for_test(route: &str) {
    append_json_witness_for_test(
        "FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST",
        &serde_json::json!({"route": route}),
    );
}

#[cfg(feature = "test-hooks")]
pub(crate) fn slice71_search_statement_trace() -> &'static Mutex<Vec<String>> {
    static TRACE: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
    TRACE.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "test-hooks")]
pub(crate) fn record_slice71_profile_statement_for_test(sql: &str) {
    let identity = if sql
        .contains("SELECT l.source_revision_id FROM _fathomdb_artifact_revisions r")
        && sql.contains("WHERE r.write_cursor=")
    {
        Some("post_filter_source_lookup")
    } else {
        None
    };
    if let Some(identity) = identity {
        if let Ok(mut trace) = slice71_search_statement_trace().lock() {
            trace.push(identity.to_string());
        }
    }
}

#[cfg(feature = "test-hooks")]
pub(crate) fn record_fts_query_plan_for_test(
    transaction: &rusqlite::Transaction<'_>,
    statement: &str,
    parameters: &[rusqlite::types::Value],
) {
    let explain = format!("EXPLAIN QUERY PLAN {statement}");
    let observation = transaction
        .prepare(&explain)
        .and_then(|mut prepared| {
            let details = prepared
                .query_map(rusqlite::params_from_iter(parameters.iter()), |row| {
                    row.get::<_, String>(3)
                })?;
            details.collect::<rusqlite::Result<Vec<_>>>()
        })
        .map(|details| {
            serde_json::json!({
                "uses_temp_btree_for_order_by": details
                    .iter()
                    .any(|detail| detail.contains("USE TEMP B-TREE FOR ORDER BY")),
            })
        });
    if let Ok(observation) = observation {
        append_json_witness_for_test("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", &observation);
    }
}

/// F5 (0.8.14 Slice 10, fix-1) — tokenizer for the in-engine BM25F scorer.
///
/// Tokenizes `text` through the SAME FTS5 tokenizer that `search_index_v2` uses
/// for candidate recall (`porter unicode61 remove_diacritics 2`), so the scorer
/// measures term-frequency, document-frequency, field length, and average field
/// length under the index's own tokenization — porter stemming + unicode61
/// case-fold + diacritic folding. The previous implementation hand-rolled a
/// second lowercase-alnum splitter; a stemmed/diacritic variant recalled by
/// `MATCH` (e.g. query `run` vs indexed `running`, or `cafe` vs `café`) was then
/// scored as if the term were absent, so ranking was wrong for exactly those
/// variants (codex §9 fix-1 finding 1). Reusing FTS5 itself makes scoring
/// tokenization-faithful without re-implementing porter/unicode61 in Rust.
///
/// Mechanism: round-trip `text` through a temp single-column FTS5 table with the
/// identical tokenizer, then read the emitted token instances back via the
/// `fts5vocab(..., 'instance')` companion. The token multiset is returned in
/// index order (duplicates kept) so callers count tf and field length directly.
/// Query terms and every candidate field go through this one path, so all four
/// statistics are consistent with each other and with the FTS5 index the scorer
/// ranks.
fn fts5_tokenize(connection: &Connection, text: &str) -> rusqlite::Result<Vec<String>> {
    connection.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS temp.bm25f_tok
             USING fts5(t, tokenize = 'porter unicode61 remove_diacritics 2');
         CREATE VIRTUAL TABLE IF NOT EXISTS temp.bm25f_tok_vocab
             USING fts5vocab('bm25f_tok', 'instance');
         DELETE FROM temp.bm25f_tok;",
    )?;
    connection.execute("INSERT INTO temp.bm25f_tok(t) VALUES(?1)", params![text])?;
    let mut stmt =
        connection.prepare("SELECT term FROM temp.bm25f_tok_vocab ORDER BY \"offset\"")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect()
}

/// F5 (0.8.14 Slice 10) — build the FTS5 `MATCH` expression for candidate
/// recall from the query's tokens: each token is a double-quoted FTS5 string
/// (tokens are FTS5-emitted stems — unicode61 alnum, no embedded quotes),
/// OR-joined.
fn bm25f_match_expression(terms: &[String]) -> String {
    terms.iter().map(|t| format!("\"{t}\"")).collect::<Vec<_>>().join(" OR ")
}

/// F5 (0.8.14 Slice 10) — the BM25F score for one candidate document.
///
/// Standard BM25F: per query term, accumulate a length-normalized,
/// field-weighted pseudo term-frequency across the fields, then apply the BM25
/// saturation once. `norm_f = 1 - b + b*(len_f/avglen_f)` is the per-field
/// length normalization (this is where tunable `b` bites); `weight_f` is the
/// field boost (this is where the R-F5-1 field weighting bites).
fn bm25f_score_doc(
    plan: &Bm25fQueryPlan,
    query_terms: &[String],
    // (weight, doc field length, corpus avg field length, per-term tf in field)
    fields: &[(f64, f64, f64, &HashMap<String, u32>)],
    doc_count: usize,
    df: &HashMap<String, usize>,
) -> f64 {
    let mut score = 0.0_f64;
    for term in query_terms {
        let mut weighted_tf = 0.0_f64;
        for (weight, len_f, avglen_f, tf_map) in fields {
            if *weight == 0.0 || *avglen_f <= 0.0 {
                continue;
            }
            let tf = *tf_map.get(term).unwrap_or(&0) as f64;
            if tf == 0.0 {
                continue;
            }
            let norm = 1.0 - plan.b + plan.b * (len_f / avglen_f);
            if norm <= 0.0 {
                continue;
            }
            weighted_tf += weight * tf / norm;
        }
        if weighted_tf <= 0.0 {
            continue;
        }
        let dfq = *df.get(term).unwrap_or(&0);
        if dfq == 0 {
            continue;
        }
        let n = doc_count as f64;
        let idf = ((n - dfq as f64 + 0.5) / (dfq as f64 + 0.5) + 1.0).ln();
        score += idf * (weighted_tf * (plan.k1 + 1.0)) / (plan.k1 + weighted_tf);
    }
    score
}

/// F5 (0.8.14 Slice 10) — connection-level implementation of the BM25F lexical
/// arm. See [`Engine::bm25f_search`].
pub(crate) fn bm25f_search_inner(
    connection: &Connection,
    query: &str,
    plan: &Bm25fQueryPlan,
) -> rusqlite::Result<Vec<(u64, f64)>> {
    let query_terms: Vec<String> = {
        let mut seen = BTreeSet::new();
        fts5_tokenize(connection, query)?.into_iter().filter(|t| seen.insert(t.clone())).collect()
    };
    if query_terms.is_empty() {
        return Ok(Vec::new());
    }
    let effective_at = current_epoch_seconds();
    let strict_eligibility = dependency_closure::read_eligibility_sql("cn", false, false, false, 1);

    // Corpus pass over ACTIVE rows (superseded versions excluded): accumulate
    // N, total field length per field (for avg field length), and per-term
    // document frequency — all under the SAME FTS5 tokenization the index uses.
    let mut doc_count: usize = 0;
    let mut total_len = [0.0_f64; 3]; // kind, body, status
    let mut df: HashMap<String, usize> = HashMap::new();
    {
        let sql = format!(
            "SELECT v.kind, v.body, v.status
             FROM search_index_v2 v
             JOIN canonical_nodes cn ON cn.write_cursor = v.write_cursor
             WHERE cn.superseded_at IS NULL AND cn.state = 'active'{strict_eligibility}"
        );
        let mut stmt = connection.prepare(&sql)?;
        let mut rows = stmt.query([effective_at])?;
        while let Some(row) = rows.next()? {
            let fields =
                [row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?];
            doc_count += 1;
            let mut present: BTreeSet<String> = BTreeSet::new();
            for (i, field) in fields.iter().enumerate() {
                let toks = fts5_tokenize(connection, field)?;
                total_len[i] += toks.len() as f64;
                for tok in toks {
                    if query_terms.contains(&tok) {
                        present.insert(tok);
                    }
                }
            }
            for term in present {
                *df.entry(term).or_insert(0) += 1;
            }
        }
    }
    if doc_count == 0 {
        return Ok(Vec::new());
    }
    let avglen = [
        total_len[0] / doc_count as f64,
        total_len[1] / doc_count as f64,
        total_len[2] / doc_count as f64,
    ];

    // Active write_cursor set, to filter FTS5 MATCH candidates (search_index_v2
    // retains superseded versions, exactly like search_index).
    let active: BTreeSet<i64> = {
        let sql = format!(
            "SELECT cn.write_cursor FROM canonical_nodes cn \
             WHERE cn.superseded_at IS NULL AND cn.state = 'active'{strict_eligibility}"
        );
        let mut stmt = connection.prepare(&sql)?;
        let rows = stmt.query_map([effective_at], |r| r.get::<_, i64>(0))?;
        rows.collect::<rusqlite::Result<BTreeSet<i64>>>()?
    };

    // Candidate recall through the FTS5 index (this is what makes the v2 index
    // load-bearing), then score each candidate with the in-engine BM25F.
    let match_expr = bm25f_match_expression(&query_terms);
    let mut scored: Vec<(u64, f64)> = Vec::new();
    {
        let mut stmt = connection.prepare(
            "SELECT write_cursor, kind, body, status
             FROM search_index_v2
             WHERE search_index_v2 MATCH ?1",
        )?;
        let mut rows = stmt.query([match_expr.as_str()])?;
        while let Some(row) = rows.next()? {
            let wc = row.get::<_, i64>(0)?;
            if !active.contains(&wc) {
                continue;
            }
            let kind = row.get::<_, String>(1)?;
            let body = row.get::<_, String>(2)?;
            let status = row.get::<_, String>(3)?;

            let mut tf_kind: HashMap<String, u32> = HashMap::new();
            let mut len_kind = 0.0_f64;
            for tok in fts5_tokenize(connection, &kind)? {
                len_kind += 1.0;
                *tf_kind.entry(tok).or_insert(0) += 1;
            }
            let mut tf_body: HashMap<String, u32> = HashMap::new();
            let mut len_body = 0.0_f64;
            for tok in fts5_tokenize(connection, &body)? {
                len_body += 1.0;
                *tf_body.entry(tok).or_insert(0) += 1;
            }
            let mut tf_status: HashMap<String, u32> = HashMap::new();
            let mut len_status = 0.0_f64;
            for tok in fts5_tokenize(connection, &status)? {
                len_status += 1.0;
                *tf_status.entry(tok).or_insert(0) += 1;
            }

            let fields = [
                (plan.weights.kind, len_kind, avglen[0], &tf_kind),
                (plan.weights.body, len_body, avglen[1], &tf_body),
                (plan.weights.status, len_status, avglen[2], &tf_status),
            ];
            let score = bm25f_score_doc(plan, &query_terms, &fields, doc_count, &df);
            scored.push((wc as u64, score));
        }
    }

    // Descending score; write_cursor ascending as the deterministic tiebreak.
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0))
    });
    Ok(scored)
}

impl SearchOriginCapture for NoEvidenceCapture {
    type Output = (
        u64,
        Option<SoftFallback>,
        Vec<SearchHit>,
        GraphFrontierStats,
        Option<Explanation>,
        Option<SearchExpandResult>,
    );

    fn finish(
        self,
        _connection: &Connection,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<SearchHit>,
        graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        expanded: Option<SearchExpandResult>,
    ) -> Result<Self::Output, SearchReaderError> {
        Ok((cursor, soft_fallback, results, graph_stats, explanation, expanded))
    }
}

impl SearchOriginCapture for EvidenceCapture {
    type Output = EvidenceSearchResultV1;

    fn record_graph_origin(&mut self, node_cursor: u64, origin: CapturedGraphOrigin) {
        self.graph_origins.entry(node_cursor).or_insert(origin);
    }

    fn finish(
        self,
        connection: &Connection,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<SearchHit>,
        _graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        _expanded: Option<SearchExpandResult>,
    ) -> Result<Self::Output, SearchReaderError> {
        evidence_linearization_hooks::fire_before_sidecar();
        let search_result =
            SearchResult { projection_cursor: cursor, soft_fallback, results, explanation };
        evidence::build_search_result(
            connection,
            &self.frozen,
            search_result,
            self.include_explanation,
            &self.graph_origins,
        )
        .map_err(SearchReaderError::Evidence)
    }
}

#[cfg(test)]
impl SearchStatement<'_> {
    pub(crate) fn was_reused(&self) -> bool {
        self.statement.get_status(rusqlite::StatementStatus::Run) > 0
    }

    pub(crate) fn reprepare_count(&self) -> i32 {
        self.statement.get_status(rusqlite::StatementStatus::RePrepare)
    }
}

impl<'connection> std::ops::Deref for SearchStatement<'connection> {
    type Target = Statement<'connection>;

    fn deref(&self) -> &Self::Target {
        &self.statement
    }
}

impl std::ops::DerefMut for SearchStatement<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.statement
    }
}
