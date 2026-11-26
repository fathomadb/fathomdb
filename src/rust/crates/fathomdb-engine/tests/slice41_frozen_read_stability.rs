//! 0.8.32 Slice 41 (`WP-41D`, `#267` Part 3, `ADR #103`) — Schema 39 partitioned
//! frozen-read additive stability (`AC32-FROZEN-STAB-01..06`).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    migrate_38_to_39, ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash,
    CleanupResult, DependencyTraceDirectionV1, DependencyTraceRequestV1, EmbedderChoice, Engine,
    EngineConfig, EngineError, EvidenceErrorReasonV1, EvidenceRefV1, EvidenceResolveManyItemV1,
    EvidenceResolveManyRequestV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    FrozenReadErrorReason, GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, InitialState, LatestStateActuationV1, LifecycleState, PageRequestV1,
    PhysicalErasurePolicy, PhysicalErasureSelector, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, PropertyEntityClass, ProvenancedEdgeV1, ProvenancedNodeV1, QueueResult,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, TestEmbedderChoice, TraversalDirection,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct DeterministicEmbedder;

impl Embedder for DeterministicEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice41-deterministic", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0_f32; 8];
        for (idx, byte) in text.bytes().enumerate() {
            vector[idx % 8] += f32::from(byte) / 255.0;
        }
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        } else {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn sha256_hex(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn canonical_node(
    logical_id: &str,
    kind: &str,
    rev_id: &str,
    sv_id: &str,
    source_id: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(sv_id).unwrap(),
        ),
        properties: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_node(
    logical_id: &str,
    kind: &str,
    rev_id: &str,
    sv_id: &str,
    src_rev_id: &str,
    source_id: &str,
    source_body: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(sv_id).unwrap(),
            SourceRevisionId::new(src_rev_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
        ),
        properties: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_edge(
    logical_id: &str,
    kind: &str,
    from: &str,
    to: &str,
    rev_id: &str,
    sv_id: &str,
    src_rev_id: &str,
    source_id: &str,
    source_body: &str,
    body: Option<&str>,
) -> PreparedWrite {
    PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source_id).unwrap(),
        body: body.map(str::to_string),
        t_valid: None,
        t_invalid: None,
        confidence: Some(0.9),
        extractor_model_id: None,
        temporal_fallback: Some(false),
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(sv_id).unwrap(),
            SourceRevisionId::new(src_rev_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body)).unwrap(),
        ),
        properties: None,
    })
}

