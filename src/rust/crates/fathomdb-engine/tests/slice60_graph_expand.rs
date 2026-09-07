//! Slice 60 RED oracle for bounded, constrained, one-transaction graph expansion.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_graph_expand_after_pin_hook_for_test, arm_graph_expand_before_pin_hook_for_test,
    graph_expansion_degradation_codes_for_test, Engine, EngineError, GraphExpandRequestV1,
    GraphExpansionDegradationCodeV1, GraphExpansionErrorReasonV1, GraphProjectionOriginV1,
    GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1, GraphSeedSourceV1,
    GraphSeedV1, IdSpace, InitialState, LifecycleState, PreparedWrite, ReadContextV1, ReadView,
    SearchFilter, SourceId, TraversalDirection, TOP_K_BIT_CANDIDATES,
};
use fathomdb_schema::{SCHEMA_VERSION, SQLITE_SUFFIX};
use tempfile::TempDir;

const INSTANT: i64 = 1_700_000_000;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn node_with_window(
    id: Option<&str>,
    kind: &str,
    body: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: id.map(str::to_owned),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new("test:slice60").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
    }
}

fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    node_with_window(Some(id), kind, body, None, None)
}

fn pending_node(id: &str, kind: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: kind.into(),
        body: id.into(),
        source_id: SourceId::new("test:slice60").unwrap(),
        state: InitialState::Pending,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge_with_time(
    id: &str,
    kind: &str,
    from: &str,
    to: &str,
    t_valid: Option<i64>,
    t_invalid: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("test:slice60").unwrap(),
        logical_id: Some(id.into()),
        body: None,
        t_valid,
        t_invalid,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn edge(id: &str, kind: &str, from: &str, to: &str) -> PreparedWrite {
    edge_with_time(id, kind, from, to, None, None)
}

fn context(view: ReadView, eligibility: SearchFilter) -> ReadContextV1 {
    ReadContextV1::new(view, eligibility).unwrap()
}

fn current(view: ReadView, eligibility: SearchFilter) -> GraphReadContextV1 {
    GraphReadContextV1::Current { schema_version: 1, context: context(view, eligibility) }
}

fn explicit_request(ids: &[&str], direction: TraversalDirection) -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: ids.iter().map(|id| IdSpace::logical(*id)).collect(),
        },
        direction,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: current(
            ReadView { valid_as_of: Some(INSTANT), ..ReadView::default() },
            SearchFilter::default(),
        ),
        max_depth: 3,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
    }
}

fn graph_error(
    result: Result<fathomdb_engine::GraphExpandResultV1, EngineError>,
) -> (GraphExpansionErrorReasonV1, String) {
    match result.unwrap_err() {
        EngineError::GraphExpansion(error) => (error.reason, error.field_path),
        other => panic!("expected graph-expansion error, got {other:?}"),
    }
}

fn target_ids(result: &fathomdb_engine::GraphExpandResultV1) -> Vec<&str> {
    result.targets.iter().map(|target| target.logical_id.as_str()).collect()
}

#[test]
fn explicit_seeds_preserve_order_depth_zero_reads_no_edges_and_excludes_all_seeds() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "depth-zero")).unwrap();
    opened
        .engine
        .write(&[
            node("seed-b", "seed", "B"),
            node("seed-a", "seed", "A"),
            edge("ab", "link", "seed-a", "seed-b"),
        ])
        .unwrap();

    let mut request = explicit_request(&["seed-b", "seed-a"], TraversalDirection::Both);
    request.max_depth = 0;
    request.max_work_units = 1;
    let result = opened.engine.graph_expand(&request).unwrap();

    assert_eq!(
        result.seeds.iter().map(|seed| (&*seed.logical_id, seed.seed_ordinal)).collect::<Vec<_>>(),
        vec![("seed-b", 0), ("seed-a", 1)]
    );
    assert!(result.seeds.iter().all(|seed| seed.query_score.is_none()));
    assert!(result.targets.is_empty());
    assert_eq!(result.work_units, 0);
    assert!(result.complete);
}

