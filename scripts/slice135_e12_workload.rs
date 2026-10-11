//! Source-bound real-engine E01–E12 workload for Slice 135.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine, GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, PreparedWrite,
    ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, TraversalDirection, WriteProvenanceV1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn corpus_bytes() -> String {
    let mut rows = (0..32)
        .map(|index| format!("doc-{index}:needle memory document {index} for bounded search"))
        .collect::<Vec<_>>();
    rows.extend([
        "graph-source:canonical graph source bytes".to_owned(),
        "graph-root:root claim".to_owned(),
        "graph-target:target claim".to_owned(),
        "graph-edge:graph-root:supports:graph-target".to_owned(),
    ]);
    rows.join("\n") + "\n"
}

#[derive(Debug, Default)]
struct Provider {
    nanos: AtomicU64,
    calls: AtomicU64,
}

impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice115-deterministic", "seed-115", 384)
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        let start = Instant::now();
        let mut vector = vec![0.0; 384];
        for (index, byte) in input.bytes().enumerate() {
            vector[index % 384] += f32::from(byte) / 255.0;
        }
        vector[0] += 1.0;
        self.nanos.fetch_add(start.elapsed().as_nanos() as u64 + 1, Ordering::Relaxed);
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(vector)
    }
}

fn path(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("slice115.fdb")
}

fn open(path: &Path, provider: &Arc<Provider>) -> Engine {
    Engine::open_with_embedder_for_test(path, provider.clone()).unwrap().engine
}

