use fathomdb_sdk::{
    graph, read, ArtifactRevisionId, CanonicalHash, DenseReadiness, Engine,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, InitialState, NeighborsOptions, OpenOptions,
    PreparedWrite, ProjectionFts, ProjectionRole, ProjectionSpec, ProjectionVector,
    ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SearchOptions,
    SoftFallbackBranch, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    TextSearchOptions, WriteProvenanceV1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::sync::Barrier;
use std::thread;
use std::time::{Duration, Instant};

const SOURCE_SHA: &str = "3f29d649d0213e595c0dab251a449d92fd625792";
const SOURCE_BODY: &str = "{\"summary\": \"s02 canonical source evidence bytes\"}";
const CLAIM_TOKEN: &str = "slice135s02evidenceneedle";

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

fn corpus() -> Vec<PreparedWrite> {
    let mut records = vec![
        node("A", "harbor lantern ferry timetable brightblue", "slice135-rust-corpus"),
        node("B", "orchard apple harvest calendar redgreen", "slice135-rust-corpus"),
    ];
    for index in 0..30 {
        records.push(node(
            &format!("F{index:04}"),
            &format!("neutral archive ledger inventory entry {index} papers"),
            "slice135-rust-corpus",
        ));
    }
    records.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: SOURCE_BODY.into(),
        source_id: SourceId::new("slice135-rust-graph").expect("graph source"),
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
    for (id, revision, body) in [
        ("s02-root", "s02-root-r1", "{\"summary\": \"s02 graph root\"}".to_string()),
        ("s02-claim", "s02-claim-r1", format!("{{\"summary\": \"{CLAIM_TOKEN}\"}}")),
    ] {
        records.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "doc".into(),
            body,
            source_id: SourceId::new("slice135-rust-graph").expect("graph source"),
            logical_id: Some(id.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: derived(revision),
        }));
    }
    records.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: "s02-root".into(),
        to: "s02-claim".into(),
        source_id: SourceId::new("slice135-rust-graph").expect("graph source"),
        logical_id: Some("s02-edge".into()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: derived("s02-edge-r1"),
    }));
    records
}

fn overlap(writer: &[Value], reader: &[Value]) -> usize {
    reader
        .iter()
        .filter(|read| {
            writer.iter().any(|write| {
                write["start_ns"].as_u64().expect("writer start")
                    < read["end_ns"].as_u64().expect("reader end")
                    && write["end_ns"].as_u64().expect("writer end")
                        > read["start_ns"].as_u64().expect("reader start")
            })
        })
        .count()
}

