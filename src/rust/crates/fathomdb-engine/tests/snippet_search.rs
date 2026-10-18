//! EP-17 match-centred snippets on real stores
//! (`dev/design/agent-access/match-centred-snippets.md` §§ 6, 7.2, 8).

use std::sync::{Arc, Mutex, MutexGuard};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    take_slice71_search_statement_trace_for_test, Engine, InitialState, OpenedEngine,
    PreparedWrite, ReadContextV1, ReadView, SearchFilter, SearchResult, SnippetAnchor,
    SnippetOptions, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Serialises the tests: some read the process-wide statement trace.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("snippet-search", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        Ok(vector)
    }
}

struct Store {
    _dir: TempDir,
    opened: OpenedEngine,
}

impl Store {
    fn engine(&self) -> &Engine {
        &self.opened.engine
    }
}

fn store(bodies: &[&str]) -> Store {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("snippets{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let writes: Vec<PreparedWrite> =
        bodies.iter().enumerate().map(|(i, body)| node(&format!("n{i}"), body)).collect();
    if !writes.is_empty() {
        opened.engine.write(&writes).expect("write");
    }
    opened.engine.drain(10_000).expect("drain");
    Store { _dir: dir, opened }
}

fn node(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:snippets").expect("source id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn text(
    engine: &Engine,
    query: &str,
    limit: usize,
    snippet: Option<&SnippetOptions>,
) -> SearchResult {
    engine
        .search_text_only_view_with_snippets(query, &ReadView::default(), limit, snippet)
        .expect("text search")
}

fn hybrid(engine: &Engine, query: &str, snippet: Option<&SnippetOptions>) -> SearchResult {
    engine
        .search_reranked_view_with_snippets(
            query,
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            snippet,
        )
        .expect("hybrid search")
}

fn frozen(engine: &Engine, query: &str, snippet: Option<&SnippetOptions>) -> SearchResult {
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    engine
        .search_frozen_with_snippets(query, &context, 0, false, 0.3, 0, false, 10, snippet)
        .expect("frozen search")
}

/// A body of `len` bytes of filler words with `term` placed at byte `at`.
fn long_body(at: usize, term: &str, len: usize) -> String {
    let mut body = String::new();
    while body.len() < at {
        body.push_str("lorem ");
    }
    body.truncate(at);
    body.push_str(term);
    while body.len() < len {
        body.push_str(" ipsum");
    }
    body
}

// § 7.2: the sidecar is present, parallel to `results`, when asked for, on
// every search entry point that takes snippet options.
#[test]
fn sidecar_is_some_when_requested() {
    let _guard = lock();
    let s = store(&["a short pear note", &long_body(3000, "pear", 4096)]);
    let options = SnippetOptions::default();
    for (path, result) in [
        ("text", text(s.engine(), "pear", 10, Some(&options))),
        ("hybrid", hybrid(s.engine(), "pear", Some(&options))),
        ("frozen", frozen(s.engine(), "pear", Some(&options))),
    ] {
        assert!(!result.results.is_empty(), "{path}: no hits");
        let snippets = result.snippets.as_ref().unwrap_or_else(|| panic!("{path}: no sidecar"));
        assert_eq!(snippets.len(), result.results.len(), "{path}");
    }
    assert!(text(s.engine(), "pear", 10, None).snippets.is_none());
}

// § 7.2: out-of-range snippet options are InvalidArgument on every entry
// point that takes them.
#[test]
fn entry_points_reject_invalid_snippet_options() {
    let _guard = lock();
    let s = store(&["a short pear note"]);
    let bad = SnippetOptions { max_bytes: 39, ..SnippetOptions::default() };
    let context = s
        .engine()
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    let results = [
        (
            "text",
            s.engine().search_text_only_view_with_snippets(
                "pear",
                &ReadView::default(),
                10,
                Some(&bad),
            ),
        ),
        (
            "hybrid",
            s.engine().search_reranked_view_with_snippets(
                "pear",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
                Some(&bad),
            ),
        ),
        (
            "frozen",
            s.engine().search_frozen_with_snippets(
                "pear",
                &context,
                0,
                false,
                0.3,
                0,
                false,
                10,
                Some(&bad),
            ),
        ),
    ];
    for (path, result) in results {
        assert!(
            matches!(result, Err(fathomdb_engine::EngineError::InvalidArgument { .. })),
            "{path}: {result:?}"
        );
    }
}

fn snippet_of<'a>(result: &'a SearchResult, body: &str) -> &'a fathomdb_engine::SnippetV1 {
    let index = result
        .results
        .iter()
        .position(|hit| hit.body == body)
        .unwrap_or_else(|| panic!("hit missing: {result:?}"));
    &result.snippets.as_ref().expect("sidecar")[index]
}