fn plain_node(logical_id: &str, kind: &str, source_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn doc_context() -> ReadContextV1 {
    let mut filter = SearchFilter::default();
    filter.kind = Some("Doc".into());
    ReadContextV1::new(ReadView::default(), filter).unwrap()
}

fn default_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

fn assert_state_drifted<T: std::fmt::Debug>(res: Result<T, EngineError>, label: &str) {
    assert!(
        matches!(
            res,
            Err(EngineError::FrozenRead(ref fe)) if fe.reason == FrozenReadErrorReason::StateDrifted
        ),
        "{label}: expected FrozenRead(StateDrifted), got {res:?}"
    );
}

fn assert_evidence_unavailable<T: std::fmt::Debug>(res: Result<T, EngineError>, label: &str) {
    assert!(
        matches!(
            res,
            Err(EngineError::Evidence(ref ev))
                if ev.reason == EvidenceErrorReasonV1::EvidenceUnavailable
        ),
        "{label}: expected Evidence(EvidenceUnavailable), got {res:?}"
    );
}

fn open_scoped(path: &Path, scope: &str) -> Engine {
    let cfg = EngineConfig { scope_policy: ScopePolicy::single(scope), ..Default::default() };
    Engine::open_with_choice_and_config(path, EmbedderChoice::None, cfg).unwrap().engine
}

fn open_scoped_with_embedder(path: &Path, scope: &str) -> Engine {
    let cfg = EngineConfig { scope_policy: ScopePolicy::single(scope), ..Default::default() };
    Engine::open_with_test_choice_and_config_for_test(
        path,
        TestEmbedderChoice::Caller(Arc::new(DeterministicEmbedder)),
        cfg,
    )
    .unwrap()
    .engine
}

fn seed_doc_corpus(engine: &Engine, prefix: &str) -> String {
    let src_body = format!("{prefix} canonical source transcript about stellar mechanics");
    let src_id = format!("{prefix}-src");
    let src_rev = format!("{prefix}-src-r1");
    let sv_id = format!("{prefix}-sv-1");
    let bucket = format!("{prefix}-bucket");
    engine
        .write(&[
            canonical_node(&src_id, "SourceDoc", &src_rev, &sv_id, &bucket, &src_body),
            derived_node(
                &format!("{prefix}-doc-1"),
                "Doc",
                &format!("{prefix}-doc-1-r1"),
                &sv_id,
                &src_rev,
                &bucket,
                &src_body,
                &format!("{prefix} stellar mechanics primary doc one"),
            ),
            derived_node(
                &format!("{prefix}-doc-2"),
                "Doc",
                &format!("{prefix}-doc-2-r1"),
                &sv_id,
                &src_rev,
                &bucket,
                &src_body,
                &format!("{prefix} stellar mechanics secondary doc two"),
            ),
            derived_edge(
                &format!("{prefix}-edge-1"),
                "supports",
                &format!("{prefix}-doc-1"),
                &format!("{prefix}-doc-2"),
                &format!("{prefix}-edge-1-r1"),
                &sv_id,
                &src_rev,
                &bucket,
                &src_body,
                None,
            ),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    src_body
}

#[test]
fn concurrent_disjoint_kind_write_and_actuate_zero_drift() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ac01-concurrent{SQLITE_SUFFIX}"));
    let engine = Arc::new(Engine::open(&path).unwrap().engine);
    seed_doc_corpus(&engine, "c1");

    let stop = Arc::new(AtomicBool::new(false));
    let drift_errors = Arc::new(AtomicUsize::new(0));
    let completed_cycles = Arc::new(AtomicUsize::new(0));
    let start_barrier = Arc::new(Barrier::new(9)); // 8 readers + 1 writer

    let mut handles = Vec::new();
    for reader_id in 0..8usize {
        let engine = Arc::clone(&engine);
        let stop = Arc::clone(&stop);
        let drift_errors = Arc::clone(&drift_errors);
        let completed_cycles = Arc::clone(&completed_cycles);
        let barrier = Arc::clone(&start_barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            for iter in 0..20usize {
                if stop.load(Ordering::Relaxed) && iter >= 8 {
                    break;
                }
                let frozen = engine.freeze_read_context(&doc_context()).unwrap();
                // Introduce workflow think time so writers commit mid-workflow.
                thread::sleep(Duration::from_millis(2));

                let mut ev_req = EvidenceSearchRequestV1::new("stellar".into(), frozen.clone());
                ev_req.limit = 5;
                match engine.search_with_evidence(&ev_req) {
                    Ok(ev_res) => {
                        assert!(
                            !ev_res.evidence.is_empty(),
                            "reader {reader_id} expected evidence"
                        );
                        let refs: Vec<EvidenceRefV1> =
                            ev_res.evidence.iter().map(|e| e.evidence_ref.clone()).collect();
                        thread::sleep(Duration::from_millis(2));
                        match engine.resolve_evidence_many(&EvidenceResolveManyRequestV1 {
                            schema_version: 1,
                            evidence_refs: refs,
                            context: frozen.clone(),
                        }) {
                            Ok(items) => {
                                assert!(
                                    items.iter().all(|i| matches!(
                                        i,
                                        EvidenceResolveManyItemV1::Resolved(_)
                                    )),
                                    "reader {reader_id} unexpected refused item: {items:?}"
                                );
                            }
                            Err(err) => {
                                eprintln!("reader {reader_id} resolve_evidence_many err: {err:?}");
                                drift_errors.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("reader {reader_id} search_with_evidence err: {err:?}");
                        drift_errors.fetch_add(1, Ordering::SeqCst);
                    }
                }

                // Also validate search_frozen, read_canonical_page("Doc"), and graph_expand
                if let Err(err) =
                    engine.search_frozen("stellar", &frozen, 0, false, 0.3, 0, false, 5)
                {
                    eprintln!("reader {reader_id} search_frozen err: {err:?}");
                    drift_errors.fetch_add(1, Ordering::SeqCst);
                }
                if let Err(err) = engine.read_canonical_page(
                    "Doc",
                    &frozen,
                    &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
                ) {
                    eprintln!("reader {reader_id} read_canonical_page err: {err:?}");
                    drift_errors.fetch_add(1, Ordering::SeqCst);
                }
                let expand_req = GraphExpandRequestV1::new(
                    GraphSeedV1::Explicit {
                        schema_version: 1,
                        logical_ids: vec![IdSpace::logical("c1-doc-1")],
                    },
                    TraversalDirection::Outgoing,
                    GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
                    1,
                    10,
                    10_000,
                );
                if let Err(err) = engine.graph_expand(&expand_req) {
                    eprintln!("reader {reader_id} graph_expand err: {err:?}");
                    drift_errors.fetch_add(1, Ordering::SeqCst);
                }
                completed_cycles.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    // Writer commits disjoint kind="AppTurn" via both Engine::write and Engine::actuate.
    let writer_engine = Arc::clone(&engine);
    let writer_stop = Arc::clone(&stop);
    let writer_barrier = Arc::clone(&start_barrier);
    let writer = thread::spawn(move || {
        writer_barrier.wait();
        for step in 0..40usize {
            if step % 2 == 0 {
                writer_engine
                    .write(&[plain_node(
                        &format!("turn-w-{step}"),
                        "AppTurn",
                        "src:appturn",
                        &format!("conversation turn written {step}"),
                    )])
                    .unwrap();
            } else {
                let batch = ActuationBatchV1::new(
                    format!("act-turn-{step}"),
                    vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                        logical_id: Some(format!("turn-a-{step}")),
                        kind: "AppTurn".into(),
                        body: format!("conversation turn actuated {step}"),
                        source_id: SourceId::new("src:appturn").unwrap(),
                        state: InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        provenance: WriteProvenanceV1::canonical(
                            ArtifactRevisionId::new(format!("rev-turn-{step}")).unwrap(),
                            SourceVersionId::new(format!("sv-turn-{step}")).unwrap(),
                        ),
                        properties: None,
                    })],
                )
                .unwrap();
                let receipt = writer_engine.actuate(batch).unwrap();
                assert_eq!(receipt.outcome, fathomdb_engine::ActuationOutcomeV1::Committed);
            }
            thread::sleep(Duration::from_millis(2));
        }
        writer_stop.store(true, Ordering::Relaxed);
    });

    writer.join().unwrap();
    for h in handles {
        h.join().unwrap();
    }

    assert!(completed_cycles.load(Ordering::Relaxed) >= 64);
    assert_eq!(
        drift_errors.load(Ordering::SeqCst),
        0,
        "AC32-FROZEN-STAB-01: expected 0% drift under concurrent disjoint AppTurn write/actuate"
    );

    Arc::try_unwrap(engine).unwrap_or_else(|_| panic!("engine still shared")).close().unwrap();
}

#[test]
fn append_only_log_and_latest_state_isolation_no_deadlock() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ac02-opstore{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    seed_doc_corpus(&engine, "op");

    // Register AppendOnlyLog and LatestState collections + initial LatestState record.
    engine
        .write(&[
            PreparedWrite::AdminSchema {
                name: "audit_events".into(),
                kind: "append_only_log".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            },
            PreparedWrite::AdminSchema {
                name: "agent_state".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            },
        ])
        .unwrap();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "agent_state".into(),
            record_key: "k1".into(),
            schema_id: None,
            body: r#"{"step":0}"#.into(),
        }])
        .unwrap();
    engine.drain(10_000).unwrap();

    let frozen_doc = engine.freeze_read_context(&doc_context()).unwrap();
    let frozen_default = engine.freeze_read_context(&default_context()).unwrap();
    let frozen_op = engine.freeze_read_context(&default_context()).unwrap();

    let ev_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("stellar".into(), frozen_doc.clone()))
        .unwrap();
    let doc_ref = ev_search.evidence[0].evidence_ref.clone();

    // High-frequency AppendOnlyLog appends (25 writes).
    for i in 0..25usize {
        engine
            .write(&[PreparedWrite::OpStore {
                collection: "audit_events".into(),
                record_key: format!("evt-{i}"),
                schema_id: None,
                body: format!(r#"{{"seq":{i}}}"#),
            }])
            .unwrap();
    }
    // Verify scheduler drains cleanly with zero deadlock and preserves ('up_to_date', 'non_canonical').
    engine.drain(10_000).unwrap();
    {
        let conn = Connection::open(&path).unwrap();
        let non_canonical_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_projection_terminal
                 WHERE state = 'up_to_date' AND owner_kind = 'non_canonical'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            non_canonical_count >= 25,
            "expected >=25 ('up_to_date', 'non_canonical') rows, got {non_canonical_count}"
        );
    }

    // 0% drift across canonical, search, evidence, graph, trace, AND operational_state!
    engine.search_frozen("stellar", &frozen_doc, 0, false, 0.3, 0, false, 10).unwrap();
    engine.search_frozen("stellar", &frozen_default, 0, false, 0.3, 0, false, 10).unwrap();
    engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: doc_ref.clone(),
            context: frozen_doc.clone(),
        })
        .unwrap();
    engine
        .read_canonical_page(
            "Doc",
            &frozen_doc,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        )
        .unwrap();
    engine
        .graph_expand(&GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("op-doc-1")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen_default.clone() },
            1,
            10,
            10_000,
        ))
        .unwrap();
    engine
        .trace_dependency(DependencyTraceRequestV1 {
            schema_version: 1,
            direction: DependencyTraceDirectionV1::ToDependents,
            root_revision_id: "op-src-r1".into(),
            context: frozen_default.clone(),
            max_relations: 10,
            max_work_units: 100,
        })
        .unwrap();
    let op_rec = engine.read_operational_state("agent_state", "k1", Some(&frozen_op)).unwrap();
    assert!(op_rec.is_some());
    engine
        .read_operational_state_page(
            "agent_state",
            &frozen_op,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        )
        .unwrap();

    // Mixed [Node(AppTurn), OpStore(AppendOnlyLog)] batch must NOT drift frozen_doc or frozen_op.
    engine
        .write(&[
            plain_node("mixed-turn-1", "AppTurn", "src:mixed", "mixed batch turn"),
            PreparedWrite::OpStore {
                collection: "audit_events".into(),
                record_key: "evt-mixed".into(),
                schema_id: None,
                body: r#"{"mixed":true}"#.into(),
            },
        ])
        .unwrap();
    engine.search_frozen("stellar", &frozen_doc, 0, false, 0.3, 0, false, 10).unwrap();
    engine.read_operational_state("agent_state", "k1", Some(&frozen_op)).unwrap().unwrap();

    // Re-mint frozen_default after mixed AppTurn node write, then upsert LatestState (`operational_state`).
    let frozen_default2 = engine.freeze_read_context(&default_context()).unwrap();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "agent_state".into(),
            record_key: "k1".into(),
            schema_id: None,
            body: r#"{"step":1}"#.into(),
        }])
        .unwrap();
    let act_op = ActuationBatchV1::new(
        "op-latest-state-2",
        vec![ActuationOperationV1::PutLatestState(LatestStateActuationV1 {
            collection: "agent_state".into(),
            record_key: "k2".into(),
            payload_json: r#"{"step":2}"#.into(),
        })],
    )
    .unwrap();
    engine.actuate(act_op).unwrap();
    engine.drain(10_000).unwrap();

    // LatestState upserts MUST NOT drift canonical/search/evidence/graph/trace (even SearchFilter::default()).
    engine.search_frozen("stellar", &frozen_doc, 0, false, 0.3, 0, false, 10).unwrap();
    engine.search_frozen("stellar", &frozen_default2, 0, false, 0.3, 0, false, 10).unwrap();
    engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: doc_ref,
            context: frozen_doc.clone(),
        })
        .unwrap();
    engine
        .read_canonical_page(
            "Doc",
            &frozen_doc,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        )
        .unwrap();
    engine
        .graph_expand(&GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("op-doc-1")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen_default2.clone() },
            1,
            10,
            10_000,
        ))
        .unwrap();
    engine
        .trace_dependency(DependencyTraceRequestV1 {
            schema_version: 1,
            direction: DependencyTraceDirectionV1::ToDependents,
            root_revision_id: "op-src-r1".into(),
            context: frozen_default2,
            max_relations: 10,
            max_work_units: 100,
        })
        .unwrap();

    // LatestState upserts MUST drift ReadSurface::OperationalState!
    assert_state_drifted(
        engine.read_operational_state("agent_state", "k1", Some(&frozen_op)),
        "read_operational_state after LatestState upsert",
    );
    assert_state_drifted(
        engine.read_operational_state_page(
            "agent_state",
            &frozen_op,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        ),
        "read_operational_state_page after LatestState upsert",
    );

    engine.close().unwrap();
}

