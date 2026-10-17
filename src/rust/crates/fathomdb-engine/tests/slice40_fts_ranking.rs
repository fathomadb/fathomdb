//! Slice 40 (0.8.28) — FTS ranking defects.
//!
//! Defect A: RRF fusion credited one arm more than once for a body that
//! several rows share byte-for-byte, so a duplicate pair at ranks 13/14 beat
//! the true rank-1 hit. The contract pinned here: each arm contributes at most
//! once per body, at the positional rank of the body's first occurrence in that
//! arm; different arms still accumulate (agreement boosts a body).
//!
//! Defect B: the query compiler dropped 1–2 character tokens ("C", "D3",
//! "II", "1"), so the engine ranked as if they were absent.
//!
//! The fusion contract is unit-tested on the pure `fuse_three_arms` /
//! `fuse_rrf` functions and by property; the end-to-end tests run
//! `Engine::search_text_only*` on a real database (no mocking).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    fuse_rrf, fuse_three_arms, EmbedderChoice, Engine, IdSpace, InitialState, PreparedWrite,
    SearchHit, SoftFallbackBranch, SourceId, RRF_K, RRF_WEIGHT_GRAPH, RRF_WEIGHT_TEXT,
    RRF_WEIGHT_VECTOR,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use tempfile::TempDir;

fn hit(cursor: u64, body: &str, branch: SoftFallbackBranch) -> SearchHit {
    SearchHit {
        id: IdSpace::content(cursor.to_string()),
        write_cursor: cursor,
        kind: "doc".to_string(),
        body: body.to_string(),
        score: 0.0,
        branch,
        source_id: None,
        ce_score: None,
    }
}

fn text(cursor: u64, body: &str) -> SearchHit {
    hit(cursor, body, SoftFallbackBranch::Text)
}

fn rrf(weight: f64, rank1: usize) -> f64 {
    weight / (RRF_K + rank1 as f64)
}

fn bodies(hits: &[SearchHit]) -> Vec<&str> {
    hits.iter().map(|h| h.body.as_str()).collect()
}

// ---------------------------------------------------------------------------
// Defect A — unit tests on the pure fusion functions
// ---------------------------------------------------------------------------

/// AC40-01: the measured NFCorpus shape. A byte-identical pair at text ranks
/// 13 and 14 must not outrank the distinct rank-1 body.
#[test]
fn single_arm_duplicate_body_does_not_outrank_higher_ranked_distinct_body() {
    let mut arm: Vec<SearchHit> =
        (0..12).map(|i| text(100 + i, &format!("distinct {i:02}"))).collect();
    arm.push(text(200, "duplicate body"));
    arm.push(text(201, "duplicate body"));
    arm.extend((12..20).map(|i| text(100 + i, &format!("distinct {i:02}"))));

    let fused = fuse_three_arms(Vec::new(), arm, Vec::new());

    assert_eq!(
        fused[0].body,
        "distinct 00",
        "the rank-1 distinct body must stay first; fused order was {:?}",
        bodies(&fused)
    );
    let dup = fused.iter().position(|h| h.body == "duplicate body").expect("duplicate present");
    assert_eq!(dup, 12, "the duplicate keeps its first-occurrence place (13th)");
}

/// AC40-02: a duplicated body scores once, at its first occurrence's
/// positional rank; bodies below it keep their positional ranks.
#[test]
fn single_arm_duplicate_body_scores_once_at_first_occurrence_rank() {
    let arm = vec![text(1, "a"), text(2, "dup"), text(3, "b"), text(4, "dup")];

    let fused = fuse_three_arms(Vec::new(), arm, Vec::new());

    assert_eq!(bodies(&fused), vec!["a", "dup", "b"]);
    assert_eq!(fused[0].score, rrf(RRF_WEIGHT_TEXT, 1), "a = w_text/(K+1)");
    assert_eq!(
        fused[1].score,
        rrf(RRF_WEIGHT_TEXT, 2),
        "dup = w_text/(K+2) only; its second occurrence at rank 4 contributes nothing"
    );
    assert_eq!(fused[2].score, rrf(RRF_WEIGHT_TEXT, 3), "b keeps its positional rank 3");
}