// T-1: a 4 KiB record whose only query term is at byte 3000 gets a window
// centred on the term; today's prefix would miss it.
#[test]
fn t1_term_far_into_a_long_record_anchors_the_snippet() {
    let _guard = lock();
    let body = long_body(3000, "zanzibar", 4096);
    let s = store(&[&body]);
    let options = SnippetOptions::default();
    for (path, result) in [
        ("text", text(s.engine(), "zanzibar", 10, Some(&options))),
        ("hybrid", hybrid(s.engine(), "zanzibar", Some(&options))),
    ] {
        let snippet = snippet_of(&result, &body);
        assert_eq!(snippet.anchor, SnippetAnchor::Match, "{path}: {snippet:?}");
        assert!(snippet.text.contains("zanzibar"), "{path}: {snippet:?}");
        assert!(snippet.text.len() <= 300);
        assert_eq!(snippet.spans, vec![(3000, 3008)], "{path}");
    }
}

// T-2: porter parity. The query `running` matches `runs` far into the record
// (same stem); the window covers `runs`.
#[test]
fn t2_porter_stem_match_is_the_window() {
    let _guard = lock();
    // `long_body` places the term after whole filler words: 2502 = 417 × 6.
    let body = long_body(2502, "runs", 4096);
    let s = store(&[&body]);
    let result = text(s.engine(), "running", 10, Some(&SnippetOptions::default()));
    let snippet = snippet_of(&result, &body);
    assert_eq!(snippet.anchor, SnippetAnchor::Match, "{snippet:?}");
    assert!(snippet.text.contains("runs"), "{snippet:?}");
    assert_eq!(snippet.spans, vec![(2502, 2506)]);
}

// T-3: diacritics. `cafe` matches `Café` beyond the prefix in NFC and NFD
// forms; the highlight covers the word's bytes exactly, the NFD combining
// accent included (§ 6.1).
#[test]
fn t3_diacritic_match_highlight_covers_the_whole_word() {
    let _guard = lock();
    let lit = SnippetOptions { highlights: true, ..SnippetOptions::default() };
    for word in ["Caf\u{e9}", "Cafe\u{301}"] {
        let body = long_body(3000, word, 4096);
        let s = store(&[&body]);
        let result = text(s.engine(), "cafe", 10, Some(&lit));
        let snippet = snippet_of(&result, &body);
        assert_eq!(snippet.anchor, SnippetAnchor::Match, "{word:?}: {snippet:?}");
        assert_eq!(snippet.spans, vec![(3000, (3000 + word.len()) as u32)], "{word:?}");
        let lit_text: Vec<&str> = snippet
            .highlights
            .iter()
            .map(|&(a, b)| &snippet.text[a as usize..b as usize])
            .collect();
        assert_eq!(lit_text, [word], "{word:?}: {snippet:?}");
    }
}

fn highlight_queries() -> usize {
    take_slice71_search_statement_trace_for_test()
        .iter()
        .filter(|event| event.as_str() == "snippet_highlight")
        .count()
}

// T-19: rule 0. A body that fits, with highlights off, issues no point query
// and returns no spans; with highlights on it issues one.
#[test]
fn t19_a_body_that_fits_needs_no_point_query() {
    let _guard = lock();
    let body = "a short pear note";
    let s = store(&[body]);
    take_slice71_search_statement_trace_for_test();
    let result = text(s.engine(), "pear", 10, Some(&SnippetOptions::default()));
    assert_eq!(highlight_queries(), 0);
    let snippet = snippet_of(&result, body);
    assert_eq!(snippet.anchor, SnippetAnchor::Whole);
    assert!(snippet.spans.is_empty(), "{snippet:?}");

    let lit = SnippetOptions { highlights: true, ..SnippetOptions::default() };
    let result = text(s.engine(), "pear", 10, Some(&lit));
    assert_eq!(highlight_queries(), 1);
    let snippet = snippet_of(&result, body);
    assert_eq!(snippet.anchor, SnippetAnchor::Whole);
    assert_eq!(snippet.spans, vec![(8, 12)]);
    assert_eq!(snippet.highlights, vec![(8, 12)]);
}

