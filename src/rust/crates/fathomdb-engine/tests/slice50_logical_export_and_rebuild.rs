//! Slice 50 (`WP-50B`) — Internal `fathomdb.logical-export/v1` round-trip,
//! `MEAN_VEC_PIN_THRESHOLD` (`256`) mean derivation parity, atomic 4-condition
//! pending physical-erasure refusal, malformed/truncated archive fail-closed
//! rejection, and Engine resource-leak / lifecycle state-machine checks
//! (`AC32-BACKUP-REBUILD-01..04`).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    audit_schema_39_sqlite_master, decode_logical_export_v1, encode_logical_export_v1,
    export_logical_archive_v1, recover_complete_erasures, restore_logical_archive_v1,
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CanonicalHash, CheckIntegrityOpts,
    DataPlaneIntegrityCheckV1, DataPlaneIntegrityRequestV1, EmbedderChoice, Engine, EngineConfig,
    EngineError, EvidenceResolveRequestV1, EvidenceSearchRequestV1, ExportClass,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState,
    LatestStateActuationV1, LogicalExportErrorCode, LogicalExportOptionsV1, PreparedWrite,
    ProjectionRole, ProjectionSpec, PropertyComparisonOp, PropertyEntityClass, PropertyMap,
    PropertyPredicate, PropertyValue, ProvenancedNodeV1, ReadContextV1, ReadView, ScopePolicy,
    SearchFilter, Section, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, TestEmbedderChoice, TraversalDirection, WriteProvenanceV1,
    LOGICAL_EXPORT_FORMAT_V1, MEAN_VEC_PIN_THRESHOLD,
};
use fathomdb_schema::{SCHEMA_VERSION, SQLITE_SUFFIX, TABLE_MANIFEST};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const DIM: u32 = 384;
const BGE_NAME: &str = "fathomdb-bge-small-en-v1.5";
const BGE_REV: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";

#[derive(Clone, Debug)]
struct SimulatedBgeEmbedder {
    identity: EmbedderIdentity,
}

impl Default for SimulatedBgeEmbedder {
    fn default() -> Self {
        Self { identity: EmbedderIdentity::new(BGE_NAME, BGE_REV, DIM) }
    }
}

impl Embedder for SimulatedBgeEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        Ok(deterministic_vector(input))
    }
}