/// AC40-03: the representative of a duplicated body is its first occurrence.
#[test]
fn duplicate_body_representative_is_first_occurrence() {
    let arm = vec![text(10, "x"), text(20, "dup"), text(21, "dup")];

    let fused = fuse_three_arms(Vec::new(), arm, Vec::new());

    let dup = fused.iter().find(|h| h.body == "dup").expect("dup present");
    assert_eq!(dup.write_cursor, 20, "first occurrence's cursor is kept");
    assert_eq!(dup.id, IdSpace::content("20"));
    assert_eq!(fused.iter().filter(|h| h.body == "dup").count(), 1, "dedup on body");
}

/// AC40-04: the once-per-arm rule holds on the vector arm.
#[test]
fn vector_arm_duplicate_body_scores_once() {
    let arm = vec![
        hit(1, "dup", SoftFallbackBranch::Vector),
        hit(2, "x", SoftFallbackBranch::Vector),
        hit(3, "dup", SoftFallbackBranch::Vector),
    ];

    let fused = fuse_three_arms(arm, Vec::new(), Vec::new());

    assert_eq!(bodies(&fused), vec!["dup", "x"]);
    assert_eq!(fused[0].score, rrf(RRF_WEIGHT_VECTOR, 1), "dup = w_vec/(K+1) only");
}

/// AC40-04: the once-per-arm rule holds on the graph arm.
#[test]
fn graph_arm_duplicate_body_scores_once() {
    let arm = vec![
        hit(1, "g", SoftFallbackBranch::GraphArm),
        hit(2, "dup", SoftFallbackBranch::GraphArm),
        hit(3, "dup", SoftFallbackBranch::GraphArm),
    ];

    let fused = fuse_three_arms(Vec::new(), Vec::new(), arm);

    assert_eq!(bodies(&fused), vec!["g", "dup"]);
    assert_eq!(fused[1].score, rrf(RRF_WEIGHT_GRAPH, 2), "dup = w_graph/(K+2) only");
}

/// AC40-05: the dedupe is per arm, not global. A body duplicated inside the
/// text arm and also present in the vector arm gets exactly one vector term
/// and one text term.
#[test]
fn duplicate_dedupe_is_per_arm_not_global() {
    let vector = vec![hit(1, "dup", SoftFallbackBranch::Vector)];
    let text_arm = vec![text(1, "dup"), text(2, "dup")];

    let fused = fuse_rrf(vector, text_arm);

    assert_eq!(fused.len(), 1);
    assert_eq!(
        fused[0].score,
        rrf(RRF_WEIGHT_VECTOR, 1) + rrf(RRF_WEIGHT_TEXT, 1),
        "one vector term + one text term"
    );
    assert_eq!(fused[0].branch, SoftFallbackBranch::Vector, "vector-first representative");
}

/// AC40-06: cross-arm agreement still accumulates when one arm also holds
/// the body twice.
#[test]
fn cross_arm_agreement_still_accumulates_with_in_arm_duplicates() {
    let vector = vec![hit(9, "agree", SoftFallbackBranch::Vector)];
    let text_arm = vec![text(1, "t0"), text(9, "agree"), text(10, "agree")];

    let fused = fuse_rrf(vector, text_arm);

    assert_eq!(fused[0].body, "agree");
    assert_eq!(
        fused[0].score,
        rrf(RRF_WEIGHT_VECTOR, 1) + rrf(RRF_WEIGHT_TEXT, 2),
        "agree = w_vec/(K+1) + w_text/(K+2); the rank-3 duplicate adds nothing"
    );
}

/// AC40-06: agreement across all three arms, without duplicates, sums one
/// term per arm (regression guard for the existing contract).
#[test]
fn three_arm_agreement_sums_one_term_per_arm() {
    let vector =
        vec![hit(5, "v0", SoftFallbackBranch::Vector), hit(1, "all", SoftFallbackBranch::Vector)];
    let text_arm = vec![text(6, "t0"), text(7, "t1"), text(1, "all")];
    let graph = vec![hit(1, "all", SoftFallbackBranch::GraphArm)];

    let fused = fuse_three_arms(vector, text_arm, graph);

    let all = fused.iter().find(|h| h.body == "all").expect("all present");
    assert_eq!(
        all.score,
        rrf(RRF_WEIGHT_VECTOR, 2) + rrf(RRF_WEIGHT_TEXT, 3) + rrf(RRF_WEIGHT_GRAPH, 1)
    );
    assert_eq!(fused[0].body, "all", "agreement across three arms wins");
}

// ---------------------------------------------------------------------------
// Defect A — properties
// ---------------------------------------------------------------------------