// T-20: the point-query cap. With 30 long hits and max_point_queries = 20,
// hits 1–20 in rank order are anchored on the match and hits 21–30 get the
// record start, with 20 point queries, the same on repeat.
#[test]
fn t20_point_query_cap_anchors_the_rest_at_the_start() {
    let _guard = lock();
    let bodies: Vec<String> =
        (0..30).map(|i| format!("{} doc{i}", long_body(3000, "zanzibar", 4096))).collect();
    let refs: Vec<&str> = bodies.iter().map(String::as_str).collect();
    let s = store(&refs);
    let capped = SnippetOptions { max_point_queries: 20, ..SnippetOptions::default() };
    take_slice71_search_statement_trace_for_test();
    let first = text(s.engine(), "zanzibar", 30, Some(&capped));
    assert_eq!(highlight_queries(), 20);
    assert_eq!(first.results.len(), 30);
    let anchors: Vec<SnippetAnchor> =
        first.snippets.as_ref().expect("sidecar").iter().map(|s| s.anchor).collect();
    assert_eq!(&anchors[..20], &[SnippetAnchor::Match; 20]);
    assert_eq!(&anchors[20..], &[SnippetAnchor::Start; 10]);
    let again = text(s.engine(), "zanzibar", 30, Some(&capped));
    assert_eq!(again.snippets, first.snippets);
}

// T-10: marker selection (§ 6.1). A body containing U+E000 uses the next
// free pair and still anchors on the match; a body containing all three
// pairs gets the record start, with no marker in the text.
#[test]
fn t10_markers_avoid_code_points_the_body_contains() {
    let _guard = lock();
    let one = format!("{} \u{E000} glyph", long_body(3000, "zanzibar", 4096));
    let all = format!(
        "{} \u{E000}\u{E001}\u{F8FE}\u{F8FF}\u{FDD0}\u{FDD1} glyphs",
        long_body(3000, "zanzibar", 4096)
    );
    let s = store(&[&one, &all]);
    let result = text(s.engine(), "zanzibar", 10, Some(&SnippetOptions::default()));

    let snippet = snippet_of(&result, &one);
    assert_eq!(snippet.anchor, SnippetAnchor::Match, "{snippet:?}");
    assert_eq!(snippet.spans, vec![(3000, 3008)]);

    let snippet = snippet_of(&result, &all);
    assert_eq!(snippet.anchor, SnippetAnchor::Start, "{snippet:?}");
    assert!(snippet.spans.is_empty());
    let (start, end) = snippet.windows[0];
    assert_eq!(start, 0);
    assert_eq!(snippet.text, format!("{} …", &all[..end as usize]));
}