fn deterministic_vector(input: &str) -> Vector {
    let mut seed: u64 = 0xcbf29ce484222325;
    for b in input.bytes() {
        seed ^= u64::from(b);
        seed = seed.wrapping_mul(0x100000001b3);
    }
    let mut v = vec![0.0f32; DIM as usize];
    for (i, slot) in v.iter_mut().enumerate() {
        let mixed = seed.wrapping_add(i as u64).wrapping_mul(2654435761);
        *slot = ((mixed >> 8) as u32 as f32) / (u32::MAX as f32) - 0.35;
    }
    v
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn open_bge_scoped(path: &Path, scope: ScopePolicy) -> Engine {
    Engine::open_with_test_choice_and_config_for_test(
        path,
        TestEmbedderChoice::Caller(Arc::new(SimulatedBgeEmbedder::default())),
        EngineConfig::default().with_scope_policy(scope),
    )
    .expect("open bge scoped engine")
    .engine
}

fn open_empty_schema39_db(path: &Path) -> Connection {
    let engine =
        Engine::open_with_choice(path, EmbedderChoice::None).expect("bootstrap schema39").engine;
    engine.close().expect("close bootstrap engine");
    Connection::open(path).expect("open raw connection")
}

fn assert_integrity_clean(engine: &Engine) {
    let report = engine
        .check_integrity(CheckIntegrityOpts { full: true, ..Default::default() })
        .expect("check_integrity");
    assert_eq!(report.physical, Section::Clean, "{report:?}");
    assert_eq!(report.logical, Section::Clean, "{report:?}");
    assert_eq!(report.semantic, Section::Clean, "{report:?}");

    let dpi = engine
        .check_data_plane_integrity(
            DataPlaneIntegrityRequestV1::new(
                vec![
                    DataPlaneIntegrityCheckV1::DependencyChain,
                    DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
                    DataPlaneIntegrityCheckV1::ProjectionGeneration,
                    DataPlaneIntegrityCheckV1::MutationReadiness,
                    DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity,
                ],
                10_000,
                100,
            )
            .expect("valid dpi request"),
        )
        .expect("check_data_plane_integrity");
    assert!(dpi.findings.is_empty(), "expected 0 data-plane findings, got: {dpi:#?}");
}

#[cfg(target_os = "linux")]
fn count_open_fds() -> usize {
    std::fs::read_dir("/proc/self/fd").expect("read /proc/self/fd").filter_map(Result::ok).count()
}

#[test]
fn logical_export_round_trip_multi_scope_unpinned_mean_and_full_parity() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = dir.path().join(format!("source{SQLITE_SUFFIX}"));
    let dst_path = dir.path().join(format!("restored{SQLITE_SUFFIX}"));

    // Build a real multi-scope Schema 39 store with typed properties, bitemporal
    // history, edges, provenance/evidence spans, operational collections/state,
    // actuation receipts, and < 256 vectors (unpinned mean_vec).
    let scope_a_source_body =
        "Scope alpha canonical document: orbital propulsion telemetry calibration.";
    let span_start = scope_a_source_body.find("orbital propulsion").expect("span start");
    let span_end = span_start + "orbital propulsion".len();
    let span_hash = sha256_hex(scope_a_source_body.as_bytes());

    {
        let eng = open_bge_scoped(&src_path, ScopePolicy::single("scope-a"));
        eng.configure_vector_kind_for_test("doc").expect("vector kind doc");
        eng.configure_vector_kind_for_test("note").expect("vector kind note");
        eng.configure_projections(
            &[
                ProjectionSpec {
                    name: "tier".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "score".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "weight".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Edge,
                },
            ],
            &[],
        )
        .expect("configure projections");

        // Operational collections (both append_only_log and latest_state).
        eng.write(&[
            PreparedWrite::AdminSchema {
                name: "audit_events".into(),
                kind: "append_only_log".into(),
                schema_json: r#"{"type":"object","properties":{"evt":{"type":"string"}},"required":["evt"]}"#.into(),
                retention_json: "{}".into(),
            },
            PreparedWrite::AdminSchema {
                name: "checkpoints".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object","properties":{"step":{"type":"integer"}},"required":["step"]}"#.into(),
                retention_json: "{}".into(),
            },
        ])
        .expect("register collections");

        // Canonical source + derived semantic node with Utf8Bytes evidence span + dependency.
        eng.write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src:alpha".into()),
                kind: "doc".into(),
                body: scope_a_source_body.into(),
                source_id: SourceId::new("src-alpha").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_000),
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-alpha-src-1").unwrap(),
                    SourceVersionId::new("sv-alpha-1").unwrap(),
                ),
                properties: Some(PropertyMap::from([
                    ("tier".into(), PropertyValue::String("gold".into())),
                    ("score".into(), PropertyValue::Float(0.92)),
                ])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("claim:alpha".into()),
                kind: "note".into(),
                body: "Derived orbital propulsion claim alpha".into(),
                source_id: SourceId::new("src-alpha").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_000),
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-alpha-claim-1").unwrap(),
                    SourceVersionId::new("sv-alpha-1").unwrap(),
                    SourceRevisionId::new("rev-alpha-src-1").unwrap(),
                    SourceLocator::utf8_bytes(span_start as u64, span_end as u64),
                    CanonicalHash::sha256(span_hash).unwrap(),
                ),
                properties: Some(PropertyMap::from([
                    ("tier".into(), PropertyValue::String("gold".into())),
                    ("score".into(), PropertyValue::Float(0.95)),
                ])),
            }),
        ])
        .expect("write provenanced nodes");

        eng.register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "dep-alpha-1",
                "rev-alpha-src-1",
                "rev-alpha-claim-1",
            )
            .unwrap(),
        )
        .expect("register source dependency");

        // Bitemporal supersession history on node `history:alpha` + edge with typed property.
        eng.write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "History version one".into(),
            source_id: SourceId::new("src-history").unwrap(),
            logical_id: Some("history:alpha".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(100),
            valid_until: Some(500),
            properties: Some(PropertyMap::from([(
                "tier".into(),
                PropertyValue::String("silver".into()),
            )])),
        }])
        .expect("write history v1");
        eng.drain(15_000).expect("drain after v1");

        eng.write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "History version two current".into(),
                source_id: SourceId::new("src-history").unwrap(),
                logical_id: Some("history:alpha".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500),
                valid_until: None,
                properties: Some(PropertyMap::from([
                    ("tier".into(), PropertyValue::String("gold".into())),
                    ("score".into(), PropertyValue::Float(0.88)),
                ])),
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "claim:alpha".into(),
                to: "history:alpha".into(),
                source_id: SourceId::new("src-alpha").unwrap(),
                logical_id: Some("edge:alpha:1".into()),
                body: Some("Claim alpha supports history alpha".into()),
                t_valid: Some(1_000),
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: Some("extractor-v1".into()),
                temporal_fallback: None,
                properties: Some(PropertyMap::from([(
                    "weight".into(),
                    PropertyValue::Float(0.85),
                )])),
            },
            PreparedWrite::OpStore {
                collection: "audit_events".into(),
                record_key: "evt-1".into(),
                body: r#"{"evt":"seed"}"#.into(),
                schema_id: None,
            },
            PreparedWrite::OpStore {
                collection: "checkpoints".into(),
                record_key: "main".into(),
                body: r#"{"step":7}"#.into(),
                schema_id: None,
            },
        ])
        .expect("write v2, edge, op records");

        // Actuation receipt in scope-a.
        let batch = ActuationBatchV1::new(
            "op-actuate-alpha-1",
            vec![ActuationOperationV1::PutLatestState(
                LatestStateActuationV1::new("checkpoints", "actuated", r#"{"step":9}"#).unwrap(),
            )],
        )
        .expect("actuation batch");
        let receipt = eng.actuate(batch).expect("actuate");
        assert_eq!(receipt.operation_id, "op-actuate-alpha-1");

        eng.drain(15_000).expect("drain scope-a");
        assert_integrity_clean(&eng);
        eng.close().expect("close scope-a");
    }

    // Second scope (`scope-b`) in the same multi-scope store.
    {
        let eng = open_bge_scoped(&src_path, ScopePolicy::single("scope-b"));
        eng.write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "Scope beta isolated maritime navigation log".into(),
                source_id: SourceId::new("src-beta").unwrap(),
                logical_id: Some("doc:beta".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(PropertyMap::from([
                    ("tier".into(), PropertyValue::String("bronze".into())),
                    ("score".into(), PropertyValue::Float(0.45)),
                ])),
            },
            PreparedWrite::OpStore {
                collection: "checkpoints".into(),
                record_key: "main".into(),
                body: r#"{"step":42}"#.into(),
                schema_id: None,
            },
        ])
        .expect("write scope-b");
        eng.drain(15_000).expect("drain scope-b");
        assert_integrity_clean(&eng);
        eng.close().expect("close scope-b");
    }

    // Discharge FTS sanitation certificate before logical export so all 4
    // pending-erasure checks observe a clean store.
    recover_complete_erasures(&src_path).expect("complete erasures before export");

    // Export via internal `fathomdb.logical-export/v1`.
    let mut src_conn = Connection::open(&src_path).expect("open source raw");
    let archive = export_logical_archive_v1(
        &mut src_conn,
        LogicalExportOptionsV1 { include_audit: true, include_vectors: true },
    )
    .expect("export logical archive");
    assert_eq!(archive.format_version, LOGICAL_EXPORT_FORMAT_V1);
    assert_eq!(archive.schema_version, SCHEMA_VERSION);
    assert!(archive.include_audit);
    assert!(archive.include_vectors);

    // Serialize and prove ZERO embedding_bin, ZERO mean_vec, ZERO _fathomdb_embed_probe.
    let encoded = encode_logical_export_v1(&archive).expect("encode archive");
    let encoded_str = std::str::from_utf8(&encoded).expect("utf8 json");
    assert!(!encoded_str.contains("embedding_bin"), "archive must never emit embedding_bin");
    assert!(!encoded_str.contains("mean_vec"), "archive must never emit mean_vec");
    assert!(
        !encoded_str.contains("_fathomdb_embed_probe"),
        "archive must never emit _fathomdb_embed_probe"
    );

    let decoded = decode_logical_export_v1(&encoded).expect("decode archive");
    assert_eq!(decoded, archive);

    // Capture exact original write_cursors from source DB.
    let src_node_cursors: Vec<(i64, String, Option<String>)> = src_conn
        .prepare(
            "SELECT write_cursor, scope, logical_id FROM canonical_nodes ORDER BY write_cursor",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let src_edge_cursors: Vec<(i64, String)> = src_conn
        .prepare("SELECT write_cursor, scope FROM canonical_edges ORDER BY write_cursor")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(src_conn);

    // Restore into a fresh Schema 39 database.
    let mut dst_conn = open_empty_schema39_db(&dst_path);
    let restore_report =
        restore_logical_archive_v1(&mut dst_conn, &decoded).expect("restore archive");
    assert!(!restore_report.mean_vec_pinned, "< 256 vectors must keep mean_vec NULL");
    assert!(restore_report.vectors_restored > 0);

    // Verify exact write_cursor preservation, NULL mean_vec, empty embed_probe before reopen.
    let dst_node_cursors: Vec<(i64, String, Option<String>)> = dst_conn
        .prepare(
            "SELECT write_cursor, scope, logical_id FROM canonical_nodes ORDER BY write_cursor",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let dst_edge_cursors: Vec<(i64, String)> = dst_conn
        .prepare("SELECT write_cursor, scope FROM canonical_edges ORDER BY write_cursor")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(dst_node_cursors, src_node_cursors);
    assert_eq!(dst_edge_cursors, src_edge_cursors);

    let mean_vec_is_null: bool = dst_conn
        .query_row(
            "SELECT mean_vec IS NULL FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(mean_vec_is_null);
    let probe_count: i64 =
        dst_conn.query_row("SELECT COUNT(*) FROM _fathomdb_embed_probe", [], |r| r.get(0)).unwrap();
    assert_eq!(probe_count, 0, "restore must not pre-populate _fathomdb_embed_probe");
    drop(dst_conn);

    // Reopen restored store with matching embedder (seeds fresh _fathomdb_embed_probe)
    // and assert clean integrity + identical revisions, evidence spans, typed properties,
    // scopes, operational state, vector search, and graph expansion!
    let eng_a_src = open_bge_scoped(&src_path, ScopePolicy::single("scope-a"));
    let eng_a_dst = open_bge_scoped(&dst_path, ScopePolicy::single("scope-a"));
    assert_integrity_clean(&eng_a_dst);

    // 1. Typed property search + evidence span resolution parity.
    let mut filter = SearchFilter::default();
    filter.property_predicates.push(PropertyPredicate {
        key: "score".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.90),
    });
    filter.attributes.push(("tier".into(), "gold".into()));
    let ctx_src = eng_a_src
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter.clone()).unwrap())
        .unwrap();
    let ctx_dst = eng_a_dst
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).unwrap())
        .unwrap();
    let ev_src = eng_a_src
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "orbital propulsion".into(),
            ctx_src.clone(),
        ))
        .unwrap();
    let ev_dst = eng_a_dst
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "orbital propulsion".into(),
            ctx_dst.clone(),
        ))
        .unwrap();
    assert_eq!(ev_dst.evidence.len(), ev_src.evidence.len());
    assert!(!ev_dst.evidence.is_empty());
    let claim_ev_dst = ev_dst
        .evidence
        .iter()
        .find(|e| e.artifact_revision_id == "rev-alpha-claim-1")
        .expect("claim revision present in restored evidence");
    let resolved_dst = eng_a_dst
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: claim_ev_dst.evidence_ref.clone(),
            context: ctx_dst,
        })
        .unwrap();
    assert_eq!(resolved_dst.canonical_source_body, scope_a_source_body);
    assert_eq!(resolved_dst.locator, SourceLocator::utf8_bytes(span_start as u64, span_end as u64));

    // 2. Graph expansion parity (with edge property predicate).
    let mut g_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("claim:alpha")],
        },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        2,
        16,
        1_000,
    );
    g_req.edge_predicates.push(PropertyPredicate {
        key: "weight".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let g_src = eng_a_src.graph_expand(&g_req).unwrap();
    let g_dst = eng_a_dst.graph_expand(&g_req).unwrap();
    assert_eq!(g_dst.targets.len(), g_src.targets.len());
    assert_eq!(g_dst.targets[0].logical_id, "history:alpha");
    assert_eq!(g_dst.targets[0].body, "History version two current");

    // 3. Operational state parity.
    assert_eq!(
        eng_a_dst.read_operational_state("checkpoints", "main", None).unwrap().map(|r| r.payload),
        Some(r#"{"step":7}"#.into())
    );
    assert_eq!(
        eng_a_dst
            .read_operational_state("checkpoints", "actuated", None)
            .unwrap()
            .map(|r| r.payload),
        Some(r#"{"step":9}"#.into())
    );

    // 4. Vector search parity (hybrid search with dense arm active).
    let hits_src = eng_a_src.search("orbital propulsion telemetry").unwrap();
    let hits_dst = eng_a_dst.search("orbital propulsion telemetry").unwrap();
    assert_eq!(
        hits_dst.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
        hits_src.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
    );

    eng_a_src.close().unwrap();
    eng_a_dst.close().unwrap();

    // 5. Scope-b isolation parity.
    let eng_b_dst = open_bge_scoped(&dst_path, ScopePolicy::single("scope-b"));
    assert_eq!(
        eng_b_dst.read_get("doc:beta", &ReadView::default()).unwrap().map(|n| n.body),
        Some("Scope beta isolated maritime navigation log".into())
    );
    assert!(eng_b_dst.read_get("claim:alpha", &ReadView::default()).unwrap().is_none());
    assert_eq!(
        eng_b_dst.read_operational_state("checkpoints", "main", None).unwrap().map(|r| r.payload),
        Some(r#"{"step":42}"#.into())
    );
    eng_b_dst.close().unwrap();
}

#[test]
fn logical_export_round_trip_pinned_mean_at_or_above_256_vectors() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = dir.path().join(format!("pinned-src{SQLITE_SUFFIX}"));
    let dst_path = dir.path().join(format!("pinned-dst{SQLITE_SUFFIX}"));

    let count = MEAN_VEC_PIN_THRESHOLD; // exact 256 threshold
    {
        let eng = open_bge_scoped(&src_path, ScopePolicy::default());
        eng.configure_vector_kind_for_test("doc").unwrap();
        let writes: Vec<PreparedWrite> = (0..count)
            .map(|i| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("Pinned corpus document number {i} with distinct payload token-{i}"),
                source_id: SourceId::new("src-pinned").unwrap(),
                logical_id: Some(format!("pinned:{i}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            })
            .collect();
        eng.write(&writes).unwrap();
        eng.drain(30_000).unwrap();
        assert_integrity_clean(&eng);
        eng.close().unwrap();
    }

    recover_complete_erasures(&src_path).expect("complete erasures before 256-vector export");

    let mut src_conn = Connection::open(&src_path).unwrap();
    let src_mean: Vec<u8> = src_conn
        .query_row(
            "SELECT mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        )
        .unwrap()
        .expect("source must have pinned mean_vec at 256 vectors");
    assert_eq!(src_mean.len(), (DIM as usize) * 4);
    let src_bins: Vec<(i64, Vec<u8>)> = src_conn
        .prepare("SELECT rowid, embedding_bin FROM vector_default ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(src_bins.len() as u64, count);

    let archive = export_logical_archive_v1(
        &mut src_conn,
        LogicalExportOptionsV1 { include_audit: false, include_vectors: true },
    )
    .expect("export 256-vector archive");
    let encoded = encode_logical_export_v1(&archive).unwrap();
    let encoded_str = std::str::from_utf8(&encoded).unwrap();
    assert!(!encoded_str.contains("embedding_bin"));
    assert!(!encoded_str.contains("mean_vec"));
    assert!(!encoded_str.contains("_fathomdb_embed_probe"));
    let decoded = decode_logical_export_v1(&encoded).unwrap();
    drop(src_conn);

    let mut dst_conn = open_empty_schema39_db(&dst_path);
    let report = restore_logical_archive_v1(&mut dst_conn, &decoded).expect("restore 256-vector");
    assert!(report.mean_vec_pinned, ">= 256 vectors must derive and pin mean_vec");
    assert_eq!(report.vectors_restored, count);

    let dst_mean: Vec<u8> = dst_conn
        .query_row(
            "SELECT mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        )
        .unwrap()
        .expect("restored must have pinned mean_vec");
    assert_eq!(dst_mean, src_mean, "derived mean_vec must match pinned mean bit-for-bit");

    let dst_bins: Vec<(i64, Vec<u8>)> = dst_conn
        .prepare("SELECT rowid, embedding_bin FROM vector_default ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(dst_bins, src_bins, "bit-exact embedding_bin parity on identical 256-vector set");
    drop(dst_conn);

    let eng_dst = open_bge_scoped(&dst_path, ScopePolicy::default());
    assert_integrity_clean(&eng_dst);
    let hits = eng_dst.search("Pinned corpus document number 42 token-42").unwrap();
    assert_eq!(hits.results[0].id.value.as_str(), "pinned:42");
    eng_dst.close().unwrap();
}

#[test]
fn logical_export_fails_closed_on_all_four_pending_erasure_conditions_and_manifest_coverage() {
    // Verify complete TableExportClass coverage across all 36 TABLE_MANIFEST tables.
    assert_eq!(TABLE_MANIFEST.len(), 36);
    let count_class =
        |c: ExportClass| TABLE_MANIFEST.iter().filter(|e| e.export_class == c).count();
    assert_eq!(count_class(ExportClass::DurableAuthority), 12);
    assert_eq!(count_class(ExportClass::OptInAudit), 3);
    assert_eq!(count_class(ExportClass::OptInVector), 5);
    assert_eq!(count_class(ExportClass::DerivedProjection), 6);
    assert_eq!(count_class(ExportClass::InternalRuntime), 10);

    let dir = TempDir::new().expect("tempdir");

    // Helper to build a fresh clean Schema 39 DB and verify sqlite_master audit passes.
    let make_fresh = |name: &str| {
        let p = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
        let conn = open_empty_schema39_db(&p);
        assert!(audit_schema_39_sqlite_master(&conn).is_ok());
        conn
    };

    // Condition 1: dependency_closure::pending_physical_closures
    {
        let mut conn = make_fresh("cond1");
        conn.execute(
            "INSERT INTO _fathomdb_dependency_closures(
                schema_version, closure_operation_id, root_kind, root_value, cause,
                effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation,
                closure_sequence, retry_fingerprint, phase, affected_count, blocker_code,
                structural_proof_write_boundary, proof_json, scope
             ) VALUES(
                1, '_fdb:c:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef', 'source_bucket', 'src-1', 'source_erased',
                1000, 1, 0, 1,
                '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
                'at_rest_pending', 1, NULL, 1, '{}', ''
             )",
            [],
        )
        .unwrap();
        let err = export_logical_archive_v1(&mut conn, LogicalExportOptionsV1::default())
            .expect_err("must refuse pending physical closure");
        assert_eq!(err.code, LogicalExportErrorCode::PendingPhysicalErasure);
    }

    // Condition 2: physical_erasure::active / diagnostics (FTS sanitation in cleaning phase)
    {
        let mut conn = make_fresh("cond2");
        conn.execute(
            "INSERT OR REPLACE INTO _fathomdb_open_state(key, value) VALUES(
                '_fathomdb_fts_sanitation',
                '{\"version\":1,\"phase\":\"cleaning\",\"family\":0,\"started\":true}'
             )",
            [],
        )
        .unwrap();
        let err = export_logical_archive_v1(&mut conn, LogicalExportOptionsV1::default())
            .expect_err("must refuse active physical erasure");
        assert_eq!(err.code, LogicalExportErrorCode::PendingPhysicalErasure);
    }

    // Condition 3: non-terminal _fathomdb_erasure_requests (phase NOT IN ('complete', 'withdrawn_before_removal'))
    {
        let mut conn = make_fresh("cond3");
        conn.execute(
            "INSERT INTO _fathomdb_erasure_requests(ticket, idempotency_key, payload, scope) VALUES(
                'tkt-1', 'idem-1',
                '{\"ticket\":\"tkt-1\",\"key\":\"idem-1\",\"status\":{\"phase\":\"queued\"}}',
                ''
             )",
            [],
        )
        .unwrap();
        let err = export_logical_archive_v1(&mut conn, LogicalExportOptionsV1::default())
            .expect_err("must refuse non-terminal erasure request");
        assert_eq!(err.code, LogicalExportErrorCode::PendingPhysicalErasure);
    }

    // Condition 4: operational_mutations row in ERASURE_PENDING_REDACTION_COLLECTION
    {
        let mut conn = make_fresh("cond4");
        conn.execute(
            "INSERT INTO operational_mutations(collection_name, record_key, op_kind, payload_json, schema_id, write_cursor, scope)
             VALUES('erasure_pending_redaction', 'rk-1', 'append', '{}', NULL, 1, '')",
            [],
        )
        .unwrap();
        let err = export_logical_archive_v1(&mut conn, LogicalExportOptionsV1::default())
            .expect_err("must refuse pending redaction operational mutation");
        assert_eq!(err.code, LogicalExportErrorCode::PendingPhysicalErasure);
    }
}

#[test]
fn logical_export_rejects_malformed_and_truncated_archives_without_mutating_target() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = dir.path().join(format!("valid-src{SQLITE_SUFFIX}"));
    let dst_path = dir.path().join(format!("untouched-dst{SQLITE_SUFFIX}"));

    let mut src_conn = open_empty_schema39_db(&src_path);
    let valid = export_logical_archive_v1(
        &mut src_conn,
        LogicalExportOptionsV1 { include_audit: false, include_vectors: false },
    )
    .unwrap();
    let valid_bytes = encode_logical_export_v1(&valid).unwrap();

    // 1. Truncated payload.
    let truncated = &valid_bytes[..valid_bytes.len() / 2];
    assert_eq!(
        decode_logical_export_v1(truncated).unwrap_err().code,
        LogicalExportErrorCode::MalformedArchive
    );

    // 2. Invalid UTF-8.
    assert_eq!(
        decode_logical_export_v1(&[0xff, 0xfe, 0xfd]).unwrap_err().code,
        LogicalExportErrorCode::MalformedArchive
    );

    // 3. Unknown injected field (`embedding_bin`).
    let mut injected: serde_json::Value = serde_json::from_slice(&valid_bytes).unwrap();
    injected["embedding_bin"] = serde_json::json!("bad");
    assert_eq!(
        decode_logical_export_v1(&serde_json::to_vec(&injected).unwrap()).unwrap_err().code,
        LogicalExportErrorCode::MalformedArchive
    );

    // 4. Wrong format_version.
    let mut bad_fmt = valid.clone();
    bad_fmt.format_version = "fathomdb.logical-export/v99".into();
    assert_eq!(
        decode_logical_export_v1(&serde_json::to_vec(&bad_fmt).unwrap()).unwrap_err().code,
        LogicalExportErrorCode::UnsupportedFormatVersion
    );

    // 5. Wrong schema_version.
    let mut bad_schema = valid.clone();
    bad_schema.schema_version = 38;
    assert_eq!(
        decode_logical_export_v1(&serde_json::to_vec(&bad_schema).unwrap()).unwrap_err().code,
        LogicalExportErrorCode::UnsupportedSchemaVersion
    );

    // 6. Malformed archive passed directly to restore must NOT mutate target DB.
    let mut dst_conn = open_empty_schema39_db(&dst_path);
    let mut invalid_flag = valid.clone();
    invalid_flag.include_vectors = true; // vectors is None!
    let err = restore_logical_archive_v1(&mut dst_conn, &invalid_flag).unwrap_err();
    assert_eq!(err.code, LogicalExportErrorCode::MalformedArchive);
    let count: i64 =
        dst_conn.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 0);

    // 7. Non-empty target DB must be refused without mutating it.
    let eng = Engine::open_with_choice(&dst_path, EmbedderChoice::None).unwrap().engine;
    eng.write(&[PreparedWrite::Node {
        kind: "doc".into(),
        body: "already exists".into(),
        source_id: SourceId::new("existing").unwrap(),
        logical_id: Some("existing:1".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }])
    .unwrap();
    eng.close().unwrap();
    let mut non_empty = Connection::open(&dst_path).unwrap();
    let err = restore_logical_archive_v1(&mut non_empty, &valid).unwrap_err();
    assert_eq!(err.code, LogicalExportErrorCode::TargetDatabaseNotEmpty);
}

#[test]
fn engine_repeated_open_write_search_close_drop_lifecycle_leaks_and_closing_state() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("lifecycle{SQLITE_SUFFIX}"));

    // Warmup cycles to stabilize thread-local/allocator/SQLite caches.
    for i in 0..2 {
        let eng = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
        eng.write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: format!("warmup body {i}"),
            source_id: SourceId::new("warmup").unwrap(),
            logical_id: Some(format!("warmup:{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
        eng.drain(5_000).unwrap();
        let _ = eng.search("warmup").unwrap();
        eng.close().unwrap();
    }

    #[cfg(target_os = "linux")]
    let fds_before = count_open_fds();

    for i in 0..10 {
        let eng = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
        eng.write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: format!("lifecycle cycle {i} payload"),
            source_id: SourceId::new("cycle").unwrap(),
            logical_id: Some(format!("cycle:{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
        eng.drain(5_000).unwrap();
        let hits = eng.search(&format!("cycle {i}")).unwrap();
        assert!(!hits.results.is_empty());

        if i % 2 == 0 {
            eng.close().unwrap();
            // Idempotent close & deterministic Closing refusal on post-close operations.
            eng.close().unwrap();
            assert!(matches!(
                eng.read_get("cycle:0", &ReadView::default()),
                Err(EngineError::Closing)
            ));
            assert!(matches!(eng.search("cycle"), Err(EngineError::Closing)));
            assert!(matches!(eng.drain(1_000), Err(EngineError::Closing)));
            assert!(matches!(
                eng.write(&[PreparedWrite::Node {
                    kind: "note".into(),
                    body: "after close".into(),
                    source_id: SourceId::new("cycle").unwrap(),
                    logical_id: Some("after:close".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                }]),
                Err(EngineError::Closing)
            ));
        } else {
            // Drop without explicit close must still release lock and FDs cleanly.
            drop(eng);
        }

        // Verify lock released immediately: fresh open succeeds without DatabaseLocked.
        let probe = Engine::open_with_choice(&path, EmbedderChoice::None)
            .expect("lock must be released after close/drop")
            .engine;
        probe.close().unwrap();
    }

    #[cfg(target_os = "linux")]
    {
        let fds_after = count_open_fds();
        assert!(
            fds_after <= fds_before + 2,
            "open FD leak across 10 open/write/search/close/drop cycles: before={fds_before}, after={fds_after}"
        );
    }
}
