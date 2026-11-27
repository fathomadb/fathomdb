#![no_main]

use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    decode_dependency_trace_result_v1, decode_graph_expand_request_v1,
    decode_graph_expand_result_v1, decode_logical_export_v1, decode_round_body,
    decode_round_body_json, encode_logical_export_v1, encode_round_body, encode_round_body_json,
    validate_logical_export_v1, ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId,
    CanonicalHash, ContextPart, EmbedderChoice, Engine, InitialState, LatestStateActuationV1,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteOptions, WriteProvenanceV1, LOGICAL_EXPORT_FORMAT_V1,
};
use libfuzzer_sys::fuzz_target;
use tempfile::TempDir;

struct WireFixture {
    _dir: TempDir,
    engine: Engine,
    accepted_writes: u64,
}

fn fixture() -> &'static Mutex<WireFixture> {
    static FIXTURE: OnceLock<Mutex<WireFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_wire_codecs");
        let path = dir.path().join("wire_fuzz.sqlite");
        let engine = Engine::open_with_choice(&path, EmbedderChoice::None)
            .expect("open wire fixture engine")
            .engine;
        Mutex::new(WireFixture { _dir: dir, engine, accepted_writes: 0 })
    })
}

fn arb_str(u: &mut Unstructured<'_>, max_len: usize) -> String {
    let raw: String = u.arbitrary().unwrap_or_default();
    raw.chars().take(max_len).collect()
}

fn arb_opt_str(u: &mut Unstructured<'_>, max_len: usize) -> Option<String> {
    if u.arbitrary().unwrap_or(false) {
        Some(arb_str(u, max_len))
    } else {
        None
    }
}

