#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Barrier};
use std::thread;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, EmbedderChoice, Engine,
    EngineConfig, EngineError, EvidenceSearchRequestV1, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, InitialState, MutationProjectionStatusRequestV1, PhysicalErasurePolicy,
    PreparedWrite, ProjectionFts, ProjectionGenerationErrorReason, ProjectionReadinessV1,
    ProjectionRole, ProjectionSpec, ProjectionVector, PropertyEntityClass, ProvenancedNodeV1,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceId, SourceVersionId,
    StoredHeadCoverageUnavailableReason, StoredHeadCoverageV1, TestEmbedderChoice,
    TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedTestEmbedder;

impl Embedder for FixedTestEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-status-cache-test", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.5, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0])
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

fn node_write(id: &str, body: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn seed_store(path: &std::path::Path) -> Vec<u64> {
    let opened =
        Engine::open_with_embedder_for_test(path, Arc::new(FixedTestEmbedder)).expect("open seed");
    opened.engine.configure_projections(&[vector_spec()], &[]).expect("configure projections");
    let mut ops = Vec::new();
    for i in 0..8 {
        ops.push(ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
            kind: "doc".into(),
            body: format!("aurora quantum telemetry document {i}"),
            source_id: SourceId::new(format!("src:seed-{i}")).unwrap(),
            logical_id: Some(format!("doc-{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new(format!("rev-seed-{i}")).unwrap(),
                SourceVersionId::new(format!("ver-seed-{i}")).unwrap(),
            ),
            properties: None,
        }));
    }
    let act = opened
        .engine
        .actuate(ActuationBatchV1::new("seed-status-cache", ops).unwrap())
        .expect("actuate seed");
    opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "relates".into(),
            from: "doc-0".into(),
            to: "doc-1".into(),
            source_id: SourceId::new("src:edge-0").unwrap(),
            logical_id: Some("edge-0".into()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.8),
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        }])
        .expect("write seed edge");
    opened.engine.drain(30_000).expect("drain seed");
    opened.engine.close().expect("close seed");
    act.pending_projection_write_cursors
}

fn run_mixed_pooled_call(
    engine: &Engine,
    frozen: &fathomdb_engine::FrozenReadContextV1,
    call_idx: usize,
) {
    match call_idx % 4 {
        0 => {
            let res = engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
            assert!(res.explanation.is_some());
        }
        1 => {
            let res =
                engine.search_frozen("aurora quantum", frozen, 0, false, 0.3, 0, true, 10).unwrap();
            assert!(res.explanation.is_some());
        }
        2 => {
            let mut req = EvidenceSearchRequestV1::new("aurora quantum".into(), frozen.clone());
            req.include_explanation = true;
            let res = engine.search_with_evidence(&req).unwrap();
            assert!(res.search_result.explanation.is_some());
        }
        _ => {
            let req = GraphExpandRequestV1::new(
                GraphSeedV1::Query {
                    schema_version: 1,
                    text: "aurora quantum".into(),
                    ranked_limit: 5,
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Current {
                    schema_version: 1,
                    context: ReadContextV1::new(ReadView::default(), SearchFilter::default())
                        .unwrap(),
                },
                1,
                10,
                100,
            );
            let _res = engine.graph_expand(&req).unwrap();
        }
    }
}

fn assert_pooled_o1_cache(engine: Arc<Engine>) {
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();

    // Serialize 1 cold first query before round-robin and concurrent bursts.
    let before_cold = engine.projection_generation_status_full_owner_scan_count_for_test();
    let first = engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
    assert!(first.explanation.is_some());
    let after_cold = engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(
        after_cold - before_cold,
        1,
        "cold first query on seeded startup must perform exactly 1 full owner scan"
    );

    // 24 sequential round-robin mixed calls spanning all 8 reader workers -> delta == 0.
    for idx in 0..24 {
        run_mixed_pooled_call(&engine, &frozen, idx);
    }
    let after_seq = engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(
        after_seq - after_cold,
        0,
        "24 sequential pooled calls across all 8 workers must hit cache with delta == 0"
    );

    // Concurrent mixed burst across 8 threads (32 calls total) -> delta == 0.
    let barrier = Arc::new(Barrier::new(8));
    let mut handles = Vec::new();
    for tid in 0..8 {
        let engine = Arc::clone(&engine);
        let frozen = frozen.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            for step in 0..4 {
                run_mixed_pooled_call(&engine, &frozen, tid + step);
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let after_conc = engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(
        after_conc - after_seq,
        0,
        "concurrent mixed burst across all 8 workers must hit cache with delta == 0"
    );
}

#[test]
fn seeded_startup_pooled_reader_cache_without_embedder() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("status-cache-absent{SQLITE_SUFFIX}"));
    seed_store(&path);

    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("reopen absent");
    let engine = Arc::new(opened.engine);
    assert_pooled_o1_cache(Arc::clone(&engine));
    engine.close().unwrap();
}

#[test]
fn seeded_startup_pooled_reader_cache_with_embedder() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("status-cache-usable{SQLITE_SUFFIX}"));
    seed_store(&path);

    let opened = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedTestEmbedder)),
        EngineConfig::default(),
    )
    .expect("reopen usable");
    // Serialize drain() before measuring so no background projection commits race.
    opened.engine.drain(30_000).expect("drain after reopen");
    let engine = Arc::new(opened.engine);
    assert_pooled_o1_cache(Arc::clone(&engine));
    engine.close().unwrap();
}

