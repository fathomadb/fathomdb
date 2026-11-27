//! Slice 50 (`WP-50B`, `AC32-BACKUP-REBUILD-04`) — Installed-artifact CLI
//! backup, restore, and rebuild proof:
//! `fathomdb doctor safe-export` (`VACUUM INTO`) -> copy ->
//! `fathomdb recover --accept-data-loss --rebuild-projections` (separate invocation) ->
//! `fathomdb recover --accept-data-loss --rebuild-vec0` (separate invocation) ->
//! reopen dense store with embedder + `drain()` ->
//! `fathomdb doctor check-integrity --full` + `fathomdb doctor data-plane-integrity --json`
//! (`0` findings) + full semantic parity across scopes, revisions, evidence spans,
//! typed properties, operational state, vector search, and graph expansion.

use std::collections::BTreeSet;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Child, Command, Output};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineConfig, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ProjectionRole, ProjectionSpec, PropertyComparisonOp,
    PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue, ProvenancedNodeV1,
    ReadContextV1, ReadView, ScopePolicy, SearchFilter, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, TestEmbedderChoice, TraversalDirection,
    WriteProvenanceV1,
};
use serde_json::Value;
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
        Ok(v)
    }
}

struct SupervisedChild(Option<Child>);
impl Drop for SupervisedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn bounded_output(mut command: Command) -> Output {
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    let mut child = SupervisedChild(Some(
        command
            .stdout(stdout.try_clone().unwrap())
            .stderr(stderr.try_clone().unwrap())
            .spawn()
            .unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            child.0.take();
            break status;
        }
        assert!(Instant::now() < deadline, "cli watchdog expired");
        std::thread::sleep(Duration::from_millis(10));
    };
    stdout.seek(SeekFrom::Start(0)).unwrap();
    stderr.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout.read_to_end(&mut out).unwrap();
    stderr.read_to_end(&mut err).unwrap();
    Output { status, stdout: out, stderr: err }
}

