pub(crate) struct EvidenceCapture {
    frozen: FrozenReadContextV1,
    include_explanation: bool,
    graph_origins: HashMap<u64, CapturedGraphOrigin>,
}

pub(crate) type ProjectedTextReaderResponse = Result<SearchResult, SearchReaderError>;

impl EvidenceCapture {
    pub(crate) fn new(frozen: FrozenReadContextV1, include_explanation: bool) -> Self {
        Self { frozen, include_explanation, graph_origins: HashMap::new() }
    }
}

/// Capability payload kept behind one pointer so adding optional search state
/// cannot inflate every request crossing the bounded reader channel.
pub(crate) struct SearchReaderWork {
    #[cfg(feature = "test-hooks")]
    pub(crate) d27_owner: Option<crate::embed_dispatch::d27_observation::Owner>,
    compiled: Option<fathomdb_query::CompiledQuery>,
    query_vector: Option<String>,
    query_vector_bin: Option<String>,
    result_limit: usize,
    candidate_limit: usize,
    direct_text_candidate_limit: Option<usize>,
    filter: Option<Box<SearchFilter>>,
    recency_enabled: bool,
    importance_enabled: bool,
    vector_stage_only: bool,
    raw_query: Box<str>,
    rerank_depth: usize,
    use_graph_arm: bool,
    alpha: f64,
    pool_n: usize,
    explain: bool,
    projection_runtime_state: ProjectionRuntimeStateV1,
    view: ReadView,
    frozen_binding: Option<Box<frozen_read::FrozenReadBinding>>,
    frozen_query_runtime: Option<Box<FrozenQueryRuntime>>,
    expand_depth: Option<u32>,
}