#[test]
fn direction_edge_kinds_and_return_only_target_kinds_are_exact() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "constraints")).unwrap();
    opened
        .engine
        .write(&[
            node("a", "x", "A"),
            node("b", "y", "B"),
            node("c", "x", "C"),
            node("d", "x", "D"),
            edge("ab", "supports", "a", "b"),
            edge("bc", "supports", "b", "c"),
            edge("da", "refutes", "d", "a"),
        ])
        .unwrap();

    let mut outgoing = explicit_request(&["a"], TraversalDirection::Outgoing);
    outgoing.edge_kinds = vec!["supports".into()];
    outgoing.target_kinds = vec!["x".into()];
    outgoing.max_depth = 2;
    assert_eq!(target_ids(&opened.engine.graph_expand(&outgoing).unwrap()), ["c"]);

    outgoing.edge_kinds = vec!["absent-open-kind".into()];
    assert!(opened.engine.graph_expand(&outgoing).unwrap().targets.is_empty());

    let mut incoming = explicit_request(&["a"], TraversalDirection::Incoming);
    incoming.edge_kinds = vec!["refutes".into()];
    incoming.max_depth = 1;
    let incoming_result = opened.engine.graph_expand(&incoming).unwrap();
    assert_eq!(target_ids(&incoming_result), ["d"]);
    assert_eq!(incoming_result.targets[0].origin.terminal_direction, TraversalDirection::Incoming);

    let both = explicit_request(&["a"], TraversalDirection::Both);
    let both_result = opened.engine.graph_expand(&both).unwrap();
    assert!(target_ids(&both_result).contains(&"b"));
    assert!(target_ids(&both_result).contains(&"d"));
}

#[test]
fn cycles_self_loops_parallel_edges_and_multiple_origins_are_deterministic() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "origins")).unwrap();
    opened
        .engine
        .write(&[
            node("a", "seed", "A"),
            node("b", "seed", "B"),
            node("c", "fact", "C"),
            edge("self", "loop", "a", "a"),
            edge("ab", "cycle", "a", "b"),
            edge("ba", "cycle", "b", "a"),
            edge("z-path", "zeta", "a", "c"),
            edge("a-path", "alpha", "a", "c"),
            edge("b-path", "alpha", "b", "c"),
        ])
        .unwrap();

    let result = opened
        .engine
        .graph_expand(&explicit_request(&["b", "a"], TraversalDirection::Both))
        .unwrap();
    assert_eq!(target_ids(&result), ["c"]);
    assert_eq!(result.work_units, 22, "parallel, loop, and per-seed rows all count");
    let origin = &result.targets[0].origin;
    assert_eq!(origin.hop_count, 1);
    assert_eq!(origin.seed_ordinal, 0);
    assert_eq!(origin.seed_logical_id, "b");
    assert_eq!(origin.terminal_edge_kind, "alpha");
    assert_eq!(origin.target_logical_id, "c");
    assert!(!target_ids(&result).contains(&"a"));
    assert!(!target_ids(&result).contains(&"b"));
}

#[test]
fn edge_only_insertion_permutations_have_identical_complete_response_bytes() {
    let dir = TempDir::new().unwrap();
    let mut encoded = Vec::new();
    let edge_orders = [
        vec![("z", "zeta"), ("a", "alpha"), ("m", "middle")],
        vec![("m", "middle"), ("z", "zeta"), ("a", "alpha")],
        vec![("a", "alpha"), ("m", "middle"), ("z", "zeta")],
    ];
    for (index, order) in edge_orders.iter().enumerate() {
        let opened = Engine::open(path(&dir, &format!("permutation-{index}"))).unwrap();
        opened
            .engine
            .write(&[node("root", "seed", "root"), node("target", "fact", "body")])
            .unwrap();
        let edges =
            order.iter().map(|(id, kind)| edge(id, kind, "root", "target")).collect::<Vec<_>>();
        opened.engine.write(&edges).unwrap();
        let result = opened
            .engine
            .graph_expand(&explicit_request(&["root"], TraversalDirection::Outgoing))
            .unwrap();
        encoded.push(fathomdb_engine::encode_graph_expand_result_v1(&result).unwrap());
    }
    assert!(encoded.windows(2).all(|pair| pair[0] == pair[1]));
}