#[test]
fn internal_write_and_projection_publication_invalidate_immediately_then_settle() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("status-cache-write-inval{SQLITE_SUFFIX}"));
    seed_store(&path);

    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).unwrap();
    opened.engine.drain(30_000).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);

    // Warm the cache.
    opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
    let status_before = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status_before.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status_before.pending_count, 0);

    // Write a new node while scheduler is frozen -> advances visibility_generation.
    let receipt = opened
        .engine
        .write(&[node_write("doc-new", "aurora quantum fresh pending", "src:new")])
        .unwrap();
    let new_cursor = receipt.row_cursors[0];

    let status_after_write = opened.engine.read_projection_generation_status().unwrap();
    assert!(status_after_write.observed_boundary >= new_cursor);
    assert_eq!(status_after_write.pending_count, 1);
    assert_eq!(status_after_write.readiness, ProjectionReadinessV1::Processing);

    // Each worker entering the new visibility_generation verifies at most once (<= 8 scans).
    let scans_before_workers =
        opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    for _ in 0..16 {
        let res = opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
        assert!(res.explanation.is_some());
    }
    let scans_after_workers =
        opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    assert!(
        scans_after_workers - scans_before_workers <= 8,
        "at most 1 verification scan per worker after visibility advance, got {}",
        scans_after_workers - scans_before_workers
    );

    // Once all 8 workers verified the new visibility_generation, steady state is delta == 0.
    for _ in 0..16 {
        opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
    }
    let scans_steady = opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(
        scans_steady - scans_after_workers,
        0,
        "steady-state pooled reads after worker verification must have delta == 0"
    );

    // Unfreeze & drain -> projection publication invalidates and settles back to Ready.
    opened.engine.set_projection_scheduler_frozen_for_test(false);
    opened.engine.drain(30_000).unwrap();
    let status_ready = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status_ready.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status_ready.pending_count, 0);

    opened.engine.close().unwrap();
}

fn assert_corrupt_err<T: std::fmt::Debug>(res: Result<T, EngineError>, context: &str) {
    assert!(
        matches!(
            res,
            Err(EngineError::ProjectionGeneration(ref e))
                if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
        ),
        "expected ProjectionGenerationCorrupt for {context}, got {res:?}"
    );
}