fn open_scoped_explicit_maint(path: &Path, scope: &str) -> Engine {
    let cfg = EngineConfig {
        scope_policy: ScopePolicy::single(scope),
        physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
        ..Default::default()
    };
    Engine::open_with_choice_and_config(path, EmbedderChoice::None, cfg).unwrap().engine
}

#[test]
fn cross_scope_isolation_including_destructive_ops_and_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ac03-cross-scope{SQLITE_SUFFIX}"));

    // Part A: Inline destructive ops (additive write, transition, purge, erase_source) in tenant-b.
    let (frozen_a_doc, frozen_a_default, ev_ref_a) = {
        let eng_a = open_scoped(&path, "tenant-a");
        seed_doc_corpus(&eng_a, "ta");
        eng_a
            .write(&[PreparedWrite::AdminSchema {
                name: "tenant_kv".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: "{}".into(),
            }])
            .unwrap();
        eng_a
            .write(&[PreparedWrite::OpStore {
                collection: "tenant_kv".into(),
                record_key: "a1".into(),
                schema_id: None,
                body: r#"{"tenant":"a"}"#.into(),
            }])
            .unwrap();
        eng_a
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dep-ta-1", "ta-src-r1", "ta-doc-1-r1")
                    .unwrap(),
            )
            .unwrap();
        eng_a.drain(10_000).unwrap();

        let frozen_doc = eng_a.freeze_read_context(&doc_context()).unwrap();
        let frozen_def = eng_a.freeze_read_context(&default_context()).unwrap();
        let ev_res = eng_a
            .search_with_evidence(&EvidenceSearchRequestV1::new(
                "stellar".into(),
                frozen_doc.clone(),
            ))
            .unwrap();
        let ev_ref = ev_res.evidence[0].evidence_ref.clone();
        eng_a.close().unwrap();
        (frozen_doc, frozen_def, ev_ref)
    };

    {
        let eng_b = open_scoped(&path, "tenant-b");
        seed_doc_corpus(&eng_b, "tb");
        eng_b
            .write(&[
                plain_node("tb-purge-target", "Doc", "tb-purge-src", "tenant b purgeable"),
                plain_node("tb-erase-target", "Doc", "tb-erase-src", "tenant b erasable"),
                PreparedWrite::OpStore {
                    collection: "tenant_kv".into(),
                    record_key: "a1".into(),
                    schema_id: None,
                    body: r#"{"tenant":"b"}"#.into(),
                },
            ])
            .unwrap();
        eng_b
            .transition("tb-purge-target", LifecycleState::Deleted, Some("cleanup".into()))
            .unwrap();
        eng_b.purge("tb-purge-target").unwrap();
        eng_b.erase_source("tb-erase-src").unwrap();
        eng_b.close().unwrap();
    }

    // Reopen tenant-a: BOTH frozen_a_doc and frozen_a_default MUST survive with 0% drift!
    let eng_a = open_scoped(&path, "tenant-a");
    eng_a.search_frozen("stellar", &frozen_a_doc, 0, false, 0.3, 0, false, 10).unwrap();
    eng_a.search_frozen("stellar", &frozen_a_default, 0, true, 0.3, 0, false, 10).unwrap();
    eng_a
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: ev_ref_a,
            context: frozen_a_doc.clone(),
        })
        .unwrap();
    eng_a
        .read_canonical_page(
            "Doc",
            &frozen_a_doc,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        )
        .unwrap();
    let op = eng_a.read_operational_state("tenant_kv", "a1", Some(&frozen_a_default)).unwrap();
    assert!(op.is_some());
    eng_a
        .graph_expand(&GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("ta-doc-1")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen_a_default.clone() },
            1,
            10,
            10_000,
        ))
        .unwrap();
    eng_a
        .trace_dependency(DependencyTraceRequestV1 {
            schema_version: 1,
            direction: DependencyTraceDirectionV1::ToDependents,
            root_revision_id: "ta-src-r1".into(),
            context: frozen_a_default,
            max_relations: 10,
            max_work_units: 100,
        })
        .unwrap();
    eng_a.close().unwrap();

    // Part B: ExplicitMaintenance physical erasure in tenant-b must NOT drift tenant-a!
    let phys_path = dir.path().join(format!("ac03-cross-scope-phys{SQLITE_SUFFIX}"));
    let frozen_phys_a = {
        let eng_a = open_scoped_explicit_maint(&phys_path, "tenant-a");
        seed_doc_corpus(&eng_a, "pa");
        let f = eng_a.freeze_read_context(&default_context()).unwrap();
        eng_a.close().unwrap();
        f
    };
    {
        let eng_b = Arc::new(open_scoped_explicit_maint(&phys_path, "tenant-b"));
        eng_b
            .write(&[plain_node("tb-phys", "Doc", "tb-phys-src", "tenant b physical erase")])
            .unwrap();
        let mut handle = eng_b.begin_erasure_maintenance(1000).unwrap();
        let queued = handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "tb-phys-src".into() },
                "00000000000000000000000000000001",
            )
            .unwrap();
        let ticket = match queued {
            QueueResult::Accepted { ticket, .. } => ticket,
            other => panic!("expected Accepted, got {other:?}"),
        };
        let done = handle.complete_physical_erasure(&ticket).unwrap();
        assert!(matches!(done, CleanupResult::Complete { .. }), "{done:?}");
        handle.close().unwrap();
        eng_b.close().unwrap();
    }
    let eng_a = open_scoped_explicit_maint(&phys_path, "tenant-a");
    eng_a.search_frozen("stellar", &frozen_phys_a, 0, true, 0.3, 0, false, 10).unwrap();
    eng_a.close().unwrap();
}

