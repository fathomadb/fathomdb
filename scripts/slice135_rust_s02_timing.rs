use fathomdb_sdk::{
    graph, read, ArtifactRevisionId, CanonicalHash, DenseReadiness, Engine, ErrorKind,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, GraphEvidenceArtifactV1,
    GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, NeighborsOptions, OpenOptions, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1,
    ReadView, SearchFilter, SearchOptions, SoftFallbackBranch, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, TextSearchOptions, TraversalDirection, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::time::Instant;

const SOURCE_BODY: &str = "{\"summary\": \"s02 canonical source evidence bytes\"}";
const CLAIM_TOKEN: &str = "slice135s02evidenceneedle";
const SOURCE_SHA: &str = "3f29d649d0213e595c0dab251a449d92fd625792";

macro_rules! timed {
    ($stages:expr, $name:literal, $call:expr) => {{
        let started = Instant::now();
        let result = $call;
        $stages.insert($name, started.elapsed().as_nanos());
        result
    }};
}

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

fn main() {
    let path = env::args().nth(1).expect("database path argument");
    let options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
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
    let mut stages = BTreeMap::<&str, u128>::new();
    let whole_start = Instant::now();
    let engine = timed!(stages, "open", Engine::open(&path, options.clone()).expect("open"));
    assert_eq!(engine.open_report().default_embedder.name, "fathomdb-bge-small-en-v1.5");
    timed!(stages, "write", engine.write(&corpus).expect("write"));
    let spec = ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    let delta = timed!(
        stages,
        "configure_projection",
        engine.configure_projections(&[spec], &[]).expect("projection")
    );
    assert!(delta.vector_unsupported_kinds.is_empty());
    timed!(stages, "drain", engine.drain(120_000).expect("drain"));
    let projection = read::projections(&engine)
        .expect("read projections")
        .into_iter()
        .find(|item| item.name == "summary")
        .expect("summary projection");
    assert_eq!(
        projection.vector.expect("vector projection").dense_readiness,
        Some(DenseReadiness::Ready)
    );
    let anchor = read::get(&engine, "A", None).expect("read A").expect("A present");
    assert_eq!(anchor.logical_id, "A");
    assert_eq!(anchor.body, "{\"summary\": \"harbor lantern ferry timetable brightblue\"}");
    let text = timed!(
        stages,
        "text",
        engine.search_text_only("brightblue", TextSearchOptions::default()).expect("text")
    );
    assert_eq!(text.results.len(), 1);
    assert_eq!(text.results[0].id.value, "A");
    assert_eq!(text.results[0].branch, SoftFallbackBranch::Text);
    assert!(timed!(
        stages,
        "vector_lexical_control",
        engine
            .search_text_only("boat schedule across water", TextSearchOptions::default())
            .expect("vector lexical control")
    )
    .results
    .is_empty());
    let vector = timed!(
        stages,
        "vector",
        engine.search("boat schedule across water", SearchOptions::default()).expect("vector")
    );
    assert!(vector.results.iter().any(|hit| hit.branch == SoftFallbackBranch::Vector));
    let hybrid = timed!(
        stages,
        "hybrid",
        engine.search("harbor ferry", SearchOptions::default()).expect("hybrid")
    );
    assert!(hybrid.results.iter().any(|hit| hit.id.value == "A"));
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
    let evidence = timed!(
        stages,
        "evidence_search",
        engine
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
            .expect("evidence search")
    );
    assert_eq!(evidence.evidence.len(), 1);
    let resolved = timed!(
        stages,
        "evidence_resolve",
        engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: evidence.evidence[0].evidence_ref.clone(),
                context: frozen.clone(),
            })
            .expect("resolve evidence")
    );
    assert_eq!(resolved.logical_id.as_deref(), Some("s02-claim"));
    assert_eq!(resolved.canonical_source_body, SOURCE_BODY);
    let expanded = timed!(
        stages,
        "graph_expand",
        graph::expand(
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
            }
        )
        .expect("graph expand")
    );
    assert_eq!(expanded.targets.len(), 1);
    let graph_evidence = expanded.evidence.expect("graph evidence sidecar");
    let entry = &graph_evidence.entries[0];
    let target = timed!(
        stages,
        "graph_target_resolve",
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.target_evidence_ref.clone(),
                context: frozen.clone(),
            })
            .expect("resolve graph target")
    );
    assert_eq!(target.artifact_revision_id.as_str(), "s02-claim-r1");
    assert_eq!(target.canonical_source_body, SOURCE_BODY);
    let edge = timed!(
        stages,
        "graph_edge_resolve",
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.terminal_edge_evidence_ref.clone(),
                context: frozen,
            })
            .expect("resolve graph edge")
    );
    assert!(matches!(edge.artifact, GraphEvidenceArtifactV1::Edge {
        ref from, ref to, ..
    } if from == "s02-root" && to == "s02-claim"));
    let near = timed!(
        stages,
        "graph_neighbors",
        graph::neighbors(&engine, "s02-root", 1, NeighborsOptions::default()).expect("neighbors")
    );
    assert!(near.iter().any(|item| item.logical_id == "s02-claim"));
    let erased =
        timed!(stages, "erase", engine.erase_source("slice135-rust-graph").expect("erase"));
    assert_eq!((erased.nodes_excised, erased.edges_excised), (3, 1));
    assert_eq!(
        timed!(
            stages,
            "erase_again",
            engine.erase_source("slice135-rust-graph").expect("erase again")
        )
        .nodes_excised,
        0
    );
    assert!(read::get(&engine, "s02-root", None).expect("read erased root").is_none());
    assert!(read::get(&engine, "A", None).expect("read retained A").is_some());
    timed!(stages, "close", engine.close().expect("close"));
    let reopened = timed!(stages, "reopen", Engine::open(&path, options).expect("reopen"));
    let reopened_graph_absent =
        read::get(&reopened, "s02-root", None).expect("reopened root").is_none()
            && graph::neighbors(&reopened, "s02-root", 1, NeighborsOptions::default())
                .expect("reopened neighbors")
                .is_empty();
    assert!(reopened_graph_absent);
    let reopened_anchor_retained = read::get(&reopened, "A", None).expect("reopened A").is_some();
    assert!(reopened_anchor_retained);
    timed!(stages, "reopened_close", reopened.close().expect("reopened close"));
    let whole_product_ns = whole_start.elapsed().as_nanos();
    let receipt = serde_json::json!({
        "schema_version": 1,
        "status": "TIMED_RUST_SDK_S02_FUNCTIONAL_OK",
        "source_sha": SOURCE_SHA,
        "whole_product_ns": whole_product_ns,
        "stage_ns": stages,
        "observed": {
            "corpus_nodes": 32,
            "erased_graph_nodes": erased.nodes_excised,
            "erased_graph_edges": erased.edges_excised,
            "reopened_graph_absent": reopened_graph_absent,
            "reopened_anchor_retained": reopened_anchor_retained,
            "evidence_resolved": true,
            "projection_ready": true,
        },
    });
    println!("SLICE135_RUST_S02 {receipt}");
}
