use fathomdb_sdk::{
    admin, graph, read, rerank, ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1,
    ArtifactRevisionId, CanonicalHash, ClosureCauseV1, ClosureLookupV1, ClosurePhaseV1,
    DenseReadiness, DependencyDerivedLookupV1, DependencySourceLookupV1,
    DependencyTraceDirectionV1, DependencyTraceRequestV1, EmbeddingReadinessState, Engine,
    ErrorKind, EvidenceResolveRequestV1, EvidenceSearchRequestV1, FrozenSearchOptions,
    GraphEvidenceArtifactV1, GraphEvidenceResolveRequestV1, GraphExpandRequestV1,
    GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, LifecycleActuationV1, LifecycleState,
    ListOptions, MutationProjectionStatusRequestV1, NeighborsOptions, OpenOptions, PageCursor,
    PageRequestV1, PreparedWrite, ProjectedTextSearchOptions, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1,
    ReadView, RerankOptions, RerankPassage, SearchExpandOptions, SearchFilter, SearchOptions,
    SoftFallbackBranch, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, TextSearchOptions, TraversalDirection, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;

fn observed(id: &str, detail: &str) {
    println!("OP|{id}|{detail}");
}

const SOURCE_BODY: &str = "{\"summary\": \"s02 canonical source evidence bytes\"}";
const CLAIM_TOKEN: &str = "slice135s02evidenceneedle";

fn derived(revision: &str) -> WriteProvenanceV1 {
    let digest = Sha256::digest(SOURCE_BODY.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    WriteProvenanceV1::derived(
        ArtifactRevisionId::new(revision).expect("revision"),
        SourceVersionId::new("s02-source-v1").expect("version"),
        SourceRevisionId::new("s02-source-r1").expect("source revision"),
        SourceLocator::whole_body(),
        CanonicalHash::sha256(digest).expect("canonical hash"),
    )
}

fn node(id: &str, summary: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("{{\"summary\": \"{summary}\"}}"),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn exercise_more(engine: &Engine) -> String {
    let schema =
        admin::configure(engine, "settings", r#"{"type":"object"}"#).expect("admin configure");
    assert!(schema.cursor > 0);
    observed("admin.configure", "registered latest_state collection");
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "events".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .expect("append log schema");
    let mutation = engine
        .write(&[
            PreparedWrite::OpStore {
                collection: "events".into(),
                record_key: "first".into(),
                schema_id: None,
                body: r#"{"value":1}"#.into(),
            },
            PreparedWrite::OpStore {
                collection: "events".into(),
                record_key: "second".into(),
                schema_id: None,
                body: r#"{"value":2}"#.into(),
            },
        ])
        .expect("operational writes");
    assert_eq!(mutation.row_cursors.len(), 2);
    engine
        .write(&[
            PreparedWrite::OpStore {
                collection: "settings".into(),
                record_key: "first".into(),
                schema_id: None,
                body: r#"{"value":1}"#.into(),
            },
            PreparedWrite::OpStore {
                collection: "settings".into(),
                record_key: "second".into(),
                schema_id: None,
                body: r#"{"value":2}"#.into(),
            },
        ])
        .expect("latest-state writes");
    let collection = read::collection(engine, "events", None, 10).expect("collection");
    assert_eq!(collection.len(), 2);
    assert_eq!(collection[0].record_key, "first");
    observed("read.collection", "two real op-store rows in insertion order");
    let mutations =
        read::mutations(engine, "events", Some(collection[0].id), 10).expect("mutations");
    assert_eq!(mutations.len(), 1);
    assert_eq!(mutations[0].record_key, "second");
    observed("read.mutations", "after-id excludes first mutation");
    let point = read::operational_state(engine, "settings", "first", None)
        .expect("operational state")
        .expect("first state");
    assert_eq!(point.payload, r#"{"value":1}"#);
    observed("read.operational_state", "point payload matches governed mutation");

    let many = read::get_many(engine, &["A".into(), "missing".into()], None).expect("get many");
    assert_eq!(many.len(), 2);
    assert_eq!(many[0].as_ref().map(|n| n.logical_id.as_str()), Some("A"));
    assert!(many[1].is_none());
    observed("read.get_many", "ordered present and absent slots");
    let listed = read::list(engine, "doc", ListOptions::default()).expect("list");
    assert!(listed.iter().any(|n| n.logical_id == "A"));
    observed("read.list", "doc kind includes retained anchor");
    let page_context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default())
                .expect("page context"),
        )
        .expect("freeze page context");
    let first = PageRequestV1 { schema_version: 1, limit: 1, cursor: None };
    let canonical =
        read::canonical_page(engine, "doc", &page_context, &first).expect("canonical first page");
    assert_eq!(canonical.items.len(), 1);
    let canonical_cursor = canonical.next_cursor.expect("canonical continuation");
    let second = read::canonical_page(
        engine,
        "doc",
        &page_context,
        &PageRequestV1 { cursor: Some(canonical_cursor), ..first.clone() },
    )
    .expect("canonical second page");
    assert_eq!(second.items.len(), 1);
    assert_ne!(canonical.items[0].logical_id, second.items[0].logical_id);
    for item in canonical.items.iter().chain(second.items.iter()) {
        let point =
            read::get(engine, &item.logical_id, None).expect("page point").expect("page id");
        assert_eq!(&point, item);
    }
    let malformed =
        PageRequestV1 { cursor: Some(PageCursor("fdbpg1.invalid".into())), ..first.clone() };
    assert!(read::canonical_page(engine, "doc", &page_context, &malformed).is_err());
    observed("read.canonical_page", "two pages agree with point reads; malformed cursor refused");
    let operational = read::operational_state_page(engine, "settings", &page_context, &first)
        .expect("operational first page");
    assert_eq!(operational.items.len(), 1);
    let op_cursor = operational.next_cursor.expect("operational continuation");
    let operational_second = read::operational_state_page(
        engine,
        "settings",
        &page_context,
        &PageRequestV1 { cursor: Some(op_cursor), ..first.clone() },
    )
    .expect("operational second page");
    assert_eq!(operational_second.items.len(), 1);
    for item in operational.items.iter().chain(operational_second.items.iter()) {
        let point = read::operational_state(engine, "settings", &item.record_key, None)
            .expect("operational point")
            .expect("operational key");
        assert_eq!(&point, item);
    }
    assert!(read::operational_state_page(engine, "settings", &page_context, &malformed).is_err());
    observed(
        "read.operational_state_page",
        "two pages agree with point reads; malformed cursor refused",
    );

    let frozen = engine
        .search_frozen("brightblue", &page_context, FrozenSearchOptions::default())
        .expect("frozen search");
    assert!(frozen.results.iter().any(|hit| hit.id.value == "A"));
    observed("engine.search_frozen", "frozen ranked search includes anchor");
    let expanded = engine
        .search_expand_frozen("brightblue", &page_context, 1, SearchExpandOptions::default())
        .expect("frozen expand");
    assert!(!expanded.search_hits.is_empty());
    observed("engine.search_expand_frozen", "frozen search returns seed hits");
    let graph_expanded =
        graph::search_expand(engine, "s02", 1, None, SearchExpandOptions::default())
            .expect("graph search expand");
    assert!(!graph_expanded.search_hits.is_empty());
    observed("graph.search_expand", "ranked graph seeds returned");
    let projected = engine
        .search_projected_text("brightblue", "summary", ProjectedTextSearchOptions::default())
        .expect("projected text");
    assert_eq!(projected.results.len(), 1);
    assert_eq!(projected.results[0].id.value, "A");
    observed("engine.search_projected_text", "declared summary FTS finds anchor");

    let status = read::projection_status(engine).expect("projection status");
    assert!(status.projections.iter().any(|p| p.name == "summary"));
    observed("read.projection_status", "summary declaration present in runtime status");
    let readiness = read::embedding_readiness(engine).expect("embedding readiness");
    assert_eq!(readiness.state, EmbeddingReadinessState::Ready);
    observed("read.embedding_readiness", "drained vector work reports ready");
    let generation = read::projection_generation_status(engine).expect("projection generation");
    assert_eq!(generation.schema_version, 1);
    observed("read.projection_generation_status", "versioned serving generation returned");
    let vector = engine.embed("harbor ferry").expect("engine embed");
    assert!(!vector.is_empty());
    assert!(vector.iter().all(|n| n.is_finite()));
    observed("engine.embed", "default embedder returned finite vector");

    let dep = SourceDependencyRegistrationV1::new(
        "slice135-capability-dep",
        "s02-source-r1",
        "s02-claim-r1",
    )
    .expect("dependency request");
    let registered = engine.register_source_dependency(dep).expect("register dependency");
    assert_eq!(registered.dependency_id.as_str(), "slice135-capability-dep");
    observed("engine.register_source_dependency", "source to derived revision registered");
    let by_source = engine
        .dependencies_for_source(
            DependencySourceLookupV1::new("s02-source-r1").expect("source lookup"),
        )
        .expect("dependencies for source");
    assert!(by_source.items.iter().any(|item| item == &registered));
    observed("engine.dependencies_for_source", "reciprocal source lookup contains dependency");
    let by_derived = engine
        .dependency_for_derived(
            DependencyDerivedLookupV1::new("s02-claim-r1").expect("derived lookup"),
        )
        .expect("dependency for derived");
    assert_eq!(by_derived, Some(registered));
    observed("engine.dependency_for_derived", "derived lookup agrees with source lookup");
    let trace_context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default())
                .expect("trace context"),
        )
        .expect("freeze trace context");
    let traced = engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "s02-claim-r1",
                DependencyTraceDirectionV1::ToSource,
                trace_context,
            )
            .expect("trace request"),
        )
        .expect("trace dependency");
    assert!(!traced.nodes.is_empty());
    observed("engine.trace_dependency", "derived-to-source trace contains nodes");
    let closure_hash = Sha256::digest(SOURCE_BODY.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: SOURCE_BODY.into(),
                source_id: SourceId::new("slice135-closure").expect("closure source"),
                logical_id: Some("closure-source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("closure-source-r1").expect("closure source revision"),
                    SourceVersionId::new("closure-source-v1").expect("closure source version"),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: "closure derived body".into(),
                source_id: SourceId::new("slice135-closure").expect("closure source"),
                logical_id: Some("closure-derived".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("closure-derived-r1")
                        .expect("closure derived revision"),
                    SourceVersionId::new("closure-source-v1").expect("closure source version"),
                    SourceRevisionId::new("closure-source-r1").expect("closure source revision"),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(closure_hash).expect("closure hash"),
                ),
            }),
        ])
        .expect("closure seed");
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "slice135-closure-dep",
                "closure-source-r1",
                "closure-derived-r1",
            )
            .expect("closure dep request"),
        )
        .expect("closure dependency");
    let close_request = ActuationBatchV1::new(
        "slice135-closure-transition",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "closure-source",
                ArtifactRevisionId::new("closure-source-r1").expect("revision"),
                LifecycleState::Deleted,
                None,
            )
            .expect("lifecycle request"),
        )],
    )
    .expect("closure request");
    let closure_receipt = engine.actuate(close_request.clone()).expect("closure actuation");
    assert_eq!(closure_receipt.outcome, ActuationOutcomeV1::CommittedClosurePending);
    assert_eq!(engine.actuate(close_request).expect("closure replay"), closure_receipt);
    assert_eq!(closure_receipt.closure_operation_ids.len(), 1);
    let closure_id = closure_receipt.closure_operation_ids[0].clone();
    let closure = engine
        .read_dependency_closure(ClosureLookupV1::new(closure_id.clone()).expect("closure lookup"))
        .expect("closure read")
        .expect("closure present");
    assert_eq!(closure.phase, ClosurePhaseV1::Complete);
    assert_eq!(closure.cause, ClosureCauseV1::SoftDeleted);
    let proof = closure.proof.expect("closure proof");
    assert_eq!(proof.current_active_dependent_nodes, 0);
    assert!(read::get(engine, "closure-derived", None).expect("closed derived").is_none());
    let absent_closure = engine
        .read_dependency_closure(
            ClosureLookupV1::new(format!("_fdb:c:{}", "a".repeat(64))).expect("closure lookup"),
        )
        .expect("closure absence");
    assert!(absent_closure.is_none());
    observed(
        "engine.read_dependency_closure",
        "committed closure has complete zero proof; unknown ID absent",
    );

    let act_node = ProvenancedNodeV1 {
        kind: "doc".into(),
        body: r#"{"summary":"actuation replay proof"}"#.into(),
        source_id: SourceId::new("slice135-actuation").expect("act source"),
        logical_id: Some("act-node".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("act-node-r1").expect("act revision"),
            SourceVersionId::new("act-source-v1").expect("act source version"),
        ),
    };
    let act = ActuationBatchV1::new(
        "slice135-capability-act",
        vec![ActuationOperationV1::PutCanonicalNode(act_node.clone())],
    )
    .expect("act batch");
    let first_receipt = engine.actuate(act.clone()).expect("actuate");
    assert_eq!(first_receipt.outcome, ActuationOutcomeV1::Committed);
    assert_eq!(engine.actuate(act).expect("replay"), first_receipt);
    assert!(read::get(engine, "act-node", None).expect("act node").is_some());
    let refused = engine
        .actuate(
            ActuationBatchV1::new(
                "slice135-capability-refused",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    logical_id: Some("refused-node".into()),
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("refused-node-r1").expect("refused revision"),
                        SourceVersionId::new("refused-source-v1").expect("refused source version"),
                    ),
                    ..act_node
                })],
            )
            .expect("refusal batch")
            .with_expected_write_boundary(0),
        )
        .expect("refusal receipt");
    assert_eq!(refused.outcome, ActuationOutcomeV1::Refused);
    assert!(read::get(engine, "refused-node", None).expect("refused absence").is_none());
    observed(
        "engine.actuate",
        "committed receipt replay identical; stale boundary refused without row",
    );
    if let (Some(cursor), Some(generation_id)) = (
        first_receipt.pending_projection_write_cursors.first(),
        first_receipt.projection_generation_id.clone(),
    ) {
        let status = read::mutation_projection_status(
            engine,
            MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: first_receipt.operation_id.clone(),
                write_cursor: *cursor,
                expected_generation_id: generation_id.clone(),
            },
        )
        .expect("mutation projection status");
        assert_eq!(status.operation_id, first_receipt.operation_id);
        assert_eq!(status.generation_id, generation_id);
        observed(
            "read.mutation_projection_status",
            "actuation receipt cursor resolves to generation",
        );
    } else {
        observed(
            "read.mutation_projection_status",
            "gap: no pending projection cursor on actuation receipt",
        );
    }

    engine
        .write(&[node("lifecycle-node", "temporary record", "slice135-lifecycle")])
        .expect("lifecycle seed");
    engine
        .transition("lifecycle-node", LifecycleState::Deleted, Some("capability exercise"))
        .expect("transition");
    assert!(read::get(engine, "lifecycle-node", None).expect("deleted read").is_none());
    observed("engine.transition", "deleted lifecycle node excluded from current read");
    engine.purge("lifecycle-node").expect("purge");
    assert!(read::get(engine, "lifecycle-node", None).expect("purged read").is_none());
    observed("engine.purge", "deleted node purged from current read");

    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "time-window".into(),
            source_id: SourceId::new("slice135-window").expect("window source"),
            logical_id: Some("window-node".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(100),
            valid_until: Some(200),
        }])
        .expect("window write");
    let crossings = read::crossed_boundary_since(engine, 0, None).expect("crossings");
    assert!(!crossings.is_empty());
    observed("read.crossed_boundary_since", "written validity window produced boundaries");

    let reranked = rerank(
        "query",
        &[RerankPassage { id: 7, body: "answer".into(), score: 0.5 }],
        0,
        RerankOptions::default(),
    )
    .expect("identity rerank");
    assert_eq!(reranked.len(), 1);
    assert_eq!(reranked[0].id, 7);
    assert_eq!(reranked[0].ce_score, None);
    observed("rerank", "identity path exercised; real cross-encoder qualification unavailable");

    closure_id
}