#[test]
fn async_projection_worker_kind_and_scope_isolation_and_mean_vec_pin() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ac04-projection{SQLITE_SUFFIX}"));

    // Configure vector projections for Doc and AppTurn and seed Doc in tenant-a.
    {
        let eng = open_scoped_with_embedder(&path, "tenant-a");
        eng.configure_projections(
            &[ProjectionSpec {
                name: "body".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: None,
                vector: Some(ProjectionVector {
                    embedder: None,
                    dense_readiness: None,
                    kinds: Some(vec!["Doc".into(), "AppTurn".into()]),
                }),
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
        seed_doc_corpus(&eng, "va");
        eng.drain(10_000).unwrap();
        eng.close().unwrap();
    }

    // Open tenant-a with embedder, mint frozen Doc context.
    let frozen_a_doc = {
        let eng_a = open_scoped_with_embedder(&path, "tenant-a");
        let frozen_doc = eng_a.freeze_read_context(&doc_context()).unwrap();
        // Write AppTurn in tenant-a and let background projection_worker embed it!
        eng_a
            .write(&[plain_node("va-turn-1", "AppTurn", "src:va-turn", "background embedded turn")])
            .unwrap();
        eng_a.drain(10_000).unwrap();
        // Verify AppTurn vector actually landed in _fathomdb_vector_rows!
        {
            let conn = Connection::open(&path).unwrap();
            let appturn_vecs: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind = 'AppTurn'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(
                appturn_vecs >= 1,
                "expected AppTurn vector row committed by projection_worker"
            );
        }
        // frozen_a_doc MUST have 0% drift after AppTurn vector commit!
        eng_a.search_frozen("stellar", &frozen_doc, 0, false, 0.3, 0, false, 10).unwrap();
        eng_a
            .read_canonical_page(
                "Doc",
                &frozen_doc,
                &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
            )
            .unwrap();
        eng_a.close().unwrap();
        frozen_doc
    };

    // Embed a "Doc" vector in "tenant-b": MUST NOT drift "tenant-a" frozen_a_doc!
    {
        let eng_b = open_scoped_with_embedder(&path, "tenant-b");
        eng_b.write(&[plain_node("vb-doc-1", "Doc", "src:vb-doc", "tenant b doc vector")]).unwrap();
        eng_b.drain(10_000).unwrap();
        eng_b.close().unwrap();
    }
    {
        let eng_a = open_scoped_with_embedder(&path, "tenant-a");
        eng_a.search_frozen("stellar", &frozen_a_doc, 0, false, 0.3, 0, false, 10).unwrap();
        eng_a.close().unwrap();
    }

    // Write a new "Doc" in "tenant-a" with EmbedderChoice::None so its vector is pending,
    // mint frozen_before_embed, then reopen with DeterministicEmbedder and drain ->
    // embedding "Doc" in "tenant-a" MUST drift frozen_before_embed on search_frozen,
    // while NOT drifting CanonicalPage!
    let frozen_before_embed = {
        let eng_no_emb = open_scoped(&path, "tenant-a");
        eng_no_emb
            .write(&[plain_node("va-doc-pending", "Doc", "src:va", "pending doc for worker")])
            .unwrap();
        let frozen = eng_no_emb.freeze_read_context(&doc_context()).unwrap();
        eng_no_emb.close().unwrap();
        frozen
    };
    {
        let eng_a = open_scoped_with_embedder(&path, "tenant-a");
        eng_a.drain(10_000).unwrap();
        assert_state_drifted(
            eng_a.search_frozen("stellar", &frozen_before_embed, 0, false, 0.3, 0, false, 10),
            "embedding Doc in tenant-a must drift SearchNoGraph",
        );
        // CanonicalPage ignores vector readiness!
        eng_a
            .read_canonical_page(
                "Doc",
                &frozen_before_embed,
                &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
            )
            .unwrap();

        // Pinning _fathomdb_embedder_profiles.mean_vec MUST drift deterministically (global_struct).
        let frozen_before_pin = eng_a.freeze_read_context(&doc_context()).unwrap();
        let mean_blob = vec![0_u8; 8 * 4];
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "UPDATE _fathomdb_embedder_profiles SET mean_vec = ?1 WHERE profile = 'default'",
                [mean_blob.as_slice()],
            )
            .unwrap();
        }
        assert_state_drifted(
            eng_a.search_frozen("stellar", &frozen_before_pin, 0, false, 0.3, 0, false, 10),
            "mean_vec pin must drift SearchNoGraph",
        );
        assert_state_drifted(
            eng_a.read_canonical_page(
                "Doc",
                &frozen_before_pin,
                &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
            ),
            "mean_vec pin must drift CanonicalPage (global_struct)",
        );
        eng_a.close().unwrap();
    }
}

