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

fn fixture_with_source(source: &str) -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
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

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    fixture_with_source("canonical source bytes")
}

fn unavailable(error: fathomdb_engine::EngineError) {
    assert!(matches!(error, fathomdb_engine::EngineError::Evidence(ref value)
        if value.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && value.field_path == "/evidenceRef"));
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

fn fixture_max_work() -> (TempDir, Engine, GraphExpandRequestV1) {
    let (directory, engine, mut request) = fixture_many(50);
    let mut writes = Vec::with_capacity(9_949);
    for index in 0..9_949 {
        writes.push(PreparedWrite::Edge {
            logical_id: Some(format!("noise-edge-{index:04}")),
            kind: "noise".into(),
            from: "root".into(),
            to: format!("missing-noise-target-{index:04}"),
            source_id: SourceId::new("noise-owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        });
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
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    let mut incomplete_request = request;
    incomplete_request.context = GraphReadContextV1::Frozen {
        schema_version: 1,
        context: incomplete.freeze_read_context(&context).unwrap(),
    };
    assert!(incomplete.graph_expand_with_graph_evidence_for_test(&incomplete_request).is_err());
}

#[test]
fn corrupt_hash_and_locator_refuse_the_entire_treatment() {
    for (name, sql) in [
        (
            "hash",
            "UPDATE _fathomdb_source_links SET hash_digest=\
             'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff' \
             WHERE artifact_revision_id='target-r1'",
        ),
        (
            "locator",
            "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=999,end_byte=1000 \
             WHERE artifact_revision_id='target-r1'",
        ),
    ] {
        let (directory, engine, mut request) = fixture();
        engine.close().unwrap();
        let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
        rusqlite::Connection::open(&path).unwrap().execute(sql, []).unwrap();
        let reopened = Engine::open(&path).unwrap().engine;
        let context = match &request.context {
            GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
            _ => unreachable!(),
        };
        request.context = GraphReadContextV1::Frozen {
            schema_version: 1,
            context: reopened.freeze_read_context(&context).unwrap(),
        };
        let error = reopened.graph_expand_with_graph_evidence_for_test(&request).unwrap_err();
        assert!(matches!(error, fathomdb_engine::EngineError::Evidence(_)), "{name}: {error:?}");
    }
}

#[test]
fn context_mismatch_foreign_context_and_post_erasure_share_nondisclosure() {
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let reference = treated.evidence[0].target_ref.clone();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    let mut different_filter = SearchFilter::default();
    different_filter.kind = Some("document".into());
    let mismatch = engine
        .freeze_read_context(&ReadContextV1::new(context.view.clone(), different_filter).unwrap())
        .unwrap();
    unavailable(engine.resolve_graph_evidence_for_test(&reference, &mismatch).unwrap_err());

    let foreign_directory = TempDir::new().unwrap();
    let foreign = Engine::open(foreign_directory.path().join(format!("foreign{SQLITE_SUFFIX}")))
        .unwrap()
        .engine;
    let foreign_context = foreign.freeze_read_context(&context).unwrap();
    unavailable(engine.resolve_graph_evidence_for_test(&reference, &foreign_context).unwrap_err());

    engine.erase_source("source-owner").unwrap();
    unavailable(
        engine
            .resolve_graph_evidence_for_test(
                &reference,
                &match &request.context {
                    GraphReadContextV1::Frozen { context, .. } => context.clone(),
                    _ => unreachable!(),
                },
            )
            .unwrap_err(),
    );
}

#[test]
fn held_wal_reader_preserves_typed_erasure_incomplete() {
    let (directory, engine, _request) = fixture();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let holder = rusqlite::Connection::open(&path).unwrap();
    holder.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").unwrap();
    let result = engine.erase_source("source-owner");
    holder.execute_batch("ROLLBACK").unwrap();
    match result {
        Err(fathomdb_engine::EngineError::ErasureIncomplete { stage, .. }) => {
            assert_eq!(stage, "wal_checkpoint");
        }
        other => panic!("expected typed held-WAL refusal, got {other:?}"),
    }
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
    let mut concurrent_control_50_us = Vec::with_capacity(1_600);
    let mut concurrent_hydrated_50_us = Vec::with_capacity(1_600);
    let many = Arc::new(many);
    for hydrated in [false, true] {
        let barrier = Arc::new(Barrier::new(9));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let engine = Arc::clone(&many);
            let request = many_request.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                (0..200)
                    .map(|_| {
                        let started = Instant::now();
                        if hydrated {
                            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
                        } else {
                            engine.graph_expand(&request).unwrap();
                        }
                        started.elapsed().as_nanos() as u64 / 1_000
                    })
                    .collect::<Vec<_>>()
            }));
        }
        barrier.wait();
        let destination =
            if hydrated { &mut concurrent_hydrated_50_us } else { &mut concurrent_control_50_us };
        for worker in workers {
            destination.extend(worker.join().unwrap());
        }
    }

    let (_max_directory, max_engine, max_request) = fixture_max_work();
    assert_eq!(max_engine.graph_expand(&max_request).unwrap().work_units, 10_000);
    let mut control_10k_us = Vec::with_capacity(30);
    let mut hydrated_10k_us = Vec::with_capacity(30);
    for index in 0..30 {
        let measure_control = || {
            let started = Instant::now();
            let result = max_engine.graph_expand(&max_request).unwrap();
            assert_eq!(result.work_units, 10_000);
            started.elapsed().as_nanos() as u64 / 1_000
        };
        let measure_hydrated = || {
            let started = Instant::now();
            let result =
                max_engine.graph_expand_with_graph_evidence_for_test(&max_request).unwrap();
            assert_eq!(result.graph.work_units, 10_000);
            started.elapsed().as_nanos() as u64 / 1_000
        };
        if index % 2 == 0 {
            control_10k_us.push(measure_control());
            hydrated_10k_us.push(measure_hydrated());
        } else {
            hydrated_10k_us.push(measure_hydrated());
            control_10k_us.push(measure_control());
        }
    }

    let large_source = "x".repeat(100 * 1_024);
    let (_large_directory, large, large_request) = fixture_with_source(&large_source);
    let large_treated = large.graph_expand_with_graph_evidence_for_test(&large_request).unwrap();
    let large_frozen = match &large_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let mut point_node_100k_us = Vec::with_capacity(100);
    let mut point_edge_100k_us = Vec::with_capacity(100);
    for index in 0..100 {
        let (first, second) = if index % 2 == 0 {
            (
                large_treated.evidence[0].target_ref.clone(),
                large_treated.evidence[0].terminal_edge_ref.clone().unwrap(),
            )
        } else {
            (
                large_treated.evidence[0].terminal_edge_ref.clone().unwrap(),
                large_treated.evidence[0].target_ref.clone(),
            )
        };
        let started = Instant::now();
        large.resolve_graph_evidence_for_test(&first, large_frozen).unwrap();
        let first_us = started.elapsed().as_nanos() as u64 / 1_000;
        let started = Instant::now();
        large.resolve_graph_evidence_for_test(&second, large_frozen).unwrap();
        let second_us = started.elapsed().as_nanos() as u64 / 1_000;
        if index % 2 == 0 {
            point_node_100k_us.push(first_us);
            point_edge_100k_us.push(second_us);
        } else {
            point_edge_100k_us.push(first_us);
            point_node_100k_us.push(second_us);
        }
    }

    let sidecar_1_bytes = serde_json::to_vec(&serde_json::json!({
        "graph": serde_json::from_slice::<serde_json::Value>(&encode_graph_expand_result_v1(&treated.graph).unwrap()).unwrap(),
        "evidence": treated.evidence.iter().map(|entry| serde_json::json!({
            "targetRevisionId": entry.target_revision_id,
            "targetEvidenceRef": entry.target_ref.as_str(),
            "terminalEdgeRevisionId": entry.terminal_edge_revision_id,
            "terminalEdgeEvidenceRef": entry.terminal_edge_ref.as_ref().map(EvidenceRefV1::as_str),
        })).collect::<Vec<_>>(),
    })).unwrap().len();
    let sidecar_50_bytes = serde_json::to_vec(&serde_json::json!({
        "graph": serde_json::from_slice::<serde_json::Value>(&encode_graph_expand_result_v1(&many_treated.graph).unwrap()).unwrap(),
        "evidence": many_treated.evidence.iter().map(|entry| serde_json::json!({
            "targetRevisionId": entry.target_revision_id,
            "targetEvidenceRef": entry.target_ref.as_str(),
            "terminalEdgeRevisionId": entry.terminal_edge_revision_id,
            "terminalEdgeEvidenceRef": entry.terminal_edge_ref.as_ref().map(EvidenceRefV1::as_str),
        })).collect::<Vec<_>>(),
    })).unwrap().len();
    let inline_50_bytes = serde_json::to_vec(&serde_json::json!({
        "targets": many_treated.graph.targets.iter().zip(&many_treated.evidence).map(|(target, entry)| serde_json::json!({
            "logicalId": target.logical_id, "kind": target.kind, "body": target.body,
            "writeCursor": target.write_cursor.to_string(), "origin": {
                "seedLogicalId": target.origin.seed_logical_id,
                "predecessorLogicalId": target.origin.predecessor_logical_id,
                "targetLogicalId": target.origin.target_logical_id,
                "hopCount": target.origin.hop_count,
                "terminalEdgeKind": target.origin.terminal_edge_kind,
                "terminalDirection": match target.origin.terminal_direction {
                    TraversalDirection::Outgoing => "outgoing",
                    TraversalDirection::Incoming => "incoming",
                    TraversalDirection::Both => "both",
                },
            },
            "targetRevisionId": entry.target_revision_id,
            "targetEvidenceRef": entry.target_ref.as_str(),
            "terminalEdgeRevisionId": entry.terminal_edge_revision_id,
            "terminalEdgeEvidenceRef": entry.terminal_edge_ref.as_ref().map(EvidenceRefV1::as_str),
        })).collect::<Vec<_>>()
    })).unwrap().len();
    println!(
        "SLICE15_RAW={}",
        serde_json::json!({
            "unit": "microseconds", "control_1": control_us, "hydrated_1": hydrated_us,
            "point_node_1k": point_node_us, "point_edge_1k": point_edge_us,
            "concurrent_control_8x200": concurrent_control_us,
            "concurrent_hydrated_8x200": concurrent_hydrated_us,
            "concurrent_point_8x200": concurrent_point_us,
            "control_50": control_50_us, "hydrated_50": hydrated_50_us,
            "concurrent_control_50_8x200": concurrent_control_50_us,
            "concurrent_hydrated_50_8x200": concurrent_hydrated_50_us,
            "control_10k_work": control_10k_us, "hydrated_10k_work": hydrated_10k_us,
            "memex_batch_50": memex_batch_50_us,
            "point_node_100k": point_node_100k_us, "point_edge_100k": point_edge_100k_us,
            "canonical_source_100k_bytes": large_source.len(),
            "control_response_50_bytes": encode_graph_expand_result_v1(&many_treated.graph).unwrap().len(),
            "control_response_bytes": encode_graph_expand_result_v1(&treated.graph).unwrap().len(),
            "sidecar_1_bytes": sidecar_1_bytes, "sidecar_50_bytes": sidecar_50_bytes,
            "inline_50_bytes": inline_50_bytes,
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

#[cfg(target_os = "linux")]
fn peak_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.split_whitespace().next())
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1_024
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "Slice 15 process-isolated RSS campaigns"]
fn isolated_rss_emits_campaigns() {
    const CHILD: &str = "FATHOMDB_SLICE15_RSS_CHILD";
    const PREFIX: &str = "SLICE15_RSS_CHILD=";
    if let Ok(mode) = std::env::var(CHILD) {
        let (_directory, engine, request) = fixture_many(50);
        let baseline = peak_rss_bytes();
        if mode == "treatment" {
            engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
        } else {
            engine.graph_expand(&request).unwrap();
        }
        println!("{PREFIX}{}", peak_rss_bytes().saturating_sub(baseline));
        return;
    }

    let executable = std::env::current_exe().unwrap();
    let mut samples = Vec::new();
    for campaign in 0..5 {
        let order =
            if campaign % 2 == 0 { ["control", "treatment"] } else { ["treatment", "control"] };
        for mode in order {
            let output = std::process::Command::new(&executable)
                .arg("--exact")
                .arg("isolated_rss_emits_campaigns")
                .arg("--ignored")
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(CHILD, mode)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let stdout = String::from_utf8(output.stdout).unwrap();
            let bytes = stdout
                .lines()
                .find_map(|line| line.split_once(PREFIX).map(|(_, value)| value))
                .unwrap()
                .parse::<u64>()
                .unwrap();
            samples.push(serde_json::json!({
                "campaign": campaign, "mode": mode, "peak_rss_delta_bytes": bytes,
            }));
        }
    }
    println!("SLICE15_RSS={}", serde_json::Value::Array(samples));
}

#[cfg(feature = "operator")]
#[test]
#[ignore = "Slice 15 paired erase/excise latency campaigns"]
fn erasure_latency_emits_paired_observations() {
    use std::time::Instant;

    fn held_observation(excise: bool) -> (u64, &'static str) {
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
        let resolver = thread::spawn(move || {
            resolver_engine.resolve_graph_evidence_for_test(&reference, &frozen)
        });
        entered.wait();
        let eraser_engine = Arc::clone(&engine);
        let eraser = thread::spawn(move || {
            let started = Instant::now();
            let outcome = if excise {
                eraser_engine.excise_source("source-owner").map(|_| ())
            } else {
                eraser_engine.erase_source("source-owner").map(|_| ())
            };
            (started.elapsed().as_nanos() as u64 / 1_000, outcome)
        });
        thread::yield_now();
        release.wait();
        resolver.join().unwrap().unwrap();
        let (elapsed, outcome) = eraser.join().unwrap();
        let label = match outcome {
            Ok(()) => "complete",
            Err(fathomdb_engine::EngineError::ErasureIncomplete { stage, .. })
                if stage == "wal_checkpoint" =>
            {
                "wal_checkpoint"
            }
            Err(error) => panic!("unexpected erasure result: {error:?}"),
        };
        (elapsed, label)
    }

    let mut samples = Vec::new();
    for campaign in 0..20 {
        for excise in [false, true] {
            let (_directory, engine, _request) = fixture();
            let started = Instant::now();
            let idle = if excise {
                engine.excise_source("source-owner").map(|_| ())
            } else {
                engine.erase_source("source-owner").map(|_| ())
            };
            let idle_us = started.elapsed().as_nanos() as u64 / 1_000;
            assert!(idle.is_ok());
            let (held_us, outcome) = held_observation(excise);
            samples.push(serde_json::json!({
                "campaign": campaign,
                "operation": if excise { "excise" } else { "erase" },
                "idle_us": idle_us, "held_us": held_us, "held_outcome": outcome,
            }));
        }
    }
    println!("SLICE15_ERASURE={}", serde_json::Value::Array(samples));
}