fn cli(args: &[&str]) -> (i32, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command.args(args);
    let output = bounded_output(command);
    let body = String::from_utf8(output.stdout).unwrap();
    let value = serde_json::from_str(&body).unwrap_or_else(|e| {
        panic!(
            "invalid json ({e}): stdout={body:?} stderr={:?}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code().expect("exit code"), value)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn open_bge(path: &Path, scope: ScopePolicy) -> Engine {
    Engine::open_with_test_choice_and_config_for_test(
        path,
        TestEmbedderChoice::Caller(Arc::new(SimulatedBgeEmbedder::default())),
        EngineConfig::default().with_scope_policy(scope),
    )
    .expect("open bge engine")
    .engine
}

#[test]
fn ac32_backup_rebuild_04_safe_export_recover_rebuild_and_full_parity() {
    let dir = TempDir::new().expect("tempdir");
    let src_path = dir.path().join("source.sqlite");
    let export_path = dir.path().join("safe-export.sqlite");
    let manifest_path = dir.path().join("safe-export.manifest.json");
    let restored_path = dir.path().join("restored.sqlite");

    let canonical_body =
        "Geothermal reservoir pressure report: valve array nominal at sector seven.";
    let span_start = canonical_body.find("valve array nominal").unwrap();
    let span_end = span_start + "valve array nominal".len();
    let span_hash = sha256_hex(canonical_body.as_bytes());

    // 1. Seed a real multi-scope Schema 39 store.
    {
        let eng = open_bge(&src_path, ScopePolicy::single("ops-a"));
        eng.configure_vector_kind_for_test("doc").unwrap();
        eng.configure_vector_kind_for_test("note").unwrap();
        eng.configure_projections(
            &[
                ProjectionSpec {
                    name: "region".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "priority".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "strength".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Edge,
                },
            ],
            &[],
        )
        .unwrap();

        eng.write(&[PreparedWrite::AdminSchema {
            name: "state_kv".into(),
            kind: "latest_state".into(),
            schema_json:
                r#"{"type":"object","properties":{"rev":{"type":"integer"}},"required":["rev"]}"#
                    .into(),
            retention_json: "{}".into(),
        }])
        .unwrap();

        eng.write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src:geo".into()),
                kind: "doc".into(),
                body: canonical_body.into(),
                source_id: SourceId::new("geo-src").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_000),
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-geo-src-1").unwrap(),
                    SourceVersionId::new("sv-geo-1").unwrap(),
                ),
                properties: Some(PropertyMap::from([
                    ("region".into(), PropertyValue::String("north".into())),
                    ("priority".into(), PropertyValue::Int(10)),
                ])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("finding:geo".into()),
                kind: "note".into(),
                body: "Geothermal valve array nominal finding".into(),
                source_id: SourceId::new("geo-src").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_000),
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-geo-finding-1").unwrap(),
                    SourceVersionId::new("sv-geo-1").unwrap(),
                    SourceRevisionId::new("rev-geo-src-1").unwrap(),
                    SourceLocator::utf8_bytes(span_start as u64, span_end as u64),
                    CanonicalHash::sha256(span_hash).unwrap(),
                ),
                properties: Some(PropertyMap::from([
                    ("region".into(), PropertyValue::String("north".into())),
                    ("priority".into(), PropertyValue::Int(10)),
                ])),
            }),
            PreparedWrite::Node {
                kind: "note".into(),
                body: "Target history v1".into(),
                source_id: SourceId::new("geo-hist").unwrap(),
                logical_id: Some("target:geo".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(100),
                valid_until: Some(500),
                properties: Some(PropertyMap::from([(
                    "region".into(),
                    PropertyValue::String("south".into()),
                )])),
            },
        ])
        .unwrap();

        eng.register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-geo-1", "rev-geo-src-1", "rev-geo-finding-1")
                .unwrap(),
        )
        .unwrap();
        eng.drain(15_000).unwrap();

        // Supersede `target:geo` and add edge + operational state.
        eng.write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "Target history v2 active".into(),
                source_id: SourceId::new("geo-hist").unwrap(),
                logical_id: Some("target:geo".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500),
                valid_until: None,
                properties: Some(PropertyMap::from([
                    ("region".into(), PropertyValue::String("north".into())),
                    ("priority".into(), PropertyValue::Int(8)),
                ])),
            },
            PreparedWrite::Edge {
                kind: "corroborates".into(),
                from: "finding:geo".into(),
                to: "target:geo".into(),
                source_id: SourceId::new("geo-src").unwrap(),
                logical_id: Some("edge:geo:1".into()),
                body: Some("Finding corroborates target history".into()),
                t_valid: Some(1_000),
                t_invalid: None,
                confidence: Some(0.95),
                extractor_model_id: Some("ext-1".into()),
                temporal_fallback: None,
                properties: Some(PropertyMap::from([(
                    "strength".into(),
                    PropertyValue::Float(0.9),
                )])),
            },
            PreparedWrite::OpStore {
                collection: "state_kv".into(),
                record_key: "cursor".into(),
                body: r#"{"rev":3}"#.into(),
                schema_id: None,
            },
        ])
        .unwrap();
        eng.drain(15_000).unwrap();
        eng.close().unwrap();
    }

    {
        let eng = open_bge(&src_path, ScopePolicy::single("ops-b"));
        eng.write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "Secondary scope isolated turbine maintenance checklist".into(),
                source_id: SourceId::new("ops-b-src").unwrap(),
                logical_id: Some("doc:ops-b".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(PropertyMap::from([
                    ("region".into(), PropertyValue::String("west".into())),
                    ("priority".into(), PropertyValue::Int(2)),
                ])),
            },
            PreparedWrite::OpStore {
                collection: "state_kv".into(),
                record_key: "cursor".into(),
                body: r#"{"rev":99}"#.into(),
                schema_id: None,
            },
        ])
        .unwrap();
        eng.drain(15_000).unwrap();
        eng.close().unwrap();
    }

    // 2. `fathomdb doctor safe-export` (`VACUUM INTO`) + SHA-256 verification.
    let (code, export_json) = cli(&[
        "doctor",
        "safe-export",
        export_path.to_str().unwrap(),
        "--manifest",
        manifest_path.to_str().unwrap(),
        src_path.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{export_json}");
    let actual_sha = sha256_hex(&std::fs::read(&export_path).unwrap());
    let manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["sha256"].as_str(), Some(actual_sha.as_str()));

    // 3. Copy exported snapshot to restored path.
    std::fs::copy(&export_path, &restored_path).unwrap();

    // 4. Separate CLI invocation: `fathomdb recover --accept-data-loss --rebuild-projections`
    let (code, rebuild_proj) = cli(&[
        "recover",
        "--accept-data-loss",
        "--rebuild-projections",
        "--json",
        restored_path.to_str().unwrap(),
    ]);
    assert_eq!(code, 64, "{rebuild_proj}");
    assert_eq!(rebuild_proj["verb"], "rebuild-projections");

    // Reopen with embedder + drain() after `--rebuild-projections` so dense vectors
    // are re-materialized before exercising the separate `--rebuild-vec0` pass.
    {
        let eng = open_bge(&restored_path, ScopePolicy::default());
        eng.drain(30_000).unwrap();
        assert!(eng.vector_row_count_for_test().unwrap() >= 4);
        eng.close().unwrap();
    }

    // 5. Separate CLI invocation: `fathomdb recover --accept-data-loss --rebuild-vec0`
    let (code, rebuild_vec0) = cli(&[
        "recover",
        "--accept-data-loss",
        "--rebuild-vec0",
        "--json",
        restored_path.to_str().unwrap(),
    ]);
    assert_eq!(code, 64, "{rebuild_vec0}");
    assert_eq!(rebuild_vec0["verb"], "rebuild-vec0");

    // 6. Reopen dense store with embedder + `drain()` to re-materialize vectors,
    //    then discharge FTS sanitation certificate (`--complete-erasures`) so
    //    `doctor check-integrity --full` reports zero retained-terms debt.
    {
        let eng = open_bge(&restored_path, ScopePolicy::default());
        eng.drain(30_000).unwrap();
        assert!(eng.vector_row_count_for_test().unwrap() >= 4);
        eng.close().unwrap();
    }
    let (code, complete_json) = cli(&[
        "recover",
        "--accept-data-loss",
        "--complete-erasures",
        "--json",
        restored_path.to_str().unwrap(),
    ]);
    assert_eq!(code, 64, "{complete_json}");

    // 7. `fathomdb doctor check-integrity --full` -> 0 findings.
    let (code, integrity) =
        cli(&["doctor", "check-integrity", "--full", "--json", restored_path.to_str().unwrap()]);
    assert_eq!(code, 0, "{integrity}");
    assert_eq!(integrity["physical"]["status"], "clean");
    assert_eq!(integrity["logical"]["status"], "clean");
    assert_eq!(integrity["semantic"]["status"], "clean");

    // 8. `fathomdb doctor data-plane-integrity --json` -> 0 findings.
    let (code, dpi) =
        cli(&["doctor", "data-plane-integrity", "--json", restored_path.to_str().unwrap()]);
    assert_eq!(code, 0, "{dpi}");
    assert_eq!(dpi["status"], "clean", "{dpi}");
    assert_eq!(dpi["report"]["findings"].as_array().map(Vec::len), Some(0), "{dpi}");

    // 9. Assert full semantic parity between source and restored store.
    let src_a = open_bge(&src_path, ScopePolicy::single("ops-a"));
    let dst_a = open_bge(&restored_path, ScopePolicy::single("ops-a"));

    let mut filter = SearchFilter::default();
    filter.attributes.push(("region".into(), "north".into()));
    filter.property_predicates.push(PropertyPredicate {
        key: "priority".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Int(10),
    });
    let ctx_src = src_a
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter.clone()).unwrap())
        .unwrap();
    let ctx_dst = dst_a
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).unwrap())
        .unwrap();
    let ev_src = src_a
        .search_with_evidence(&EvidenceSearchRequestV1::new("valve array nominal".into(), ctx_src))
        .unwrap();
    let ev_dst = dst_a
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "valve array nominal".into(),
            ctx_dst.clone(),
        ))
        .unwrap();
    assert_eq!(ev_dst.evidence.len(), ev_src.evidence.len());
    let finding_ev = ev_dst
        .evidence
        .iter()
        .find(|e| e.artifact_revision_id == "rev-geo-finding-1")
        .expect("finding revision present");
    let resolved = dst_a
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: finding_ev.evidence_ref.clone(),
            context: ctx_dst,
        })
        .unwrap();
    assert_eq!(resolved.canonical_source_body, canonical_body);
    assert_eq!(resolved.locator, SourceLocator::utf8_bytes(span_start as u64, span_end as u64));

    // Graph expansion parity.
    let mut g_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("finding:geo")],
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
        key: "strength".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Float(0.8),
    });
    let g_src = src_a.graph_expand(&g_req).unwrap();
    let g_dst = dst_a.graph_expand(&g_req).unwrap();
    assert_eq!(g_dst.targets.len(), g_src.targets.len());
    assert_eq!(g_dst.targets[0].logical_id, "target:geo");
    assert_eq!(g_dst.targets[0].body, "Target history v2 active");

    // Operational state & vector search parity.
    assert_eq!(
        dst_a.read_operational_state("state_kv", "cursor", None).unwrap().map(|r| r.payload),
        Some(r#"{"rev":3}"#.into())
    );
    let hits_src = src_a.search("Geothermal valve array nominal").unwrap();
    let hits_dst = dst_a.search("Geothermal valve array nominal").unwrap();
    assert_eq!(
        hits_dst.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
        hits_src.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>()
    );
    src_a.close().unwrap();
    dst_a.close().unwrap();

    // Scope-b isolation parity.
    let dst_b = open_bge(&restored_path, ScopePolicy::single("ops-b"));
    assert_eq!(
        dst_b.read_get("doc:ops-b", &ReadView::default()).unwrap().map(|n| n.body),
        Some("Secondary scope isolated turbine maintenance checklist".into())
    );
    assert!(dst_b.read_get("finding:geo", &ReadView::default()).unwrap().is_none());
    assert_eq!(
        dst_b.read_operational_state("state_kv", "cursor", None).unwrap().map(|r| r.payload),
        Some(r#"{"rev":99}"#.into())
    );
    dst_b.close().unwrap();
}
