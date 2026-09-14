//! Transient Slice 15 RED oracles for the graph-evidence decision spike.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    arm_evidence_before_resolve_return_hook_for_test, encode_graph_expand_result_v1,
    ArtifactRevisionId, CanonicalHash, Engine, EvidenceErrorReasonV1, EvidenceRefV1,
    GraphArtifactClassForTest, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceId, SourceLocator, SourceRevisionId, SourceVersionId, TraversalDirection,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use std::sync::{Arc, Barrier};
use std::thread;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source = "canonical source bytes";
    let provenance = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("z-later".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.7),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-later-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("a-winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-winner-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        filter,
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["claim".into()],
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
    };
    (directory, opened.engine, request)
}

fn fixture_many(count: usize) -> (TempDir, Engine, GraphExpandRequestV1) {
    let (directory, engine, mut request) = fixture();
    let source = "canonical source bytes";
    let mut writes = Vec::new();
    for index in 1..count {
        let provenance = |revision: String| {
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(revision).unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(source)).unwrap(),
            )
        };
        writes.push(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some(format!("target-{index:02}")),
            kind: "claim".into(),
            body: format!("target {index}"),
            source_id: SourceId::new("source-owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: provenance(format!("target-{index:02}-r1")),
        }));
        writes.push(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: Some(format!("edge-{index:02}")),
            kind: "supports".into(),
            from: "root".into(),
            to: format!("target-{index:02}"),
            source_id: SourceId::new("source-owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.8),
            extractor_model_id: None,
            temporal_fallback: None,
            provenance: provenance(format!("edge-{index:02}-r1")),
        }));
    }
    engine.write(&writes).unwrap();
    engine.drain(30_000).unwrap();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Frozen {
        schema_version: 1,
        context: engine.freeze_read_context(&context).unwrap(),
    };
    (directory, engine, request)
}

#[test]
fn treatment_off_preserves_literal_bytes_and_winning_parallel_edge() {
    let (_directory, engine, request) = fixture();
    let ordinary = engine.graph_expand(&request).unwrap();
    let bytes = encode_graph_expand_result_v1(&ordinary).unwrap();
    assert_eq!(bytes, br#"{"schemaVersion":1,"seeds":[{"schemaVersion":1,"logicalId":"root","seedOrdinal":0,"queryScore":null}],"targets":[{"schemaVersion":1,"logicalId":"target","kind":"claim","body":"target","writeCursor":"3","origin":{"schemaVersion":1,"seedLogicalId":"root","seedOrdinal":0,"predecessorLogicalId":"root","targetLogicalId":"target","hopCount":1,"terminalEdgeKind":"supports","terminalDirection":"outgoing"}}],"complete":true,"workUnits":"2","degradationCodes":[],"explanation":null}"#);
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    assert_eq!(treated.graph, ordinary);
    assert_eq!(treated.evidence.len(), 1);
    assert_eq!(treated.evidence[0].target_revision_id, "target-r1");
    assert_eq!(treated.evidence[0].terminal_edge_revision_id.as_deref(), Some("edge-winner-r1"));
}

#[test]
fn authenticated_target_and_edge_refs_resolve_intrinsic_evidence() {
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let target =
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
    let edge = engine
        .resolve_graph_evidence_for_test(
            treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
            frozen,
        )
        .unwrap();
    assert_eq!(target.artifact_class, GraphArtifactClassForTest::Node);
    assert_eq!(target.artifact_revision_id, "target-r1");
    assert_eq!(edge.artifact_class, GraphArtifactClassForTest::Edge);
    assert_eq!(edge.artifact_revision_id, "edge-winner-r1");
    assert_eq!(target.canonical_source_body, "canonical source bytes");
    assert_eq!(edge.canonical_source_body, "canonical source bytes");
}

#[test]
fn current_context_and_incomplete_provenance_refuse_atomically() {
    let (_directory, engine, mut request) = fixture();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Current { schema_version: 1, context };
    assert!(engine.graph_expand_with_graph_evidence_for_test(&request).is_err());
}

#[test]
fn preflight_is_two_indexed_class_specific_statements() {
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    assert_eq!(treated.preflight_plans.len(), 2);
    for plan in treated.preflight_plans {
        assert!(plan.contains("_fathomdb_artifact_revisions"));
        assert!(plan.contains("INDEX"), "unindexed preflight: {plan}");
    }
}

#[test]
fn restart_tamper_and_incomplete_provenance_are_fail_closed() {
    let (directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        _ => unreachable!(),
    };
    let reference = treated.evidence[0].target_ref.clone();
    engine.close().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let reopened = Engine::open(&path).unwrap().engine;
    assert_eq!(
        reopened.resolve_graph_evidence_for_test(&reference, &frozen).unwrap().artifact_revision_id,
        "target-r1"
    );
    let mut tampered = reference.as_str().as_bytes().to_vec();
    *tampered.last_mut().unwrap() = if tampered.last() == Some(&b'0') { b'1' } else { b'0' };
    let tampered = EvidenceRefV1::new(String::from_utf8(tampered).unwrap()).unwrap();
    let error = reopened.resolve_graph_evidence_for_test(&tampered, &frozen).unwrap_err();
    assert!(matches!(error, fathomdb_engine::EngineError::Evidence(ref value)
        if value.reason == EvidenceErrorReasonV1::EvidenceUnavailable));
    reopened.close().unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' \
         WHERE revision_id='target-r1'",
            [],
        )
        .unwrap();
    let incomplete = Engine::open(&path).unwrap().engine;
    assert!(incomplete.graph_expand_with_graph_evidence_for_test(&request).is_err());
}