/// The intended fusion, written from the contract: per arm, skip a body the
/// arm already credited; credit `weight / (K + positional rank)`; accumulate
/// across arms in vector, text, graph order; sort by score descending, then
/// vector-first, then first-insertion order.
fn reference_fusion(
    vector: &[SearchHit],
    text_arm: &[SearchHit],
    graph: &[SearchHit],
) -> Vec<SearchHit> {
    struct Entry {
        hit: SearchHit,
        score: f64,
        in_vector: bool,
        order: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut by_body: HashMap<String, usize> = HashMap::new();
    for (arm, weight, in_vector) in [
        (vector, RRF_WEIGHT_VECTOR, true),
        (text_arm, RRF_WEIGHT_TEXT, false),
        (graph, RRF_WEIGHT_GRAPH, false),
    ] {
        let mut credited: HashSet<&str> = HashSet::new();
        for (rank0, h) in arm.iter().enumerate() {
            if !credited.insert(h.body.as_str()) {
                continue;
            }
            let contrib = weight / (RRF_K + (rank0 as f64 + 1.0));
            if let Some(&index) = by_body.get(&h.body) {
                entries[index].score += contrib;
            } else {
                by_body.insert(h.body.clone(), entries.len());
                let order = entries.len();
                entries.push(Entry { hit: h.clone(), score: contrib, in_vector, order });
            }
        }
    }
    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.in_vector.cmp(&a.in_vector))
            .then_with(|| a.order.cmp(&b.order))
    });
    entries
        .into_iter()
        .map(|mut e| {
            e.hit.score = e.score;
            e.hit
        })
        .collect()
}

fn arm_from(symbols: &[u8], branch: SoftFallbackBranch, cursor_base: u64) -> Vec<SearchHit> {
    symbols
        .iter()
        .enumerate()
        .map(|(i, s)| hit(cursor_base + i as u64, &format!("body-{s}"), branch))
        .collect()
}

fn distinct_in_order(symbols: Vec<u8>) -> Vec<u8> {
    let mut seen = HashSet::new();
    symbols.into_iter().filter(|s| seen.insert(*s)).collect()
}

proptest! {
    /// AC40-07: with only the text arm, fused order of distinct bodies equals
    /// the arm's first-occurrence order, and scores are strictly decreasing.
    #[test]
    fn prop_text_only_fusion_is_monotone_in_first_occurrence_rank(
        symbols in prop::collection::vec(0u8..8, 0..40),
    ) {
        let arm = arm_from(&symbols, SoftFallbackBranch::Text, 1000);
        let fused = fuse_three_arms(Vec::new(), arm.clone(), Vec::new());

        let mut first_rank: Vec<(String, usize)> = Vec::new();
        let mut seen = HashSet::new();
        for (rank0, h) in arm.iter().enumerate() {
            if seen.insert(h.body.clone()) {
                first_rank.push((h.body.clone(), rank0 + 1));
            }
        }
        let expected: Vec<&str> = first_rank.iter().map(|(b, _)| b.as_str()).collect();
        prop_assert_eq!(bodies(&fused), expected);
        for (h, (_, rank1)) in fused.iter().zip(&first_rank) {
            prop_assert_eq!(h.score, rrf(RRF_WEIGHT_TEXT, *rank1));
        }
        for pair in fused.windows(2) {
            prop_assert!(pair[0].score > pair[1].score, "strictly decreasing scores");
        }
    }

    /// AC40-08: for any three arms (duplicates allowed), fusion equals the
    /// once-per-arm reference: order, score bits, and representative identity.
    #[test]
    fn prop_fusion_matches_once_per_arm_reference(
        v in prop::collection::vec(0u8..10, 0..25),
        t in prop::collection::vec(0u8..10, 0..25),
        g in prop::collection::vec(0u8..10, 0..10),
    ) {
        let vector = arm_from(&v, SoftFallbackBranch::Vector, 1000);
        let text_arm = arm_from(&t, SoftFallbackBranch::Text, 2000);
        let graph = arm_from(&g, SoftFallbackBranch::GraphArm, 3000);

        let expected = reference_fusion(&vector, &text_arm, &graph);
        let fused = fuse_three_arms(vector, text_arm, graph);
        prop_assert_eq!(fused, expected);
    }

    /// AC40-09: when no arm holds a body twice, the fused output is unchanged
    /// by this slice: it equals the reference, which for such inputs is the
    /// pre-slice algorithm (regression guard for order, scores, tiebreak).
    #[test]
    fn prop_distinct_within_arm_inputs_are_unchanged(
        v in prop::collection::vec(0u8..12, 0..25),
        t in prop::collection::vec(0u8..12, 0..25),
        g in prop::collection::vec(0u8..12, 0..10),
    ) {
        let vector = arm_from(&distinct_in_order(v), SoftFallbackBranch::Vector, 1000);
        let text_arm = arm_from(&distinct_in_order(t), SoftFallbackBranch::Text, 2000);
        let graph = arm_from(&distinct_in_order(g), SoftFallbackBranch::GraphArm, 3000);

        let expected = reference_fusion(&vector, &text_arm, &graph);
        let fused = fuse_three_arms(vector, text_arm, graph);
        prop_assert_eq!(fused, expected);
    }

    /// AC40-10: the two-arm contract `fuse_rrf(v, t) == fuse_three_arms(v, t, [])`
    /// holds for any inputs, duplicates included.
    #[test]
    fn prop_two_arm_fusion_equals_three_arm_with_empty_graph(
        v in prop::collection::vec(0u8..10, 0..25),
        t in prop::collection::vec(0u8..10, 0..25),
    ) {
        let vector = arm_from(&v, SoftFallbackBranch::Vector, 1000);
        let text_arm = arm_from(&t, SoftFallbackBranch::Text, 2000);

        prop_assert_eq!(
            fuse_rrf(vector.clone(), text_arm.clone()),
            fuse_three_arms(vector, text_arm, Vec::new())
        );
    }
}

