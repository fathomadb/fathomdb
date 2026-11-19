use crate::slice30_observer::{RankProbeRoute, Slice30ObservationGuard};
use crate::{
    Engine, IdSpaceKind, InitialState, PreparedWrite, ReadView, SearchResult, SnippetOptions,
    SourceId,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use serde_json::{json, Value};
use std::sync::Arc;

const TWINS: &str = "\u{001e}\u{0011}café pear";
const NEWCOMER: &str = "nearby vector";
const AS_OF: i64 = 1_800_000_000;

struct RankingProvider;

impl Embedder for RankingProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("ranking-probe", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        vector[0] = 1.0;
        if text == TWINS {
            vector[1] = 1.0;
        }
        Ok(vector)
    }

    fn truncation_offset(&self, text: &str) -> Result<Option<usize>, EmbedderError> {
        Ok((text == TWINS).then_some(7))
    }
}

fn node(source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        logical_id: None,
        source_id: SourceId::new(source).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

struct RankedRead {
    result: SearchResult,
    authority: Value,
    candidates: Vec<(u64, u64)>,
    ranks: Value,
}

fn read(
    engine: &Engine,
    observer: &Slice30ObservationGuard,
    operation: u64,
    route: RankProbeRoute,
) -> RankedRead {
    let snippets = SnippetOptions { highlights: true, ..SnippetOptions::default() };
    let result = observer
        .with_rank_probe(operation, route, || {
            engine.search_reranked_view_with_snippets(
                "cafe",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView { valid_as_of: Some(AS_OF), ..ReadView::default() },
                2,
                Some(&snippets),
            )
        })
        .unwrap();
    let batch = observer.take();
    assert_eq!(batch.dropped_records, 0);
    assert!(batch.issues.is_empty());
    assert!(batch.records.iter().all(|record| record.operation_id == operation));
    let facts = |phase| {
        let mut rows = batch.records.iter().filter(|record| record.phase == phase);
        let row = rows.next().unwrap_or_else(|| panic!("missing {phase}: {:?}", batch.records));
        assert!(rows.next().is_none(), "repeated {phase}");
        assert_eq!(row.status, "ok");
        row.facts.clone()
    };
    assert_eq!(
        facts("s50_ranking_route"),
        match route {
            RankProbeRoute::Reference => "hybrid_full_sort_forced",
            RankProbeRoute::Optimized => "winner_hydration",
        }
    );
    let authority = facts("observer_snapshot_capture");
    assert_eq!(authority["projection_cursor"], result.projection_cursor);
    assert_eq!(authority["validity_binding_epoch_s"], AS_OF);
    RankedRead {
        result,
        authority,
        candidates: serde_json::from_value(facts("s50_vector_candidates")).unwrap(),
        ranks: facts("s50_rank_inputs"),
    }
}

fn assert_same_state(reference: &RankedRead, optimized: &RankedRead) {
    assert_eq!(reference.authority, optimized.authority);
    assert_eq!(reference.candidates, optimized.candidates);
    assert_eq!(reference.ranks, optimized.ranks);
    assert_eq!(reference.result, optimized.result);
    for (a, b) in reference.result.results.iter().zip(&optimized.result.results) {
        assert_eq!(a.score.to_bits(), b.score.to_bits());
    }
}

fn assert_fixture(read: &RankedRead, twins: &[u64], newcomer: Option<u64>) {
    assert_eq!(read.result.results.len(), 2);
    assert!(read.result.degradations.is_empty());
    assert_eq!(read.result.soft_fallback, None);
    assert!(read.result.explanation.is_none());
    assert_eq!(read.candidates.len(), 2 + usize::from(newcomer.is_some()));
    let tie_start = usize::from(newcomer.is_some());
    if let Some(cursor) = newcomer {
        assert_eq!(read.candidates[0], (cursor, 0.0_f64.to_bits()));
    }
    let tied = &read.candidates[tie_start..];
    assert!(tied.iter().all(|&(_, distance)| distance == 1.0_f64.to_bits()));
    let mut membership: Vec<_> = tied.iter().map(|&(cursor, _)| cursor).collect();
    membership.sort_unstable();
    assert_eq!(membership, twins);

    let vector = read.ranks["vector"].as_array().unwrap();
    let text = read.ranks["text"].as_array().unwrap();
    assert_eq!(vector.len(), read.candidates.len());
    assert_eq!(text.len(), twins.len());
    for (index, &(cursor, distance)) in read.candidates.iter().enumerate() {
        assert_eq!(vector[index], json!([index + 1, ["node", cursor], distance]));
    }
    assert_eq!(text[0][2], text[1][2], "identical indexed bodies are lexical ties");
    let result = &read.result.results;
    let snippets = read.result.snippets.as_ref().unwrap();
    assert_eq!(snippets.len(), 2);
    assert_eq!(result[0].id, result[1].id);
    assert_eq!(result[0].id.space, IdSpaceKind::Content);
    for (index, (&cursor, source)) in twins.iter().zip(["rank-probe:a", "rank-probe:b"]).enumerate()
    {
        let text_rank = index + 1;
        let vector_rank = read.candidates.iter().position(|&(id, _)| id == cursor).unwrap() + 1;
        assert_eq!(text[index], json!([text_rank, ["node", cursor], text[0][2]]));
        assert_eq!(result[index].write_cursor, cursor);
        assert_eq!(result[index].source_id.as_deref(), Some(source));
        assert_eq!(result[index].body, TWINS);
        let expected = 1.0_f64 / (30 + vector_rank) as f64 + 3.0_f64 / (30 + text_rank) as f64;
        assert_eq!(result[index].score.to_bits(), expected.to_bits());
        assert_eq!(snippets[index].scored_bytes, Some(7));
        assert_eq!(snippets[index].text, TWINS);
        assert_eq!(snippets[index].windows, [(0, 12)]);
        assert_eq!(snippets[index].spans, [(2, 7)]);
        assert_eq!(snippets[index].highlights, [(2, 7)]);
    }
}

#[test]
fn projection_step_moves_vector_ranks_with_exact_fixed_state_controls() {
    let directory = tempfile::tempdir().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        directory.path().join("ranking.sqlite"),
        Arc::new(RankingProvider),
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let twins = engine
        .write(&[node("rank-probe:a", TWINS), node("rank-probe:b", TWINS)])
        .unwrap()
        .row_cursors;
    engine.drain(10_000).unwrap();
    engine.set_projection_scheduler_frozen_for_test(true);
    let newcomer = engine.write(&[node("rank-probe:c", NEWCOMER)]).unwrap().row_cursors[0];
    let observer = engine.begin_slice30_observation(64).unwrap();

    let before = read(engine, &observer, 1, RankProbeRoute::Reference);
    let before_optimized = read(engine, &observer, 2, RankProbeRoute::Optimized);
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(10_000).unwrap();
    let after = read(engine, &observer, 3, RankProbeRoute::Reference);
    let after_optimized = read(engine, &observer, 4, RankProbeRoute::Optimized);

    assert_same_state(&before, &before_optimized);
    assert_same_state(&after, &after_optimized);
    assert_fixture(&before, &twins, None);
    assert_fixture(&after, &twins, Some(newcomer));
    assert_eq!(before.result.projection_cursor, twins[1]);
    assert_eq!(after.result.projection_cursor, newcomer);
    assert_ne!(before.authority["visibility_nonce"], after.authority["visibility_nonce"]);
    assert_eq!(before.authority["dependency_generation"], after.authority["dependency_generation"]);
    assert_eq!(before.ranks["text"], after.ranks["text"]);
    assert_eq!(before.result.snippets, after.result.snippets);
    let mut score_decreased = false;
    for (a, b) in before.result.results.iter().zip(&after.result.results) {
        assert_eq!(a.id, b.id);
        assert!(a.score >= b.score, "inserting a nearer vector cannot improve either rank");
        score_decreased |= a.score > b.score;
    }
    assert!(score_decreased, "inserting a nearer vector must move at least one rank");

    for (label, read) in [("pending", &before), ("committed", &after)] {
        eprintln!(
            "{label}: authority={}, candidates={:?}, ranks={}, score_bits={:?}",
            read.authority,
            read.candidates,
            read.ranks,
            read.result.results.iter().map(|hit| hit.score.to_bits()).collect::<Vec<_>>()
        );
    }
    engine.close().unwrap();
}

#[test]
fn node_only_winner_checks_eligibility_of_matches_without_a_global_scan() {
    let make = |id: &str, from, until, state| {
        let mut write = node("eligibility:fixture", &format!("cafe {id}"));
        if let PreparedWrite::Node { logical_id, valid_from, valid_until, state: initial, .. } =
            &mut write
        {
            *logical_id = Some(id.into());
            *valid_from = from;
            *valid_until = until;
            *initial = state;
        }
        write
    };
    for shape in ["all-eligible", "early-ineligible", "mixed"] {
        let directory = tempfile::tempdir().unwrap();
        let opened = Engine::open_with_embedder_for_test(
            directory.path().join("eligibility.sqlite"),
            Arc::new(RankingProvider),
        )
        .unwrap();
        let engine = &opened.engine;
        engine.configure_vector_kind_for_test("doc").unwrap();
        let mut writes = Vec::new();
        if shape != "all-eligible" {
            writes.push(make("expired", None, Some(AS_OF), InitialState::Active));
        }
        writes.extend([
            make("starts", Some(AS_OF), None, InitialState::Active),
            make("unbounded", None, None, InitialState::Active),
        ]);
        if shape == "mixed" {
            writes.extend([
                make("future", Some(AS_OF + 1), None, InitialState::Active),
                make("pending", None, None, InitialState::Pending),
            ]);
        }
        engine.write(&writes).unwrap();
        let replacement = if shape == "mixed" {
            let mut replacement = make("unbounded", None, None, InitialState::Active);
            if let PreparedWrite::Node { body, .. } = &mut replacement {
                *body = "cafe replacement".into();
            }
            Some(engine.write(&[replacement]).unwrap().row_cursors[0])
        } else {
            None
        };
        engine.drain(10_000).unwrap();
        let observer = engine.begin_slice30_observation(128).unwrap();
        let query = || {
            engine.search_reranked_view_with_snippets(
                "cafe",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView { valid_as_of: Some(AS_OF), ..ReadView::default() },
                10,
                Some(&SnippetOptions { highlights: true, ..SnippetOptions::default() }),
            )
        };

        let optimized = observer.with_rank_probe(1, RankProbeRoute::Optimized, query).unwrap();
        let work = observer.take();
        assert_eq!(work.dropped_records, 0);
        assert!(work.issues.is_empty());
        let scan_count = work
            .records
            .iter()
            .filter(|record| {
                record.phase == "all_nodes_directly_eligible"
                    && record.facts["sql_executed"] == true
            })
            .count();
        assert_eq!(scan_count, 0, "{shape} performed a whole-store eligibility query");
        assert!(work.records.iter().any(|record| {
            record.phase == "s50_ranking_route" && record.facts == "winner_hydration"
        }));
        let controlled = observer.with_rank_probe(2, RankProbeRoute::Reference, query).unwrap();
        let reference_work = observer.take();
        assert_eq!(reference_work.dropped_records, 0);
        assert!(reference_work.issues.is_empty());
        assert_eq!(optimized, controlled);
        let snapshot = |records: &[crate::Slice30ObservationRecord]| {
            records
                .iter()
                .find(|record| record.phase == "observer_snapshot_capture")
                .unwrap()
                .facts
                .clone()
        };
        assert_eq!(snapshot(&work.records), snapshot(&reference_work.records));
        for (hit, reference) in optimized.results.iter().zip(&controlled.results) {
            assert_eq!(hit.score.to_bits(), reference.score.to_bits());
            assert_eq!(hit.source_id.as_deref(), Some("eligibility:fixture"));
        }
        let mut ids = optimized.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>();
        ids.sort();
        assert_eq!(ids, ["l:starts", "l:unbounded"]);
        if let Some(cursor) = replacement {
            let hit =
                optimized.results.iter().find(|hit| hit.id.to_prefixed() == "l:unbounded").unwrap();
            assert_eq!(hit.write_cursor, cursor);
            assert_eq!(hit.body, "cafe replacement");
        }
    }
}