#[test]
fn resolver_bytes_are_released_before_erasure_can_complete() {
    let (_directory, engine, request) = fixture();
    let engine = Arc::new(engine);
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        _ => unreachable!(),
    };
    let reference = treated.evidence[0].target_ref.clone();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_entered = Arc::clone(&entered);
    let hook_release = Arc::clone(&release);
    arm_evidence_before_resolve_return_hook_for_test(Box::new(move || {
        hook_entered.wait();
        hook_release.wait();
    }));
    let resolver_engine = Arc::clone(&engine);
    let resolver =
        thread::spawn(move || resolver_engine.resolve_graph_evidence_for_test(&reference, &frozen));
    entered.wait();
    let (finished_send, finished_receive) = std::sync::mpsc::sync_channel(1);
    let eraser_engine = Arc::clone(&engine);
    let eraser = thread::spawn(move || {
        let result = eraser_engine.erase_source("source-owner");
        finished_send.send(()).unwrap();
        result
    });
    assert!(finished_receive.recv_timeout(std::time::Duration::from_millis(50)).is_err());
    release.wait();
    assert_eq!(resolver.join().unwrap().unwrap().canonical_source_body, "canonical source bytes");
    let _ = eraser.join().unwrap();
    finished_receive.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
}

#[test]
#[ignore = "Slice 15 controlled release-mode measurement"]
fn measurement_matrix_emits_raw_samples() {
    use std::time::Instant;

    let (_directory, engine, request) = fixture();
    let engine = Arc::new(engine);
    for _ in 0..50 {
        engine.graph_expand(&request).unwrap();
        engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    }
    let mut control_us = Vec::with_capacity(1_000);
    let mut hydrated_us = Vec::with_capacity(1_000);
    for index in 0..1_000 {
        if index % 2 == 0 {
            let start = Instant::now();
            engine.graph_expand(&request).unwrap();
            control_us.push(start.elapsed().as_nanos() as u64 / 1_000);
            let start = Instant::now();
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
            hydrated_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        } else {
            let start = Instant::now();
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
            hydrated_us.push(start.elapsed().as_nanos() as u64 / 1_000);
            let start = Instant::now();
            engine.graph_expand(&request).unwrap();
            control_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        }
    }
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut point_node_us = Vec::with_capacity(1_000);
    let mut point_edge_us = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let start = Instant::now();
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
        point_node_us.push(start.elapsed().as_nanos() as u64 / 1_000);
        let start = Instant::now();
        engine
            .resolve_graph_evidence_for_test(
                treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
                frozen,
            )
            .unwrap();
        point_edge_us.push(start.elapsed().as_nanos() as u64 / 1_000);
    }
    let mut concurrent_control_us = Vec::with_capacity(1_600);
    let mut concurrent_hydrated_us = Vec::with_capacity(1_600);
    for hydrated in [false, true] {
        let barrier = Arc::new(Barrier::new(9));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let engine = Arc::clone(&engine);
            let request = request.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                (0..200)
                    .map(|_| {
                        let start = Instant::now();
                        if hydrated {
                            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
                        } else {
                            engine.graph_expand(&request).unwrap();
                        }
                        start.elapsed().as_nanos() as u64 / 1_000
                    })
                    .collect::<Vec<_>>()
            }));
        }
        barrier.wait();
        let destination =
            if hydrated { &mut concurrent_hydrated_us } else { &mut concurrent_control_us };
        for worker in workers {
            destination.extend(worker.join().unwrap());
        }
    }
    let mut concurrent_point_us = Vec::with_capacity(1_600);
    let barrier = Arc::new(Barrier::new(9));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let engine = Arc::clone(&engine);
        let frozen = frozen.clone();
        let reference = treated.evidence[0].target_ref.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(thread::spawn(move || {
            barrier.wait();
            (0..200)
                .map(|_| {
                    let start = Instant::now();
                    engine.resolve_graph_evidence_for_test(&reference, &frozen).unwrap();
                    start.elapsed().as_nanos() as u64 / 1_000
                })
                .collect::<Vec<_>>()
        }));
    }
    barrier.wait();
    for worker in workers {
        concurrent_point_us.extend(worker.join().unwrap());
    }
    let (_many_directory, many, many_request) = fixture_many(50);
    for _ in 0..20 {
        many.graph_expand(&many_request).unwrap();
        many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
    }
    let mut control_50_us = Vec::with_capacity(1_000);
    let mut hydrated_50_us = Vec::with_capacity(1_000);
    for index in 0..1_000 {
        let control_first = index % 2 == 0;
        let measure_control = || {
            let started = Instant::now();
            many.graph_expand(&many_request).unwrap();
            started.elapsed().as_nanos() as u64 / 1_000
        };
        let measure_hydrated = || {
            let started = Instant::now();
            many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
            started.elapsed().as_nanos() as u64 / 1_000
        };
        if control_first {
            control_50_us.push(measure_control());
            hydrated_50_us.push(measure_hydrated());
        } else {
            hydrated_50_us.push(measure_hydrated());
            control_50_us.push(measure_control());
        }
    }
    let many_treated = many.graph_expand_with_graph_evidence_for_test(&many_request).unwrap();
    let many_frozen = match &many_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut memex_batch_50_us = Vec::with_capacity(30);
    for _ in 0..30 {
        let started = Instant::now();
        for entry in many_treated.evidence.iter().take(25) {
            many.resolve_graph_evidence_for_test(&entry.target_ref, many_frozen).unwrap();
            many.resolve_graph_evidence_for_test(
                entry.terminal_edge_ref.as_ref().unwrap(),
                many_frozen,
            )
            .unwrap();
        }
        memex_batch_50_us.push(started.elapsed().as_nanos() as u64 / 1_000);
    }
    println!(
        "SLICE15_RAW={}",
        serde_json::json!({
            "unit": "microseconds", "control_1": control_us, "hydrated_1": hydrated_us,
            "point_node_1k": point_node_us, "point_edge_1k": point_edge_us,
            "concurrent_control_8x200": concurrent_control_us,
            "concurrent_hydrated_8x200": concurrent_hydrated_us,
            "concurrent_point_8x200": concurrent_point_us,
            "control_50": control_50_us, "hydrated_50": hydrated_50_us,
            "memex_batch_50": memex_batch_50_us,
            "control_response_50_bytes": encode_graph_expand_result_v1(&many_treated.graph).unwrap().len(),
            "control_response_bytes": encode_graph_expand_result_v1(&treated.graph).unwrap().len(),
            "sidecar_reference_bytes": treated.evidence[0].target_ref.as_str().len()
                + treated.evidence[0].terminal_edge_ref.as_ref().unwrap().as_str().len()
                + treated.evidence[0].target_revision_id.len()
                + treated.evidence[0].terminal_edge_revision_id.as_ref().unwrap().len(),
        })
    );
}