fn star(
    direction: TraversalDirection,
    count: usize,
    budget: u64,
) -> Result<fathomdb_engine::GraphExpandResultV1, EngineError> {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "star")).unwrap();
    let mut writes = vec![node("root", "seed", "root")];
    for index in 0..count {
        let id = format!("n-{index:05}");
        writes.push(node(&id, "fact", &id));
        let (from, to) = match direction {
            TraversalDirection::Incoming => (id.as_str(), "root"),
            TraversalDirection::Outgoing | TraversalDirection::Both => ("root", id.as_str()),
        };
        writes.push(edge(&format!("e-{index:05}"), "link", from, to));
    }
    opened.engine.write(&writes).unwrap();
    let mut request = explicit_request(&["root"], direction);
    request.max_depth = 1;
    request.max_work_units = budget;
    opened.engine.graph_expand(&request)
}

#[test]
fn work_budget_is_exact_exhaustive_and_all_or_nothing_in_every_direction() {
    for direction in
        [TraversalDirection::Incoming, TraversalDirection::Outgoing, TraversalDirection::Both]
    {
        let exact = star(direction, 3, 3).unwrap();
        assert_eq!(exact.work_units, 3);
        assert_eq!(exact.targets.len(), 3);
        assert!(exact.complete);

        let error = graph_error(star(direction, 4, 3));
        assert_eq!(error.0, GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded);
        assert_eq!(error.1, "/maxWorkUnits");
    }
}

#[test]
fn result_limit_selects_top_n_only_after_the_walk_is_complete() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "top-n")).unwrap();
    opened
        .engine
        .write(&[
            node("root", "seed", "root"),
            node("z-hop1", "fact", "z"),
            node("a-hop2", "fact", "a"),
            node("middle", "other", "middle"),
            edge("z", "zeta", "root", "z-hop1"),
            edge("m", "alpha", "root", "middle"),
            edge("a", "alpha", "middle", "a-hop2"),
        ])
        .unwrap();
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.result_limit = 1;
    request.max_depth = 2;
    let result = opened.engine.graph_expand(&request).unwrap();
    assert_eq!(target_ids(&result), ["z-hop1"], "hop count outranks lexical target ID");
    assert_eq!(result.work_units, 3, "traversal did not stop when one result was found");
    assert!(result.complete);
}