fn arb_f64_edge(u: &mut Unstructured<'_>) -> f64 {
    match u.int_in_range(0..=7).unwrap_or(0) {
        0 => f64::NAN,
        1 => f64::INFINITY,
        2 => f64::NEG_INFINITY,
        3 => -0.0,
        4 => 0.0,
        5 => 1.0,
        6 => -0.5,
        _ => f64::from_bits(u.arbitrary().unwrap_or(0)),
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // 1. Always fuzz raw wire decoders directly on untrusted bytes.
    if let Ok(archive) = decode_logical_export_v1(data) {
        // Any decoded archive must validate and re-encode cleanly.
        assert!(validate_logical_export_v1(&archive).is_ok());
        assert!(encode_logical_export_v1(&archive).is_ok());
    }
    let _ = decode_graph_expand_request_v1(data);
    let _ = decode_graph_expand_result_v1(data);
    let _ = decode_dependency_trace_result_v1(data);

    if let Ok(text) = std::str::from_utf8(data) {
        let _ = decode_round_body(text);
        let _ = decode_round_body_json(text);
        let _ = encode_round_body_json(text);
    }

    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=3).unwrap_or(0);

    match mode {
        0 => {
            // Structured fathomdb.logical-export/v1 JSON archive fuzzing.
            let include_audit = u.arbitrary().unwrap_or(false);
            let include_vectors = u.arbitrary().unwrap_or(false);
            let policy = match u.int_in_range(0..=2).unwrap_or(0) {
                0 => "autonomous".to_string(),
                1 => "explicit_maintenance".to_string(),
                _ => arb_str(&mut u, 24),
            };
            let dim = u.int_in_range(0..=8).unwrap_or(4);
            let vec_len = u.int_in_range(0..=8).unwrap_or(dim);
            let mut emb = Vec::with_capacity(vec_len);
            for _ in 0..vec_len {
                let v = if u.arbitrary().unwrap_or(true) { 0.25_f32 } else { 1.0_f32 };
                emb.push(v);
            }
            let payload = serde_json::json!({
                "format_version": if u.arbitrary().unwrap_or(true) {
                    LOGICAL_EXPORT_FORMAT_V1.to_string()
                } else {
                    arb_str(&mut u, 32)
                },
                "schema_version": if u.arbitrary().unwrap_or(true) { 39_u32 } else { u.arbitrary().unwrap_or(0) },
                "include_audit": include_audit,
                "include_vectors": include_vectors,
                "physical_erasure_policy": policy,
                "dependency_generation": u.arbitrary::<u64>().unwrap_or(0),
                "authority": {
                    "canonical_nodes": [{
                        "write_cursor": u.arbitrary::<i64>().unwrap_or(1),
                        "kind": arb_str(&mut u, 24),
                        "body": arb_str(&mut u, 64),
                        "source_id": arb_opt_str(&mut u, 24),
                        "logical_id": arb_opt_str(&mut u, 24),
                        "superseded_at": Option::<i64>::None,
                        "row_kind": if u.arbitrary().unwrap_or(true) { "leaf".to_string() } else { arb_str(&mut u, 16) },
                        "importance": Option::<f64>::None,
                        "state": if u.arbitrary().unwrap_or(true) { "active".to_string() } else { arb_str(&mut u, 16) },
                        "reason": Option::<String>::None,
                        "valid_from": Option::<i64>::None,
                        "valid_until": Option::<i64>::None,
                        "properties": arb_opt_str(&mut u, 64),
                        "scope": arb_str(&mut u, 24),
                    }],
                    "canonical_edges": [],
                    "operational_collections": [],
                    "operational_mutations": [],
                    "operational_state": [],
                    "artifact_revisions": [],
                    "source_versions": [],
                    "source_links": [],
                    "source_dependencies": [],
                    "projection_registry": [],
                    "vector_kinds": [],
                    "commit_timestamps": []
                },
                "audit": if include_audit {
                    Some(serde_json::json!({
                        "actuation_receipts": [],
                        "actuation_receipt_source_refs": [],
                        "dependency_closures": [{
                            "schema_version": 1,
                            "closure_operation_id": arb_str(&mut u, 24),
                            "root_kind": "source_id",
                            "root_value": "s1",
                            "cause": if u.arbitrary().unwrap_or(true) { "purged" } else { "superseded" },
                            "effective_at_epoch_s": 1,
                            "admitted_write_boundary": 1,
                            "admitted_dependency_generation": 0,
                            "closure_sequence": 1,
                            "retry_fingerprint": "fp",
                            "phase": if u.arbitrary().unwrap_or(true) { "complete" } else { "pending" },
                            "affected_count": 0,
                            "blocker_code": null,
                            "structural_proof_write_boundary": null,
                            "proof_json": null,
                            "scope": ""
                        }]
                    }))
                } else {
                    None
                },
                "vectors": if include_vectors {
                    Some(serde_json::json!({
                        "embedder": {
                            "profile": if u.arbitrary().unwrap_or(true) { "default".to_string() } else { arb_str(&mut u, 16) },
                            "name": arb_str(&mut u, 24),
                            "revision": arb_str(&mut u, 24),
                            "dimension": dim as u32
                        },
                        "rows": [{
                            "rowid": u.arbitrary::<i64>().unwrap_or(1),
                            "write_cursor": u.arbitrary::<u64>().unwrap_or(1),
                            "kind": "note",
                            "created_at": 1,
                            "window_number": u.arbitrary::<i64>().unwrap_or(0),
                            "byte_start": u.arbitrary::<i64>().unwrap_or(0),
                            "byte_end": Option::<i64>::None,
                            "body_length": u.arbitrary::<i64>().unwrap_or(10),
                            "embedding": emb
                        }]
                    }))
                } else {
                    None
                }
            });
            let encoded = payload.to_string();
            if let Ok(archive) = decode_logical_export_v1(encoded.as_bytes()) {
                let _ = encode_logical_export_v1(&archive);
            }
        }
        1 => {
            // Identity constructors + ActuationBatchV1 construction & validation.
            let op_id = arb_str(&mut u, 96);
            let policy_id = arb_opt_str(&mut u, 96);
            let mut ops = Vec::new();
            let op_count = u.int_in_range(0..=6).unwrap_or(0);
            for _ in 0..op_count {
                match u.int_in_range(0..=5).unwrap_or(0) {
                    0 => {
                        if let (Ok(src), Ok(rev), Ok(sv)) = (
                            SourceId::new(arb_str(&mut u, 40)),
                            ArtifactRevisionId::new(arb_str(&mut u, 40)),
                            SourceVersionId::new(arb_str(&mut u, 40)),
                        ) {
                            ops.push(ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                                kind: arb_str(&mut u, 24),
                                body: arb_str(&mut u, 80),
                                source_id: src,
                                logical_id: arb_opt_str(&mut u, 32),
                                state: if u.arbitrary().unwrap_or(true) {
                                    InitialState::Active
                                } else {
                                    InitialState::Pending
                                },
                                reason: arb_opt_str(&mut u, 24),
                                valid_from: u.arbitrary().ok(),
                                valid_until: u.arbitrary().ok(),
                                properties: None,
                                provenance: WriteProvenanceV1::canonical(rev, sv),
                            }));
                        }
                    }
                    1 => {
                        if let (Ok(src), Ok(rev), Ok(sv), Ok(srev), Ok(hash)) = (
                            SourceId::new(arb_str(&mut u, 40)),
                            ArtifactRevisionId::new(arb_str(&mut u, 40)),
                            SourceVersionId::new(arb_str(&mut u, 40)),
                            SourceRevisionId::new(arb_str(&mut u, 40)),
                            CanonicalHash::sha256(arb_str(&mut u, 72)),
                        ) {
                            let locator = if u.arbitrary().unwrap_or(true) {
                                SourceLocator::whole_body()
                            } else {
                                SourceLocator::utf8_bytes(
                                    u.arbitrary().unwrap_or(0),
                                    u.arbitrary().unwrap_or(0),
                                )
                            };
                            ops.push(ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
                                kind: arb_str(&mut u, 24),
                                from: arb_str(&mut u, 32),
                                to: arb_str(&mut u, 32),
                                source_id: src,
                                logical_id: arb_opt_str(&mut u, 32),
                                body: arb_opt_str(&mut u, 48),
                                t_valid: u.arbitrary().ok(),
                                t_invalid: u.arbitrary().ok(),
                                confidence: if u.arbitrary().unwrap_or(false) {
                                    Some(arb_f64_edge(&mut u))
                                } else {
                                    None
                                },
                                extractor_model_id: arb_opt_str(&mut u, 24),
                                temporal_fallback: u.arbitrary().ok(),
                                properties: None,
                                provenance: WriteProvenanceV1::derived(
                                    rev, sv, srev, locator, hash,
                                ),
                            }));
                        }
                    }
                    2 => {
                        if let Ok(reg) = SourceDependencyRegistrationV1::new(
                            arb_str(&mut u, 40),
                            arb_str(&mut u, 40),
                            arb_str(&mut u, 40),
                        ) {
                            ops.push(ActuationOperationV1::RegisterSourceDependency(reg));
                        }
                    }
                    3 => {
                        if let Ok(rev) = ArtifactRevisionId::new(arb_str(&mut u, 40)) {
                            let target = match u.int_in_range(0..=3).unwrap_or(0) {
                                0 => LifecycleState::Active,
                                1 => LifecycleState::Deleted,
                                2 => LifecycleState::Pending,
                                _ => LifecycleState::Purged,
                            };
                            if let Ok(lc) = LifecycleActuationV1::new(
                                arb_str(&mut u, 40),
                                rev,
                                target,
                                arb_opt_str(&mut u, 24),
                            ) {
                                ops.push(ActuationOperationV1::TransitionLifecycle(lc));
                            }
                        }
                    }
                    _ => {
                        if let Ok(ls) = LatestStateActuationV1::new(
                            arb_str(&mut u, 32),
                            arb_str(&mut u, 32),
                            arb_str(&mut u, 96),
                        ) {
                            ops.push(ActuationOperationV1::PutLatestState(ls));
                        }
                    }
                }
            }
            if let Ok(mut batch) = ActuationBatchV1::new(op_id, ops) {
                if let Some(pid) = policy_id {
                    if let Ok(b2) = batch.clone().with_decision_policy_id(pid) {
                        batch = b2;
                    }
                }
                if u.arbitrary().unwrap_or(false) {
                    batch = batch.with_expected_write_boundary(u.arbitrary().unwrap_or(0));
                }
                let mut guard = fixture().lock().unwrap_or_else(|p| p.into_inner());
                if guard.accepted_writes < 256 {
                    if guard.engine.actuate(batch).is_ok() {
                        guard.accepted_writes += 1;
                    }
                }
            }
        }
        2 => {
            // PreparedWrite::Node / Edge validation & write path.
            let mut batch = Vec::new();
            let count = u.int_in_range(0..=4).unwrap_or(0);
            for _ in 0..count {
                if let Ok(src) = SourceId::new(arb_str(&mut u, 32)) {
                    if u.arbitrary().unwrap_or(true) {
                        batch.push(PreparedWrite::Node {
                            kind: arb_str(&mut u, 24),
                            body: arb_str(&mut u, 96),
                            source_id: src,
                            logical_id: arb_opt_str(&mut u, 32),
                            state: if u.arbitrary().unwrap_or(true) {
                                InitialState::Active
                            } else {
                                InitialState::Pending
                            },
                            reason: arb_opt_str(&mut u, 24),
                            valid_from: if u.arbitrary().unwrap_or(false) {
                                Some(u.arbitrary().unwrap_or(0))
                            } else {
                                None
                            },
                            valid_until: if u.arbitrary().unwrap_or(false) {
                                Some(u.arbitrary().unwrap_or(0))
                            } else {
                                None
                            },
                            properties: None,
                        });
                    } else {
                        batch.push(PreparedWrite::Edge {
                            kind: arb_str(&mut u, 24),
                            from: arb_str(&mut u, 32),
                            to: arb_str(&mut u, 32),
                            source_id: src,
                            logical_id: arb_opt_str(&mut u, 32),
                            body: arb_opt_str(&mut u, 64),
                            t_valid: if u.arbitrary().unwrap_or(false) {
                                Some(u.arbitrary().unwrap_or(0))
                            } else {
                                None
                            },
                            t_invalid: if u.arbitrary().unwrap_or(false) {
                                Some(u.arbitrary().unwrap_or(0))
                            } else {
                                None
                            },
                            confidence: if u.arbitrary().unwrap_or(false) {
                                Some(arb_f64_edge(&mut u))
                            } else {
                                None
                            },
                            extractor_model_id: arb_opt_str(&mut u, 24),
                            temporal_fallback: u.arbitrary().ok(),
                            properties: None,
                        });
                    }
                }
            }
            let opts = if u.arbitrary().unwrap_or(true) {
                WriteOptions::strict()
            } else {
                WriteOptions::bulk_non_strict()
            };
            let mut guard = fixture().lock().unwrap_or_else(|p| p.into_inner());
            if guard.accepted_writes < 256 {
                if guard.engine.write_with_options(&batch, &opts).is_ok() {
                    guard.accepted_writes += 1;
                }
            }
        }
        _ => {
            // ContextPack round-body encode/decode round-trip.
            let n = u.int_in_range(0..=5).unwrap_or(0);
            let mut parts = Vec::with_capacity(n);
            for _ in 0..n {
                parts.push(ContextPart {
                    role: arb_opt_str(&mut u, 24),
                    text: arb_str(&mut u, 128),
                });
            }
            if let Ok(encoded) = encode_round_body(&parts) {
                let decoded = decode_round_body(&encoded);
                assert!(decoded.is_ok(), "encode_round_body output must decode cleanly");
            }
        }
    }
});