impl SearchReaderWork {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hybrid(
        compiled: Option<fathomdb_query::CompiledQuery>,
        query_vector: Option<String>,
        query_vector_bin: Option<String>,
        result_limit: usize,
        candidate_limit: usize,
        filter: Option<SearchFilter>,
        recency_enabled: bool,
        importance_enabled: bool,
        vector_stage_only: bool,
        raw_query: &str,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        projection_runtime_state: ProjectionRuntimeStateV1,
        view: ReadView,
        frozen_binding: Option<frozen_read::FrozenReadBinding>,
        frozen_query_runtime: Option<Box<FrozenQueryRuntime>>,
        expand_depth: Option<u32>,
    ) -> Self {
        Self {
            #[cfg(feature = "test-hooks")]
            d27_owner: crate::embed_dispatch::d27_observation::current_owner(),
            compiled,
            query_vector,
            query_vector_bin,
            result_limit,
            candidate_limit,
            direct_text_candidate_limit: None,
            filter: filter.map(Box::new),
            recency_enabled,
            importance_enabled,
            vector_stage_only,
            raw_query: Box::from(raw_query),
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            projection_runtime_state,
            view,
            frozen_binding: frozen_binding.map(Box::new),
            frozen_query_runtime,
            expand_depth,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn evidence(
        result_limit: usize,
        candidate_limit: usize,
        filter: SearchFilter,
        recency_enabled: bool,
        importance_enabled: bool,
        vector_stage_only: bool,
        raw_query: &str,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        projection_runtime_state: ProjectionRuntimeStateV1,
        view: ReadView,
        frozen_binding: frozen_read::FrozenReadBinding,
        frozen_query_runtime: FrozenQueryRuntime,
    ) -> Self {
        Self::hybrid(
            None,
            None,
            None,
            result_limit,
            candidate_limit,
            Some(filter),
            recency_enabled,
            importance_enabled,
            vector_stage_only,
            raw_query,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            true,
            projection_runtime_state,
            view,
            Some(frozen_binding),
            Some(Box::new(frozen_query_runtime)),
            None,
        )
    }

    pub(crate) fn text_only(
        compiled: fathomdb_query::CompiledQuery,
        query: &str,
        result_limit: usize,
        candidate_limit: usize,
        view: ReadView,
    ) -> Self {
        Self {
            #[cfg(feature = "test-hooks")]
            d27_owner: crate::embed_dispatch::d27_observation::current_owner(),
            compiled: Some(compiled),
            query_vector: None,
            query_vector_bin: None,
            result_limit,
            candidate_limit,
            direct_text_candidate_limit: Some(MAX_SEARCH_RESULT_LIMIT),
            filter: None,
            recency_enabled: false,
            importance_enabled: false,
            vector_stage_only: false,
            raw_query: Box::from(query),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            explain: false,
            projection_runtime_state: ProjectionRuntimeStateV1::Absent,
            view,
            frozen_binding: None,
            frozen_query_runtime: None,
            expand_depth: None,
        }
    }
}

pub(crate) struct FrozenQueryRuntime {
    embed_dispatch: Arc<EmbedDispatcher>,
    embedder_identity: EmbedderIdentity,
    dense_disabled_reason: Option<String>,
    observed_generation: Arc<AtomicU64>,
}

impl FrozenQueryRuntime {
    pub(crate) fn new(
        embed_dispatch: Arc<EmbedDispatcher>,
        embedder_identity: EmbedderIdentity,
        dense_disabled_reason: Option<String>,
        observed_generation: Arc<AtomicU64>,
    ) -> Self {
        Self { embed_dispatch, embedder_identity, dense_disabled_reason, observed_generation }
    }
}

fn structural_lifecycle_state(
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

/// 0.8.20 keystone closeout fix-3 (codex §9 [P2], TOCTOU) — the error a Search
/// reader worker can return. Two arms:
///   * `Sqlite` — a backend/storage failure (the pre-fix-3 `rusqlite::Result`
///     behaviour verbatim; the caller emits the internal-error event and maps to
///     `EngineError::Storage`);
///   * `InvalidFilter` — a filter naming an UNDECLARED `filterable` attribute,
///     detected on the reader's OWN transaction snapshot (see
///     [`validate_filter_attributes_on_snapshot`]). The caller re-raises it as the
///     EXISTING `EngineError::InvalidFilter { reason }` typed variant.
///
/// Why a channel-carried variant and not a pre-dispatch check on the writer
/// connection: fix-2 validated on `self.connection` BEFORE dispatch, then the
/// reader prepared the vec0 query on a DIFFERENT connection/snapshot. A
/// `configure_projections` DROP landing in that window let the vec0 `attr_<hex>`
/// column vanish AFTER validation passed → an opaque `no such column` `Storage`
/// error (the exact untyped failure fix-2 meant to prevent). Validating INSIDE
/// the reader transaction that also compiles+executes the search binds the check
/// and the query to ONE snapshot, closing the race; carrying the typed reason
/// back through this variant keeps the outcome `InvalidFilter`, never `Storage`.
pub(crate) enum SearchReaderError {
    Sqlite(rusqlite::Error),
    Evidence(EngineError),
    InvalidFilter(String),
    RerankerDevicePolicy(RerankerDevicePolicyError),
    FrozenRead(FrozenReadError),
    VectorEquivalenceMismatch(String),
    WriteValidation,
    InvalidArgument(String),
}

impl From<rusqlite::Error> for SearchReaderError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<SnapshotFilterError> for SearchReaderError {
    fn from(error: SnapshotFilterError) -> Self {
        match error {
            SnapshotFilterError::Sqlite(error) => Self::Sqlite(error),
            SnapshotFilterError::InvalidFilter(reason) => Self::InvalidFilter(reason),
        }
    }
}

impl From<graph_expand::SearchExpandHandlerError> for SearchReaderError {
    fn from(error: graph_expand::SearchExpandHandlerError) -> Self {
        match error {
            graph_expand::SearchExpandHandlerError::Sqlite(error) => Self::Sqlite(error),
            graph_expand::SearchExpandHandlerError::FrozenRead(error) => Self::FrozenRead(error),
            graph_expand::SearchExpandHandlerError::InvalidFilter(reason) => {
                Self::InvalidFilter(reason)
            }
        }
    }
}

// Ordinary search retains a zero-sized capture strategy: evidence state and
// provenance collection are absent unless the explicit evidence operation is used.
const _: () = assert!(std::mem::size_of::<NoEvidenceCapture>() == 0);

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

fn load_projection_cursor_for_search(connection: &Connection) -> rusqlite::Result<u64> {
    prepare_search_statement(connection, "SELECT value FROM _fathomdb_open_state WHERE key = ?1")?
        .query_row([PROJECTION_CURSOR_KEY], |row| row.get::<_, String>(0))
        .map(|value| value.parse::<u64>().unwrap_or(0))
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(0),
            _ => Err(err),
        })
}

fn rank_search_hit_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
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

fn collect_complete_rank_boundary(
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
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    if let Ok(mut file) = OpenOptions::open(&options, path) {
        let _ = writeln!(file, "{value}");
    }
}

#[cfg(feature = "test-hooks")]
fn record_fts_route_for_test(route: &str) {
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
fn record_fts_query_plan_for_test(
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

/// R3 (Slice 30) + C1 (0.8.1 graph-arm seeding) — graph-arm BFS candidate generation.
///
/// **C1 seeding (the BLOCK-1 fix):** the frontier is seeded from the graph's OWN
/// query-matched text surfaces — NOT from doc-node hits (doc nodes carry
/// `logical_id = NULL`, so the old doc-seeding produced an empty frontier). Two
/// seed sources are unioned on `match_expression` (the compiled FTS query):
///   A. **edge-fact FTS** (`search_index_edges`) — both endpoints (`from_id`,
///      `to_id`) of matched, temporally-live, non-fallback edges;
///   B. **entity-node FTS** (`search_index` ⋈ `canonical_nodes`) — matched nodes
///      with `logical_id IS NOT NULL` (excludes doc nodes — the bug surface).
/// Each distinct candidate `logical_id` is counted in `seeds_considered`; those
/// confirmed active in `canonical_nodes` are `seeds_resolved` and pushed onto the
/// frontier (dangling edge endpoints count considered-but-unresolved).
///
/// Phase 2 is unchanged: BFS over `canonical_edges` with the temporal filter,
/// carrying each traversed edge's `source_id` (G0 BLOCK-2) onto the emitted hit.
/// Collects reachable node bodies (up to `cap`) as [`SearchHit`]s tagged
/// `SoftFallbackBranch::GraphArm`. Score = `1.0 / (1.0 + hop_count)` with a
/// synthesized-node penalty (`kind = 'unknown'` → score *= 0.3). Bodies already
/// present in `fused_hits` are excluded (already covered by the two-arm result).
///
/// **F9 (0.8.16 Slice 5) confidence carry:** the third tuple element maps each
/// emitted graph-arm hit's `write_cursor` (its `SearchHit.id`, a NODE cursor) to
/// the `confidence` of the EDGE traversed to reach that node — the input the F9
/// reweight (`graph_rrf_score(edge) = confidence × 1/(K+bfs_rank)`) consumes.
/// `build_importance_confidence_maps` keys edge confidence on the EDGE
/// `write_cursor`, which never equals a reached node's cursor, so without this
/// carry edge confidence never reaches a graph-arm hit. **Determinism rule (matches
/// the BLOCK-2 provenance carry):** when several edges reach the same node, the
/// FIRST edge to claim the node in the `visited` dedup wins — i.e. the edge that
/// produced the node's winning `bfs_rank` (seeds are considered before Phase-2
/// neighbors; within a phase, `ORDER BY write_cursor` makes the earliest-written
/// edge win). A NULL edge confidence is simply not inserted ⇒ neutral (1.0).
#[allow(clippy::too_many_arguments)] // Shared graph capture preserves the zero-cost default path.
fn bfs_graph_arm_candidates<C: SearchOriginCapture>(
    tx: &Connection,
    fused_hits: &[SearchHit],
    match_expression: &str,
    max_depth: u32,
    cap: usize,
    view: FrozenView,
    filter: Option<&SearchFilter>,
    capture: &mut C,
) -> rusqlite::Result<(Vec<SearchHit>, GraphFrontierStats, HashMap<u64, f64>)> {
    // fix-2 (codex §9 [P2]): the opt-in graph arm hydrates NODES too, so it takes
    // the same validity conjunct as the vector and FTS branches — otherwise
    // `search_reranked(.., use_graph_arm = true)` would keep the exact leak the
    // other two branches just closed. Same generator, same bound `:now`.
    //
    // fix-3 (F2): the instant arrives ALREADY RESOLVED in the `FrozenView` — it
    // is the identical value the vector and FTS arms bound. This arm cannot
    // re-read the clock: a `FrozenView` carries no route to one.
    let now_param = view.now_param();
    // C1 — seed-FTS fan-out cap per source (A: edge endpoints, B: entity nodes).
    const SEED_FTS_N: usize = 10;
    const SYNTHESIZED_PENALTY: f64 = 0.3;

    // Bodies already in the fused result — exclude these from graph arm output.
    let seed_bodies: std::collections::HashSet<&str> =
        fused_hits.iter().map(|h| h.body.as_str()).collect();

    let mut frontier: VecDeque<(String, u32)> = VecDeque::new(); // (logical_id, depth)
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut candidates: Vec<SearchHit> = Vec::new();
    // F9 (0.8.16 Slice 5) — per-hit traversing-edge confidence, keyed by the
    // emitted hit's NODE `write_cursor`. First edge to reach a node wins (visited
    // dedup); NULL confidence is never inserted (⇒ neutral in the reweight).
    let mut edge_confidence_by_cursor: HashMap<u64, f64> = HashMap::new();
    // G0 Phase-2 (BLOCK-1) frontier meter — distinct seed candidates considered vs
    // resolved-active; `resolved_seed_rate` flips 0→>0 once entities/edge-facts seed.
    let mut stats = GraphFrontierStats::default();
    {
        // C1 seeding — gather distinct candidate (logical_id, provenance source_id)
        // pairs from the graph's OWN query-matched FTS surfaces (NOT doc-node hits).
        // Order-preserving dedup (first provenance wins) so `seeds_considered` counts
        // each candidate once. `source_id` is the session the seed traces back to: the
        // matched edge's `source_id` (source A) or the entity node's own (source B).
        // F9: each seed carries the confidence of the edge that surfaced it
        // (`None` for entity-FTS seeds, which have no traversing edge).
        let mut candidate_seeds: Vec<(String, Option<String>, Option<f64>, CapturedGraphOrigin)> =
            Vec::new();
        let mut seen_candidates: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let push_candidate = |lid: String,
                              source_id: Option<String>,
                              confidence: Option<f64>,
                              origin: CapturedGraphOrigin,
                              seen: &mut std::collections::HashSet<String>,
                              out: &mut Vec<(
            String,
            Option<String>,
            Option<f64>,
            CapturedGraphOrigin,
        )>| {
            if seen.insert(lid.clone()) {
                out.push((lid, source_id, confidence, origin));
            }
        };

        // Seed source A — edge-fact endpoints (primary). Both endpoints of each
        // matched, temporally-live, non-fallback edge are candidate seeds, tagged with
        // the edge's `source_id` provenance and (F9) `confidence`. `search_index_edges`
        // may be absent on very old DBs (< step-14) — degrade to no edge seeds rather
        // than error.
        // TC-33: `?1` MATCH, `?2` LIMIT ⇒ the edge `:now` binds at `?3`.
        let mut edge_seed_params: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(match_expression.to_string()),
            rusqlite::types::Value::Integer(SEED_FTS_N as i64),
            rusqlite::types::Value::Integer(view.edge_now()),
        ];
        let endpoint_now_index = 4;
        if let Some(now) = now_param {
            edge_seed_params.push(rusqlite::types::Value::Integer(now));
        }
        let from_filter = append_node_eligibility_sql(filter, "ef", &mut edge_seed_params);
        let to_filter = append_node_eligibility_sql(filter, "et", &mut edge_seed_params);
        let from_validity = view.validity_sql("ef", endpoint_now_index);
        let to_validity = view.validity_sql("et", endpoint_now_index);
        let from_dependency = dependency_closure::read_eligibility_sql(
            "ef",
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            endpoint_now_index,
        );
        let to_dependency = dependency_closure::read_eligibility_sql(
            "et",
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            endpoint_now_index,
        );
        if let Ok(mut edge_seed_stmt) = tx.prepare(&format!(
            "SELECT ce.from_id, ce.to_id, ce.source_id, ce.confidence, ce.write_cursor \
             FROM search_index_edges sei \
             JOIN canonical_edges ce ON ce.write_cursor = sei.write_cursor \
             WHERE search_index_edges MATCH ?1 \
               AND ce.superseded_at IS NULL{} \
               AND (ce.temporal_fallback IS NULL OR ce.temporal_fallback = 0) \
               AND (EXISTS(SELECT 1 FROM canonical_nodes ef WHERE ef.logical_id=ce.from_id \
                    AND ef.superseded_at IS NULL AND ef.state='active'\
                    {from_validity}{from_dependency}{from_filter}) \
                 OR EXISTS(SELECT 1 FROM canonical_nodes et WHERE et.logical_id=ce.to_id \
                    AND et.superseded_at IS NULL AND et.state='active'\
                    {to_validity}{to_dependency}{to_filter})) \
             ORDER BY bm25(search_index_edges), sei.write_cursor \
             LIMIT ?2",
            edge_validity_sql_for_view("ce", 3, &view.view)
        )) {
            let rows = edge_seed_stmt.query_map(
                rusqlite::params_from_iter(edge_seed_params.iter()),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<f64>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )?;
            for quintuple in rows {
                let (from_id, to_id, source_id, confidence, edge_cursor) = quintuple?;
                push_candidate(
                    from_id,
                    source_id.clone(),
                    confidence,
                    CapturedGraphOrigin::EdgeSeed { edge_cursor: edge_cursor as u64 },
                    &mut seen_candidates,
                    &mut candidate_seeds,
                );
                push_candidate(
                    to_id,
                    source_id,
                    confidence,
                    CapturedGraphOrigin::EdgeSeed { edge_cursor: edge_cursor as u64 },
                    &mut seen_candidates,
                    &mut candidate_seeds,
                );
            }
        }

        // Seed source B — entity-node FTS (isolated / strongly-named entities).
        // `logical_id IS NOT NULL` structurally excludes doc nodes (the bug surface).
        // Provenance = the node's own `source_id` (the session it was extracted from).
        {
            // `?1` MATCH, `?2` LIMIT ⇒ `:now` binds at `?3`.
            let seed_validity = view.validity_sql("cn", 3);
            let seed_eligibility = dependency_closure::read_eligibility_sql(
                "cn",
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                3,
            );
            let mut seed_params: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(match_expression.to_string()),
                rusqlite::types::Value::Integer(SEED_FTS_N as i64),
            ];
            if let Some(now) = now_param {
                seed_params.push(rusqlite::types::Value::Integer(now));
            }
            let seed_filter = append_node_eligibility_sql(filter, "cn", &mut seed_params);
            let mut node_seed_stmt = tx.prepare(&format!(
                "SELECT cn.logical_id, cn.source_id \
                 FROM search_index si \
                 JOIN canonical_nodes cn ON cn.write_cursor = si.write_cursor \
                 WHERE search_index MATCH ?1 \
                   AND cn.superseded_at IS NULL \
                   AND cn.state = 'active' \
                   AND cn.logical_id IS NOT NULL\
                   {seed_validity}{seed_eligibility}{seed_filter} \
                 ORDER BY bm25(search_index), si.write_cursor \
                 LIMIT ?2"
            ))?;
            let rows = node_seed_stmt
                .query_map(rusqlite::params_from_iter(seed_params.iter()), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })?;
            for pair in rows {
                let (lid, source_id) = pair?;
                // Entity-FTS seed: no traversing edge ⇒ no edge confidence (neutral).
                push_candidate(
                    lid,
                    source_id,
                    None,
                    CapturedGraphOrigin::EntitySeed,
                    &mut seen_candidates,
                    &mut candidate_seeds,
                );
            }
        }

        // Resolve + emit: a seed is `resolved` only if an ACTIVE canonical_node carries
        // that logical_id (dangling edge endpoints count considered-not-resolved). A
        // resolved seed is BOTH a BFS root AND emitted as a graph-arm candidate (depth
        // 0, hop_score 1.0) — so an edge-only query match surfaces the connected ENTITY
        // nodes, not just the fact body (codex §9 [P2]). Seeds whose body is already in
        // the two-arm result are skipped; the cap is respected.
        let active_validity = view.validity_sql("canonical_nodes", 2);
        let active_eligibility = dependency_closure::read_eligibility_sql(
            "canonical_nodes",
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            2,
        );
        let mut active_params_template = vec![rusqlite::types::Value::Text(String::new())];
        if let Some(now) = now_param {
            active_params_template.push(rusqlite::types::Value::Integer(now));
        }
        let active_filter =
            append_node_eligibility_sql(filter, "canonical_nodes", &mut active_params_template);
        let mut active_stmt = tx.prepare(&format!(
            "SELECT kind, body, write_cursor FROM canonical_nodes \
             WHERE logical_id = ?1 AND superseded_at IS NULL AND state = 'active'\
             {active_validity}{active_eligibility}{active_filter} LIMIT 1"
        ))?;
        for (lid, source_id, seed_confidence, graph_origin) in candidate_seeds {
            stats.seeds_considered += 1;
            let mut active_params = active_params_template.clone();
            active_params[0] = rusqlite::types::Value::Text(lid.clone());
            let row: Option<(String, String, i64)> = active_stmt
                .query_row(rusqlite::params_from_iter(active_params.iter()), |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
                })
                .optional()?;
            if let Some((kind, body, write_cursor)) = row {
                stats.seeds_resolved += 1;
                if visited.insert(lid.clone()) {
                    // Cause-A: the seed's `logical_id` is in hand (`lid`) — derive the
                    // stable id before `lid` is moved onto the frontier (zero extra query).
                    let id = derive_stable_id(Some(&lid), &body);
                    frontier.push_back((lid, 0));
                    if !seed_bodies.contains(body.as_str()) && candidates.len() < cap {
                        // depth-0 hop_score = 1.0/(1.0+0) = 1.0; synthesized penalty for
                        // 'unknown' kind (mirrors the Phase-2 neighbor scoring).
                        let score = if kind == "unknown" { SYNTHESIZED_PENALTY } else { 1.0 };
                        // F9: an edge-seeded endpoint carries its seeding edge's
                        // confidence (source A); entity-FTS seeds carry None.
                        if let Some(c) = seed_confidence {
                            edge_confidence_by_cursor.insert(write_cursor as u64, c);
                        }
                        capture.record_graph_origin(write_cursor as u64, graph_origin);
                        candidates.push(SearchHit {
                            id,
                            write_cursor: write_cursor as u64,
                            kind,
                            body,
                            score,
                            branch: SoftFallbackBranch::GraphArm,
                            source_id,
                            ce_score: None,
                        });
                    }
                }
            }
        }
    }
    stats.frontier_nonempty = !frontier.is_empty();

    // Phase 2: BFS over canonical_edges (temporal filter). `candidates` already
    // holds the depth-0 emitted seeds; BFS appends the reachable neighbors.
    // Both statements are prepared ONCE outside the loops — re-preparing inside
    // would issue O(frontier_size × neighbors) sqlite3_prepare_v2 calls.
    let target_node = view.node_sql("target", 3);
    let mut edge_params_template = vec![
        rusqlite::types::Value::Text(String::new()),
        rusqlite::types::Value::Integer(view.edge_now()),
    ];
    if let Some(now) = now_param {
        edge_params_template.push(rusqlite::types::Value::Integer(now));
    }
    let target_filter = append_node_eligibility_sql(filter, "target", &mut edge_params_template);
    let mut edge_stmt = tx.prepare(
        // G0 Phase-2 (BLOCK-2): carry the traversed edge's `source_id` so a
        // graph-reached neighbor can resolve back to the session it was extracted
        // from. `ORDER BY e.write_cursor` makes the traversal deterministic: when
        // several active edges connect this node to the SAME neighbor with
        // different `source_id`s, the earliest-written edge wins the `visited`
        // dedup, so the carried provenance is stable (not SQLite-order-dependent).
        // (codex §9 [P2]; the design §B already rejected the memo's arbitrary
        // `LIMIT 1` lookup for the same reason.)
        // F9: also carry the traversed edge's `confidence` — the reweight input for
        // the reached node (keyed downstream by the node's `write_cursor`). Same
        // determinism as `source_id`: the earliest-written edge wins the `visited`
        // dedup, so the reached node's confidence is the winning-`bfs_rank` edge's.
        // TC-33: `?1` is the anchor logical_id ⇒ the edge `:now` binds at `?2`.
        &format!(
            "SELECT e.from_id, e.to_id, e.source_id, e.confidence, e.write_cursor \
             FROM canonical_edges e \
             JOIN canonical_nodes target ON target.logical_id = \
               CASE WHEN e.from_id = ?1 THEN e.to_id ELSE e.from_id END \
             WHERE (e.from_id = ?1 OR e.to_id = ?1) \
               AND e.superseded_at IS NULL{} \
               AND (e.temporal_fallback IS NULL OR e.temporal_fallback = 0) \
               {target_node}{target_filter} \
             ORDER BY e.write_cursor \
             LIMIT 64",
            edge_validity_sql_for_view("e", 2, &view.view)
        ),
    )?;
    // Fetch write_cursor alongside kind+body so graph-arm hits carry a real id
    // for apply_recency_reweight (id=0 would force min_id=0 and distort span).
    let body_validity = view.validity_sql("canonical_nodes", 2);
    let body_eligibility = dependency_closure::read_eligibility_sql(
        "canonical_nodes",
        view.view.include_superseded,
        view.view.include_inactive,
        view.view.include_out_of_window,
        2,
    );
    let mut body_params_template = vec![rusqlite::types::Value::Text(String::new())];
    if let Some(now) = now_param {
        body_params_template.push(rusqlite::types::Value::Integer(now));
    }
    let body_filter =
        append_node_eligibility_sql(filter, "canonical_nodes", &mut body_params_template);
    let mut body_stmt = tx.prepare(&format!(
        "SELECT kind, body, write_cursor FROM canonical_nodes \
         WHERE logical_id = ?1 AND superseded_at IS NULL AND state = 'active'\
         {body_validity}{body_eligibility}{body_filter} \
         LIMIT 1"
    ))?;

    'frontier: while let Some((lid, depth)) = frontier.pop_front() {
        if depth >= max_depth {
            continue;
        }

        // Fetch temporal-live neighbors via edges, each paired with the
        // traversing edge's `source_id` (BLOCK-2 provenance carry) and (F9)
        // `confidence` (the reweight input for the reached node).
        let neighbors: Vec<(String, Option<String>, Option<f64>, u64)> = {
            let mut edge_params = edge_params_template.clone();
            edge_params[0] = rusqlite::types::Value::Text(lid.clone());
            let rows =
                edge_stmt.query_map(rusqlite::params_from_iter(edge_params.iter()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<f64>>(3)?,
                        row.get::<_, i64>(4)? as u64,
                    ))
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .map(|(from_id, to_id, source_id, confidence, edge_cursor)| {
                    let neighbor = if from_id == lid { to_id } else { from_id };
                    (neighbor, source_id, confidence, edge_cursor)
                })
                .collect()
        };

        for (neighbor, edge_source_id, edge_confidence, edge_cursor) in neighbors {
            if visited.contains(&neighbor) {
                continue;
            }
            visited.insert(neighbor.clone());

            // Fetch neighbor body + write_cursor from canonical_nodes.
            let mut body_params = body_params_template.clone();
            body_params[0] = rusqlite::types::Value::Text(neighbor.clone());
            let row: Option<(String, String, i64)> = body_stmt
                .query_row(rusqlite::params_from_iter(body_params.iter()), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
                })
                .optional()?;

            if let Some((kind, body, write_cursor)) = row {
                // Skip bodies already covered by the two-arm result.
                if !seed_bodies.contains(body.as_str()) {
                    if candidates.len() >= cap {
                        stats.bound_reached = true;
                        break 'frontier;
                    }
                    let hop_score = 1.0 / (1.0 + (depth + 1) as f64);
                    let score =
                        if kind == "unknown" { hop_score * SYNTHESIZED_PENALTY } else { hop_score };
                    // Cause-A: the neighbor's `logical_id` is `neighbor` (still in
                    // scope here; only moved onto the frontier below) — derive the
                    // stable id with no extra query.
                    let id = derive_stable_id(Some(&neighbor), &body);
                    // F9: record the traversing edge's confidence for this node
                    // (first edge wins — this is the winning-`bfs_rank` edge).
                    if let Some(c) = edge_confidence {
                        edge_confidence_by_cursor.insert(write_cursor as u64, c);
                    }
                    capture.record_graph_origin(
                        write_cursor as u64,
                        CapturedGraphOrigin::Traversal { edge_cursor, hop_count: depth + 1 },
                    );
                    candidates.push(SearchHit {
                        id,
                        write_cursor: write_cursor as u64,
                        kind,
                        body,
                        score,
                        branch: SoftFallbackBranch::GraphArm,
                        // BLOCK-2: the session this fact-edge was extracted from.
                        source_id: edge_source_id.clone(),
                        ce_score: None,
                    });
                }
                // Always push neighbor to frontier for further BFS expansion.
                frontier.push_back((neighbor, depth + 1));
            }
        }
    }

    drop(edge_stmt);
    drop(body_stmt);
    stats.graph_candidates_emitted = candidates.len() as u32;
    Ok((candidates, stats, edge_confidence_by_cursor))
}