#[test]
fn validation_bounds_duplicates_and_nonlogical_seeds_are_typed() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "validation")).unwrap();
    opened.engine.write(&[node("root", "seed", "root")]).unwrap();

    let mut cases = Vec::new();
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.max_depth = 4;
    cases.push((request, GraphExpansionErrorReasonV1::GraphDepthInvalid, "/maxDepth"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.result_limit = 0;
    cases.push((request, GraphExpansionErrorReasonV1::GraphResultLimitInvalid, "/resultLimit"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.result_limit = 51;
    cases.push((request, GraphExpansionErrorReasonV1::GraphResultLimitInvalid, "/resultLimit"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.max_work_units = 0;
    cases.push((request, GraphExpansionErrorReasonV1::GraphWorkLimitInvalid, "/maxWorkUnits"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.max_work_units = 10_001;
    cases.push((request, GraphExpansionErrorReasonV1::GraphWorkLimitInvalid, "/maxWorkUnits"));
    let mut request = explicit_request(&["root", "root"], TraversalDirection::Outgoing);
    cases.push((request, GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/logicalIds/1"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.seed =
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::content("root")] };
    cases.push((
        request,
        GraphExpansionErrorReasonV1::GraphSeedInvalid,
        "/seed/logicalIds/0/space",
    ));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.edge_kinds = vec!["link".into(), "link".into()];
    cases.push((request, GraphExpansionErrorReasonV1::GraphEdgeKindsInvalid, "/edgeKinds/1"));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.target_kinds = vec![" ".into()];
    cases.push((request, GraphExpansionErrorReasonV1::GraphTargetKindsInvalid, "/targetKinds/0"));

    for (request, reason, path) in cases {
        assert_eq!(graph_error(opened.engine.graph_expand(&request)), (reason, path.into()));
    }
}

#[test]
fn explicit_resolution_is_all_or_nothing_and_authentication_precedes_seed_validation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "all-or-nothing")).unwrap();
    opened.engine.write(&[node("visible", "seed", "visible")]).unwrap();

    let request = explicit_request(&["visible", "missing"], TraversalDirection::Outgoing);
    assert_eq!(
        graph_error(opened.engine.graph_expand(&request)),
        (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/1".into())
    );

    let frozen_context = context(ReadView::default(), SearchFilter::default());
    let mut frozen = opened.engine.freeze_read_context(&frozen_context).unwrap();
    frozen.token.push('0');
    let mut malformed = explicit_request(&["visible", "visible"], TraversalDirection::Outgoing);
    malformed.max_depth = 4;
    malformed.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen };
    assert!(matches!(opened.engine.graph_expand(&malformed), Err(EngineError::FrozenRead(_))));
}

#[test]
fn lifecycle_supersession_and_indexed_eligibility_apply_before_admission() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "eligibility")).unwrap();
    let mut writes = vec![
        node("root", "eligible", "root"),
        pending_node("pending-seed", "eligible"),
        node("inactive-target", "eligible", "inactive"),
    ];
    for index in 0..64 {
        let id = format!("excluded-{index:02}");
        writes.push(node(&id, "excluded", &id));
        writes.push(edge(&format!("excluded-edge-{index:02}"), "link", "root", &id));
    }
    writes.push(node("eligible-65", "eligible", "eligible"));
    writes.push(edge("eligible-edge-65", "link", "root", "eligible-65"));
    writes.push(edge("inactive-edge", "link", "root", "inactive-target"));
    opened.engine.write(&writes).unwrap();
    opened.engine.transition("inactive-target", LifecycleState::Deleted, None).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("eligible".into());
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.max_depth = 1;
    request.context =
        current(ReadView { valid_as_of: Some(INSTANT), ..ReadView::default() }, eligibility);
    let result = opened.engine.graph_expand(&request).unwrap();
    assert_eq!(target_ids(&result), ["eligible-65"]);
    assert_eq!(result.work_units, 66, "filtered and inactive edge rows still consume work");

    let pending = explicit_request(&["root", "pending-seed"], TraversalDirection::Outgoing);
    assert_eq!(
        graph_error(opened.engine.graph_expand(&pending)),
        (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/1".into())
    );

    let relaxed =
        ReadView { include_inactive: true, valid_as_of: Some(INSTANT), ..ReadView::default() };
    let mut unsupported = explicit_request(&["root"], TraversalDirection::Outgoing);
    unsupported.context = current(relaxed, SearchFilter::default());
    assert_eq!(
        graph_error(opened.engine.graph_expand(&unsupported)),
        (GraphExpansionErrorReasonV1::GraphContextInvalid, "/context".into())
    );
}

#[test]
fn superseded_edges_consume_work_but_cannot_supply_a_target() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "edge-supersession")).unwrap();
    opened
        .engine
        .write(&[
            node("root", "seed", "root"),
            node("old", "fact", "old"),
            node("new", "fact", "new"),
            edge("route", "link", "root", "old"),
        ])
        .unwrap();
    opened.engine.write(&[edge("route", "link", "root", "new")]).unwrap();
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.max_depth = 1;
    let result = opened.engine.graph_expand(&request).unwrap();
    assert_eq!(target_ids(&result), ["new"]);
    assert_eq!(result.work_units, 2, "the raw superseded edge row is charged before liveness");

    let expected = fathomdb_engine::encode_graph_expand_result_v1(&result).unwrap();
    opened.engine.close().unwrap();
    let reopened = Engine::open(path(&dir, "edge-supersession")).unwrap();
    let actual = reopened.engine.graph_expand(&request).unwrap();
    assert_eq!(fathomdb_engine::encode_graph_expand_result_v1(&actual).unwrap(), expected);
}