// ---------------------------------------------------------------------------
// End to end through `Engine::search_text_only*` (real database)
// ---------------------------------------------------------------------------

fn open(dir: &TempDir, name: &str) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_choice(
        dir.path().join(format!("{name}{SQLITE_SUFFIX}")),
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .expect("open")
}

fn node(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice40").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

const DUP_QUERY: &str = "s40dup";
const DUP_BODY: &str = "s40dup duplicate document body";

fn distinct_body(i: usize) -> String {
    // Same token count as `DUP_BODY`, so every row has the same bm25 and the
    // FTS rank is write order (bm25 ties break on write_cursor).
    format!("{DUP_QUERY} distinct document {i:02}")
}

/// Twenty distinct rows with a byte-identical pair written 13th and 14th: the
/// pair sits at FTS ranks 13/14, the measured NFCorpus shape.
fn write_duplicate_corpus(engine: &Engine) {
    let mut writes: Vec<PreparedWrite> =
        (0..12).map(|i| node(&format!("D{i:02}"), &distinct_body(i))).collect();
    writes.push(node("DUP-A", DUP_BODY));
    writes.push(node("DUP-B", DUP_BODY));
    writes.extend((12..20).map(|i| node(&format!("D{i:02}"), &distinct_body(i))));
    engine.write(&writes).expect("write duplicate corpus");
    engine.drain(10_000).expect("drain");
}

/// AC40-11: through the direct text-only API, the duplicate pair does not win
/// rank 1, and the top 10 are the first ten distinct rows in FTS rank order.
#[test]
fn text_only_duplicate_bodies_do_not_win_rank_one() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_dup_rank");
    write_duplicate_corpus(&opened.engine);

    let result = opened.engine.search_text_only(DUP_QUERY).expect("search_text_only");

    let got = bodies(&result.results);
    assert_eq!(
        result.results[0].body,
        distinct_body(0),
        "the rank-1 distinct row must stay first; got {got:?}"
    );
    let expected: Vec<String> = (0..10).map(distinct_body).collect();
    assert_eq!(got, expected, "top 10 are the first ten distinct rows in FTS rank order");
    opened.engine.close().expect("close");
}

/// AC40-12: the duplicate pair surfaces once, at its first-occurrence place,
/// and every result body is distinct.
#[test]
fn text_only_duplicate_body_surfaces_once_at_first_occurrence() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_dup_place");
    write_duplicate_corpus(&opened.engine);

    let result =
        opened.engine.search_text_only_with_limit(DUP_QUERY, 20).expect("search_text_only");

    let got = bodies(&result.results);
    let position = got.iter().position(|b| *b == DUP_BODY).expect("duplicate body present");
    assert_eq!(position, 12, "the pair sits at its first FTS rank (13th); got {got:?}");
    let unique: HashSet<&str> = got.iter().copied().collect();
    assert_eq!(unique.len(), got.len(), "result bodies are distinct");
    opened.engine.close().expect("close");
}