#[test]
#[ignore = "Slice 15 writer-interference campaigns"]
fn writer_interference_emits_campaigns() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    let mut output = Vec::new();
    for campaign in 0..5 {
        for mode in ["alone", "graph", "point"] {
            let (_directory, engine, request) = fixture();
            let engine = Arc::new(engine);
            let running = Arc::new(AtomicBool::new(true));
            let background = if mode == "alone" {
                None
            } else {
                let engine = Arc::clone(&engine);
                let running = Arc::clone(&running);
                let template = request.clone();
                Some(thread::spawn(move || {
                    let context = match &template.context {
                        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
                        _ => unreachable!(),
                    };
                    while running.load(Ordering::Acquire) {
                        let Ok(frozen) = engine.freeze_read_context(&context) else { continue };
                        let mut request = template.clone();
                        request.context = GraphReadContextV1::Frozen {
                            schema_version: 1,
                            context: frozen.clone(),
                        };
                        if let Ok(treated) =
                            engine.graph_expand_with_graph_evidence_for_test(&request)
                        {
                            if mode == "point" {
                                let _ = engine.resolve_graph_evidence_for_test(
                                    &treated.evidence[0].target_ref,
                                    &frozen,
                                );
                            }
                        }
                    }
                }))
            };
            let mut latencies = Vec::with_capacity(200);
            let started = Instant::now();
            for index in 0..200 {
                let write_started = Instant::now();
                engine
                    .write(&[PreparedWrite::Node {
                        logical_id: Some(format!("writer-{campaign}-{mode}-{index}")),
                        kind: "noise".into(),
                        body: "writer interference payload".into(),
                        source_id: SourceId::new("writer-campaign").unwrap(),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                    }])
                    .unwrap();
                latencies.push(write_started.elapsed().as_nanos() as u64 / 1_000);
            }
            let elapsed = started.elapsed().as_secs_f64();
            running.store(false, Ordering::Release);
            if let Some(background) = background {
                background.join().unwrap();
            }
            output.push(serde_json::json!({
                "campaign": campaign, "mode": mode, "elapsed_us": (elapsed * 1_000_000.0) as u64,
                "throughput_per_s": 200.0 / elapsed, "latencies_us": latencies,
            }));
        }
    }
    println!("SLICE15_WRITER={}", serde_json::Value::Array(output));
}