#[test]
fn node_relaxation_never_relaxes_shipped_edge_recency_and_t_valid_never_gates() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "temporal")).unwrap();
    opened
        .engine
        .write(&[
            node("root", "seed", "root"),
            node_with_window(Some("future-node"), "fact", "future", Some(INSTANT + 10), None),
            node("expired-edge", "fact", "expired"),
            node("equal-edge", "fact", "equal"),
            node("later-edge", "fact", "later"),
            node("future-valid-edge", "fact", "future valid provenance"),
            edge("future-node-edge", "link", "root", "future-node"),
            edge_with_time("expired", "link", "root", "expired-edge", None, Some(INSTANT - 1)),
            edge_with_time("equal", "link", "root", "equal-edge", None, Some(INSTANT)),
            edge_with_time("later", "link", "root", "later-edge", None, Some(INSTANT + 1)),
            edge_with_time(
                "future-valid",
                "link",
                "root",
                "future-valid-edge",
                Some(INSTANT + 1_000),
                None,
            ),
            edge("future-node-edge-in", "link", "future-node", "root"),
            edge_with_time("expired-in", "link", "expired-edge", "root", None, Some(INSTANT - 1)),
            edge_with_time("equal-in", "link", "equal-edge", "root", None, Some(INSTANT)),
            edge_with_time("later-in", "link", "later-edge", "root", None, Some(INSTANT + 1)),
            edge_with_time(
                "future-valid-in",
                "link",
                "future-valid-edge",
                "root",
                Some(INSTANT + 1_000),
                None,
            ),
        ])
        .unwrap();

    for frozen in [false, true] {
        for direction in
            [TraversalDirection::Incoming, TraversalDirection::Outgoing, TraversalDirection::Both]
        {
            for relaxed in [false, true] {
                let view = ReadView {
                    include_out_of_window: relaxed,
                    valid_as_of: Some(INSTANT),
                    ..ReadView::default()
                };
                let read_context = context(view, SearchFilter::default());
                let mut request = explicit_request(&["root"], direction);
                request.max_depth = 1;
                request.context = if frozen {
                    GraphReadContextV1::Frozen {
                        schema_version: 1,
                        context: opened.engine.freeze_read_context(&read_context).unwrap(),
                    }
                } else {
                    GraphReadContextV1::Current { schema_version: 1, context: read_context }
                };
                let result = opened.engine.graph_expand(&request).unwrap();
                let ids = target_ids(&result);
                assert_eq!(ids.contains(&"future-node"), relaxed);
                assert!(!ids.contains(&"expired-edge"));
                assert!(!ids.contains(&"equal-edge"));
                assert!(ids.contains(&"later-edge"));
                assert!(ids.contains(&"future-valid-edge"));
            }
        }
    }
}

#[derive(Clone)]
struct CountingEmbedder(Arc<AtomicUsize>);

impl Embedder for CountingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice60-counting", "1", 384)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let mut vector = vec![0.0; 384];
        vector[0] = 1.0;
        Ok(vector)
    }
}

#[test]
fn query_seed_declines_dense_before_embedding_and_nonlogical_vector_cap() {
    let dir = TempDir::new().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let opened = Engine::open_with_embedder_for_test(
        path(&dir, "query-seeds"),
        Arc::new(CountingEmbedder(Arc::clone(&calls))),
    )
    .unwrap();
    opened.engine.configure_vector_kind_for_test("noise").unwrap();
    let mut writes = (0..=TOP_K_BIT_CANDIDATES)
        .map(|index| {
            node_with_window(
                None,
                "noise",
                &format!("nearer vector-only anonymous row {index}"),
                None,
                None,
            )
        })
        .collect::<Vec<_>>();
    writes.push(node("logical-seed", "doc", "farther lexical lodestar"));
    opened.engine.write(&writes).unwrap();
    opened.engine.drain(10_000).unwrap();
    calls.store(0, Ordering::SeqCst);

    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Query {
            schema_version: 1,
            text: "farther lexical lodestar".into(),
            ranked_limit: 1,
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: current(ReadView::default(), SearchFilter::default()),
        max_depth: 0,
        result_limit: 1,
        max_work_units: 1,
        include_explanation: true,
    };
    let result = opened.engine.graph_expand(&request).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 0, "query graph seeding entered embedding/KNN");
    assert_eq!(result.seeds.len(), 1);
    assert_eq!(result.seeds[0].logical_id, "logical-seed");
    assert_eq!(result.seeds[0].seed_ordinal, 0);
    assert!(result.seeds[0].query_score.unwrap().is_finite());
    assert_eq!(result.degradation_codes, [GraphExpansionDegradationCodeV1::QuerySeedTextFallback]);
    let explanation = result.explanation.unwrap();
    assert_eq!(explanation.seed_source, GraphSeedSourceV1::Query);
    assert_eq!(explanation.read_mode, GraphReadModeV1::Current);
}