/// AC40-13: with more distinct matching bodies than the limit, the caller gets
/// exactly `limit` results, all distinct, even with a duplicate pair present.
#[test]
fn text_only_duplicate_bodies_still_fill_the_limit() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_dup_limit");
    write_duplicate_corpus(&opened.engine);

    let result = opened.engine.search_text_only(DUP_QUERY).expect("search_text_only");

    assert_eq!(result.results.len(), 10, "default limit is filled");
    let unique: HashSet<&str> = result.results.iter().map(|h| h.body.as_str()).collect();
    assert_eq!(unique.len(), 10, "all ten results are distinct bodies");
    opened.engine.close().expect("close");
}

// ---------------------------------------------------------------------------
// Defect B — end to end
// ---------------------------------------------------------------------------

/// AC40-14: a 1-character query token reaches FTS5 and ranks the row that
/// carries it. Both rows match "vitamin"; only the second carries "d". Before
/// the fix the query compiled to `"vitamin"` alone and the tie broke on write
/// order, putting the "c" row first.
#[test]
fn text_only_single_char_token_ranks_matching_row_first() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_short_token");
    opened
        .engine
        .write(&[
            node("VC", "vitamin c supplement trial"),
            node("VD", "vitamin d supplement trial"),
        ])
        .expect("write");
    opened.engine.drain(10_000).expect("drain");

    let result = opened.engine.search_text_only("Vitamin D").expect("search_text_only");

    assert_eq!(
        result.results.first().map(|h| h.body.as_str()),
        Some("vitamin d supplement trial"),
        "the row carrying the short token ranks first; got {:?}",
        bodies(&result.results)
    );
    opened.engine.close().expect("close");
}

/// AC40-15: a 2-character alphanumeric token ("d3") and a 1-digit token ("1")
/// each lift the row that carries them.
#[test]
fn text_only_two_char_and_digit_tokens_rank_matching_rows_first() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_short_token_2");
    opened
        .engine
        .write(&[
            node("D2", "vitamin d2 dosing study"),
            node("D3", "vitamin d3 dosing study"),
            node("IGF2", "serum igf 2 levels measured"),
            node("IGF1", "serum igf 1 levels measured"),
        ])
        .expect("write");
    opened.engine.drain(10_000).expect("drain");

    let d3 = opened.engine.search_text_only("vitamin D3 dosing").expect("search d3");
    assert_eq!(
        d3.results.first().map(|h| h.body.as_str()),
        Some("vitamin d3 dosing study"),
        "\"d3\" must reach FTS5; got {:?}",
        bodies(&d3.results)
    );
    let igf1 = opened.engine.search_text_only("IGF-1 levels").expect("search igf-1");
    assert_eq!(
        igf1.results.first().map(|h| h.body.as_str()),
        Some("serum igf 1 levels measured"),
        "\"1\" must reach FTS5; got {:?}",
        bodies(&igf1.results)
    );
    opened.engine.close().expect("close");
}

/// AC40-25: a possessive query compiles to "cow" OR "s" OR "milk". The
/// fragment "s" matches every row with a possessive, but the row carrying
/// the topical terms still ranks first, and the query does not error.
#[test]
fn text_only_possessive_query_ranks_topical_row_first() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_possessive");
    opened
        .engine
        .write(&[
            node("GOAT", "goat's cheese intake survey"),
            node("COW", "cow milk intake survey"),
            node("CHILD", "children's bread intake survey"),
        ])
        .expect("write");
    opened.engine.drain(10_000).expect("drain");

    let result = opened.engine.search_text_only("Cow's milk").expect("search_text_only");

    assert_eq!(
        result.results.first().map(|h| h.body.as_str()),
        Some("cow milk intake survey"),
        "the cow/milk row ranks first; got {:?}",
        bodies(&result.results)
    );
    opened.engine.close().expect("close");
}

/// AC40-16: short tokens mixed with FTS5 control syntax never raise an error
/// (AC-038 injection safety with short tokens).
#[test]
fn text_only_short_tokens_with_fts5_control_syntax_never_error() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, "slice40_short_injection");
    opened.engine.write(&[node("X", "vitamin d3 c 1 ii study")]).expect("write");
    opened.engine.drain(10_000).expect("drain");

    let queries = [
        "NEAR(c d, 5)",
        "\"d3\"^2 OR c*",
        "igf-1:\"x\"",
        "c: d:",
        "(ii) AND (1)",
        "^c -d +1",
        "\"\" \" c",
        "*",
    ];
    for query in queries {
        let outcome = opened.engine.search_text_only(query);
        assert!(outcome.is_ok(), "query {query:?} must not error: {outcome:?}");
    }
    opened.engine.close().expect("close");
}