#[test]
fn fail_closed_matrix_same_kind_edges_post_decode_destructive_deps_and_raw_sql_20_tables() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("ac05-fail-closed{SQLITE_SUFFIX}"));
    let engine =
        Engine::open_with_embedder_for_test(&path, Arc::new(DeterministicEmbedder)).unwrap().engine;
    let src_body = seed_doc_corpus(&engine, "fc");
    engine
        .write(&[derived_edge(
            "fc-edge-text-1",
            "supports",
            "fc-doc-1",
            "fc-doc-2",
            "fc-edge-text-1-r1",
            "fc-sv-1",
            "fc-src-r1",
            "fc-bucket",
            &src_body,
            Some("supporting link text"),
        )])
        .unwrap();
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "fc_state".into(),
            kind: "latest_state".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    engine
        .write(&[PreparedWrite::OpStore {
            collection: "fc_state".into(),
            record_key: "s1".into(),
            schema_id: None,
            body: r#"{"v":1}"#.into(),
        }])
        .unwrap();
    engine.drain(10_000).unwrap();

    // 1. Same-(scope, kind) write drifts Doc SearchNoGraph.
    let f_doc = engine.freeze_read_context(&doc_context()).unwrap();
    engine
        .write(&[derived_node(
            "fc-doc-3",
            "Doc",
            "fc-doc-3-r1",
            "fc-sv-1",
            "fc-src-r1",
            "fc-bucket",
            &src_body,
            "another doc",
        )])
        .unwrap();
    assert_state_drifted(
        engine.search_frozen("stellar", &f_doc, 0, false, 0.3, 0, false, 10),
        "same-kind Doc write",
    );

    // 2. Matching-kind edge ("Doc") drifts Doc SearchNoGraph (edge-FTS),
    //    whereas disjoint-kind edge ("relates") does NOT drift Doc SearchNoGraph,
    //    but DOES drift GraphOrEdge, search_expand_frozen, EvidenceGraph, TraceDependency,
    //    and post-decode Edge/GraphArm slots in resolve_evidence_many while node-only slot resolves!
    let f_match_edge = engine.freeze_read_context(&doc_context()).unwrap();
    engine
        .write(&[derived_edge(
            "fc-edge-doc",
            "Doc",
            "fc-doc-1",
            "fc-doc-2",
            "fc-edge-doc-r1",
            "fc-sv-1",
            "fc-src-r1",
            "fc-bucket",
            &src_body,
            Some("stellar matching kind edge"),
        )])
        .unwrap();
    assert_state_drifted(
        engine.search_frozen("stellar", &f_match_edge, 0, false, 0.3, 0, false, 10),
        "matching-kind edge insert must drift Doc SearchNoGraph",
    );
    engine.drain(10_000).unwrap();

    // Now test disjoint-kind edge ("relates"):
    // Mint frozen contexts for Doc (node-only + graph-arm mixed refs) and default.
    let f_disjoint_doc = engine.freeze_read_context(&doc_context()).unwrap();
    let f_disjoint_default = engine.freeze_read_context(&default_context()).unwrap();
    let node_ev_res = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "stellar".into(),
            f_disjoint_doc.clone(),
        ))
        .unwrap();
    let node_ref = node_ev_res.evidence[0].evidence_ref.clone();

    let mut graph_arm_req = EvidenceSearchRequestV1::new("primary".into(), f_disjoint_doc.clone());
    graph_arm_req.use_graph_arm = true;
    graph_arm_req.limit = 10;
    let graph_arm_ev = engine.search_with_evidence(&graph_arm_req).unwrap();
    // Find a GraphArm hit (fc-doc-2 reached from fc-doc-1 via supports) or an Edge hit.
    // To guarantee an Edge-class evidence_ref, also mint a default context and search for edge text.
    let edge_ev_res = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "supporting".into(),
            f_disjoint_default.clone(),
        ))
        .unwrap();
    let edge_ref_from_default = edge_ev_res
        .evidence
        .iter()
        .find(|e| e.artifact_revision_id == "fc-edge-text-1-r1")
        .map(|e| e.evidence_ref.clone());

    // Graph expand with evidence for resolve_graph_evidence test
    let g_exp = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("fc-doc-1")],
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Frozen { schema_version: 1, context: f_disjoint_doc.clone() },
                1,
                10,
                10_000,
            )
            .with_evidence(true),
        )
        .unwrap();
    let graph_ev_ref = g_exp.evidence.unwrap().entries[0].target_evidence_ref.clone();

    // Also mint a Doc-context edge ref via a GraphArm hit if present (fc-doc-2 via fc-edge-1):
    let graph_arm_ref_in_doc = graph_arm_ev
        .search_result
        .results
        .iter()
        .zip(graph_arm_ev.evidence.iter())
        .find(|(h, _)| h.id == IdSpace::logical("fc-doc-2"))
        .map(|(_, e)| e.evidence_ref.clone());

    // Insert a disjoint-kind edge ("relates") in the same scope!
    engine
        .write(&[derived_edge(
            "fc-edge-relates",
            "relates",
            "fc-doc-1",
            "fc-doc-3",
            "fc-edge-relates-r1",
            "fc-sv-1",
            "fc-src-r1",
            "fc-bucket",
            &src_body,
            Some("relates body"),
        )])
        .unwrap();

    // SearchNoGraph with kind="Doc" MUST NOT drift on disjoint edge "relates"!
    engine.search_frozen("stellar", &f_disjoint_doc, 0, false, 0.3, 0, false, 10).unwrap();
    // GraphOrEdge, search_expand_frozen, graph_expand, EvidenceGraph, TraceDependency MUST drift!
    assert_state_drifted(
        engine.search_frozen("stellar", &f_disjoint_doc, 0, true, 0.3, 0, false, 10),
        "disjoint edge must drift GraphOrEdge (use_graph_arm=true)",
    );
    assert_state_drifted(
        engine.search_expand_frozen("stellar", &f_disjoint_doc, 1, 10),
        "disjoint edge must drift search_expand_frozen",
    );
    assert_state_drifted(
        engine.graph_expand(&GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("fc-doc-1")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: f_disjoint_doc.clone() },
            1,
            10,
            10_000,
        )),
        "disjoint edge must drift graph_expand",
    );
    assert_evidence_unavailable(
        engine.resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: graph_ev_ref,
            context: f_disjoint_doc.clone(),
        }),
        "disjoint edge must drift EvidenceGraph",
    );
    assert_state_drifted(
        engine.trace_dependency(DependencyTraceRequestV1 {
            schema_version: 1,
            direction: DependencyTraceDirectionV1::ToDependents,
            root_revision_id: "fc-src-r1".into(),
            context: f_disjoint_doc.clone(),
            max_relations: 10,
            max_work_units: 100,
        }),
        "disjoint edge must drift TraceDependency",
    );

    // Post-decode per-slot Edge/GraphArm check in resolve_evidence_many!
    // Slot 0 = node-only ref (MUST resolve!), Slot 1 = GraphArm ref (MUST be Refused(EvidenceUnavailable) at /evidenceRefs/1!)
    if let Some(ga_ref) = graph_arm_ref_in_doc {
        let many = engine
            .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
                schema_version: 1,
                evidence_refs: vec![node_ref.clone(), ga_ref.clone()],
                context: f_disjoint_doc.clone(),
            })
            .unwrap();
        assert_eq!(many.len(), 2);
        assert!(
            matches!(&many[0], EvidenceResolveManyItemV1::Resolved(_)),
            "node-only slot 0 must still resolve: {many:?}"
        );
        assert!(
            matches!(
                &many[1],
                EvidenceResolveManyItemV1::Refused(err)
                    if err.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                        && err.field_path == "/evidenceRefs/1"
            ),
            "GraphArm slot 1 must be per-slot Refused(EvidenceUnavailable, /evidenceRefs/1): {many:?}"
        );
        // Single resolve_evidence on GraphArm ref fails with EvidenceUnavailable
        assert_evidence_unavailable(
            engine.resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: ga_ref,
                context: f_disjoint_doc.clone(),
            }),
            "single resolve_evidence on drifted GraphArm ref",
        );
    } else {
        panic!("expected GraphArm hit for fc-doc-2 in graph_arm_ev");
    }
    // And default context saw edge_any bump -> whole-call EvidenceUnavailable
    if let Some(er) = edge_ref_from_default {
        assert_evidence_unavailable(
            engine.resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: er,
                context: f_disjoint_default,
            }),
            "default context edge ref must drift",
        );
    }

    // 3. Destructive mutation on ANY kind ("AppTurn") in scope drifts "Doc" frozen read!
    engine.write(&[plain_node("fc-turn-del", "AppTurn", "fc-turn-src", "ephemeral turn")]).unwrap();
    let f_before_dest = engine.freeze_read_context(&doc_context()).unwrap();
    engine.transition("fc-turn-del", LifecycleState::Deleted, None).unwrap();
    assert_state_drifted(
        engine.search_frozen("stellar", &f_before_dest, 0, false, 0.3, 0, false, 10),
        "destructive transition on AppTurn must drift Doc in same scope",
    );

    // 4. source_dependencies write in scope drifts "Doc" frozen read!
    let f_before_dep = engine.freeze_read_context(&doc_context()).unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("fc-dep-1", "fc-src-r1", "fc-doc-1-r1").unwrap(),
        )
        .unwrap();
    assert_state_drifted(
        engine.search_frozen("stellar", &f_before_dep, 0, false, 0.3, 0, false, 10),
        "source_dependencies registration in scope must drift Doc",
    );

    // 5. global_struct update (configure_projections) drifts all frozen reads!
    let f_before_global = engine.freeze_read_context(&doc_context()).unwrap();
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "tag".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    assert_state_drifted(
        engine.search_frozen("stellar", &f_before_global, 0, false, 0.3, 0, false, 10),
        "configure_projections global_struct bump must drift Doc",
    );

    // 6. Validate global monotonic visibility generation regression check is preserved!
    let f_mono = engine.freeze_read_context(&doc_context()).unwrap();
    {
        let conn = Connection::open(&path).unwrap();
        let gen: i64 = conn
            .query_row(
                "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE _fathomdb_read_visibility_state SET generation = ?1 WHERE singleton = 1",
            [gen + 1],
        )
        .unwrap();
        engine.freeze_read_context(&doc_context()).unwrap();
        conn.execute(
            "UPDATE _fathomdb_read_visibility_state SET generation = ?1 WHERE singleton = 1",
            [gen],
        )
        .unwrap();
        assert!(
            matches!(
                engine.search_frozen("stellar", &f_mono, 0, false, 0.3, 0, false, 10),
                Err(EngineError::FrozenRead(ref fe))
                    if fe.reason == FrozenReadErrorReason::StateUnavailable
            ),
            "regressed global generation must fail with StateUnavailable"
        );
        // Restore generation so subsequent checks run cleanly.
        conn.execute(
            "UPDATE _fathomdb_read_visibility_state SET generation = ?1 WHERE singleton = 1",
            [gen + 1],
        )
        .unwrap();
    }

    // 7. Raw coalescing=0 SQL across ALL 20 trigger tables (including UPDATE moving scope/kind!)
    //    must fail closed!
    let conn = Connection::open(&path).unwrap();
    let assert_raw_sql_drifts = |label: &str, use_op_surface: bool, sql: &str| {
        let f = if use_op_surface {
            engine.freeze_read_context(&default_context()).unwrap()
        } else {
            engine.freeze_read_context(&doc_context()).unwrap()
        };
        conn.execute_batch(sql).unwrap_or_else(|e| panic!("raw SQL failed for {label}: {e}"));
        if use_op_surface {
            assert_state_drifted(engine.read_operational_state("fc_state", "s1", Some(&f)), label);
        } else {
            assert_state_drifted(
                engine.search_frozen("stellar", &f, 0, false, 0.3, 0, false, 10),
                label,
            );
        }
    };

    // Class 1A: canonical_nodes (INSERT same kind + UPDATE moving scope/kind paired OLD/NEW!)
    assert_raw_sql_drifts(
        "raw cn INSERT",
        false,
        "INSERT INTO canonical_nodes(write_cursor,kind,body,source_id,scope) VALUES(90001,'Doc','raw','raw-src','');",
    );
    assert_raw_sql_drifts(
        "raw cn UPDATE scope+kind",
        false,
        "UPDATE canonical_nodes SET scope='moved-scope', kind='AppTurn' WHERE write_cursor=90001;",
    );
    // Class 1B: canonical_edges (INSERT matching kind + UPDATE moving scope/kind)
    assert_raw_sql_drifts(
        "raw ce INSERT",
        false,
        "INSERT INTO canonical_edges(write_cursor,kind,from_id,to_id,source_id,scope) VALUES(90002,'Doc','a','b','raw-src','');",
    );
    assert_raw_sql_drifts(
        "raw ce UPDATE scope+kind",
        false,
        "UPDATE canonical_edges SET scope='moved-scope', kind='other' WHERE write_cursor=90002;",
    );
    // Class 1C: ar, sv, sl
    assert_raw_sql_drifts(
        "raw ar INSERT",
        false,
        "INSERT INTO _fathomdb_artifact_revisions(schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness,scope) VALUES(1,'raw-ar-1','node',90003,'legacy','complete','');",
    );
    assert_raw_sql_drifts(
        "raw sv INSERT",
        false,
        "INSERT INTO _fathomdb_source_versions(schema_version,source_id,source_version_id,source_revision_id,scope) VALUES(1,'raw-src','raw-sv-1','raw-ar-1','');",
    );
    assert_raw_sql_drifts(
        "raw sl INSERT",
        false,
        "INSERT INTO _fathomdb_source_links(schema_version,artifact_revision_id,source_id,source_version_id,source_revision_id,locator_kind,hash_algorithm,hash_digest,scope) VALUES(1,'raw-ar-1','raw-src','raw-sv-1','raw-ar-1','whole_body','sha256','0000000000000000000000000000000000000000000000000000000000000000','');",
    );
    // Class 1D: sd, dc
    assert_raw_sql_drifts(
        "raw sd INSERT",
        false,
        "INSERT INTO _fathomdb_source_dependencies(schema_version,dependency_id,derived_revision_id,registered_dependency_generation,scope) VALUES(1,'raw-sd-1','raw-ar-1',1,'');",
    );
    assert_raw_sql_drifts(
        "raw dc INSERT",
        false,
        "INSERT INTO _fathomdb_dependency_closures(schema_version,closure_operation_id,root_kind,root_value,cause,effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,closure_sequence,retry_fingerprint,phase,affected_count,scope) VALUES(1,'raw-dc-1','source_bucket','raw-src','soft_deleted',0,1,1,999,'0000000000000000000000000000000000000000000000000000000000000000','proving',1,'');",
    );
    // Class 1E: os (drifts OperationalState, does NOT drift Doc SearchNoGraph)
    assert_raw_sql_drifts(
        "raw os INSERT",
        true,
        "INSERT INTO operational_state(collection_name,record_key,payload_json,schema_id,write_cursor,scope) VALUES('fc_state','raw-k','{}',NULL,90004,'');",
    );
    // Class 2: ca, cpv, vr, vs, pt
    assert_raw_sql_drifts(
        "raw ca INSERT",
        false,
        "INSERT INTO canonical_attributes(write_cursor,attr_name,attr_value) VALUES(90005,'k','v');",
    );
    assert_raw_sql_drifts(
        "raw cpv INSERT",
        false,
        "INSERT INTO canonical_property_values(entity_class,write_cursor,attr_name,value_type,text_val) VALUES('node',90005,'k','string','v');",
    );
    assert_raw_sql_drifts(
        "raw vr INSERT",
        false,
        "INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(90006,'Doc',90006);",
    );
    assert_raw_sql_drifts(
        "raw vs INSERT",
        false,
        "INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length) VALUES(90006,90006,0,0,1,1);",
    );
    assert_raw_sql_drifts(
        "raw pt INSERT",
        false,
        "INSERT INTO _fathomdb_projection_terminal(write_cursor,state,owner_kind) VALUES(90007,'up_to_date','canonical');",
    );
    // Class 3: oc, pr, vk, ep, pg, pc, ps
    assert_raw_sql_drifts(
        "raw oc INSERT",
        false,
        "INSERT INTO operational_collections(name,kind,schema_json,retention_json,format_version,created_at) VALUES('raw_col','append_only_log','{}','{}',1,0);",
    );
    assert_raw_sql_drifts(
        "raw pr INSERT",
        false,
        "INSERT INTO _fathomdb_projection_registry(name,roles,entity_class) VALUES('raw_pr','[\"filterable\"]','node');",
    );
    conn.execute("DELETE FROM _fathomdb_projection_registry WHERE name = 'raw_pr'", []).unwrap();
    assert_raw_sql_drifts(
        "raw vk INSERT",
        false,
        "INSERT INTO _fathomdb_vector_kinds(kind,profile,created_at) VALUES('RawKind','default',0);",
    );
    assert_raw_sql_drifts(
        "raw ep UPDATE",
        false,
        "UPDATE _fathomdb_embedder_profiles SET revision = revision WHERE profile = 'default';",
    );
    assert_raw_sql_drifts(
        "raw pg INSERT",
        false,
        "INSERT INTO _fathomdb_projection_generations(schema_version,generation_id,declaration_sha256,transition_boundary,role,origin,retired_boundary) VALUES(1,'pgen1:00000000000000000000000000000000','0000000000000000000000000000000000000000000000000000000000000000',0,'retired','fresh',0);",
    );
    assert_raw_sql_drifts(
        "raw pc UPDATE",
        false,
        "UPDATE _fathomdb_projection_generation_current SET generation_id = generation_id WHERE singleton = 1;",
    );
    assert_raw_sql_drifts(
        "raw ps INSERT",
        false,
        "INSERT INTO _fathomdb_projection_state(kind,last_enqueued_cursor,updated_at) VALUES('RawKind',1,0);",
    );

    drop(conn);
    engine.close().unwrap();
}