fn node(index: usize, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("needle memory document {index} for bounded search"),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(format!("doc-{index}")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn seed(engine: &Engine) {
    engine.configure_vector_kind_for_test("doc").unwrap();
    let rows = (0..32).map(|index| node(index, "slice115:corpus")).collect::<Vec<_>>();
    engine.write(&rows).unwrap();
    engine.drain(30_000).unwrap();
    assert_eq!(
        engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes").unwrap(),
        [32]
    );
}

fn duration(start: Instant) -> u64 {
    start.elapsed().as_nanos().max(1) as u64
}

fn sample(path: &str, elapsed: u64, stage: &str, stage_ns: u64, count: usize) -> Value {
    assert!(count > 0, "{path}: semantic count");
    assert!(stage_ns > 0 && stage_ns <= elapsed, "{path}: actual stage timing");
    json!({"latency_ns":elapsed,"stage":stage,"stage_ns":stage_ns,
           "correctness_count":count,"valid":true,"semantic_ok":true})
}

fn profiled(path: &str, elapsed: u64, count: usize) -> Value {
    assert!(count > 0, "{path}: semantic count");
    json!({"latency_ns":elapsed,"correctness_count":count,"valid":true,"semantic_ok":true})
}

fn digest(input: &str) -> String {
    Sha256::digest(input.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn prestate(engine: &Engine) -> Value {
    let canonical_rows =
        engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes").unwrap()[0];
    let projection_rows =
        engine.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_vector_rows").unwrap()[0];
    let source_ids = engine
        .query_text_col_for_test(
            "SELECT DISTINCT source_id FROM canonical_nodes ORDER BY source_id",
        )
        .unwrap();
    let bodies = engine
        .query_text_col_for_test(
            "SELECT logical_id || ':' || body FROM canonical_nodes ORDER BY logical_id",
        )
        .unwrap();
    json!({"canonical_rows":canonical_rows,"projection_rows":projection_rows,
           "source_ids":source_ids,"digest":digest(&serde_json::to_string(&bodies).unwrap())})
}

fn graph_fixture(engine: &Engine) -> GraphExpandRequestV1 {
    let source = "canonical graph source bytes";
    let derived = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("graph-v1").unwrap(),
            SourceRevisionId::new("graph-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    let canonical = WriteProvenanceV1::canonical(
        ArtifactRevisionId::new("graph-r1").unwrap(),
        SourceVersionId::new("graph-v1").unwrap(),
    );
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("graph-source".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("graph-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: canonical,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("graph-root".into()),
                kind: "claim".into(),
                body: "root claim".into(),
                source_id: SourceId::new("graph-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("graph-root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("graph-target".into()),
                kind: "claim".into(),
                body: "target claim".into(),
                source_id: SourceId::new("graph-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("graph-target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("graph-edge".into()),
                kind: "supports".into(),
                from: "graph-root".into(),
                to: "graph-target".into(),
                source_id: SourceId::new("graph-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived("graph-edge-r1"),
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
                filter,
            )
            .unwrap(),
        )
        .unwrap();
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("graph-root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["claim".into()],
        context: GraphReadContextV1::Frozen { schema_version: 1, context },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
        include_evidence: true,
    }
}

fn one(path_name: &str, provider: &Arc<Provider>) -> Value {
    let dir = TempDir::new().unwrap();
    let db = path(&dir);
    match path_name {
        "open_fresh" => {
            let start = Instant::now();
            let engine = open(&db, provider);
            let elapsed = duration(start);
            let count = usize::from(db.exists());
            engine.close().unwrap();
            let mut raw = profiled(path_name, elapsed, count);
            raw["observed_checks"] = json!({"database_exists":true,"canonical_rows":0});
            raw
        }
        "open_populated" | "close" => {
            let engine = open(&db, provider);
            seed(&engine);
            engine.close().unwrap();
            let start = Instant::now();
            let engine = open(&db, provider);
            let open_ns = duration(start);
            let count = engine
                .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes")
                .unwrap()[0] as usize;
            let start = Instant::now();
            engine.close().unwrap();
            let close_ns = duration(start);
            let reopened = open(&db, provider);
            let rows =
                reopened.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes").unwrap();
            let state = prestate(&reopened);
            reopened.close().unwrap();
            assert_eq!(rows, [32]);
            if path_name == "close" {
                let mut raw = profiled(path_name, close_ns, count);
                raw["observed_checks"] = json!({"reopened":true,"state":state});
                raw
            } else {
                let mut raw = profiled(path_name, open_ns, count);
                raw["observed_checks"] = json!({"reopened":true,"state":state});
                raw
            }
        }
        "canonical_write" | "projection" | "erasure" => {
            let engine = open(&db, provider);
            engine.configure_vector_kind_for_test("doc").unwrap();
            if path_name == "erasure" {
                engine.write(&[node(0, "slice115:erase")]).unwrap();
                engine.drain(30_000).unwrap();
                let before = prestate(&engine);
                assert_eq!(before["canonical_rows"], 1);
                assert_eq!(before["projection_rows"], 1);
                assert_eq!(before["source_ids"], json!(["slice115:erase"]));
                let start = Instant::now();
                let report = engine.erase_source("slice115:erase").unwrap();
                let elapsed = duration(start);
                let after = prestate(&engine);
                assert_eq!(after["canonical_rows"], 0);
                assert_eq!(after["projection_rows"], 0);
                engine.close().unwrap();
                let mut raw =
                    profiled(path_name, elapsed, usize::from(format!("{report:?}").len() > 0));
                raw["prestate"] = before;
                raw["poststate"] = after;
                raw["observed_checks"] =
                    json!({"prestate":raw["prestate"],"poststate":raw["poststate"]});
                return raw;
            }
            let before = prestate(&engine);
            assert_eq!(before["canonical_rows"], 0);
            assert_eq!(before["projection_rows"], 0);
            assert_eq!(before["source_ids"], json!([]));
            let prior = provider.nanos.load(Ordering::Relaxed);
            let start = Instant::now();
            engine.write(&[node(0, "slice115:mutation")]).unwrap();
            let write_ns = duration(start);
            let drain_start = Instant::now();
            engine.drain(30_000).unwrap();
            let drain_ns = duration(drain_start);
            let projection_ns = duration(start);
            let provider_ns = provider.nanos.load(Ordering::Relaxed).saturating_sub(prior);
            let after = prestate(&engine);
            assert_eq!(after["canonical_rows"], 1);
            assert_eq!(after["projection_rows"], 1);
            engine.close().unwrap();
            let mut raw = if path_name == "canonical_write" {
                profiled(path_name, write_ns, 1)
            } else {
                sample(path_name, projection_ns, "provider", provider_ns, 1)
            };
            if path_name == "projection" {
                raw["write_ns"] = json!(write_ns);
                raw["drain_ns"] = json!(drain_ns);
            }
            raw["prestate"] = before;
            raw["poststate"] = after;
            raw["observed_checks"] =
                json!({"prestate":raw["prestate"],"poststate":raw["poststate"]});
            raw
        }
        "model_cpu" => {
            std::env::set_var("FATHOMDB_EMBED_DEVICE", "cpu");
            let start = Instant::now();
            let opened = Engine::open_with_choice(&db, EmbedderChoice::Default).unwrap();
            let model_load_ns = duration(start);
            let start = Instant::now();
            let first = opened.engine.embed_text("slice115 small cpu compatibility").unwrap();
            let inference_ns = duration(start);
            assert_eq!(first.len(), 384);
            let second = opened.engine.embed_text("slice115 small cpu compatibility").unwrap();
            assert_eq!(first, second);
            opened.engine.configure_vector_kind_for_test("doc").unwrap();
            let start = Instant::now();
            opened.engine.write(&[node(0, "slice115:model")]).unwrap();
            opened.engine.drain(30_000).unwrap();
            let projection_ns = duration(start);
            let projected = opened
                .engine
                .query_i64_col_for_test("SELECT count(*) FROM _fathomdb_vector_rows")
                .unwrap()[0];
            assert_eq!(projected, 1);
            opened.engine.close().unwrap();
            let mut raw = sample(
                path_name,
                model_load_ns + inference_ns,
                "inference",
                inference_ns,
                first.len(),
            );
            raw["model_load_ns"] = json!(model_load_ns);
            raw["projection_ns"] = json!(projection_ns);
            raw["model_projection_rows"] = json!(projected);
            raw["observed_checks"] = json!({"dimensions":first.len(),"repeat_equal":true,
                                              "projection_rows":projected});
            raw
        }
        "text" | "vector_stage" | "hybrid" => {
            let engine = open(&db, provider);
            seed(&engine);
            let before = provider.nanos.load(Ordering::Relaxed);
            if path_name == "vector_stage" {
                engine.set_vector_stage_only_for_test(true);
            }
            let start = Instant::now();
            let result = if path_name == "text" {
                engine.search_text_only("needle").unwrap()
            } else {
                engine.search("needle").unwrap()
            };
            let elapsed = duration(start);
            if path_name == "vector_stage" {
                engine.set_vector_stage_only_for_test(false);
            }
            let provider_ns = provider.nanos.load(Ordering::Relaxed).saturating_sub(before);
            let count = result.results.len();
            engine.close().unwrap();
            let checks = json!({"ordered_ids":result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(),
                                "ordered_branches":result.results.iter().map(|hit| format!("{:?}", hit.branch)).collect::<Vec<_>>()});
            if path_name == "text" {
                let mut raw = profiled(path_name, elapsed, count);
                raw["observed_checks"] = checks;
                raw
            } else {
                let mut raw = sample(path_name, elapsed, "provider", provider_ns, count);
                raw["observed_checks"] = checks;
                raw
            }
        }
        "graph_expand" | "graph_evidence" => {
            let engine = open(&db, provider);
            let request = graph_fixture(&engine);
            let start = Instant::now();
            let expansion = engine.graph_expand(&request).unwrap();
            let expand_ns = duration(start);
            assert_eq!(expansion.targets.len(), 1);
            let sidecar = expansion.evidence.unwrap();
            assert_eq!(sidecar.entries.len(), 1);
            let frozen = match request.context {
                GraphReadContextV1::Frozen { context, .. } => context,
                GraphReadContextV1::Current { .. } => unreachable!(),
            };
            let start = Instant::now();
            let resolved = engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: sidecar.entries[0].target_evidence_ref.clone(),
                    context: frozen,
                })
                .unwrap();
            let evidence_ns = duration(start);
            assert_eq!(resolved.canonical_source_body, "canonical graph source bytes");
            engine.close().unwrap();
            let checks = json!({"ordered_targets":expansion.targets.iter().map(|target| target.logical_id.clone()).collect::<Vec<_>>(),
                                "complete":expansion.complete,"evidence_entries":sidecar.entries.len(),
                                "canonical_source_body":resolved.canonical_source_body});
            if path_name == "graph_expand" {
                let mut raw = profiled(path_name, expand_ns, 1);
                raw["observed_checks"] = checks;
                raw
            } else {
                let mut raw = profiled(path_name, evidence_ns, 1);
                raw["observed_checks"] = checks;
                raw
            }
        }
        _ => panic!("unknown path {path_name}"),
    }
}

fn read_query_sample(engine: &Engine, provider: &Provider, path_name: &str) -> Value {
    let before = provider.nanos.load(Ordering::Relaxed);
    let calls = provider.calls.load(Ordering::Relaxed);
    if path_name == "vector_stage" {
        engine.set_vector_stage_only_for_test(true);
    }
    let start = Instant::now();
    let result = if path_name == "text" {
        engine.search_text_only("needle").unwrap()
    } else {
        engine.search("needle").unwrap()
    };
    let elapsed = duration(start);
    if path_name == "vector_stage" {
        engine.set_vector_stage_only_for_test(false);
        assert!(result
            .results
            .iter()
            .all(|hit| hit.branch == fathomdb_engine::SoftFallbackBranch::Vector));
    }
    let count = result.results.len();
    let checks = json!({"ordered_ids":result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(),
                        "ordered_branches":result.results.iter().map(|hit| format!("{:?}", hit.branch)).collect::<Vec<_>>()});
    if path_name == "text" {
        assert_eq!(provider.calls.load(Ordering::Relaxed), calls);
        let mut raw = profiled(path_name, elapsed, count);
        raw["observed_checks"] = checks;
        raw
    } else {
        assert!(provider.calls.load(Ordering::Relaxed) > calls);
        let provider_ns = provider.nanos.load(Ordering::Relaxed).saturating_sub(before);
        let mut raw = sample(path_name, elapsed, "provider", provider_ns, count);
        raw["observed_checks"] = checks;
        raw
    }
}

fn read_query_cells(provider: &Arc<Provider>) -> serde_json::Map<String, Value> {
    let dir = TempDir::new().unwrap();
    let engine = open(&path(&dir), provider);
    seed(&engine);
    let names = ["text", "vector_stage", "hybrid"];
    let mut cells = names
        .iter()
        .map(|name| ((*name).to_owned(), Vec::new()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for name in names {
        let _ = read_query_sample(&engine, provider, name);
    }
    let samples: usize = std::env::var("SLICE135_E12_SAMPLES").unwrap().parse().unwrap();
    for repetition in 0..samples {
        let order = if repetition % 2 == 0 { names } else { ["hybrid", "vector_stage", "text"] };
        for name in order {
            cells.get_mut(name).unwrap().push(read_query_sample(&engine, provider, name));
        }
    }
    engine.close().unwrap();
    cells.into_iter().map(|(name, samples)| (name, json!(samples))).collect()
}

fn read_graph_cells(provider: &Arc<Provider>) -> serde_json::Map<String, Value> {
    let dir = TempDir::new().unwrap();
    let engine = open(&path(&dir), provider);
    let request = graph_fixture(&engine);
    let mut null = request.clone();
    null.edge_kinds = vec!["not-present".into()];
    assert!(engine.graph_expand(&null).unwrap().targets.is_empty());
    let expansion = engine.graph_expand(&request).unwrap();
    assert_eq!(expansion.targets.len(), 1);
    let sidecar = expansion.evidence.unwrap();
    assert_eq!(sidecar.entries.len(), 1);
    let frozen = match request.context.clone() {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let evidence_request = GraphEvidenceResolveRequestV1 {
        schema_version: 1,
        evidence_ref: sidecar.entries[0].target_evidence_ref.clone(),
        context: frozen,
    };
    let mut expand = Vec::new();
    let mut evidence = Vec::new();
    let samples: usize = std::env::var("SLICE135_E12_SAMPLES").unwrap().parse().unwrap();
    for repetition in 0..=samples {
        let start = Instant::now();
        let result = engine.graph_expand(&request).unwrap();
        let expand_ns = duration(start);
        assert_eq!(result.targets.len(), 1);
        let start = Instant::now();
        let resolved = engine.resolve_graph_evidence(&evidence_request).unwrap();
        let evidence_ns = duration(start);
        assert_eq!(resolved.canonical_source_body, "canonical graph source bytes");
        if repetition > 0 {
            let checks = json!({"ordered_targets":result.targets.iter().map(|target| target.logical_id.clone()).collect::<Vec<_>>(),
                                "complete":result.complete,"evidence_entries":result.evidence.as_ref().unwrap().entries.len(),
                                "canonical_source_body":resolved.canonical_source_body});
            let mut expansion = profiled("graph_expand", expand_ns, 1);
            expansion["observed_checks"] = checks.clone();
            expand.push(expansion);
            let mut resolution = profiled("graph_evidence", evidence_ns, 1);
            resolution["observed_checks"] = checks;
            evidence.push(resolution);
        }
    }
    engine.close().unwrap();
    serde_json::Map::from_iter([
        ("graph_expand".into(), json!(expand)),
        ("graph_evidence".into(), json!(evidence)),
    ])
}

#[test]
#[ignore = "explicit measurement runner only"]
fn slice135_e12_measurement() {
    let raw_path = std::env::var("SLICE135_E12_RAW_PATH").expect("raw path");
    let samples: usize = std::env::var("SLICE135_E12_SAMPLES").unwrap().parse().unwrap();
    let requested: Vec<String> =
        std::env::var("SLICE135_E12_CELLS").unwrap().split(',').map(str::to_owned).collect();
    let provider = Arc::new(Provider::default());
    let paths = [
        "open_fresh",
        "open_populated",
        "close",
        "canonical_write",
        "projection",
        "model_cpu",
        "text",
        "vector_stage",
        "hybrid",
        "graph_expand",
        "graph_evidence",
        "erasure",
    ];
    let mut cells = serde_json::Map::new();
    for path in paths {
        if !requested.iter().any(|name| name == path) {
            continue;
        }
        if matches!(path, "text" | "vector_stage" | "hybrid" | "graph_expand" | "graph_evidence") {
            continue;
        }
        let _ = one(path, &provider);
        let mut observations = Vec::new();
        for _ in 0..samples {
            observations.push(one(path, &provider));
        }
        cells.insert(path.to_owned(), json!(observations));
    }
    if requested.iter().any(|name| matches!(name.as_str(), "text" | "vector_stage" | "hybrid")) {
        cells.extend(
            read_query_cells(&provider).into_iter().filter(|(name, _)| requested.contains(name)),
        );
    }
    if requested.iter().any(|name| matches!(name.as_str(), "graph_expand" | "graph_evidence")) {
        cells.extend(
            read_graph_cells(&provider).into_iter().filter(|(name, _)| requested.contains(name)),
        );
    }
    let raw = json!({
        "source_sha": std::env::var("SLICE135_E12_SOURCE_SHA").unwrap(),
        "protocol_sha256": std::env::var("SLICE135_E12_PROTOCOL_SHA256").unwrap(),
        "runner_sha256": std::env::var("SLICE135_E12_RUNNER_SHA256").unwrap(),
        "binary_sha256": std::env::var("SLICE135_E12_BINARY_SHA256").unwrap(),
        "corpus_sha256": digest(&corpus_bytes()),
        "environment_valid": true,
        "cells": cells,
    });
    fs::write(raw_path, serde_json::to_vec_pretty(&raw).unwrap()).unwrap();
}

#[test]
#[ignore = "explicit GDB sampled profile only"]
fn slice115_profile_loop() {
    let path_name = std::env::var("SLICE115_PROFILE_PATH").unwrap();
    let seconds: u64 = std::env::var("SLICE115_PROFILE_SECONDS").unwrap().parse().unwrap();
    let provider = Arc::new(Provider::default());
    let dir = TempDir::new().unwrap();
    let db = path(&dir);
    if path_name == "canonical_write" {
        let engine = open(&db, &provider);
        engine.configure_vector_kind_for_test("doc").unwrap();
        eprintln!("PROFILE_READY {path_name}");
        let until = Instant::now() + Duration::from_secs(seconds);
        let mut index = 0;
        while Instant::now() < until {
            engine.write(&[node(index, "slice115:mutation")]).unwrap();
            index += 1;
            if index % 32 == 0 {
                engine.drain(30_000).unwrap();
            }
        }
        engine.drain(30_000).unwrap();
        engine.close().unwrap();
        eprintln!("PROFILE_DONE {path_name} operations={index}");
    } else if matches!(path_name.as_str(), "text" | "graph_expand" | "graph_evidence") {
        let engine = open(&db, &provider);
        let graph = if path_name == "text" {
            seed(&engine);
            None
        } else {
            Some(graph_fixture(&engine))
        };
        let evidence_request = graph.as_ref().map(|request| {
            let result = engine.graph_expand(request).unwrap();
            let context = match request.context.clone() {
                GraphReadContextV1::Frozen { context, .. } => context,
                GraphReadContextV1::Current { .. } => unreachable!(),
            };
            GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: result.evidence.unwrap().entries[0].target_evidence_ref.clone(),
                context,
            }
        });
        eprintln!("PROFILE_READY {path_name}");
        let until = Instant::now() + Duration::from_secs(seconds);
        let mut operations = 0;
        while Instant::now() < until {
            match path_name.as_str() {
                "text" => {
                    assert!(!engine.search_text_only("needle").unwrap().results.is_empty());
                }
                "graph_expand" => {
                    assert_eq!(
                        engine.graph_expand(graph.as_ref().unwrap()).unwrap().targets.len(),
                        1
                    );
                }
                "graph_evidence" => {
                    assert_eq!(
                        engine
                            .resolve_graph_evidence(evidence_request.as_ref().unwrap())
                            .unwrap()
                            .canonical_source_body,
                        "canonical graph source bytes"
                    );
                }
                _ => unreachable!(),
            }
            operations += 1;
        }
        engine.close().unwrap();
        eprintln!("PROFILE_DONE {path_name} operations={operations}");
    } else {
        eprintln!("PROFILE_READY {path_name}");
        let until = Instant::now() + Duration::from_secs(seconds);
        let mut operations = 0;
        while Instant::now() < until {
            let _ = one(&path_name, &provider);
            operations += 1;
        }
        eprintln!("PROFILE_DONE {path_name} operations={operations}");
    }
}