#[test]
fn external_vector_default_tamper_detected_cold_warmed_and_cross_slot() {
    // (a) Cold-after-open tamper (no query before external vector_default DELETE)
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tamper-cold{SQLITE_SUFFIX}"));
        let cursors = seed_store(&path);
        let opened = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default(),
        )
        .unwrap();

        let ext = Connection::open(&path).unwrap();
        let vis_before: i64 = ext
            .query_row(
                "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        ext.execute("DELETE FROM vector_default WHERE rowid=?1", [cursors[0]]).unwrap();
        let vis_after: i64 = ext
            .query_row(
                "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vis_before, vis_after, "vector_default has no visibility trigger");
        drop(ext);

        assert_corrupt_err(
            opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0),
            "cold pooled search after external vector_default tamper",
        );
        assert_corrupt_err(
            opened.engine.read_projection_generation_status(),
            "cold primary status after external vector_default tamper",
        );
        opened.engine.close().unwrap();
    }

    // (b) Warmed tamper across all 8 workers and Primary
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tamper-warmed{SQLITE_SUFFIX}"));
        let cursors = seed_store(&path);
        let opened = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default(),
        )
        .unwrap();

        for _ in 0..16 {
            opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
        }
        opened.engine.read_projection_generation_status().unwrap();

        let ext = Connection::open(&path).unwrap();
        ext.execute("DELETE FROM vector_default WHERE rowid=?1", [cursors[0]]).unwrap();
        drop(ext);

        for slot in 0..8 {
            assert_corrupt_err(
                opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0),
                &format!("warmed pooled slot {slot} after external vector_default tamper"),
            );
        }
        assert_corrupt_err(
            opened.engine.read_projection_generation_status(),
            "warmed primary after external vector_default tamper",
        );
        opened.engine.close().unwrap();
    }

    // (c) Cross-slot tamper after visibility advance:
    // open -> write advancing vis_gen -> warm W0 only -> external vector_default tamper ->
    // query W1..W7 and Primary -> EVERY slot MUST detect corruption (no blind trust of W0!).
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("tamper-cross-slot{SQLITE_SUFFIX}"));
        let cursors = seed_store(&path);
        let opened = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default(),
        )
        .unwrap();

        // Advance visibility_generation with an internal write (workers W0..W7 still hold open_vis_gen).
        opened
            .engine
            .write(&[node_write("doc-vis-bump", "aurora quantum vis bump", "src:vis-bump")])
            .unwrap();

        // Warm ONLY W0 at the new visibility_generation.
        let res_w0 =
            opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
        assert!(res_w0.explanation.is_some());

        // External tamper on vector_default (no visibility_generation bump!).
        let ext = Connection::open(&path).unwrap();
        ext.execute("DELETE FROM vector_default WHERE rowid=?1", [cursors[0]]).unwrap();
        drop(ext);

        // W1..W7 AND W0 AND Primary MUST ALL detect the tamper!
        for slot in 0..8 {
            assert_corrupt_err(
                opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0),
                &format!("cross-slot worker {slot} after vis_gen advance + W0 warm + tamper"),
            );
        }
        assert_corrupt_err(
            opened.engine.read_projection_generation_status(),
            "cross-slot primary after vis_gen advance + W0 warm + tamper",
        );
        opened.engine.close().unwrap();
    }
}

