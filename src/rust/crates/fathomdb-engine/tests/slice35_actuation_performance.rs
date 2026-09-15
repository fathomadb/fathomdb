use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::lifecycle::{Event, Phase, Subscriber};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ActuationReceiptV1,
    ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine, EngineError, InitialState,
    PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, Subscription, WriteProvenanceV1,
    WriteReceipt,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Instant;
use tempfile::TempDir;

#[derive(Default)]
struct SlowSink {
    slow_events: AtomicUsize,
}

impl Subscriber for SlowSink {
    fn on_event(&self, event: &Event) {
        if event.phase == Phase::Slow {
            self.slow_events.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn source_hash() -> CanonicalHash {
    let digest =
        Sha256::digest(b"source body").iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    CanonicalHash::sha256(digest).unwrap()
}

fn canonical(revision: &str, logical: &str, body: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{revision}")).unwrap(),
        ),
    }
}

fn derived_node(index: usize) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived body {index}"),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(format!("derived-{index}")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(format!("derived-r{index}")).unwrap(),
            SourceVersionId::new("version-source-r1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            source_hash(),
        ),
    }
}

fn derived_edge(index: usize) -> ProvenancedEdgeV1 {
    ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: format!("derived-{index}"),
        to: "anchor".into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(format!("edge-{index}")),
        body: Some(format!("edge body {index}")),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(format!("edge-r{index}")).unwrap(),
            SourceVersionId::new("version-source-r1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            source_hash(),
        ),
    }
}

fn open_arm(dir: &TempDir, name: &str) -> (PathBuf, Engine, Arc<SlowSink>, Subscription, u64) {
    let db_path = path(dir, name);
    let opened = Engine::open_with_choice(
        &db_path,
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .unwrap();
    let sink = Arc::new(SlowSink::default());
    let subscription = opened.engine.subscribe(sink.clone());
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                format!("seed-{name}"),
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical(
                        "source-r1",
                        "source",
                        "source body",
                    )),
                    ActuationOperationV1::PutCanonicalNode(canonical(
                        "anchor-r1",
                        "anchor",
                        "anchor body",
                    )),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    let baseline_bytes = checkpointed_storage_bytes(&db_path);
    (db_path, opened.engine, sink, subscription, baseline_bytes)
}

fn candidate_request(index: usize) -> ActuationBatchV1 {
    ActuationBatchV1::new(
        format!("candidate-{index}"),
        vec![
            ActuationOperationV1::PutDerivedNode(derived_node(index)),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new(
                    format!("dependency-{index}"),
                    "source-r1",
                    format!("derived-r{index}"),
                )
                .unwrap(),
            ),
            ActuationOperationV1::PutDerivedEdge(derived_edge(index)),
        ],
    )
    .unwrap()
}

