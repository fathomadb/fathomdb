use fathomdb_engine::*;
use std::{collections::BTreeSet, sync::Arc};

struct Fixed;
impl fathomdb_embedder_api::Embedder for Fixed {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("slice83", "fixed", 8)
    }
    fn embed(
        &self,
        _: &str,
    ) -> Result<fathomdb_embedder_api::Vector, fathomdb_embedder_api::EmbedderError> {
        Ok(vec![1.; 8])
    }
}

fn node(id: &str, source: &str, body: String) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body,
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(format!("revision-{id}")).unwrap(),
            SourceVersionId::new(format!("version-{id}")).unwrap(),
        ),
    })
}
fn edge(id: &str, source: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "rel".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        body: Some("relationship".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}
fn fixture(path: &std::path::Path) -> Engine {
    let engine = Engine::open_with_embedder_for_test(path, Arc::new(Fixed)).unwrap().engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "text".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .write(&[
            node("left", "survivor", r#"{"text":"common survivor left"}"#.into()),
            node("right", "survivor", r#"{"text":"common common survivor right"}"#.into()),
            edge("kept-edge", "survivor", "left", "right"),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    assert!(
        engine.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_vector_spans").unwrap()[0]
            >= 2
    );
    engine
}
fn hits(result: SearchResult) -> Vec<(String, u64)> {
    result.results.into_iter().map(|hit| (hit.id.to_prefixed(), hit.score.to_bits())).collect()
}
fn context(view: ReadView) -> ReadContextV1 {
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".into());
    ReadContextV1::new(view, filter).unwrap()
}
fn graph(context: ReadContextV1) -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("left")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec![],
        target_kinds: vec![],
        context: GraphReadContextV1::Current { schema_version: 1, context },
        max_depth: 2,
        result_limit: 1,
        max_work_units: 100,
        include_explanation: false,
        include_evidence: false,
    }
}

#[test]
fn owed_debt_routes_match_independent_survivor_only_stores_before_caps_and_statistics() {
    for (targets, terms) in [(1, 10), (4, 300), (12, 20)] {
        let dir = tempfile::tempdir().unwrap();
        let baseline = Arc::new(fixture(&dir.path().join("baseline.sqlite")));
        let engine = Arc::new(fixture(&dir.path().join("candidate.sqlite")));
        for index in 0..targets {
            let id = format!("target{index}");
            engine
                .write(&[
                    node(
                        &id,
                        "target",
                        serde_json::json!({"text": "common ".repeat(terms)}).to_string(),
                    ),
                    edge(&format!("target-edge-{index}"), "target", "left", &id),
                ])
                .unwrap();
        }
        engine.drain(10_000).unwrap();
        let stale = engine.freeze_read_context(&context(ReadView::default())).unwrap();
        let candidate = Arc::clone(&engine);
        let control = Arc::clone(&baseline);
        arm_physical_erasure_hook_for_test(
            "after_sanitation_unit",
            Box::new(move || {
                assert!(matches!(
                    candidate.search_frozen("common", &stale, 0, false, 0.5, 10, false, 10),
                    Err(EngineError::FrozenRead(_))
                ));
                for view in [
                    ReadView::default(),
                    ReadView {
                        include_superseded: true,
                        include_inactive: true,
                        include_out_of_window: true,
                        valid_as_of: Some(1_800_000_000),
                    },
                ] {
                    assert_eq!(
                        hits(
                            candidate.search_text_only_view_with_limit("common", &view, 1).unwrap()
                        ),
                        hits(control.search_text_only_view_with_limit("common", &view, 1).unwrap())
                    );
                    assert_eq!(
                        hits(
                            candidate.search_projected_text("common", "text", None, &view).unwrap()
                        ),
                        hits(control.search_projected_text("common", "text", None, &view).unwrap())
                    );
                    assert_eq!(
                        candidate.read_list("doc", &[], 1, &view).unwrap(),
                        control.read_list("doc", &[], 1, &view).unwrap()
                    );
                    assert_eq!(
                        candidate
                            .graph_neighbors("left", 2, TraversalDirection::Outgoing, &view)
                            .unwrap(),
                        control
                            .graph_neighbors("left", 2, TraversalDirection::Outgoing, &view)
                            .unwrap()
                    );
                    if view.include_superseded || view.include_inactive {
                        assert!(matches!(
                            candidate.graph_expand(&graph(context(view))),
                            Err(EngineError::GraphExpansion(_))
                        ));
                        assert!(matches!(
                            control.graph_expand(&graph(context(view))),
                            Err(EngineError::GraphExpansion(_))
                        ));
                    } else {
                        assert_eq!(
                            candidate.graph_expand(&graph(context(view))).unwrap().targets,
                            control.graph_expand(&graph(context(view))).unwrap().targets
                        );
                    }
                    let frozen = candidate.freeze_read_context(&context(view)).unwrap();
                    let baseline_frozen = control.freeze_read_context(&context(view)).unwrap();
                    assert_eq!(
                        hits(
                            candidate
                                .search_frozen("common", &frozen, 0, true, 0.5, 10, false, 10)
                                .unwrap()
                        ),
                        hits(
                            control
                                .search_frozen(
                                    "common",
                                    &baseline_frozen,
                                    0,
                                    true,
                                    0.5,
                                    10,
                                    false,
                                    10
                                )
                                .unwrap()
                        )
                    );
                    let expanded = candidate.search_expand_frozen("common", &frozen, 2, 1).unwrap();
                    let reference =
                        control.search_expand_frozen("common", &baseline_frozen, 2, 1).unwrap();
                    assert_eq!(expanded.all_logical_ids, reference.all_logical_ids);
                    assert_eq!(expanded.expanded, reference.expanded);
                    assert_eq!(
                        expanded
                            .search_hits
                            .iter()
                            .map(|h| (&h.id, h.score.to_bits()))
                            .collect::<Vec<_>>(),
                        reference
                            .search_hits
                            .iter()
                            .map(|h| (&h.id, h.score.to_bits()))
                            .collect::<Vec<_>>()
                    );
                    let page = PageRequestV1 { schema_version: 1, limit: 1, cursor: None };
                    assert_eq!(
                        candidate.read_canonical_page("doc", &frozen, &page).unwrap().items,
                        control.read_canonical_page("doc", &baseline_frozen, &page).unwrap().items
                    );
                    let evidence = candidate
                        .search_with_evidence(&EvidenceSearchRequestV1 {
                            schema_version: 1,
                            query: "common".into(),
                            context: frozen.clone(),
                            rerank_depth: 0,
                            use_graph_arm: false,
                            alpha: 0.5,
                            pool_n: 10,
                            include_explanation: false,
                            limit: 10,
                        })
                        .unwrap();
                    assert!(!evidence.evidence.is_empty());
                    assert_eq!(evidence.evidence.len(), evidence.search_result.results.len());
                    for item in evidence.evidence {
                        assert!(matches!(
                            item.artifact_revision_id.as_str(),
                            "revision-left" | "revision-right"
                        ));
                        candidate
                            .resolve_evidence(&EvidenceResolveRequestV1 {
                                schema_version: 1,
                                evidence_ref: item.evidence_ref,
                                context: frozen.clone(),
                            })
                            .unwrap();
                    }
                }
                assert_eq!(
                    hits(candidate.search("common").unwrap()),
                    hits(control.search("common").unwrap())
                );
                let expanded = candidate.search_expand_with_limit("common", None, 2, 1).unwrap();
                let reference = control.search_expand_with_limit("common", None, 2, 1).unwrap();
                assert_eq!(expanded.all_logical_ids, reference.all_logical_ids);
                assert_eq!(expanded.expanded, reference.expanded);
                assert!(matches!(
                    candidate.rebuild_projections(),
                    Err(EngineError::ErasureIncomplete { .. })
                ));
                assert!(matches!(
                    candidate.rebuild_vec0(),
                    Err(EngineError::ErasureIncomplete { .. })
                ));
                Ok(())
            }),
        );
        engine.erase_source("target").unwrap();
        engine.close().unwrap();
        baseline.close().unwrap();
    }
}

#[test]
fn queued_projection_is_drained_before_delete_and_cannot_publish_afterward() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(fixture(&dir.path().join("queued.sqlite")));
    let (reached, release) = engine.pause_projection_worker_before_write_lock_for_test();
    let target_cursor = engine
        .write(&[node("queued-target", "target", r#"{"text":"targetneedle"}"#.into())])
        .unwrap()
        .row_cursors[0];
    reached.wait();
    let worker = Arc::clone(&engine);
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let erase = std::thread::spawn(move || {
        let result = worker.erase_source("target");
        done_tx.send(()).unwrap();
        result
    });
    assert!(done_rx.recv_timeout(std::time::Duration::from_millis(25)).is_err());
    release.wait();
    erase.join().unwrap().unwrap();
    engine.drain(10_000).unwrap();
    assert!(engine.search_text_only("targetneedle").unwrap().results.is_empty());
    assert!(engine
        .search("targetneedle")
        .unwrap()
        .results
        .iter()
        .all(|hit| hit.id != IdSpace::logical("queued-target")));
    assert_eq!(
        engine
            .query_i64_col_for_test("SELECT COUNT(*) FROM canonical_nodes WHERE source_id='target'")
            .unwrap(),
        vec![0]
    );
    assert_eq!(
        engine
            .query_i64_col_for_test(&format!(
                "SELECT COUNT(*) FROM _fathomdb_vector_spans WHERE write_cursor={target_cursor}"
            ))
            .unwrap(),
        vec![0]
    );
    engine.close().unwrap();
}