fn main() {
    let path = env::args().nth(1).expect("database path argument");
    let options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
    let engine = Engine::open(&path, options.clone()).expect("open");
    assert_eq!(engine.open_report().default_embedder.name, "fathomdb-bge-small-en-v1.5");
    observed("engine.open", "real fresh SQLite with default embedder");
    let mut corpus = vec![
        node("A", "harbor lantern ferry timetable brightblue", "slice135-rust-corpus"),
        node("B", "orchard apple harvest calendar redgreen", "slice135-rust-corpus"),
    ];
    for index in 0..30 {
        corpus.push(node(
            &format!("F{index:04}"),
            &format!("neutral archive ledger inventory entry {index} papers"),
            "slice135-rust-corpus",
        ));
    }
    corpus.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: SOURCE_BODY.into(),
        source_id: SourceId::new("slice135-rust-graph").expect("source id"),
        logical_id: Some("s02-source".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("s02-source-r1").expect("revision"),
            SourceVersionId::new("s02-source-v1").expect("version"),
        ),
    }));
    corpus.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "{\"summary\": \"s02 graph root\"}".into(),
        source_id: SourceId::new("slice135-rust-graph").expect("source id"),
        logical_id: Some("s02-root".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: derived("s02-root-r1"),
    }));
    corpus.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("{{\"summary\": \"{CLAIM_TOKEN}\"}}"),
        source_id: SourceId::new("slice135-rust-graph").expect("source id"),
        logical_id: Some("s02-claim".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: derived("s02-claim-r1"),
    }));
    corpus.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: "s02-root".into(),
        to: "s02-claim".into(),
        source_id: SourceId::new("slice135-rust-graph").expect("source id"),
        logical_id: Some("s02-edge".into()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: derived("s02-edge-r1"),
    }));
    engine.write(&corpus).expect("write");
    observed("engine.write", "32 corpus and 4 provenanced graph rows committed");
    let spec = ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    let delta = engine.configure_projections(&[spec], &[]).expect("projection");
    assert!(delta.vector_unsupported_kinds.is_empty());
    observed("engine.configure_projections", "declared searchable summary FTS and vector");
    engine.drain(120_000).expect("drain");
    let projection = read::projections(&engine)
        .expect("read projections")
        .into_iter()
        .find(|item| item.name == "summary")
        .expect("summary projection");
    assert_eq!(
        projection.vector.expect("vector projection").dense_readiness,
        Some(DenseReadiness::Ready)
    );
    observed("read.projections", "declared vector projection reports ready after drain");
    let anchor = read::get(&engine, "A", None).expect("read A").expect("A present");
    assert_eq!(anchor.logical_id, "A");
    assert_eq!(anchor.body, "{\"summary\": \"harbor lantern ferry timetable brightblue\"}");
    observed("read.get", "anchor body matches write");
    let text = engine.search_text_only("brightblue", TextSearchOptions::default()).expect("text");
    assert_eq!(text.results.len(), 1);
    assert_eq!(text.results[0].id.value, "A");
    assert_eq!(text.results[0].branch, SoftFallbackBranch::Text);
    observed("engine.search_text_only", "lexical branch finds anchor; invalid NUL refused");
    assert!(engine
        .search_text_only("boat schedule across water", TextSearchOptions::default())
        .expect("vector lexical control")
        .results
        .is_empty());
    let vector =
        engine.search("boat schedule across water", SearchOptions::default()).expect("vector");
    assert!(vector.results.iter().any(|hit| hit.branch == SoftFallbackBranch::Vector));
    let hybrid = engine.search("harbor ferry", SearchOptions::default()).expect("hybrid");
    assert!(hybrid.results.iter().any(|hit| hit.id.value == "A"));
    observed("engine.search", "vector branch and hybrid anchor; zero limit refused");
    let invalid_limit = engine
        .search("harbor ferry", SearchOptions { limit: 0, ..SearchOptions::default() })
        .expect_err("zero limit must fail");
    assert_eq!(invalid_limit.kind(), ErrorKind::InvalidArgument);
    let invalid_text = engine
        .search_text_only("a\0b", TextSearchOptions::default())
        .expect_err("embedded NUL must fail");
    assert_eq!(invalid_text.kind(), ErrorKind::WriteValidation);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default())
                .expect("read context"),
        )
        .expect("freeze");
    observed("engine.freeze_read_context", "frozen context created for evidence and graph");
    let evidence = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: CLAIM_TOKEN.into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: false,
            limit: 1,
        })
        .expect("evidence search");
    assert_eq!(evidence.evidence.len(), 1);
    observed("engine.search_with_evidence", "single source-bound evidence reference");
    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence.evidence[0].evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve evidence");
    assert_eq!(resolved.logical_id.as_deref(), Some("s02-claim"));
    assert_eq!(resolved.canonical_source_body, SOURCE_BODY);
    observed("engine.resolve_evidence", "reference resolves exact canonical source body");
    let expanded = graph::expand(
        &engine,
        &GraphExpandRequestV1 {
            schema_version: 1,
            seed: GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("s02-root")],
            },
            direction: TraversalDirection::Outgoing,
            edge_kinds: vec!["supports".into()],
            target_kinds: vec!["doc".into()],
            context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            max_depth: 1,
            result_limit: 1,
            max_work_units: 10,
            include_explanation: false,
            include_evidence: true,
        },
    )
    .expect("graph expand");
    assert_eq!(expanded.targets.len(), 1);
    observed("graph.expand", "bounded frozen expansion reaches one target");
    let graph_evidence = expanded.evidence.expect("graph evidence sidecar");
    let entry = &graph_evidence.entries[0];
    let target = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve graph target");
    assert_eq!(target.artifact_revision_id.as_str(), "s02-claim-r1");
    assert_eq!(target.canonical_source_body, SOURCE_BODY);
    let edge = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.terminal_edge_evidence_ref.clone(),
            context: frozen,
        })
        .expect("resolve graph edge");
    assert!(matches!(edge.artifact, GraphEvidenceArtifactV1::Edge {
        ref from, ref to, ..
    } if from == "s02-root" && to == "s02-claim"));
    observed("engine.resolve_graph_evidence", "node and edge evidence resolve to source");
    let near =
        graph::neighbors(&engine, "s02-root", 1, NeighborsOptions::default()).expect("neighbors");
    assert!(near.iter().any(|item| item.logical_id == "s02-claim"));
    observed("graph.neighbors", "outgoing graph neighbor contains claim");
    let closure_id = exercise_more(&engine);
    let erased = engine.erase_source("slice135-rust-graph").expect("erase");
    assert_eq!((erased.nodes_excised, erased.edges_excised), (3, 1));
    assert_eq!(engine.erase_source("slice135-rust-graph").expect("erase again").nodes_excised, 0);
    observed("engine.erase_source", "3 nodes and one edge removed; repeat idempotent");
    assert!(read::get(&engine, "s02-root", None).expect("read erased root").is_none());
    assert!(read::get(&engine, "A", None).expect("read retained A").is_some());
    engine.close().expect("close");
    observed("engine.close", "closed before durable reopen");
    let reopened = Engine::open(&path, options).expect("reopen");
    assert!(read::get(&reopened, "s02-root", None).expect("reopened root").is_none());
    assert!(graph::neighbors(&reopened, "s02-root", 1, NeighborsOptions::default())
        .expect("reopened neighbors")
        .is_empty());
    assert!(read::get(&reopened, "A", None).expect("reopened A").is_some());
    assert!(read::get(&reopened, "act-node", None).expect("reopened act node").is_some());
    assert!(read::get(&reopened, "refused-node", None).expect("reopened refusal").is_none());
    assert_eq!(
        read::operational_state(&reopened, "settings", "second", None)
            .expect("reopened state")
            .expect("second state")
            .payload,
        r#"{"value":2}"#
    );
    let closure = reopened
        .read_dependency_closure(ClosureLookupV1::new(closure_id).expect("reopened closure lookup"))
        .expect("reopened closure")
        .expect("durable closure");
    assert_eq!(closure.phase, ClosurePhaseV1::Complete);
    reopened.close().expect("reopened close");
    observed("engine.ingest_with_extractor", "unavailable: provider/model qualification");
    observed("engine.consolidate_with_provider", "unavailable: provider/model qualification");
    println!("RUST_SDK_S02_FUNCTIONAL_OK corpus=32 graph_erased_nodes=3 graph_erased_edges=1 evidence=true reopened=true");
}