#[test]
fn stored_head_coverage_available_after_pooled_warm_and_erasure_boundary_parity() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("coverage-and-erasure{SQLITE_SUFFIX}"));
    let cursors = seed_store(&path);

    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::Autonomous),
            ..EngineConfig::default()
        },
    )
    .unwrap();

    // Write a new node so stored_head_coverage transitions to Unavailable(StateChanged).
    let extra_cursor = opened
        .engine
        .write(&[node_write("doc-cov-warm", "aurora quantum coverage warm", "src:cov-warm")])
        .unwrap()
        .row_cursors[0];
    let cold_cov = opened.engine.read_projection_status().unwrap().vector_coverage;
    assert!(
        matches!(
            cold_cov,
            StoredHeadCoverageV1::Unavailable {
                reason: StoredHeadCoverageUnavailableReason::StateChanged,
                ..
            }
        ),
        "expected Unavailable(StateChanged) after write before status read, got {cold_cov:?}"
    );

    // Warm ProjectionGenerationStatusCache via pooled reader search(..., explain=true).
    let before = opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
    let after_pooled = opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(after_pooled - before, 1);

    // Primary read_projection_generation_status hits or verifies the entry and publishes
    // Available stored_head_coverage! (First Primary visit to the new vis_gen does 1
    // verification scan OR hits if Primary already observed this vis_gen; either way,
    // stored_head_coverage MUST transition to Available cleanly!)
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert!(status.observed_boundary >= extra_cursor);
    let warm_cov = opened.engine.read_projection_status().unwrap().vector_coverage;
    assert!(
        matches!(warm_cov, StoredHeadCoverageV1::Available { .. }),
        "read_projection_status().vector_coverage must be Available after pooled warm + Primary status read, got {warm_cov:?}"
    );

    // Also test reopen + pooled warm + Primary hit (delta == 0) + Available coverage on a fresh store
    // where stored_head_coverage starts Unavailable (after write), then Primary observes same vis_gen
    // and pooled reader warms cache before Primary read_projection_generation_status!
    let act_receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "wp40d-coverage-op",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: "aurora quantum actuated coverage node".into(),
                    source_id: SourceId::new("src:actuated-cov").unwrap(),
                    logical_id: Some("act-cov-1".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("rev-cov-1").unwrap(),
                        SourceVersionId::new("ver-cov-1").unwrap(),
                    ),
                    properties: None,
                })],
            )
            .unwrap(),
        )
        .unwrap();
    let op_cursor = act_receipt.pending_projection_write_cursors[0];
    let op_gen = act_receipt.projection_generation_id.unwrap();
    // Coverage is Unavailable(StateChanged) after actuate.
    assert!(matches!(
        opened.engine.read_projection_status().unwrap().vector_coverage,
        StoredHeadCoverageV1::Unavailable {
            reason: StoredHeadCoverageUnavailableReason::StateChanged,
            ..
        }
    ));
    // Warm via pooled reader, then call read_mutation_projection_status -> Available!
    opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0).unwrap();
    let _mut_status = opened
        .engine
        .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: "wp40d-coverage-op".into(),
            write_cursor: op_cursor,
            expected_generation_id: op_gen,
        })
        .unwrap();
    let cov_after_mut = opened.engine.read_projection_status().unwrap().vector_coverage;
    assert!(
        matches!(cov_after_mut, StoredHeadCoverageV1::Available { .. }),
        "read_mutation_projection_status must publish Available stored_head_coverage after pooled warm, got {cov_after_mut:?}"
    );

    // Physical erasure raising cursor_floor: erase the highest-cursor source and verify
    // observed_boundary includes cursor_floor (never regresses below erased cursor).
    let max_before_erase = op_cursor.max(*cursors.iter().max().unwrap());
    opened.engine.erase_source("src:actuated-cov").unwrap();
    let status_after_erase = opened.engine.read_projection_generation_status().unwrap();
    assert!(
        status_after_erase.observed_boundary >= max_before_erase,
        "snapshot_observed_boundary must fold physical_erasure::cursor_floor: observed={} < erased={}",
        status_after_erase.observed_boundary,
        max_before_erase
    );
    opened.engine.close().unwrap();
}