#[test]
fn zero_match_query_still_reports_truthful_text_fallback() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "empty-query")).unwrap();
    let mut request = explicit_request(&["irrelevant"], TraversalDirection::Outgoing);
    request.seed =
        GraphSeedV1::Query { schema_version: 1, text: "does-not-exist".into(), ranked_limit: 25 };
    request.max_depth = 0;
    let result = opened.engine.graph_expand(&request).unwrap();
    assert!(result.seeds.is_empty());
    assert!(result.targets.is_empty());
    assert_eq!(result.degradation_codes, [GraphExpansionDegradationCodeV1::QuerySeedTextFallback]);
}

#[test]
fn projection_degradation_composition_is_exhaustive_ordered_and_deduplicated() {
    use GraphExpansionDegradationCodeV1 as Code;
    let cases = [
        (
            GraphSeedSourceV1::Explicit,
            GraphProjectionOriginV1::NotApplicable,
            GraphProjectionReadinessV1::NotApplicable,
            Vec::new(),
        ),
        (
            GraphSeedSourceV1::Query,
            GraphProjectionOriginV1::Fresh,
            GraphProjectionReadinessV1::Ready,
            vec![Code::QuerySeedTextFallback],
        ),
        (
            GraphSeedSourceV1::Query,
            GraphProjectionOriginV1::LegacyUnverified,
            GraphProjectionReadinessV1::Degraded,
            vec![
                Code::QuerySeedTextFallback,
                Code::ProjectionLegacyUnverified,
                Code::ProjectionDegraded,
            ],
        ),
        (
            GraphSeedSourceV1::Query,
            GraphProjectionOriginV1::Configuration,
            GraphProjectionReadinessV1::Processing,
            vec![Code::QuerySeedTextFallback, Code::ProjectionProcessing],
        ),
        (
            GraphSeedSourceV1::Query,
            GraphProjectionOriginV1::Configuration,
            GraphProjectionReadinessV1::Blocked,
            vec![Code::QuerySeedTextFallback, Code::ProjectionBlocked],
        ),
        (
            GraphSeedSourceV1::Query,
            GraphProjectionOriginV1::Rebuild,
            GraphProjectionReadinessV1::Deferred,
            vec![Code::QuerySeedTextFallback, Code::ProjectionDeferred],
        ),
    ];
    for (seed, origin, readiness, expected) in cases {
        assert_eq!(graph_expansion_degradation_codes_for_test(seed, origin, readiness), expected);
    }
}

#[test]
fn explanation_is_compact_correlated_and_allocation_free_when_disabled() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "explanation")).unwrap();
    opened
        .engine
        .write(&[
            node("root", "seed", "root"),
            node("target", "fact", "body"),
            edge("e", "link", "root", "target"),
        ])
        .unwrap();

    let off = opened
        .engine
        .graph_expand(&explicit_request(&["root"], TraversalDirection::Outgoing))
        .unwrap();
    assert!(off.explanation.is_none());
    assert!(off.degradation_codes.is_empty());

    let mut explained = explicit_request(&["root"], TraversalDirection::Outgoing);
    explained.include_explanation = true;
    let first = opened.engine.graph_expand(&explained).unwrap();
    let second = opened.engine.graph_expand(&explained).unwrap();
    assert_eq!(first.targets, off.targets);
    assert_eq!(first.work_units, off.work_units);
    let first_explanation = first.explanation.unwrap();
    let second_explanation = second.explanation.unwrap();
    assert_ne!(first_explanation.correlation_id, second_explanation.correlation_id);
    assert_correlation(&first_explanation.correlation_id);
    assert_eq!(first_explanation.seed_source, GraphSeedSourceV1::Explicit);
    assert_eq!(first_explanation.projection_generation_id, None);
    assert_eq!(first_explanation.projection_origin, GraphProjectionOriginV1::NotApplicable);
    assert_eq!(first_explanation.projection_readiness, GraphProjectionReadinessV1::NotApplicable);
    assert_eq!(first_explanation.per_target.len(), first.targets.len());
    assert_eq!(first_explanation.per_target[0].target_index, 0);
    assert_eq!(first_explanation.per_target[0].origin, first.targets[0].origin);
}