fn control_request(index: usize) -> ActuationBatchV1 {
    ActuationBatchV1::new(
        format!("control-{index}"),
        vec![
            ActuationOperationV1::PutDerivedNode(derived_node(index)),
            ActuationOperationV1::RegisterSourceDependency(
                SourceDependencyRegistrationV1::new(
                    format!("dependency-{index}"),
                    "source-r1",
                    format!("derived-r{index}"),
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap()
}

fn actuation_response_bytes(receipt: &ActuationReceiptV1) -> usize {
    serde_json::to_vec(&json!({
        "schemaVersion": receipt.schema_version,
        "operationId": receipt.operation_id,
        "requestSha256": receipt.request_sha256,
        "outcome": match receipt.outcome {
            ActuationOutcomeV1::Committed => "committed",
            ActuationOutcomeV1::CommittedClosurePending => "committed_closure_pending",
            ActuationOutcomeV1::Refused => "refused",
        },
        "refusedOperationIndex": receipt.refused_operation_index,
        "refusedFieldPath": receipt.refused_field_path,
        "reasonCodes": receipt.reason_codes.iter().map(|reason| reason.as_str()).collect::<Vec<_>>(),
        "affectedRevisionIds": receipt.affected_revision_ids,
        "resultingWriteBoundary": receipt.resulting_write_boundary.map(|value| value.to_string()),
        "resultingDependencyGeneration": receipt.resulting_dependency_generation.map(|value| value.to_string()),
        "pendingProjectionWriteCursors": receipt.pending_projection_write_cursors.iter().map(u64::to_string).collect::<Vec<_>>(),
        "projectionGenerationId": receipt.projection_generation_id.as_ref().map(|value| value.as_str()),
        "closureOperationIds": receipt.closure_operation_ids,
    }))
    .unwrap()
    .len()
}

fn write_response_bytes(receipt: &WriteReceipt) -> usize {
    serde_json::to_vec(&json!({
        "cursor": receipt.cursor.to_string(),
        "rowCursors": receipt.row_cursors.iter().map(u64::to_string).collect::<Vec<_>>(),
        "danglingEdgeEndpoints": receipt.dangling_edge_endpoints.to_string(),
    }))
    .unwrap()
    .len()
}

fn try_run_candidate(engine: &Engine, index: usize) -> Result<(u128, usize, usize), EngineError> {
    let started = Instant::now();
    let receipt = engine.actuate(candidate_request(index))?;
    Ok((
        started.elapsed().as_micros(),
        actuation_response_bytes(&receipt),
        receipt.pending_projection_write_cursors.len(),
    ))
}

fn run_candidate(engine: &Engine, index: usize) -> (u128, usize, usize) {
    try_run_candidate(engine, index).unwrap()
}

fn try_run_control(engine: &Engine, index: usize) -> Result<(u128, usize, usize), EngineError> {
    let started = Instant::now();
    let actuation = engine.actuate(control_request(index))?;
    let write = engine.write(&[PreparedWrite::ProvenancedEdge(derived_edge(index))])?;
    Ok((
        started.elapsed().as_micros(),
        actuation_response_bytes(&actuation) + write_response_bytes(&write),
        actuation.pending_projection_write_cursors.len() + 1,
    ))
}

fn run_control(engine: &Engine, index: usize) -> (u128, usize, usize) {
    try_run_control(engine, index).unwrap()
}

fn latency_summary(mut samples: Vec<u128>) -> Value {
    samples.sort_unstable();
    let count = samples.len();
    let total = samples.iter().sum::<u128>();
    json!({
        "samples": count,
        "p50_us": samples[(count - 1) * 50 / 100],
        "p95_us": samples[(count - 1) * 95 / 100],
        "total_us": total,
        "throughput_per_s": (count as f64) * 1_000_000.0 / total as f64,
    })
}

fn completion_spread(samples: &[u128]) -> u128 {
    samples.iter().max().unwrap() - samples.iter().min().unwrap()
}

fn storage_bytes(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

fn checkpointed_storage_bytes(db_path: &Path) -> u64 {
    let connection = rusqlite::Connection::open(db_path).unwrap();
    let (busy, _, _): (u32, u32, u32) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(busy, 0);
    drop(connection);
    storage_bytes(db_path.parent().unwrap())
}

fn settle_close_and_size(engine: Engine, db_path: &Path) -> u64 {
    engine.drain(30_000).unwrap();
    engine.close().unwrap();
    checkpointed_storage_bytes(db_path)
}

#[test]
#[ignore = "explicit Slice 35 characterization run"]
fn characterize_current_v1_edge_actuation() {
    let candidate_dir = TempDir::new().unwrap();
    let control_dir = TempDir::new().unwrap();
    let (candidate_path, candidate, candidate_sink, _candidate_subscription, candidate_baseline) =
        open_arm(&candidate_dir, "candidate");
    let (control_path, control, control_sink, _control_subscription, control_baseline) =
        open_arm(&control_dir, "control");

    let mut candidate_unit = Vec::new();
    let mut control_unit = Vec::new();
    let mut candidate_response_bytes = 0;
    let mut control_response_bytes = 0;
    let mut candidate_pending = 0;
    let mut control_pending = 0;
    for index in 0..50 {
        let (latency, bytes, pending) = run_candidate(&candidate, index);
        candidate_unit.push(latency);
        candidate_response_bytes += bytes;
        candidate_pending += pending;
        let (latency, bytes, pending) = run_control(&control, index);
        control_unit.push(latency);
        control_response_bytes += bytes;
        control_pending += pending;
    }

    let candidate_batch =
        ActuationBatchV1::new(
            "candidate-128-batch",
            (0..64)
                .map(|offset| ActuationOperationV1::PutDerivedNode(derived_node(10_000 + offset)))
                .chain((0..64).map(|offset| {
                    ActuationOperationV1::PutDerivedEdge(derived_edge(10_000 + offset))
                }))
                .collect(),
        )
        .unwrap();
    let batch_started = Instant::now();
    let candidate_batch_receipt = candidate.actuate(candidate_batch).unwrap();
    let candidate_batch_us = batch_started.elapsed().as_micros();
    let control_nodes = ActuationBatchV1::new(
        "control-128-batch",
        (0..64)
            .map(|offset| ActuationOperationV1::PutDerivedNode(derived_node(10_000 + offset)))
            .collect(),
    )
    .unwrap();
    let control_edges = (0..64)
        .map(|offset| PreparedWrite::ProvenancedEdge(derived_edge(10_000 + offset)))
        .collect::<Vec<_>>();
    let batch_started = Instant::now();
    let control_batch_receipt = control.actuate(control_nodes).unwrap();
    let control_batch_write = control.write(&control_edges).unwrap();
    let control_batch_us = batch_started.elapsed().as_micros();

    let mut candidate_sequential = Vec::new();
    let mut control_sequential = Vec::new();
    let mut candidate_sequential_bytes = 0;
    let mut control_sequential_bytes = 0;
    let mut candidate_sequential_pending = 0;
    let mut control_sequential_pending = 0;
    for index in 20_000..21_000 {
        let (latency, bytes, pending) = run_candidate(&candidate, index);
        candidate_sequential.push(latency);
        candidate_sequential_bytes += bytes;
        candidate_sequential_pending += pending;
        let (latency, bytes, pending) = run_control(&control, index);
        control_sequential.push(latency);
        control_sequential_bytes += bytes;
        control_sequential_pending += pending;
    }

    let candidate = Arc::new(candidate);
    let unique_barrier = Arc::new(Barrier::new(8));
    let candidate_unique_started = Instant::now();
    let candidate_unique = (0..8)
        .map(|offset| {
            let engine = Arc::clone(&candidate);
            let barrier = Arc::clone(&unique_barrier);
            let group_started = candidate_unique_started;
            std::thread::spawn(move || {
                barrier.wait();
                let result = try_run_candidate(&engine, 30_000 + offset);
                (result, group_started.elapsed().as_micros())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    let candidate_unique_total_us = candidate_unique_started.elapsed().as_micros();
    let candidate_unique_lock_failures = candidate_unique
        .iter()
        .filter(|(result, _)| matches!(result, Err(EngineError::Storage)))
        .count();
    assert!(candidate_unique.iter().all(|(result, _)| result.is_ok()));
    let candidate_unique_completions =
        candidate_unique.iter().map(|(_, completed)| *completed).collect::<Vec<_>>();
    let candidate_unique =
        candidate_unique.into_iter().map(|(result, _)| result.unwrap()).collect::<Vec<_>>();

    let control = Arc::new(control);
    let unique_barrier = Arc::new(Barrier::new(8));
    let control_unique_started = Instant::now();
    let control_unique = (0..8)
        .map(|offset| {
            let engine = Arc::clone(&control);
            let barrier = Arc::clone(&unique_barrier);
            let group_started = control_unique_started;
            std::thread::spawn(move || {
                barrier.wait();
                let result = try_run_control(&engine, 30_000 + offset);
                (result, group_started.elapsed().as_micros())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    let control_unique_total_us = control_unique_started.elapsed().as_micros();
    let control_unique_lock_failures = control_unique
        .iter()
        .filter(|(result, _)| matches!(result, Err(EngineError::Storage)))
        .count();
    assert!(control_unique.iter().all(|(result, _)| result.is_ok()));
    let control_unique_completions =
        control_unique.iter().map(|(_, completed)| *completed).collect::<Vec<_>>();
    let control_unique =
        control_unique.into_iter().map(|(result, _)| result.unwrap()).collect::<Vec<_>>();

    let shared = candidate_request(40_000);
    let shared_receipt = candidate.actuate(shared.clone()).unwrap();
    let shared_barrier = Arc::new(Barrier::new(8));
    let shared_started = Instant::now();
    let shared_results = (0..8)
        .map(|_| {
            let engine = Arc::clone(&candidate);
            let barrier = Arc::clone(&shared_barrier);
            let request = shared.clone();
            let expected = shared_receipt.clone();
            let group_started = shared_started;
            std::thread::spawn(move || {
                barrier.wait();
                let started = Instant::now();
                let result = engine.actuate(request).map(|receipt| {
                    assert_eq!(receipt, expected);
                    (
                        started.elapsed().as_micros(),
                        actuation_response_bytes(&receipt),
                        receipt.pending_projection_write_cursors.len(),
                    )
                });
                (result, group_started.elapsed().as_micros())
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    let shared_total_us = shared_started.elapsed().as_micros();
    let shared_lock_failures = shared_results
        .iter()
        .filter(|(result, _)| matches!(result, Err(EngineError::Storage)))
        .count();
    assert!(shared_results.iter().all(|(result, _)| result.is_ok()));
    let shared_completions =
        shared_results.iter().map(|(_, completed)| *completed).collect::<Vec<_>>();
    let shared_results =
        shared_results.into_iter().map(|(result, _)| result.unwrap()).collect::<Vec<_>>();

    let candidate = Arc::try_unwrap(candidate).ok().unwrap();
    let control = Arc::try_unwrap(control).ok().unwrap();
    let candidate_storage_bytes = settle_close_and_size(candidate, &candidate_path);
    let control_storage_bytes = settle_close_and_size(control, &control_path);
    let candidate_storage_growth = candidate_storage_bytes.saturating_sub(candidate_baseline);
    let control_storage_growth = control_storage_bytes.saturating_sub(control_baseline);
    let candidate_logical_units = 50 + 64 + 1_000 + 8 + 1;
    let control_logical_units = 50 + 64 + 1_000 + 8;

    println!(
        "{}",
        json!({
            "schema": "fathomdb.slice35-actuation-performance/v1",
            "environment": {
                "arch": std::env::consts::ARCH,
                "os": std::env::consts::OS,
                "sqlite": rusqlite::version(),
                "profile": "cargo-test-debug",
                "embedder": "fathomdb-noop"
            },
            "three_operation_unit": {
                "candidate": latency_summary(candidate_unit),
                "control": latency_summary(control_unit),
                "candidate_api_invocations": 50,
                "control_api_invocations": 100,
                "candidate_response_bytes": candidate_response_bytes,
                "control_response_bytes": control_response_bytes,
                "candidate_pending_cursors_observed": candidate_pending,
                "control_pending_cursors_observed": control_pending
            },
            "batch_128_operations": {
                "candidate_us": candidate_batch_us,
                "control_us": control_batch_us,
                "candidate_api_invocations": 1,
                "control_api_invocations": 2,
                "candidate_response_bytes": actuation_response_bytes(&candidate_batch_receipt),
                "control_response_bytes": actuation_response_bytes(&control_batch_receipt) + write_response_bytes(&control_batch_write),
                "candidate_pending_cursors": candidate_batch_receipt.pending_projection_write_cursors.len(),
                "control_pending_cursors": control_batch_receipt.pending_projection_write_cursors.len() + 64
            },
            "sequential_1000": {
                "candidate": latency_summary(candidate_sequential),
                "control": latency_summary(control_sequential),
                "candidate_api_invocations": 1000,
                "control_api_invocations": 2000,
                "candidate_response_bytes": candidate_sequential_bytes,
                "control_response_bytes": control_sequential_bytes,
                "candidate_pending_cursors_observed": candidate_sequential_pending,
                "control_pending_cursors_observed": control_sequential_pending
            },
            "concurrent_unique_8": {
                "candidate": latency_summary(candidate_unique.iter().map(|value| value.0).collect()),
                "control": latency_summary(control_unique.iter().map(|value| value.0).collect()),
                "candidate_end_to_end_us": candidate_unique_total_us,
                "control_end_to_end_us": control_unique_total_us,
                "candidate_completion_spread_us": completion_spread(&candidate_unique_completions),
                "control_completion_spread_us": completion_spread(&control_unique_completions),
                "candidate_api_invocations": 8,
                "control_api_invocations": 16,
                "candidate_response_bytes": candidate_unique.iter().map(|value| value.1).sum::<usize>(),
                "control_response_bytes": control_unique.iter().map(|value| value.1).sum::<usize>(),
                "candidate_pending_cursors_observed": candidate_unique.iter().map(|value| value.2).sum::<usize>(),
                "control_pending_cursors_observed": control_unique.iter().map(|value| value.2).sum::<usize>(),
                "candidate_writer_lock_failures": candidate_unique_lock_failures,
                "control_writer_lock_failures": control_unique_lock_failures
            },
            "concurrent_exact_replay_8": {
                "candidate": latency_summary(shared_results.iter().map(|value| value.0).collect()),
                "candidate_end_to_end_us": shared_total_us,
                "candidate_completion_spread_us": completion_spread(&shared_completions),
                "candidate_api_invocations": 8,
                "candidate_response_bytes": shared_results.iter().map(|value| value.1).sum::<usize>(),
                "candidate_pending_cursors_observed": shared_results.iter().map(|value| value.2).sum::<usize>(),
                "control": "not_equivalent: ordinary edge write has no operation-id replay",
                "candidate_writer_lock_failures": shared_lock_failures
            },
            "at_rest": {
                "candidate_baseline_bytes": candidate_baseline,
                "control_baseline_bytes": control_baseline,
                "candidate_storage_bytes": candidate_storage_bytes,
                "control_storage_bytes": control_storage_bytes,
                "candidate_storage_growth_bytes": candidate_storage_growth,
                "control_storage_growth_bytes": control_storage_growth,
                "candidate_logical_units": candidate_logical_units,
                "control_logical_units": control_logical_units,
                "candidate_growth_bytes_per_logical_unit": candidate_storage_growth as f64 / candidate_logical_units as f64,
                "control_growth_bytes_per_logical_unit": control_storage_growth as f64 / control_logical_units as f64,
                "candidate_slow_events": candidate_sink.slow_events.load(Ordering::Relaxed),
                "control_slow_events": control_sink.slow_events.load(Ordering::Relaxed)
            }
        })
    );
}