#[test]
fn interleaved_valid_as_of_no_thrash_and_multi_scope_isolation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("interleaved-valid-as-of{SQLITE_SUFFIX}"));
    {
        let opened =
            Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).unwrap();
        opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
        opened
            .engine
            .write(&[
                node_write("n1", "alpha node one", "src:1"),
                node_write("n2", "beta node two", "src:2"),
                PreparedWrite::Edge {
                    kind: "links".into(),
                    from: "n1".into(),
                    to: "n2".into(),
                    source_id: SourceId::new("src:e1").unwrap(),
                    logical_id: Some("e1".into()),
                    body: Some("expiring edge fact".into()),
                    t_valid: Some(100),
                    t_invalid: Some(200),
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                },
            ])
            .unwrap();
        opened.engine.drain(30_000).unwrap();
        opened.engine.close().unwrap();
    }

    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .unwrap();
    let before = opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    // Cold fill for window [100, 200) and window [200, inf)
    let _s1 = opened.engine.read_projection_generation_status_at_for_test(150).unwrap();
    let _s2 = opened.engine.read_projection_generation_status_at_for_test(250).unwrap();
    let after_two_windows =
        opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(after_two_windows - before, 2);

    // Interleave both windows repeatedly -> bounded LRU (<= 8) retains both with delta == 0!
    for _ in 0..8 {
        let _a = opened.engine.read_projection_generation_status_at_for_test(150).unwrap();
        let _b = opened.engine.read_projection_generation_status_at_for_test(250).unwrap();
    }
    let after_interleave =
        opened.engine.projection_generation_status_full_owner_scan_count_for_test();
    assert_eq!(
        after_interleave - after_two_windows,
        0,
        "interleaved valid_as_of across edge expiry must not thrash cache"
    );
    opened.engine.close().unwrap();

    // Multi-scope isolation: scope A ready, scope B pending; neither contaminates the other.
    let scope_path = dir.path().join(format!("multi-scope-iso{SQLITE_SUFFIX}"));
    {
        let a = Engine::open_with_test_choice_and_config_for_test(
            &scope_path,
            TestEmbedderChoice::Caller(Arc::new(FixedTestEmbedder)),
            EngineConfig::default().with_scope_policy(ScopePolicy::single("scope-a")),
        )
        .unwrap()
        .engine;
        a.configure_projections(&[vector_spec()], &[]).unwrap();
        a.write(&[node_write("a1", "aurora scope a ready", "src:a1")]).unwrap();
        a.drain(30_000).unwrap();
        a.close().unwrap();
    }
    {
        let b = Engine::open_with_choice_and_config(
            &scope_path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(ScopePolicy::single("scope-b")),
        )
        .unwrap()
        .engine;
        b.write(&[node_write("b1", "aurora scope b pending", "src:b1")]).unwrap();
        let status_b = b.read_projection_generation_status().unwrap();
        assert_eq!(status_b.readiness, ProjectionReadinessV1::Blocked);
        assert_eq!(status_b.pending_count, 1);
        b.close().unwrap();
    }
    {
        let a = Engine::open_with_choice_and_config(
            &scope_path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(ScopePolicy::single("scope-a")),
        )
        .unwrap()
        .engine;
        let status_a = a.read_projection_generation_status().unwrap();
        assert_eq!(status_a.readiness, ProjectionReadinessV1::Ready);
        assert_eq!(status_a.pending_count, 0);
        a.close().unwrap();
    }
}

#[test]
fn primary_warmed_then_external_tamper_detected_by_cold_pooled_readers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("tamper-primary-warm-cold-readers{SQLITE_SUFFIX}"));
    let cursors = seed_store(&path);

    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .unwrap();

    // Reader workers must start with zero page-cache misses/loads beyond header probe
    // (lazy sqlite3Init: no full schema parse before first read).
    let cold_cache_status = opened.engine.cache_status_per_worker_for_test("startup");
    assert_eq!(cold_cache_status.len(), 8);
    assert!(
        cold_cache_status.iter().all(|reply| reply.cache_miss <= 2),
        "reader startup should not eagerly scan sqlite_master/tables during Engine::open: {cold_cache_status:?}"
    );

    // Warm the shared status cache via Primary ONLY while all 8 reader workers remain cold.
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.readiness, ProjectionReadinessV1::Ready);

    // External tamper on vector_default (no visibility_generation bump).
    let ext = Connection::open(&path).unwrap();
    ext.execute("DELETE FROM vector_default WHERE rowid=?1", [cursors[0]]).unwrap();
    drop(ext);

    // Every cold reader worker MUST detect the external tamper via PRAGMA data_version
    // and refuse to serve Primary's stale cached Ready status!
    for slot in 0..8 {
        assert_corrupt_err(
            opened.engine.search_explained("aurora quantum", None, 0, false, 0.3, 0),
            &format!(
                "cold pooled worker {slot} after Primary-warm + external vector_default tamper"
            ),
        );
    }
    opened.engine.close().unwrap();
}