// T-15: an edge body matched through `search_index_edges` anchors on the
// match, with spans from that table.
#[test]
fn t15_edge_hits_take_spans_from_the_edge_index() {
    let _guard = lock();
    let s = store(&["source node", "target node"]);
    let body = long_body(3000, "zanzibar", 4096);
    s.engine()
        .write(&[PreparedWrite::Edge {
            kind: "mentions".to_string(),
            from: "n0".to_string(),
            to: "n1".to_string(),
            source_id: SourceId::new("test:snippets").expect("source id"),
            logical_id: Some("edge-1".to_string()),
            body: Some(body.clone()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .expect("edge");
    for (path, result) in [
        ("text", text(s.engine(), "zanzibar", 10, Some(&SnippetOptions::default()))),
        ("hybrid", hybrid(s.engine(), "zanzibar", Some(&SnippetOptions::default()))),
    ] {
        let index = result.results.iter().position(|hit| hit.body == body).expect("edge hit");
        assert_eq!(result.results[index].branch, fathomdb_engine::SoftFallbackBranch::TextEdge);
        let snippet = &result.snippets.as_ref().expect("sidecar")[index];
        assert_eq!(snippet.anchor, SnippetAnchor::Match, "{path}: {snippet:?}");
        assert_eq!(snippet.spans, vec![(3000, 3008)], "{path}");
    }
}

/// Embeds any text containing `vectorhit`, and the query `zanzibar`, to one
/// direction and everything else to another, so the vector arm returns
/// exactly the `vectorhit` records.
#[derive(Debug)]
struct VectorHitEmbedder;

impl Embedder for VectorHitEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("snippet-vectorhit", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let near = text == "zanzibar" || text.contains("vectorhit");
        let mut vector = vec![0.0; 8];
        vector[usize::from(!near)] = 1.0;
        Ok(vector)
    }
}

// T-6: no lexical anchor. A hybrid hit found only by the vector arm, whose
// body matches none of the query terms, gets anchor Start and the prefix. A
// hit whose fused struct says `branch == Vector` but whose body does match
// gets anchor Match (§ 3.3: `branch` is not consulted).
#[test]
fn t6_vector_only_hits_get_the_record_start() {
    let _guard = lock();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("vector{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(VectorHitEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let vector_only = format!("vectorhit {}", long_body(3000, "elsewhere", 4096));
    let both = format!("vectorhit {}", long_body(2994, "zanzibar", 4096));
    opened.engine.write(&[node("v", &vector_only), node("b", &both)]).expect("write");
    opened.engine.drain(10_000).expect("drain");

    let result = hybrid(&opened.engine, "zanzibar", Some(&SnippetOptions::default()));
    let at = |body: &str| result.results.iter().position(|hit| hit.body == body);
    let snippets = result.snippets.as_ref().expect("sidecar");

    let v = at(&vector_only).unwrap_or_else(|| panic!("vector-only hit missing: {result:?}"));
    assert_eq!(result.results[v].branch, fathomdb_engine::SoftFallbackBranch::Vector);
    assert_eq!(snippets[v].anchor, SnippetAnchor::Start, "{:?}", snippets[v]);
    assert_eq!(snippets[v].windows[0].0, 0);
    assert!(snippets[v].spans.is_empty());

    let b = at(&both).unwrap_or_else(|| panic!("matching hit missing: {result:?}"));
    assert_eq!(result.results[b].branch, fathomdb_engine::SoftFallbackBranch::Vector);
    assert_eq!(snippets[b].anchor, SnippetAnchor::Match, "{:?}", snippets[b]);
    assert!(snippets[b].text.contains("zanzibar"));
}

fn without_snippets(result: &SearchResult) -> SearchResult {
    let mut result = result.clone();
    result.snippets = None;
    result
}

// T-7: off by default. With `snippet: None`, every field equals the plain
// entry point's result and `snippets` is None; with snippets on, every other
// field is unchanged.
#[test]
fn t7_snippets_off_leave_the_result_unchanged() {
    let _guard = lock();
    let long = long_body(3000, "zanzibar", 4096);
    let s = store(&["a short zanzibar note", &long]);
    let view = ReadView::default();
    let on = SnippetOptions::default();

    let plain = s.engine().search_text_only_view_with_limit("zanzibar", &view, 10).expect("plain");
    let off = text(s.engine(), "zanzibar", 10, None);
    assert_eq!(off, plain);
    assert!(off.snippets.is_none());
    assert_eq!(without_snippets(&text(s.engine(), "zanzibar", 10, Some(&on))), plain);

    let plain = s
        .engine()
        .search_reranked_view_with_limit("zanzibar", None, 0, false, 0.3, 0, false, &view, 10)
        .expect("plain hybrid");
    let off = hybrid(s.engine(), "zanzibar", None);
    assert_eq!(off, plain);
    assert!(off.snippets.is_none());
    assert_eq!(without_snippets(&hybrid(s.engine(), "zanzibar", Some(&on))), plain);

    let context = s
        .engine()
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    let plain = s
        .engine()
        .search_frozen("zanzibar", &context, 0, false, 0.3, 0, false, 10)
        .expect("plain frozen");
    let off = s
        .engine()
        .search_frozen_with_snippets("zanzibar", &context, 0, false, 0.3, 0, false, 10, None)
        .expect("frozen off");
    assert_eq!(off, plain);
    assert!(off.snippets.is_none());
}

// T-14: `head_bytes` on a single-line LoCoMo-style record with the match at
// byte 1500. The head window ends on a word boundary at or before
// head_bytes, and the match window follows.
#[test]
fn t14_head_window_keeps_the_record_header() {
    let _guard = lock();
    let header = "(unknown date) [8 May 2023] Caroline: ";
    let mut body = header.to_string();
    while body.len() < 1495 {
        body.push_str("we talked ");
    }
    let body = format!("{body}zanzibar{}", " again".repeat(300));
    let s = store(&[&body]);
    let headed = SnippetOptions { head_bytes: 48, ..SnippetOptions::default() };
    let result = text(s.engine(), "zanzibar", 10, Some(&headed));
    let snippet = snippet_of(&result, &body);
    assert_eq!(snippet.anchor, SnippetAnchor::Match, "{snippet:?}");
    assert_eq!(snippet.windows.len(), 2, "{snippet:?}");
    let (head_start, head_end) = snippet.windows[0];
    assert_eq!(head_start, 0);
    assert!(head_end <= 48);
    assert!(snippet.text.starts_with("(unknown date) [8 May 2023] Caroline:"), "{snippet:?}");
    assert!(body[head_end as usize..].starts_with(' '), "head ends on a word boundary");
    assert!(snippet.text.contains("zanzibar"));
    assert!(snippet.text.len() <= 300);
}

// T-16: frozen reads. Snippet options are presentation: `search_frozen` with
// snippets returns the same results (and explanation) as without, under the
// same frozen context, and the context's token is untouched.
#[test]
fn t16_frozen_results_do_not_depend_on_snippet_options() {
    let _guard = lock();
    let long = long_body(3000, "zanzibar", 4096);
    let s = store(&["a short zanzibar note", &long]);
    let context = s
        .engine()
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    let token = context.token.clone();
    let lit = SnippetOptions { highlights: true, head_bytes: 40, ..SnippetOptions::default() };
    for explain in [false, true] {
        let off = s
            .engine()
            .search_frozen_with_snippets("zanzibar", &context, 0, false, 0.3, 0, explain, 10, None)
            .expect("off");
        let on = s
            .engine()
            .search_frozen_with_snippets(
                "zanzibar",
                &context,
                0,
                false,
                0.3,
                0,
                explain,
                10,
                Some(&lit),
            )
            .expect("on");
        assert_eq!(on.results, off.results);
        assert_eq!(on.projection_cursor, off.projection_cursor);
        assert_eq!(on.degradations, off.degradations);
        assert_eq!(
            on.explanation.as_ref().map(|e| &e.per_hit),
            off.explanation.as_ref().map(|e| &e.per_hit)
        );
        let snippet = snippet_of(&on, &long);
        assert_eq!(snippet.anchor, SnippetAnchor::Match);
    }
    assert_eq!(context.token, token);
    // The token still authenticates after snippet searches.
    s.engine()
        .search_frozen_with_snippets("zanzibar", &context, 0, false, 0.3, 0, false, 10, Some(&lit))
        .expect("token still valid");
}

// T-9: reader-pool safety. While a second connection holds the write lock
// (BEGIN IMMEDIATE), snippet searches still succeed on the query_only reader
// pool, and the statement trace shows no write statement.
#[test]
fn t9_snippet_work_is_read_only_on_the_reader_pool() {
    let _guard = lock();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("readonly{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let long = long_body(3000, "zanzibar", 4096);
    opened.engine.write(&[node("n0", &long), node("n1", "a short zanzibar note")]).expect("write");
    opened.engine.drain(10_000).expect("drain");

    let blocker = rusqlite::Connection::open(&path).expect("second connection");
    blocker.execute_batch("BEGIN IMMEDIATE").expect("hold the write lock");
    take_slice71_search_statement_trace_for_test();
    let lit = SnippetOptions { highlights: true, ..SnippetOptions::default() };
    let text_result = text(&opened.engine, "zanzibar", 10, Some(&lit));
    let hybrid_result = hybrid(&opened.engine, "zanzibar", Some(&lit));
    let trace = take_slice71_search_statement_trace_for_test();
    blocker.execute_batch("ROLLBACK").expect("release");

    assert!(trace.iter().any(|event| event == "snippet_highlight"), "{trace:?}");
    assert!(!trace.iter().any(|event| event == "write_statement"), "{trace:?}");
    for result in [&text_result, &hybrid_result] {
        assert_eq!(snippet_of(result, &long).anchor, SnippetAnchor::Match);
    }

    // Control: the trace does see writes, so the check above is not vacuous.
    opened.engine.write(&[node("n2", "a later note")]).expect("write");
    let trace = take_slice71_search_statement_trace_for_test();
    assert!(trace.iter().any(|event| event == "write_statement"), "{trace:?}");
}