fn main() {
    let path = env::args().nth(1).expect("fresh database path");
    let options = OpenOptions { use_default_embedder: true, ..OpenOptions::default() };
    let whole_start = Instant::now();
    let engine = Engine::open(&path, options.clone()).expect("open");
    assert_eq!(engine.open_report().default_embedder.name, "fathomdb-bge-small-en-v1.5");
    engine.write(&corpus()).expect("seed corpus and graph");
    let spec = ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    let delta = engine.configure_projections(&[spec], &[]).expect("configure projection");
    assert!(delta.vector_unsupported_kinds.is_empty());
    engine.drain(120_000).expect("initial projection drain");
    let gate = Barrier::new(3);
    let (writer_calls, reader_calls) = thread::scope(|scope| {
        let writer = scope.spawn(|| {
            gate.wait();
            let mut calls = Vec::new();
            for index in 0..8 {
                let logical_id = format!("contend-{index:02}");
                let started = whole_start.elapsed().as_nanos() as u64;
                engine
                    .write(&[node(
                        &logical_id,
                        &format!("contended {index:02}"),
                        "slice135-rust-contended",
                    )])
                    .expect("concurrent write");
                let ended = whole_start.elapsed().as_nanos() as u64;
                calls.push(json!({
                    "logical_id": logical_id, "start_ns": started, "end_ns": ended,
                }));
                thread::sleep(Duration::from_millis(15));
            }
            calls
        });
        let reader = scope.spawn(|| {
            gate.wait();
            let mut calls = Vec::new();
            for _ in 0..8 {
                let started = whole_start.elapsed().as_nanos() as u64;
                let anchor = read::get(&engine, "A", None).expect("concurrent anchor read");
                let anchor_ok = anchor.is_some_and(|row| {
                    row.body == "{\"summary\": \"harbor lantern ferry timetable brightblue\"}"
                });
                let text = engine
                    .search_text_only("brightblue", TextSearchOptions::default())
                    .expect("concurrent text search");
                let text_ok = text.results.len() == 1 && text.results[0].id.value == "A";
                let vector = engine
                    .search("boat schedule across water", SearchOptions::default())
                    .expect("concurrent vector search");
                let vector_ok =
                    vector.results.iter().any(|hit| hit.branch == SoftFallbackBranch::Vector);
                let neighbors =
                    graph::neighbors(&engine, "s02-root", 1, NeighborsOptions::default())
                        .expect("concurrent graph read");
                let graph_ok = neighbors.iter().any(|item| item.logical_id == "s02-claim");
                assert!(anchor_ok && text_ok && vector_ok && graph_ok);
                calls.push(json!({
                    "start_ns": started,
                    "end_ns": whole_start.elapsed().as_nanos() as u64,
                    "anchor_ok": anchor_ok,
                    "text_ok": text_ok,
                    "vector_ok": vector_ok,
                    "graph_ok": graph_ok,
                }));
            }
            calls
        });
        gate.wait();
        (writer.join().expect("writer thread"), reader.join().expect("reader thread"))
    });
    let actual_overlap = overlap(&writer_calls, &reader_calls);
    assert!(actual_overlap >= 1, "writer and reader never overlapped");
    engine.drain(120_000).expect("post-contention projection drain");
    let readiness = read::projections(&engine)
        .expect("projection state")
        .into_iter()
        .find(|item| item.name == "summary")
        .expect("summary projection")
        .vector
        .expect("vector projection")
        .dense_readiness;
    assert_eq!(readiness, Some(DenseReadiness::Ready));
    for index in 0..8 {
        assert!(read::get(&engine, &format!("contend-{index:02}"), None)
            .expect("committed writer row")
            .is_some());
    }
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default())
                .expect("read context"),
        )
        .expect("freeze");
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
    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .expect("evidence resolve");
    assert_eq!(resolved.logical_id.as_deref(), Some("s02-claim"));
    assert_eq!(resolved.canonical_source_body, SOURCE_BODY);
    let graph_erasure = engine.erase_source("slice135-rust-graph").expect("erase graph");
    let writer_erasure =
        engine.erase_source("slice135-rust-contended").expect("erase contended writes");
    assert_eq!((graph_erasure.nodes_excised, graph_erasure.edges_excised), (3, 1));
    assert_eq!(writer_erasure.nodes_excised, 8);
    engine.close().expect("close");
    let reopened = Engine::open(&path, options).expect("reopen");
    assert!(read::get(&reopened, "A", None).expect("retained anchor").is_some());
    assert!(read::get(&reopened, "contend-00", None).expect("erased writer").is_none());
    assert!(read::get(&reopened, "s02-root", None).expect("erased root").is_none());
    reopened.close().expect("reopened close");
    let whole_product_ns = whole_start.elapsed().as_nanos();
    let receipt = json!({
        "schema_version": 1,
        "status": "RUST_S02_CONTENTION_FUNCTIONAL_OK",
        "source_sha": SOURCE_SHA,
        "whole_product_ns": whole_product_ns,
        "timeline": {"writer_calls": writer_calls, "reader_calls": reader_calls},
        "observed": {
            "writer_ids": (0..8).map(|index| format!("contend-{index:02}")).collect::<Vec<_>>(),
            "reader_cycles": reader_calls.len(),
            "overlap_cycles": actual_overlap,
            "readiness": "ready",
            "evidence_resolved": true,
            "erasure": {
                "graph_nodes": graph_erasure.nodes_excised,
                "graph_edges": graph_erasure.edges_excised,
                "contended_nodes": writer_erasure.nodes_excised,
            },
            "reopened_anchor_retained": true,
            "reopened_erased_absent": true,
        },
    });
    println!("SLICE135_RUST_S02_CONTENTION {receipt}");
}