#[test]
fn wire_token_size_legacy_compat_and_validate_latency_budget() {
    let dir = TempDir::new().unwrap();

    // 1. Exact Schema 39 wire token sizes: 492 B unscoped, 556 B scoped (both <= 1024 B).
    let unscoped_path = dir.path().join(format!("ac06-unscoped{SQLITE_SUFFIX}"));
    let eng_u = Engine::open(&unscoped_path).unwrap().engine;
    let f_u = eng_u.freeze_read_context(&doc_context()).unwrap();
    assert_eq!(
        f_u.token.len(),
        492,
        "Schema 39 Unscoped fdbfr1 token must be exactly 492 bytes (got {})",
        f_u.token.len()
    );
    assert!(f_u.token.len() <= 1024);

    // Measure validate_snapshot stage timing cleanly
    let page_req = PageRequestV1 { schema_version: 1, limit: 10, cursor: None };
    let mut samples_ns = Vec::with_capacity(25);
    for _ in 0..25 {
        let timing = eng_u.measure_slice45_frozen_stages_for_test(&f_u, &page_req).unwrap();
        samples_ns.push(timing.snapshot_binding_ns);
    }
    samples_ns.sort_unstable();
    let p50_ns = samples_ns[samples_ns.len() / 2];
    assert!(
        p50_ns < 5_000_000, // generous CI wall-clock upper bound (5 ms); 50k bench is ~0.008 ms
        "unexpectedly slow validate_snapshot p50: {p50_ns} ns"
    );
    eng_u.close().unwrap();

    let scoped_path = dir.path().join(format!("ac06-scoped{SQLITE_SUFFIX}"));
    let eng_s = open_scoped(&scoped_path, "tenant-a");
    let f_s = eng_s.freeze_read_context(&doc_context()).unwrap();
    assert_eq!(
        f_s.token.len(),
        556,
        "Schema 39 Scoped fdbfr1 token must be exactly 556 bytes (got {})",
        f_s.token.len()
    );
    assert!(f_s.token.len() <= 1024);
    eng_s.close().unwrap();

    // 2. Verify golden 0.8.31 Schema 38 -> 39 migration creates _fathomdb_scope_visibility_watermarks
    //    and 64-char lowercase hex global_struct_nonce, and opens cleanly.
    let golden_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.31-schema38.sqlite");
    let migrated_db = dir.path().join("migrated-38-to-39.sqlite");
    std::fs::copy(&golden_src, &migrated_db).unwrap();
    let backup_dir = dir.path().join("backups");
    let report = migrate_38_to_39(&migrated_db, &backup_dir).unwrap();
    assert_eq!(report.from_version, 38);
    assert_eq!(report.to_version, 39);
    {
        let conn = Connection::open(&migrated_db).unwrap();
        let (gs_gen, gs_nonce): (i64, String) = conn
            .query_row(
                "SELECT global_struct_gen, global_struct_nonce FROM _fathomdb_read_visibility_state WHERE singleton = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(gs_gen, 0);
        assert_eq!(gs_nonce.len(), 64);
        assert!(gs_nonce.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        let wm_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_fathomdb_scope_visibility_watermarks'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(wm_exists, 1);
    }
    let opened = Engine::open_with_choice_and_config(
        &migrated_db,
        EmbedderChoice::None,
        EngineConfig::default(),
    )
    .unwrap();
    let started = Instant::now();
    let f_m = opened.engine.freeze_read_context(&default_context()).unwrap();
    assert_eq!(f_m.token.len(), 492);
    assert!(started.elapsed() < Duration::from_secs(5));
    opened.engine.close().unwrap();
}
