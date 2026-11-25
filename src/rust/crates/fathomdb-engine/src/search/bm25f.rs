use crate::dependency_closure;
use crate::search_types::Bm25fQueryPlan;
use crate::temporal::current_epoch_seconds;
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};

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
/// arm. See [`crate::Engine::bm25f_search`].
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
