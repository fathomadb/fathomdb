use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, EvidenceSearchRequestV1, InitialState, PreparedWrite,
    ProjectionFts, ProjectionRole, ProjectionSpec, ProjectionVector, PropertyComparisonOp,
    PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue, ReadContextV1, ReadView,
    ScopePolicy, SearchFilter, SoftFallbackBranch, SourceId, TestEmbedderChoice,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct CosineTruthEmbedder;

impl Embedder for CosineTruthEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-cosine-truth", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text.contains("identical") || text.contains("query_unit") {
            Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("orthogonal") {
            Ok(vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("opposite") {
            Ok(vec![-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("scaled_doc") {
            Ok(vec![3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("scaled_query") {
            Ok(vec![6.0, 8.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("zero_norm") {
            Ok(vec![0.0; 8])
        } else if text.contains("angled") {
            // cos with [1,0,...] = 0.6
            Ok(vec![0.6, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else {
            Ok(vec![0.8, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        }
    }
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "memory".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds: None }),
        source: None,
        entity_class: PropertyEntityClass::Node,
    }
}

fn assert_universal_cosine_invariant(explanation: &fathomdb_engine::Explanation) {
    for p in &explanation.per_hit {
        assert!(
            p.vector_rank.is_some() || p.vector_cosine.is_none(),
            "vector_cosine must be None whenever vector_rank is None: {p:?}"
        );
        if let Some(c) = p.vector_cosine {
            assert!(c.is_finite(), "vector_cosine must be finite: {c}");
            assert!((-1.0..=1.0).contains(&c), "vector_cosine out of [-1,1]: {c}");
            if c == 0.0 {
                assert!(!c.is_sign_negative(), "zero cosine must be normalized to +0.0");
            }
        }
    }
}

#[test]
fn vec_distance_cosine_truth_table_via_engine() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("cosine-truth{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(CosineTruthEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();

    let receipt = opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "identical vector record".into(),
                source_id: SourceId::new("src:1").unwrap(),
                logical_id: Some("n-identical".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "orthogonal vector record".into(),
                source_id: SourceId::new("src:2").unwrap(),
                logical_id: Some("n-orthogonal".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "opposite vector record".into(),
                source_id: SourceId::new("src:3").unwrap(),
                logical_id: Some("n-opposite".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "scaled_doc non-unit vector record".into(),
                source_id: SourceId::new("src:4").unwrap(),
                logical_id: Some("n-scaled".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "angled vector record".into(),
                source_id: SourceId::new("src:5").unwrap(),
                logical_id: Some("n-angled".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();

    let [c_ident, c_ortho, c_opp, c_scaled, c_angled]: [u64; 5] =
        receipt.row_cursors.as_slice().try_into().unwrap();

    // Query with unit vector [1, 0, ...]
    let res = opened.engine.search_explained("query_unit", None, 0, false, 0.3, 0).unwrap();
    let exp = res.explanation.as_ref().expect("explanation");
    assert_universal_cosine_invariant(exp);

    let cosine_of = |cursor: u64| {
        exp.per_hit
            .iter()
            .find(|p| p.id == cursor)
            .unwrap_or_else(|| panic!("missing cursor {cursor} in {:?}", exp.per_hit))
            .vector_cosine
    };

    let ident = cosine_of(c_ident).expect("identical cosine");
    assert!((ident - 1.0).abs() < 1e-5, "expected ~1.0, got {ident}");

    let ortho = cosine_of(c_ortho).expect("orthogonal cosine");
    assert!(ortho.abs() < 1e-5, "expected ~0.0, got {ortho}");
    if ortho == 0.0 {
        assert!(!ortho.is_sign_negative(), "must normalize -0.0 to +0.0");
    }

    let opp = cosine_of(c_opp).expect("opposite cosine");
    assert!((opp + 1.0).abs() < 1e-5, "expected ~-1.0, got {opp}");

    let angled = cosine_of(c_angled).expect("angled cosine");
    assert!((angled - 0.6).abs() < 1e-5, "expected ~0.6, got {angled}");

    // Non-unit scaled query [6, 8, ...] vs scaled_doc [3, 4, ...] -> cosine ~ 1.0
    let scaled_res =
        opened.engine.search_explained("scaled_query", None, 0, false, 0.3, 0).unwrap();
    let scaled_exp = scaled_res.explanation.as_ref().expect("explanation");
    assert_universal_cosine_invariant(scaled_exp);
    let scaled_cos = scaled_exp
        .per_hit
        .iter()
        .find(|p| p.id == c_scaled)
        .expect("scaled hit")
        .vector_cosine
        .expect("scaled cosine");
    assert!(
        (scaled_cos - 1.0).abs() < 1e-5,
        "non-unit scaled cosine expected ~1.0, got {scaled_cos}"
    );

    // Zero-norm query vector -> vec_distance_cosine returns NULL/NaN -> vector_cosine == None!
    let zero_res = opened.engine.search_explained("zero_norm", None, 0, false, 0.3, 0).unwrap();
    let zero_exp = zero_res.explanation.as_ref().expect("explanation");
    assert_universal_cosine_invariant(zero_exp);
    for p in &zero_exp.per_hit {
        assert_eq!(
            p.vector_cosine, None,
            "zero-norm query vector must produce vector_cosine == None, got {:?}",
            p.vector_cosine
        );
    }

    opened.engine.close().unwrap();
}

#[test]
fn arm_semantics_sql_branches_evidence_and_l2_order_invariance() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("cosine-arms{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(CosineTruthEmbedder)).unwrap();
    opened
        .engine
        .configure_projections(
            &[
                vector_spec(),
                ProjectionSpec {
                    name: "rank_num".into(),
                    roles: [ProjectionRole::Filterable].into_iter().collect(),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        )
        .unwrap();

    let mut props1 = PropertyMap::new();
    props1.insert("rank_num".into(), PropertyValue::Int(10));
    let mut props2 = PropertyMap::new();
    props2.insert("rank_num".into(), PropertyValue::Int(20));

    let receipt = opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "identical hybrid anchor token".into(),
                source_id: SourceId::new("src:n1").unwrap(),
                logical_id: Some("n1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(100),
                valid_until: Some(200),
                properties: Some(props1),
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "angled secondary node".into(),
                source_id: SourceId::new("src:n2").unwrap(),
                logical_id: Some("n2".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(props2),
            },
            PreparedWrite::Node {
                kind: "unindexed_kind".into(),
                body: "graph target zeta node".into(),
                source_id: SourceId::new("src:n3").unwrap(),
                logical_id: Some("n3".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Edge {
                kind: "relates".into(),
                from: "n1".into(),
                to: "n3".into(),
                source_id: SourceId::new("src:e1").unwrap(),
                logical_id: Some("e1".into()),
                body: Some("identical edge_fact payload".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let [c_n1, c_n2, _c_n3, c_edge]: [u64; 4] = receipt.row_cursors.as_slice().try_into().unwrap();

    // 1. Hybrid Vector + Text node AND Vector-scored TextEdge (edge_fact) + L2 order invariance
    let view_in_window = ReadView { valid_as_of: Some(150), ..ReadView::default() };
    let plain = opened
        .engine
        .search_reranked_view("identical hybrid", None, 0, false, 0.3, 0, false, &view_in_window)
        .unwrap();
    let explained = opened
        .engine
        .search_reranked_view("identical hybrid", None, 0, false, 0.3, 0, true, &view_in_window)
        .unwrap();
    assert_eq!(explained.results, plain.results, "explain=true must not perturb L2/RRF ordering");

    let exp = explained.explanation.as_ref().expect("explanation");
    assert_universal_cosine_invariant(exp);
    let n1_exp = exp.per_hit.iter().find(|p| p.id == c_n1).expect("n1 hybrid hit");
    assert!(n1_exp.vector_rank.is_some());
    assert!(n1_exp.text_rank.is_some());
    assert!(
        n1_exp.vector_cosine.is_some_and(|c| (c - 1.0).abs() < 1e-5),
        "hybrid Vector+Text hit must have vector_cosine ~1.0, got {:?}",
        n1_exp.vector_cosine
    );

    let edge_exp = exp.per_hit.iter().find(|p| p.id == c_edge).expect("edge_fact hit");
    assert_eq!(edge_exp.arm, SoftFallbackBranch::TextEdge);
    assert!(edge_exp.vector_rank.is_some());
    assert!(
        edge_exp.vector_cosine.is_some_and(|c| (c - 1.0).abs() < 1e-5),
        "vector-scored TextEdge edge_fact hit must have vector_cosine ~1.0, got {:?}",
        edge_exp.vector_cosine
    );

    // 2. Preselection SQL path (filter.rs:1129): numeric Ne predicate -> non-pushable -> preselection!
    let mut preselect_filter = SearchFilter::default();
    preselect_filter.property_predicates.push(PropertyPredicate {
        key: "rank_num".into(),
        op: PropertyComparisonOp::Ne,
        value: PropertyValue::Int(10),
    });
    let preselect_res = opened
        .engine
        .search_explained("query_unit", Some(preselect_filter), 0, false, 0.3, 0)
        .unwrap();
    let preselect_exp = preselect_res.explanation.as_ref().expect("preselect explanation");
    assert_universal_cosine_invariant(preselect_exp);
    let n2_pre = preselect_exp.per_hit.iter().find(|p| p.id == c_n2).expect("n2 preselected");
    assert!(
        n2_pre.vector_cosine.is_some_and(|c| (c - 0.6).abs() < 1e-5),
        "preselection SQL path must compute vector_cosine ~0.6, got {:?}",
        n2_pre.vector_cosine
    );

    // 3. Relaxed ReadView + include_out_of_window = true (filter.rs:1158-1163)
    let oow_view =
        ReadView { valid_as_of: Some(300), include_out_of_window: true, ..ReadView::default() };
    let oow_res = opened
        .engine
        .search_reranked_view("query_unit", None, 0, false, 0.3, 0, true, &oow_view)
        .unwrap();
    let oow_exp = oow_res.explanation.as_ref().expect("oow explanation");
    assert_universal_cosine_invariant(oow_exp);
    let n1_oow = oow_exp.per_hit.iter().find(|p| p.id == c_n1).expect("n1 out-of-window hit");
    assert!(
        n1_oow.vector_cosine.is_some_and(|c| (c - 1.0).abs() < 1e-5),
        "include_out_of_window fast path must populate vector_cosine, got {:?}",
        n1_oow.vector_cosine
    );

    // 4. search_with_evidence(..., include_explanation = true) on a provenanced corpus
    let ev_path = dir.path().join(format!("cosine-evidence{SQLITE_SUFFIX}"));
    let ev_opened =
        Engine::open_with_embedder_for_test(&ev_path, Arc::new(CosineTruthEmbedder)).unwrap();
    ev_opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    ev_opened
        .engine
        .actuate(
            fathomdb_engine::ActuationBatchV1::new(
                "cosine-ev-seed",
                vec![fathomdb_engine::ActuationOperationV1::PutCanonicalNode(
                    fathomdb_engine::ProvenancedNodeV1 {
                        kind: "doc".into(),
                        body: "identical provenanced cosine payload".into(),
                        source_id: SourceId::new("src:ev-cos").unwrap(),
                        logical_id: Some("ev-cos-1".into()),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        provenance: fathomdb_engine::WriteProvenanceV1::canonical(
                            fathomdb_engine::ArtifactRevisionId::new("rev-ev-cos").unwrap(),
                            fathomdb_engine::SourceVersionId::new("ver-ev-cos").unwrap(),
                        ),
                        properties: None,
                    },
                )],
            )
            .unwrap(),
        )
        .unwrap();
    ev_opened.engine.drain(30_000).unwrap();
    let frozen = ev_opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let mut ev_req = EvidenceSearchRequestV1::new("query_unit".into(), frozen);
    ev_req.include_explanation = true;
    let ev_res = ev_opened.engine.search_with_evidence(&ev_req).unwrap();
    let ev_exp = ev_res.search_result.explanation.as_ref().expect("evidence explanation");
    assert_universal_cosine_invariant(ev_exp);
    assert!(ev_exp.per_hit.iter().any(|p| p.vector_cosine.is_some_and(|c| (c - 1.0).abs() < 1e-5)));
    ev_opened.engine.close().unwrap();

    // 5. Text-only / GraphArm hits -> vector_cosine == None
    let graph_res = opened
        .engine
        .search_reranked_view("identical hybrid", None, 0, true, 0.3, 0, true, &view_in_window)
        .unwrap();
    let graph_exp = graph_res.explanation.as_ref().expect("graph explanation");
    assert_universal_cosine_invariant(graph_exp);
    for p in &graph_exp.per_hit {
        if p.arm == SoftFallbackBranch::GraphArm {
            assert_eq!(p.vector_rank, None);
            assert_eq!(p.vector_cosine, None);
        }
    }

    // 6. Soft-fallback when vector projection is pending (frozen scheduler, unembedded node)
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let pending_cursor = opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "unembedded_unique_xylophone_token".into(),
            source_id: SourceId::new("src:pending").unwrap(),
            logical_id: Some("n-pending".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap()
        .row_cursors[0];
    let pending_res = opened
        .engine
        .search_explained("unembedded_unique_xylophone_token", None, 0, false, 0.3, 0)
        .unwrap();
    let pending_exp = pending_res.explanation.as_ref().expect("pending explanation");
    assert_universal_cosine_invariant(pending_exp);
    let pending_hit =
        pending_exp.per_hit.iter().find(|p| p.id == pending_cursor).expect("pending text hit");
    assert_eq!(pending_hit.vector_rank, None);
    assert_eq!(pending_hit.vector_cosine, None);
    opened.engine.close().unwrap();

    // 7. EmbedderChoice::None (use_default_embedder = false) -> all vector_cosine == None
    let absent =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .unwrap();
    let absent_res = absent
        .engine
        .search_reranked_view("identical hybrid", None, 0, true, 0.3, 0, true, &view_in_window)
        .unwrap();
    let absent_exp = absent_res.explanation.as_ref().expect("absent explanation");
    assert!(!absent_exp.per_hit.is_empty());
    for p in &absent_exp.per_hit {
        assert_eq!(p.vector_rank, None);
        assert_eq!(p.vector_cosine, None);
    }
    absent.engine.close().unwrap();
}

#[test]
fn multi_scope_vector_cosine_and_l2_order_invariance() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("cosine-multi-scope{SQLITE_SUFFIX}"));
    {
        let a = Engine::open_with_test_choice_and_config_for_test(
            &path,
            TestEmbedderChoice::Caller(Arc::new(CosineTruthEmbedder)),
            EngineConfig::default().with_scope_policy(ScopePolicy::single("s-a")),
        )
        .unwrap()
        .engine;
        a.configure_projections(&[vector_spec()], &[]).unwrap();
        a.write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "identical scope alpha".into(),
            source_id: SourceId::new("src:a").unwrap(),
            logical_id: Some("a1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
        a.drain(30_000).unwrap();
        a.close().unwrap();
    }
    {
        let b = Engine::open_with_test_choice_and_config_for_test(
            &path,
            TestEmbedderChoice::Caller(Arc::new(CosineTruthEmbedder)),
            EngineConfig::default().with_scope_policy(ScopePolicy::single("s-b")),
        )
        .unwrap()
        .engine;
        b.write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "angled scope beta".into(),
            source_id: SourceId::new("src:b").unwrap(),
            logical_id: Some("b1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
        b.drain(30_000).unwrap();
        b.close().unwrap();
    }

    let multi = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(CosineTruthEmbedder)),
        EngineConfig::default()
            .with_scope_policy(ScopePolicy::read_write(vec!["s-a", "s-b"], "s-a")),
    )
    .unwrap()
    .engine;

    let plain = multi.search_reranked("query_unit", None, 0, false, 0.3, 0).unwrap();
    let explained = multi.search_explained("query_unit", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(explained.results, plain.results, "multi-scope L2/RRF order must be invariant");

    let exp = explained.explanation.as_ref().expect("multi-scope explanation");
    assert_universal_cosine_invariant(exp);
    assert_eq!(exp.per_hit.len(), 2);
    let cosines: Vec<f64> = exp.per_hit.iter().map(|p| p.vector_cosine.unwrap()).collect();
    assert!((cosines[0] - 1.0).abs() < 1e-5, "top hit should be identical (~1.0), got {cosines:?}");
    assert!((cosines[1] - 0.6).abs() < 1e-5, "second hit should be angled (~0.6), got {cosines:?}");

    multi.close().unwrap();
}