fn assert_correlation(value: &str) {
    let Some((nonce, sequence)) = value.strip_prefix('x').and_then(|value| value.split_once('-'))
    else {
        panic!("invalid correlation: {value}");
    };
    assert_eq!(nonce.len(), 32);
    assert!(nonce.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    assert!(
        sequence == "0"
            || (!sequence.starts_with('0') && sequence.bytes().all(|b| b.is_ascii_digit()))
    );
}

fn hook_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn endpoint_plans_use_shipped_indexes_without_full_edge_scan_and_schema_stays_33() {
    assert_eq!(SCHEMA_VERSION, 33);
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "plans")).unwrap();
    for direction in
        [TraversalDirection::Incoming, TraversalDirection::Outgoing, TraversalDirection::Both]
    {
        let plan = opened.engine.explain_graph_expand_for_test(direction).unwrap();
        let has_from = plan.iter().any(|line| line.contains("canonical_edges_from_id_idx"));
        let has_to = plan.iter().any(|line| line.contains("canonical_edges_to_id_idx"));
        match direction {
            TraversalDirection::Incoming => assert!(has_to && !has_from, "{plan:?}"),
            TraversalDirection::Outgoing => assert!(has_from && !has_to, "{plan:?}"),
            TraversalDirection::Both => assert!(has_from && has_to, "{plan:?}"),
        }
        assert!(
            !plan.iter().any(|line| {
                line.contains("SCAN canonical_edges") && !line.contains("USING INDEX")
            }),
            "{direction:?}: full edge scan: {plan:?}"
        );
    }
}

#[test]
fn frozen_pre_pin_drift_and_post_pin_isolation_use_bounded_cancellation_safe_hooks() {
    let _serial = hook_test_lock();
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "races")).unwrap();
    let engine = Arc::new(opened.engine);
    engine.write(&[node("root", "seed", "root")]).unwrap();
    let read_context = context(
        ReadView { valid_as_of: Some(INSTANT), ..ReadView::default() },
        SearchFilter::default(),
    );
    let frozen = engine.freeze_read_context(&read_context).unwrap();

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    arm_graph_expand_before_pin_hook_for_test(Box::new(move || {
        ready_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    }));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen };
    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.graph_expand(&request))
    };
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    engine
        .write(&[node("before", "fact", "before"), edge("before-e", "link", "root", "before")])
        .unwrap();
    release_tx.send(()).unwrap();
    assert!(matches!(worker.join().unwrap(), Err(EngineError::FrozenRead(_))));

    let frozen = engine.freeze_read_context(&read_context).unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    arm_graph_expand_after_pin_hook_for_test(Box::new(move || {
        ready_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    }));
    let mut request = explicit_request(&["root"], TraversalDirection::Outgoing);
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen };
    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.graph_expand(&request))
    };
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    engine
        .write(&[node("after", "fact", "after"), edge("after-e", "link", "root", "after")])
        .unwrap();
    release_tx.send(()).unwrap();
    let result = worker.join().unwrap().unwrap();
    assert!(target_ids(&result).contains(&"before"));
    assert!(!target_ids(&result).contains(&"after"));
}

#[test]
fn current_post_pin_write_linearizes_after_the_complete_operation() {
    let _serial = hook_test_lock();
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "current-race")).unwrap();
    let engine = Arc::new(opened.engine);
    engine.write(&[node("root", "seed", "root")]).unwrap();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    arm_graph_expand_after_pin_hook_for_test(Box::new(move || {
        ready_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    }));
    let request = explicit_request(&["root"], TraversalDirection::Outgoing);
    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.graph_expand(&request))
    };
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    engine
        .write(&[node("after", "fact", "after"), edge("after-e", "link", "root", "after")])
        .unwrap();
    release_tx.send(()).unwrap();
    assert!(!target_ids(&worker.join().unwrap().unwrap()).contains(&"after"));
}