#[allow(clippy::too_many_arguments)]
fn read_search_in_tx<C: SearchOriginCapture>(
    reader: &mut Connection,
    compiled: Option<&fathomdb_query::CompiledQuery>,
    query_vector: Option<&str>,
    query_vector_bin: Option<&str>,
    final_limit: usize,
    candidate_limit: usize,
    direct_text_candidate_limit: Option<usize>,
    filter: Option<&SearchFilter>,
    recency_enabled: bool,
    importance_enabled: bool,
    vector_stage_only: bool,
    raw_query: &str,
    rerank_depth: usize,
    use_graph_arm: bool,
    alpha: f64,
    pool_n: usize,
    explain: bool,
    projection_runtime_state: ProjectionRuntimeStateV1,
    view: ReadView,
    frozen_binding: Option<&frozen_read::FrozenReadBinding>,
    frozen_query_runtime: Option<&FrozenQueryRuntime>,
    expand_depth: Option<u32>,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
    mut capture: C,
) -> Result<C::Output, SearchReaderError> {
    // 0.8.20 Slice 15b fix-2 (R-20-NV) — the `:now` instant is read HERE, in
    // Rust, ONCE per query, and bound positionally into every node-hydration
    // SELECT. Never `datetime('now')` / `strftime('%s','now')`: an inline clock
    // would make the query non-deterministic, untestable, and re-evaluated per
    // row. `None` ⇒ the view relaxes validity ⇒ no conjunct is emitted and
    // nothing is bound (`validity_sql` returns the empty string).
    //
    // fix-3 (F2): FREEZE the view here, at the single point every arm flows
    // through. `freeze()` is the only place on this path that reads the clock;
    // downstream arms hold a `FrozenView` and have no way to resolve a second,
    // different instant. Previously the graph arm re-derived it from the raw
    // `ReadView`, so a boundary-straddling query could have its arms disagree.
    let view = view.freeze();
    let now_param = view.now_param();
    // fix-3 (codex §9 [P2], TOCTOU) — test-only rendezvous: parks the worker here,
    // BEFORE the deferred snapshot is pinned, so a test can commit a concurrent
    // `configure_projections` DROP in the exact race window. Disarmed (no-op) in
    // production and on every non-race test.
    reader_search_hook::fire(attribution);
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)?;
    if let Some(expected) = frozen_binding {
        let generation =
            frozen_read::validate_snapshot(&tx, expected).map_err(SearchReaderError::FrozenRead)?;
        let observed = frozen_query_runtime
            .as_ref()
            .ok_or_else(|| {
                SearchReaderError::FrozenRead(FrozenReadError {
                    reason: FrozenReadErrorReason::StateUnavailable,
                    field_path: "/token".to_string(),
                })
            })?
            .observed_generation
            .fetch_max(generation, Ordering::AcqRel);
        if generation < observed {
            return Err(SearchReaderError::FrozenRead(FrozenReadError {
                reason: FrozenReadErrorReason::StateUnavailable,
                field_path: "/token".to_string(),
            }));
        }
        frozen_after_validation_hook::fire();
    }
    let owned_compiled;
    let owned_query_vector;
    let owned_query_vector_bin;
    let (compiled, query_vector, query_vector_bin) = if let Some(runtime) = frozen_query_runtime {
        validate_search_result_limit(final_limit)
            .map_err(|error| SearchReaderError::InvalidArgument(error.to_string()))?;
        if expand_depth.is_some_and(|depth| depth > 3) {
            return Err(SearchReaderError::InvalidArgument(format!(
                "traversal depth {} exceeds the SDK ceiling of 3",
                expand_depth.unwrap_or_default()
            )));
        }
        let max_ranking_control = u32::MAX as usize;
        if rerank_depth > max_ranking_control {
            return Err(SearchReaderError::InvalidArgument(format!(
                "rerank_depth {rerank_depth} exceeds the u32 maximum"
            )));
        }
        if pool_n > max_ranking_control {
            return Err(SearchReaderError::InvalidArgument(format!(
                "pool_n {pool_n} exceeds the u32 maximum"
            )));
        }
        if !alpha.is_finite() {
            return Err(SearchReaderError::InvalidArgument(
                "alpha must be a finite number".to_string(),
            ));
        }
        if let Some(reason) = runtime.dense_disabled_reason.as_ref() {
            return Err(SearchReaderError::VectorEquivalenceMismatch(reason.clone()));
        }
        if raw_query.trim().is_empty() || raw_query.as_bytes().contains(&0) {
            return Err(SearchReaderError::WriteValidation);
        }
        owned_compiled = compile_text_query(raw_query);
        let raw_vector = match dispatch_embed_vector(&runtime.embed_dispatch, raw_query) {
            Ok(vector) => Some(vector),
            Err(DispatchError::Panic(payload)) => std::panic::resume_unwind(payload),
            Err(_) => None,
        };
        owned_query_vector_bin = match raw_vector.as_ref() {
            Some(vector) if identity_requires_mean_centering(&runtime.embedder_identity) => {
                let pinned = read_pinned_mean_vec(&tx, runtime.embedder_identity.dimension)
                    .map_err(|_| SearchReaderError::Sqlite(rusqlite::Error::InvalidQuery))?;
                match pinned {
                    Some(mean) => serde_json::to_string(&subtract_mean(vector, &mean)).ok(),
                    None => serde_json::to_string(vector).ok(),
                }
            }
            Some(vector) => serde_json::to_string(vector).ok(),
            None => None,
        };
        owned_query_vector = raw_vector.and_then(|vector| serde_json::to_string(&vector).ok());
        (&owned_compiled, owned_query_vector.as_deref(), owned_query_vector_bin.as_deref())
    } else {
        (
            compiled.ok_or_else(|| SearchReaderError::Sqlite(rusqlite::Error::InvalidQuery))?,
            query_vector,
            query_vector_bin,
        )
    };
    let cursor = load_projection_cursor_for_search(&tx)?;
    // fix-3 (codex §9 [P2], TOCTOU) — validate every filter attribute name on THIS
    // reader transaction's snapshot, before `build_vector_phase1_sql` emits
    // `AND attr_<hex>=?` and before the FTS arm probes `canonical_attributes`. The
    // registry read and the vec0 query now share ONE snapshot (see
    // `validate_filter_attributes_on_snapshot`), so a `configure_projections` DROP
    // racing this search yields a consistent typed `InvalidFilter` — never the
    // opaque `no such column` `Storage` error fix-2's writer-connection check could
    // still leak. Skipped when the filter carries no attribute terms (common path).
    if let Some(filter) = filter {
        validate_filter_attributes_on_snapshot(&tx, filter)?;
    }
    let has_source_dependencies = dependency_closure::has_source_dependencies(&tx)?;
    let read_dependency_eligibility = |alias: &str, now_idx: usize| {
        if has_source_dependencies {
            dependency_closure::read_eligibility_sql(
                alias,
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                now_idx,
            )
        } else {
            String::new()
        }
    };
    let vector_eligibility_degraded = query_vector.is_some()
        && dependency_closure::vector_arm_requires_fallback(
            &tx,
            has_source_dependencies,
            view.view.include_superseded,
            view.view.include_inactive,
            view.view.include_out_of_window,
            view.edge_now(),
        )?;
    let vector_results = if let Some(query_vector) =
        query_vector.filter(|_| !vector_eligibility_degraded)
    {
        let mut rowids = Vec::new();
        let bin_vector = query_vector_bin.unwrap_or(query_vector);
        {
            // Phase 1: bit-KNN over `embedding_bin` to a top-K candidate
            // set; Phase 2: f32 rerank on the candidate set via
            // vec_distance_l2 against the retained `embedding` column.
            // EU-5a2: ?1 is the (possibly centered) sign-quant input,
            // ?2 is the un-centered f32 for vec_distance_l2 — both sides
            // of the f32 cosine use un-centered vectors.
            // Slice 18: `final_limit` is the caller's public result limit
            // (default 10, validated through the public API). `candidate_limit`
            // controls only vector phase-2 fanout: the test-only
            // `set_search_limit_for_test` seam may raise it for recall tests.
            // The seam never changes `final_limit`; visible results are still
            // truncated to that caller-requested limit after ranking.
            // G10: the metadata filter is appended to this single phase-1
            // statement (`AND col=?n` from ?3); `filter=None` keeps the SQL
            // byte-identical to 0.7.2. `?1`/`?2` are the sign-quant + f32 query
            // vectors; filter values bind at ?3.. in `vector_filter_clause`
            // order.
            // fix-3 (F1, codex §9 [P2]) — OVERFETCH the phase-2 rerank so the
            // validity/existence filter applied at hydration cannot starve the
            // result set.
            //
            // The defect: hydration drops rows that are expired, superseded or
            // inactive, but it ran on candidates ALREADY truncated to
            // `final_limit`. If the nearest `final_limit` neighbours were all
            // out-of-window they consumed every slot and were then dropped, so
            // valid rows just below the cutoff were never considered — a
            // default search silently returned too few hits, or none.
            //
            // Why overfetch rather than filtering in SQL: the natural fix is to
            // join `canonical_nodes` into the candidate query, but (i) phase 1
            // is a `vec0` KNN and ADR-0.8.11 D3 forbids demoting it with
            // non-metadata predicates, and (ii) there is NO index on
            // `canonical_nodes(write_cursor)`, so an `EXISTS` per candidate
            // would be a full scan × the whole pool on EVERY query — a
            // guaranteed cost to fix a degenerate case.
            //
            // Overfetching is free by comparison: phase 2 already computes
            // `vec_distance_l2` for all `TOP_K_BIT_CANDIDATES` in order to sort
            // them, so raising the LIMIT only returns more of a result set that
            // was already materialized. No extra vec0 work, no schema change,
            // no new index, no second query. The hydration loop below then
            // stops at `final_limit` SURVIVING hits, so the common case does
            // exactly as many hydration probes as before.
            //
            // `max` (not a bare constant) preserves both the test seam's
            // deeper candidate fanout and the caller's requested result limit.
            let candidate_limit = candidate_limit.max(final_limit).max(TOP_K_BIT_CANDIDATES);
            let sql = build_vector_phase1_sql(filter, candidate_limit);
            let mut params: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(bin_vector.to_string()),
                rusqlite::types::Value::Text(query_vector.to_string()),
            ];
            params.extend(vector_filter_values(filter));
            let mut statement = prepare_search_statement(&tx, &sql)?;
            let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
            })?;
            for row in rows {
                rowids.push(row?);
            }
        }
        // G1: carry the canonical row's `write_cursor` (interim id), `kind`,
        // `body`, and the `vec_distance_l2` rerank score per hit. The
        // `_fathomdb_vector_rows.rowid` equals the canonical `write_cursor`,
        // so the candidate rowid IS the hit id.
        //
        // G11 (Slice 15) fix: edge bodies are projected into vector_default under
        // kind = "edge_fact"; their write_cursor is in canonical_edges, not
        // canonical_nodes. Try canonical_nodes first; fall back to canonical_edges
        // for edge-fact hits so they are not silently dropped.
        let mut results = Vec::new();
        // Cause-A: the two node/edge SELECTs additively fetch `logical_id` so the
        // hit can carry a stable cross-session id (derive_stable_id). Read-only
        // additive column — ordering/scores are untouched.
        // fix-1 (codex §9): co-locate BOTH existence guards. Node supersession
        // is tombstone-then-insert (`commit_batch`) — the prior `canonical_nodes`
        // row is kept (same `write_cursor`, `state = 'active'`, `superseded_at`
        // set) and, unlike the edge path (fix-30), its stale `vector_default` row
        // is NOT pruned, so the phase-1 bit-KNN can still surface the OLD cursor.
        // Without `superseded_at IS NULL` here that superseded version would
        // hydrate and leak stale content through vector search. This matches the
        // edge branch below and every other retrieval site (design §2: enforce
        // the exclusion at EVERY retrieval site). It only drops already-superseded
        // rows → a no-op on the all-active / non-superseded corpus.
        // TC-31 (0.8.20 Slice 10a): both hydration SELECTs additively fetch the
        // canonical row's OWN `source_id` so a vector hit carries the provenance
        // `erase_source` consumes. These statements already read the canonical
        // row by `write_cursor`, so this is one extra COLUMN on an existing
        // lookup — NOT an extra query. (A per-hit `WHERE write_cursor = ?`
        // probe would be a full scan: there is no index on
        // `canonical_nodes(write_cursor)`. This site already pays that cost by
        // construction; TC-31 must not add a second one.) Read-only additive
        // column — row-set, ordering and scores are untouched.
        // fix-2 (codex §9 [P2]): the validity conjunct comes from
        // `ReadView::validity_sql` — the SAME generator the five read verbs use.
        // It is NOT hand-rolled here: Slice 10's whole design is that the
        // predicate exists in exactly ONE place, so no retrieval site can drift
        // from another. `?1` is the candidate rowid, so `:now` binds at `?2`.
        // On a corpus that never authored a window every row is NULL/NULL and
        // the conjunct matches everything ⇒ default behaviour is unchanged.
        let node_validity = view.validity_sql("canonical_nodes", 2);
        let node_eligibility = read_dependency_eligibility("canonical_nodes", 2);
        let mut node_stmt = prepare_search_statement(
            &tx,
            &format!(
                "SELECT kind, body, logical_id, source_id FROM canonical_nodes \
             WHERE write_cursor = ?1 AND superseded_at IS NULL AND state = 'active'\
             {node_validity}{node_eligibility} LIMIT 1"
            ),
        )?;
        // fix-2 (codex §9 [P2]): an edge body projected into `vector_default`
        // (kind = "edge_fact") is hydrated HERE by write_cursor. Gating on
        // `superseded_at` alone let an EXPIRED edge (`t_invalid <= :now`) surface
        // its body through the VECTOR arm — the same "validity enforced on
        // traversal, not on search" gap Slice 15b closed for nodes, now on the
        // edge-vector read path. Apply the shared `edge_validity_sql` predicate
        // (the ONE generator every edge read site uses) so no arm can drift.
        // `?1` is the rowid, so the edge `:now` binds at `?2`; the instant is the
        // frozen `view.edge_now()` — a bound value, never `datetime('now')`
        // (the :9161 no-inline-clock rule). edge_now is ALWAYS present, so unlike
        // node validity this conjunct is unconditional (an edge invalidated in the
        // past stays excluded even when node existence is relaxed).
        let edge_validity = edge_validity_sql_for_view("canonical_edges", 2, &view.view);
        let edge_eligibility = read_dependency_eligibility("canonical_edges", 2);
        let mut edge_stmt = prepare_search_statement(
            &tx,
            &format!(
                "SELECT body, logical_id, source_id FROM canonical_edges \
             WHERE write_cursor = ?1 AND superseded_at IS NULL AND body IS NOT NULL\
             {edge_validity}{edge_eligibility} LIMIT 1"
            ),
        )?;
        // The bound parameter list for the node lookup: the candidate rowid,
        // plus `:now` when (and only when) the view emitted a validity conjunct.
        // One instant for the whole query — resolved once, above, not per row.
        let node_params = |rowid: i64| -> Vec<rusqlite::types::Value> {
            let mut p = vec![rusqlite::types::Value::Integer(rowid)];
            if let Some(now) = now_param {
                p.push(rusqlite::types::Value::Integer(now));
            }
            p
        };
        for (rowid, score) in rowids {
            // fix-3 (F1): the candidate list is now the OVERFETCHED pool in
            // exact-L2 order, so the caller's cutoff is applied HERE — after
            // the validity/existence filter, not before it. Bounded worst case:
            // at most `TOP_K_BIT_CANDIDATES` hydration probes when nearly every
            // candidate is filtered out; exactly `final_limit` (i.e. unchanged)
            // when nothing is. Ordering is unchanged — the surviving rows are
            // still emitted nearest-first — so on a corpus with no windows this
            // loop yields byte-identical results to the pre-fix code.
            if results.len() >= final_limit {
                break;
            }
            if let Some((kind, body, logical_id, source_id)) = node_stmt
                .query_row(rusqlite::params_from_iter(node_params(rowid)), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                })
                .optional()?
            {
                let id = derive_stable_id(logical_id.as_deref(), &body);
                results.push(SearchHit {
                    id,
                    write_cursor: rowid as u64,
                    kind,
                    body,
                    score,
                    branch: SoftFallbackBranch::Vector,
                    // TC-31: the NODE's own provenance (a node hit is erased by
                    // the document it was written from).
                    source_id,
                    ce_score: None,
                });
            } else if let Some((body, logical_id, source_id)) = edge_stmt
                .query_row(rusqlite::params![rowid, view.edge_now()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .optional()?
            {
                let id = derive_stable_id(logical_id.as_deref(), &body);
                results.push(SearchHit {
                    id,
                    write_cursor: rowid as u64,
                    kind: "edge_fact".to_string(),
                    body,
                    score,
                    branch: SoftFallbackBranch::TextEdge,
                    // TC-31: the EDGE's own provenance — consistent with the
                    // graph arm's existing edge-source semantics.
                    source_id,
                    ce_score: None,
                });
            }
        }
        results
    } else {
        Vec::new()
    };
    let vector_rows_visible = !vector_results.is_empty();
    let soft_fallback = if vector_eligibility_degraded {
        Some(SoftFallback { branch: SoftFallbackBranch::Vector })
    } else if query_vector.is_some() && !vector_rows_visible {
        tx.query_row(
            "SELECT 1
             FROM search_index
             JOIN _fathomdb_vector_kinds ON _fathomdb_vector_kinds.kind = search_index.kind
             LEFT JOIN _fathomdb_projection_terminal
               ON _fathomdb_projection_terminal.write_cursor = search_index.write_cursor
             WHERE search_index MATCH ?1
              AND _fathomdb_projection_terminal.write_cursor IS NULL
             LIMIT 1",
            [compiled.match_expression.as_str()],
            |_row| Ok(SoftFallback { branch: SoftFallbackBranch::Vector }),
        )
        .ok()
    } else {
        None
    };
    // Collect the text branch (ranked by `write_cursor`, as 0.7.2), then
    // post-filter it against the same metadata the vector branch was pruned by
    // in SQL (the vector branch is filtered in phase 1; the text branch has no
    // metadata columns of its own).
    let mut deferred_text_identity = false;
    let text_candidates: Vec<SearchHit> = {
        #[cfg(feature = "tc5-benchmark")]
        tc5_benchmark::record_fts_route();
        // Optional experimental FTS5 LIMIT cap. It applies only when
        // FATHOMDB_PERF_EXPERIMENTS is present and
        // FATHOMDB_PERF_SEARCH_LIMIT parses as a number. The default path
        // keeps the uncapped candidate set.
        let perf_limit: Option<usize> = if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_some() {
            std::env::var("FATHOMDB_PERF_SEARCH_LIMIT").ok().and_then(|s| s.parse().ok())
        } else {
            None
        };
        // Only the explicit direct text-only API fixes its node candidate window.
        // A missing vector is not sufficient evidence of that API: hybrid search
        // can also take its no-vector fallback and must retain its existing input
        // behavior. The fixed direct bound makes node inputs invariant across
        // accepted public limits before edge-body fusion and final truncation.
        let fts_only_limit =
            direct_text_candidate_limit.or_else(|| query_vector.is_none().then_some(final_limit));
        // G1: SELECT body + kind + write_cursor (interim id) and the
        // `bm25()` text-relevance score. IR-C (2026-06-10,
        // `performance-output-and-compare.md`): the per-branch rank RRF fuses on
        // must be **`bm25()` relevance**, not `write_cursor` (insertion order) —
        // the prior `ORDER BY write_cursor` meant the lexical arm never ranked by
        // relevance, the single biggest fusion bug. `bm25()` is more-negative ⇒
        // better, so ascending puts best matches first; `write_cursor` is the
        // deterministic tiebreak. The filter is applied as a Rust post-filter so
        // the unfiltered path is untouched.
        let limit_clause =
            fts_only_limit.or(perf_limit).map(|k| format!(" LIMIT {k}")).unwrap_or_default();
        // Cause-A: PREFER a logical_id-bearing query — LEFT JOIN canonical_nodes so
        // node hits carry the `l:`-tagged stable id. The join is 1:1 on
        // `write_cursor` (search_index holds node bodies only; edge bodies live in
        // search_index_edges), so the row-set and the `bm25(search_index),
        // write_cursor` ordering are byte-unchanged — only `cn.logical_id` is added.
        // FALL BACK to the original (logical_id-free) query on pre-step-12 schemas
        // (v10) whose `canonical_nodes` lacks `logical_id`: those hits key by the
        // `h:` content-hash. This keeps old-schema search byte-identical to the
        // pre-Cause-A behaviour (the prepare of the plain SQL is the exact prior
        // statement). Columns are qualified because both tables expose `write_cursor`.
        // CORRECTNESS (0.8.11.2 pico): `AND cn.superseded_at IS NULL` drops
        // superseded node versions. Node supersession is tombstone-then-insert
        // (`commit_batch`): the prior `canonical_nodes` row is UPDATEd to set
        // `superseded_at` (row kept, same write_cursor) and a NEW `search_index`
        // row is inserted for the new cursor — the OLD `search_index` row is
        // never deleted, so without this filter both versions stay live in FTS
        // and the stale one is returned. The other arms already filter this way
        // (edge branch, graph-arm node seed, point-recall `read_get_by_id`);
        // only this default node-text branch was missing it. The `LEFT JOIN` is
        // KEPT (not switched to inner): an active row joins to its `cn` with
        // `superseded_at = NULL` (kept); a superseded row joins to its tombstoned
        // `cn` with `superseded_at` NOT NULL (dropped); a legacy/orphan
        // `search_index` row with no `cn` gets `superseded_at = NULL` via the
        // LEFT JOIN (KEPT — preserves prior behaviour for ownerless rows).
        // TC-31 (0.8.20 Slice 10a): `cn.source_id` is selected off the SAME
        // already-present 1:1 LEFT JOIN that supplies `cn.logical_id` — one extra
        // column, no extra query, no row-set or ordering change.
        // fix-2 (codex §9 [P2]): the node-body FTS branch takes the SAME
        // validity conjunct, generated by `ReadView::validity_sql` rather than
        // hand-rolled — the predicate lives in exactly one place (Slice 10).
        // `?1` is the MATCH expression, so `:now` binds at `?2`.
        //
        // The generated conjunct is NULL-PERMISSIVE by construction
        // (`valid_from IS NULL OR ...`), which is exactly what this LEFT JOIN
        // needs: an ownerless `search_index` row with no `cn` reads NULL on both
        // columns and is KEPT, preserving the deliberate keep-ownerless
        // behaviour the `superseded_at` / `state` conjuncts above encode with
        // their explicit `OR ... IS NULL`. No extra `OR IS NULL` is needed here,
        // and none may be added: that would be a second, drifting copy of the
        // predicate.
        //
        // NO-REGRESSION: on a corpus that never authored a window every
        // `cn.valid_from` / `cn.valid_until` is NULL (step 22 back-filled NULL
        // with no DEFAULT), so both disjuncts are TRUE for every row and the
        // row-set, the `bm25(search_index), write_cursor` ordering and the
        // scores are all byte-unchanged.
        let text_validity = view.validity_sql("cn", 2);
        let text_eligibility = read_dependency_eligibility("cn", 2);
        let mut text_params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Text(compiled.match_expression.clone())];
        if let Some(now) = now_param {
            text_params.push(rusqlite::types::Value::Integer(now));
        }
        let text_filter = append_node_eligibility_sql(filter, "cn", &mut text_params);
        #[cfg(feature = "test-hooks")]
        let force_full_sort =
            std::env::var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST").is_ok_and(|value| value == "1");
        #[cfg(not(feature = "test-hooks"))]
        let force_full_sort = false;
        let defer_text_identity = query_vector.is_some()
            && filter.is_none()
            && perf_limit.is_none()
            && !has_source_dependencies
            && !view.view.include_superseded
            && !view.view.include_inactive
            && !view.view.include_out_of_window
            && !recency_enabled
            && !importance_enabled
            && rerank_depth == 0
            && !use_graph_arm
            && !explain
            && !force_full_sort
            && dependency_closure::all_nodes_directly_eligible(
                &tx,
                view.view.include_superseded,
                view.view.include_inactive,
                view.view.include_out_of_window,
                view.edge_now(),
            )?;
        let join_sql =
            body_fts_rank_sql(&text_validity, &text_eligibility, &text_filter, &limit_clause);
        let rank_stream_requested =
            direct_text_candidate_limit.is_some() && filter.is_none() && !force_full_sort;
        let rank_stream_eligible = rank_stream_requested
            && tx
                .query_row(
                    "SELECT NOT EXISTS(SELECT 1 FROM search_index_edges LIMIT 1)",
                    [],
                    |row| row.get::<_, bool>(0),
                )
                .unwrap_or(false);
        let rank_stream_candidates = if rank_stream_eligible {
            let rank_candidate_limit = fts_only_limit.unwrap_or(final_limit);
            #[cfg(feature = "test-hooks")]
            let rank_mapping = if std::env::var_os("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST").is_some() {
                "missing_ranker()"
            } else {
                "bm25()"
            };
            #[cfg(not(feature = "test-hooks"))]
            let rank_mapping = "bm25()";
            let rank_sql = format!(
                "SELECT search_index.body, search_index.kind, search_index.write_cursor, \
                 bm25(search_index), cn.logical_id, cn.source_id FROM search_index \
                 LEFT JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor \
                 WHERE search_index MATCH ?1 \
                   AND cn.superseded_at IS NULL \
                   AND (cn.state = 'active' OR cn.state IS NULL)\
                   {text_validity}{text_eligibility}{text_filter} \
                   AND rank MATCH '{rank_mapping}' \
                 ORDER BY rank"
            );
            #[cfg(feature = "test-hooks")]
            record_fts_query_plan_for_test(&tx, &rank_sql, &text_params);
            tx.prepare(&rank_sql)
                .and_then(|mut statement| {
                    let mut rows =
                        statement.query(rusqlite::params_from_iter(text_params.iter()))?;
                    collect_complete_rank_boundary(&mut rows, rank_candidate_limit)
                })
                .ok()
        } else {
            None
        };
        let deferred_candidates = if defer_text_identity {
            let now_binding =
                now_param.map(|_| " AND (?2 IS NULL OR ?2 IS NOT NULL)").unwrap_or("");
            let sql = format!(
                "SELECT body, kind, write_cursor, bm25(search_index) FROM search_index \
                 WHERE search_index MATCH ?1{now_binding}{limit_clause}"
            );
            let mut candidates =
                prepare_search_statement(&tx, &sql).and_then(|mut statement| {
                    let rows = statement.query_map(
                        rusqlite::params_from_iter(text_params.iter()),
                        |row| {
                            let body = row.get::<_, String>(0)?;
                            Ok(SearchHit {
                                id: IdSpace::content(String::new()),
                                body,
                                kind: row.get::<_, String>(1)?,
                                write_cursor: row.get::<_, i64>(2)? as u64,
                                score: row.get::<_, f64>(3)?,
                                branch: SoftFallbackBranch::Text,
                                source_id: None,
                                ce_score: None,
                            })
                        },
                    )?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })?;
            candidates.sort_by(|left, right| {
                left.score
                    .total_cmp(&right.score)
                    .then_with(|| left.write_cursor.cmp(&right.write_cursor))
            });
            Some(candidates)
        } else {
            None
        };
        if let Some(candidates) = deferred_candidates {
            deferred_text_identity = true;
            #[cfg(feature = "test-hooks")]
            record_fts_route_for_test("hybrid_deferred_identity");
            candidates
        } else if let Some((candidates, _crossed_boundary_tie)) = rank_stream_candidates {
            #[cfg(feature = "test-hooks")]
            record_fts_route_for_test(if _crossed_boundary_tie {
                "rank_stream_tie_completed"
            } else {
                "rank_stream_strict_boundary"
            });
            candidates
        } else if let Ok(mut statement) = tx.prepare(&join_sql) {
            #[cfg(feature = "test-hooks")]
            if query_vector.is_some() && force_full_sort {
                record_fts_route_for_test("hybrid_full_sort_forced");
            } else if rank_stream_eligible {
                record_fts_route_for_test("full_sort_fallback");
            } else if rank_stream_requested {
                record_fts_route_for_test("full_sort_ineligible");
            }
            let rows =
                statement.query_map(rusqlite::params_from_iter(text_params.iter()), |row| {
                    let body = row.get::<_, String>(0)?;
                    let logical_id = row.get::<_, Option<String>>(4)?;
                    Ok(SearchHit {
                        id: derive_stable_id(logical_id.as_deref(), &body),
                        body,
                        kind: row.get::<_, String>(1)?,
                        write_cursor: row.get::<_, i64>(2)? as u64,
                        score: row.get::<_, f64>(3)?,
                        branch: SoftFallbackBranch::Text,
                        // TC-31: the NODE's own provenance. NULL for a legacy /
                        // TC-11-spared governed row, and NULL for an ownerless
                        // `search_index` row the LEFT JOIN keeps with no `cn`.
                        source_id: row.get::<_, Option<String>>(5)?,
                        ce_score: None,
                    })
                })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            // No `superseded_at IS NULL` filter here (and none is possible): this
            // fallback fires only on pre-step-12 schemas whose `canonical_nodes`
            // lacks `logical_id` — and step-12 adds `logical_id` and
            // `superseded_at` in the SAME migration, so this schema has neither
            // column. Supersession (`commit_batch`) is a no-op without
            // `logical_id`, so no superseded node rows can exist on this path.
            //
            // TC-31 (0.8.20 Slice 10a): `source_id` arrived in step 8, `logical_id`
            // in step 12, so a schema that lands HERE (no `logical_id`) may still
            // HAVE `source_id` — steps 8..11. Try a provenance-bearing variant
            // first, adding only `cn.source_id` over the SAME 1:1 LEFT JOIN shape
            // used above (row-set and ordering unchanged; a missing `cn` row keeps
            // NULL as before). Fall back to the historical, byte-identical
            // provenance-free statement on a pre-step-8 schema, where the column
            // genuinely does not exist and `None` is the only truthful answer.
            let source_sql = format!(
                "SELECT search_index.body, search_index.kind, search_index.write_cursor, \
                 bm25(search_index), cn.source_id FROM search_index \
                 LEFT JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor \
                 WHERE search_index MATCH ?1 \
                 ORDER BY bm25(search_index), search_index.write_cursor{limit_clause}"
            );
            if let Ok(mut statement) = tx.prepare(&source_sql) {
                let rows = statement.query_map([compiled.match_expression.as_str()], |row| {
                    let body = row.get::<_, String>(0)?;
                    Ok(SearchHit {
                        // No logical_id column on this schema → content-hash id.
                        id: derive_stable_id(None, &body),
                        body,
                        kind: row.get::<_, String>(1)?,
                        write_cursor: row.get::<_, i64>(2)? as u64,
                        score: row.get::<_, f64>(3)?,
                        branch: SoftFallbackBranch::Text,
                        source_id: row.get::<_, Option<String>>(4)?,
                        ce_score: None,
                    })
                })?;
                rows.flatten().collect()
            } else {
                // Pre-step-8: no `source_id` column anywhere. Byte-identical to
                // the historical statement.
                let plain_sql = format!(
                    "SELECT body, kind, write_cursor, bm25(search_index) FROM search_index \
                     WHERE search_index MATCH ?1 \
                     ORDER BY bm25(search_index), write_cursor{limit_clause}"
                );
                let mut statement = tx.prepare(&plain_sql)?;
                let rows = statement.query_map([compiled.match_expression.as_str()], |row| {
                    let body = row.get::<_, String>(0)?;
                    Ok(SearchHit {
                        // No logical_id column on this schema → content-hash id.
                        id: derive_stable_id(None, &body),
                        body,
                        kind: row.get::<_, String>(1)?,
                        write_cursor: row.get::<_, i64>(2)? as u64,
                        score: row.get::<_, f64>(3)?,
                        branch: SoftFallbackBranch::Text,
                        source_id: None,
                        ce_score: None,
                    })
                })?;
                rows.flatten().collect()
            }
        }
    };
    let mut text_results: Vec<SearchHit> = text_candidates;

    // G11 (Slice 15) — edge-body FTS branch from `search_index_edges`.
    // Appended to text_results; tagged with SoftFallbackBranch::TextEdge so
    // callers can distinguish edge hits from node hits.
    //
    // fix-1 [P2]: JOIN canonical_edges to exclude superseded edge rows
    // (invalidate-not-accumulate can leave a superseded body in the FTS index).
    // fix-2 [P2]: use edge_fts_hit_passes_filter (NOT text_hit_passes_filter).
    // Edge hits always have source_type="edge_fact"; text_hit_passes_filter
    // calls resolve_source_type(relation_kind) which returns Err for unknown
    // relation kinds, silently rejecting every edge hit when a source_type
    // filter is set — the exact inverse of correct behaviour.
    // fix-3 [P2]: edge_fts_hit_passes_filter now queries vector_default for
    // created_after/status (mirroring text_hit_passes_filter). Collect edge
    // candidates into a Vec first (drops stmt borrow on tx) so we can pass
    // &tx to edge_fts_hit_passes_filter without a borrow conflict.
    let edge_candidates: Vec<SearchHit> = {
        // Cause-A: the JOIN to canonical_edges already exists; additively select
        // `ce.logical_id` (edges always carry one) for the stable hit-id. No
        // ordering/row-set change.
        // TC-31 (0.8.20 Slice 10a): `ce.source_id` rides the SAME existing inner
        // JOIN as `ce.logical_id` — one extra column, no extra query, no
        // row-set/ordering change. An edge hit carries the EDGE's own provenance,
        // matching the graph arm's edge-source semantics.
        // fix-2 (codex §9 [P2]): the JOIN already dropped superseded edge rows,
        // but a body-bearing edge written with `t_invalid <= :now` (expired /
        // invalidated) still MATCHed and surfaced its body through ordinary
        // search — edge temporal validity was enforced on the graph-traversal and
        // projection paths but NOT on this FTS read path. Apply the shared
        // `edge_validity_sql` conjunct (the ONE generator every edge read site
        // uses, so no path can drift). `?1` is the MATCH expression, so the edge
        // `:now` binds at `?2`; the instant is the frozen `view.edge_now()` — a
        // bound value, never `datetime('now')` (the :9161 no-inline-clock rule),
        // and always present (edge invalidation is not relaxed by node existence
        // relaxation).
        let edge_validity = edge_validity_sql_for_view("ce", 2, &view.view);
        let edge_dependency = read_dependency_eligibility("ce", 2);
        let mut edge_params = vec![
            rusqlite::types::Value::Text(compiled.match_expression.clone()),
            rusqlite::types::Value::Integer(view.edge_now()),
        ];
        let edge_filter = append_edge_eligibility_sql(filter, "ce", &mut edge_params);
        let edge_eligibility = format!("{edge_dependency}{edge_filter}");
        let edge_sql = edge_fts_rank_sql(&edge_validity, &edge_eligibility);
        let mut stmt = prepare_search_statement(&tx, &edge_sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(edge_params.iter()), |row| {
            let body = row.get::<_, String>(0)?;
            let logical_id = row.get::<_, Option<String>>(4)?;
            Ok(SearchHit {
                id: derive_stable_id(logical_id.as_deref(), &body),
                body,
                kind: row.get::<_, String>(1)?,
                write_cursor: row.get::<_, i64>(2)? as u64,
                score: row.get::<_, f64>(3)?,
                branch: SoftFallbackBranch::TextEdge,
                // TC-31: the EDGE's own provenance.
                source_id: row.get::<_, Option<String>>(5)?,
                ce_score: None,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    // Attribute predicates intentionally apply only to node projections. Count
    // edge-FTS candidates that would otherwise pass when the caller requested
    // the opt-in explanation, without adding work to the default search path.
    let dropped_edge_hits =
        if explain && filter.is_some_and(|active_filter| !active_filter.attributes.is_empty()) {
            let mut non_attribute_filter = filter.cloned().unwrap_or_default();
            non_attribute_filter.attributes.clear();
            let edge_validity = edge_validity_sql_for_view("ce", 2, &view.view);
            let mut params = vec![
                rusqlite::types::Value::Text(compiled.match_expression.clone()),
                rusqlite::types::Value::Integer(view.edge_now()),
            ];
            let eligibility =
                append_edge_eligibility_sql(Some(&non_attribute_filter), "ce", &mut params);
            #[cfg(feature = "test-hooks")]
            let cursor_column =
                if std::env::var_os("FATHOMDB_EDGE_EXPLANATION_BAD_CURSOR_FOR_TEST").is_some() {
                    "CAST(ce.write_cursor AS BLOB)"
                } else {
                    "ce.write_cursor"
                };
            #[cfg(not(feature = "test-hooks"))]
            let cursor_column = "ce.write_cursor";
            let sql = format!(
                "SELECT {cursor_column} FROM search_index_edges sei \
             JOIN canonical_edges ce ON ce.write_cursor=sei.write_cursor \
             WHERE search_index_edges MATCH ?1 AND ce.superseded_at IS NULL\
             {edge_validity}{eligibility}"
            );
            let mut dropped = 0_u32;
            let mut statement = tx.prepare(&sql)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(params.iter()), |row| row.get::<_, i64>(0))?;
            for cursor in rows {
                if !hit_attributes_pass_filter(
                    &tx,
                    cursor? as u64,
                    filter.expect("attribute filter checked above"),
                )? {
                    dropped = dropped.saturating_add(1);
                }
            }
            dropped
        } else {
            0
        };
    text_results.extend(edge_candidates);
    // GA-2 / Slice-40 (◆ B-1) measurement seam: when `vector_stage_only` is set
    // (only ever by the eu7 recall harness via `set_vector_stage_only_for_test`,
    // off for every production caller), return the pre-fusion VECTOR-branch
    // ranking (bit-KNN K=192 + f32 rerank) verbatim, skipping `fuse_rrf` /
    // recency / `rerank_fused`. This exposes the ANN-quantization FIDELITY
    // signal — vector top-N vs the exact-f32 VECTOR top-10 ground truth — that
    // the AC-075 0.90 floor is defined to measure. It is NOT a `fusion_mode`
    // knob: the production branch below is byte-unchanged and RRF stays
    // unconditional.
    // G0 Phase-2 (BLOCK-1) side-channel meter — default (all-zero, rate 0.0) on
    // the non-graph-arm paths; populated by the BFS seed phase when graph-arm runs.
    let mut graph_stats = GraphFrontierStats::default();

    // 0.8.8 EXP-OBS (Slice 5) — capture per-arm rank maps + counts BEFORE the arms
    // are consumed by fusion. All reads; only when `explain` (else zero work).
    // `body_rank_map` keeps the FIRST occurrence (== the rank `fuse_three_arms`
    // uses, which dedups keeping the first). `*_fused_scores` is captured from the
    // post-recency / pre-CE intermediate so `fused_score` is faithful to what
    // `ce_rerank` normalizes.
    let body_rank_map = |hits: &[SearchHit]| -> HashMap<String, u32> {
        let mut m: HashMap<String, u32> = HashMap::new();
        for (i, h) in hits.iter().enumerate() {
            m.entry(h.body.clone()).or_insert(i as u32);
        }
        m
    };
    let body_score_map = |hits: &[SearchHit]| -> HashMap<String, f64> {
        hits.iter().map(|h| (h.body.clone(), h.score)).collect()
    };

    let (exp_vector_ranks, exp_text_ranks, exp_vector_n, exp_text_n) = if explain {
        (
            Some(body_rank_map(&vector_results)),
            Some(body_rank_map(&text_results)),
            vector_results.len() as u32,
            text_results.len() as u32,
        )
    } else {
        (None, None, 0, 0)
    };
    let mut exp_graph_ranks: Option<HashMap<String, u32>> = None;
    let mut exp_fused_scores: Option<HashMap<String, f64>> = None;
    let mut exp_graph_n: u32 = 0;
    // F9 (0.8.16 Slice 5) — per-hit importance/confidence contribution maps
    // (keyed by hit id == write_cursor), captured for the explain sidecar.
    let mut exp_importance: Option<HashMap<u64, f64>> = None;
    let mut exp_confidence: Option<HashMap<u64, f64>> = None;

    let mut results = if vector_stage_only {
        vector_results
    } else if use_graph_arm {
        // R3 (Slice 30) — graph arm: BFS over temporal fact-edges seeded from
        // the top-10 two-arm fused candidates, depth ≤ 3, cap 50.
        // Temporal filter: superseded_at IS NULL AND (t_invalid IS NULL OR t_invalid > now).
        // Synthesized-node penalty: kind = 'unknown' → score *= 0.3.
        //
        // Approach: compute the two-arm fused result first (for BFS seeding),
        // then fuse three arms: the two-arm result (as "vector" arm), an empty
        // text arm, and the graph candidates. The two-arm result preserves all
        // existing ranking semantics; the graph arm contributes new candidates.
        let two_arm_fused = fuse_rrf(vector_results, text_results);
        // C1: seed the graph arm from the query's FTS match expression (entities /
        // edge-facts), not the doc-node fused hits. `fused_hits` is still passed for
        // the seed-body exclusion set.
        let (graph_candidates, stats, graph_edge_confidence) = bfs_graph_arm_candidates(
            &tx,
            &two_arm_fused,
            compiled.match_expression.as_str(),
            3,
            50,
            view,
            filter,
            &mut capture,
        )?;
        graph_stats = stats;
        if explain {
            exp_graph_ranks = Some(body_rank_map(&graph_candidates));
            exp_graph_n = graph_candidates.len() as u32;
        }
        // Named intermediate (byte-identical to the prior nested call) so explain
        // can read the pre-CE fused scores without perturbing the ranking.
        let fused = apply_recency_reweight(
            fuse_three_arms(two_arm_fused, vec![], graph_candidates),
            recency_enabled,
        );
        // F9 — importance (node) / confidence (edge) reweight, OFF by default.
        // Order: AFTER recency (consistent placement), BEFORE the CE rerank seam.
        let (imp_map, mut conf_map) = if importance_enabled || explain {
            build_importance_confidence_maps(&tx, &fused)?
        } else {
            (HashMap::new(), HashMap::new())
        };
        // F9 FIX-1: `build_importance_confidence_maps` keys edge confidence on the
        // EDGE `write_cursor`, which never matches a graph-arm NODE hit's cursor —
        // so it alone leaves graph-arm hits with no edge confidence. Merge the
        // BFS-collected per-node traversing-edge confidence (node cursor ⇒ conf).
        // Node/edge cursors are globally unique, so there is never a key collision
        // with the edge-fact confidence above; `or_insert` documents that intent.
        if importance_enabled || explain {
            for (cursor, conf) in &graph_edge_confidence {
                conf_map.entry(*cursor).or_insert(*conf);
            }
        }
        let fused = apply_importance_reweight(fused, &imp_map, &conf_map, importance_enabled);
        if explain {
            exp_importance = Some(imp_map);
            exp_confidence = Some(conf_map);
            exp_fused_scores = Some(body_score_map(&fused));
        }
        try_rerank_fused(raw_query, fused, rerank_depth, alpha, pool_n)
            .map_err(SearchReaderError::RerankerDevicePolicy)?
    } else {
        // G9 + G12: RRF-fuse the two ranked branches (keyed on body, vector-first
        // tiebreak) into the unconditional new ranking, recency-reweight (gated,
        // off by default), then pass through the identity rerank seam. The
        // vector-empty `soft_fallback` signal was computed above, BEFORE this
        // branch-collapse.
        let fused = apply_recency_reweight(fuse_rrf(vector_results, text_results), recency_enabled);
        // F9 — importance (node) / confidence (edge) reweight, OFF by default.
        // Same placement as the graph-arm branch: after recency, before CE rerank.
        let (imp_map, conf_map) = if importance_enabled || explain {
            build_importance_confidence_maps(&tx, &fused)?
        } else {
            (HashMap::new(), HashMap::new())
        };
        let fused = apply_importance_reweight(fused, &imp_map, &conf_map, importance_enabled);
        if explain {
            exp_importance = Some(imp_map);
            exp_confidence = Some(conf_map);
            exp_fused_scores = Some(body_score_map(&fused));
        }
        try_rerank_fused(raw_query, fused, rerank_depth, alpha, pool_n)
            .map_err(SearchReaderError::RerankerDevicePolicy)?
    };

    results.truncate(final_limit);

    if deferred_text_identity {
        let mut identity_stmt = tx.prepare_cached(
            "SELECT logical_id, source_id FROM canonical_nodes WHERE write_cursor=?1 LIMIT 1",
        )?;
        for hit in &mut results {
            if hit.branch != SoftFallbackBranch::Text {
                continue;
            }
            if let Ok((logical_id, source_id)) = identity_stmt
                .query_row([hit.write_cursor], |row| {
                    Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?))
                })
            {
                hit.id = derive_stable_id(logical_id.as_deref(), &hit.body);
                hit.source_id = source_id;
            } else {
                hit.id = derive_stable_id(None, &hit.body);
            }
        }
    }

    let projection_status = explain
        .then(|| {
            projection_generation::status_in_snapshot(
                &tx,
                projection_runtime_state,
                view.edge_now(),
                load_next_cursor(&tx),
            )
        })
        .transpose()
        .map_err(SearchReaderError::Evidence)?;

    // 0.8.8 EXP-OBS — assemble the sidecar `Explanation` from the captured maps +
    // the final `results`. `embedder_id` is left empty here (the worker has no
    // identity) and filled by `search_inner_with_stats`.
    let explanation = if explain {
        let fused_scores = exp_fused_scores.unwrap_or_default();
        let per_hit: Vec<PerHitExplain> = results
            .iter()
            .map(|h| -> rusqlite::Result<PerHitExplain> {
                Ok(PerHitExplain {
                    // `PerHitExplain.id` carries the engine-internal positional
                    // `write_cursor` (the pre-C-2 `SearchHit.id`), matching the
                    // telemetry `result_ids` / importance-map key space; the typed
                    // `SearchHit.id` is the separate caller-facing identity.
                    id: h.write_cursor,
                    arm: h.branch,
                    vector_rank: exp_vector_ranks.as_ref().and_then(|m| m.get(&h.body).copied()),
                    text_rank: exp_text_ranks.as_ref().and_then(|m| m.get(&h.body).copied()),
                    graph_rank: exp_graph_ranks.as_ref().and_then(|m| m.get(&h.body).copied()),
                    fused_score: fused_scores.get(&h.body).copied().unwrap_or(h.score),
                    ce_score: h.ce_score,
                    blended: h.score,
                    importance: exp_importance
                        .as_ref()
                        .and_then(|m| m.get(&h.write_cursor).copied()),
                    confidence: exp_confidence
                        .as_ref()
                        .and_then(|m| m.get(&h.write_cursor).copied()),
                    structural: {
                        let projection_origin = match h.branch {
                            SoftFallbackBranch::Vector => {
                                StructuralProjectionOriginV1::CurrentDenseGeneration
                            }
                            SoftFallbackBranch::GraphArm => {
                                StructuralProjectionOriginV1::GraphTraversal
                            }
                            SoftFallbackBranch::Text | SoftFallbackBranch::TextEdge => {
                                StructuralProjectionOriginV1::SynchronousBodyFts
                            }
                        };
                        let lifecycle_state = structural_lifecycle_state(&tx, h.write_cursor)?;
                        let mut degradation_codes = match (&soft_fallback, h.branch) {
                            (Some(_), SoftFallbackBranch::Text) => {
                                vec![StructuralDegradationCodeV1::SoftFallbackText]
                            }
                            (Some(_), SoftFallbackBranch::TextEdge) => {
                                vec![StructuralDegradationCodeV1::SoftFallbackTextEdge]
                            }
                            _ => Vec::new(),
                        };
                        if let Some(status) = projection_status.as_ref() {
                            let code = if status.origin
                                == ProjectionGenerationOriginV1::LegacyUnverified
                            {
                                Some(StructuralDegradationCodeV1::ProjectionLegacyUnverified)
                            } else {
                                match status.readiness {
                                    ProjectionReadinessV1::Blocked => {
                                        Some(StructuralDegradationCodeV1::ProjectionBlocked)
                                    }
                                    ProjectionReadinessV1::Deferred => {
                                        Some(StructuralDegradationCodeV1::ProjectionDeferred)
                                    }
                                    _ => None,
                                }
                            };
                            if let Some(code) = code {
                                degradation_codes.push(code);
                            }
                        }
                        if graph_stats.bound_reached {
                            degradation_codes.push(StructuralDegradationCodeV1::GraphBoundReached);
                        }
                        degradation_codes.sort_unstable();
                        degradation_codes.dedup();
                        StructuralInclusionV1 {
                            schema_version: 1,
                            inclusion_state: if degradation_codes.is_empty() {
                                StructuralInclusionStateV1::Included
                            } else {
                                StructuralInclusionStateV1::Degraded
                            },
                            projection_origin,
                            dependency_state: structural_dependency_state(
                                &tx,
                                h.write_cursor,
                                view.edge_now(),
                            )?,
                            lifecycle_state,
                            degradation_codes,
                        }
                    },
                })
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let ce_active = rerank_depth > 0 && per_hit.iter().any(|p| p.ce_score.is_some());
        Some(Explanation {
            trace: QueryTrace {
                query_chars: raw_query.chars().count() as u32,
                k: final_limit as u32,
                rerank_depth: rerank_depth as u32,
                pool_n: pool_n as u32,
                alpha,
                use_graph_arm,
                recency: recency_enabled,
                embedder_id: String::new(),
                ce_active,
                vector_hits: exp_vector_n,
                text_hits: exp_text_n,
                graph_hits: exp_graph_n,
                dropped_edge_hits,
            },
            per_hit,
            correlation_id: String::new(),
        })
    } else {
        None
    };

    let expanded = expand_depth
        .map(|depth| search_expand_on_snapshot(&tx, &results, depth, view, filter))
        .transpose()?;
    let output =
        capture.finish(&tx, cursor, soft_fallback, results, graph_stats, explanation, expanded)?;
    tx.commit()?;
    Ok(output)
}
use crate::dependency_closure;
use crate::embed_dispatch::{DispatchError, EmbedDispatcher};
use crate::embedding::dispatch_embed_vector;
use crate::errors::EngineError;
use crate::evidence::{self, EvidenceSearchResultV1};
use crate::filter::{
    append_edge_eligibility_sql, append_node_eligibility_sql, body_fts_rank_sql,
    build_vector_phase1_sql, edge_fts_rank_sql, hit_attributes_pass_filter, property_fts_rank_sql,
    validate_filter_attributes_on_snapshot, vector_filter_values, SearchFilter,
    SnapshotFilterError,
};
use crate::frozen_read::{self, FrozenReadContextV1, FrozenReadError, FrozenReadErrorReason};
use crate::fusion::{
    apply_importance_reweight, apply_recency_reweight, build_importance_confidence_maps, fuse_rrf,
    fuse_three_arms,
};
use crate::graph_expand::{self, search_expand_on_snapshot, SearchExpandResult};
use crate::identity::{derive_stable_id, IdSpace};
use crate::mean::{identity_requires_mean_centering, read_pinned_mean_vec, subtract_mean};
use crate::projection_commit::load_projection_cursor;
use crate::projection_generation::{
    self, ProjectionGenerationOriginV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
};
use crate::projection_registry::load_projection_registry;
use crate::projection_runtime::PROJECTION_CURSOR_KEY;
use crate::reader_transaction::begin_attributed_reader_tx;
use crate::rerank::try_rerank_fused;
use crate::search_types::{
    validate_search_result_limit, Bm25fQueryPlan, Explanation, GraphFrontierStats, PerHitExplain,
    QueryTrace, SearchHit, SearchResult, SoftFallback, SoftFallbackBranch,
    StructuralDegradationCodeV1, StructuralInclusionStateV1, StructuralInclusionV1,
    StructuralLifecycleStateV1, StructuralProjectionOriginV1, MAX_SEARCH_RESULT_LIMIT,
    TOP_K_BIT_CANDIDATES,
};
use crate::structural_state::structural_dependency_state;
#[cfg(feature = "tc5-benchmark")]
use crate::tc5_benchmark;
use crate::temporal::{current_epoch_seconds, edge_validity_sql_for_view, FrozenView, ReadView};
use crate::test_hooks::{
    evidence_linearization_hooks, frozen_after_validation_hook, reader_search_hook,
};
use crate::wal_attribution::WalAttributionCollector;
use crate::write_commit::load_next_cursor;
use fathomdb_embedder::RerankerDevicePolicyError;
use fathomdb_embedder_api::EmbedderIdentity;
use fathomdb_query::compile_text_query;
use rusqlite::{params, CachedStatement, Connection, OptionalExtension, Statement};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[cfg(feature = "test-hooks")]
use std::{fs::OpenOptions, io::Write, sync::Mutex};
