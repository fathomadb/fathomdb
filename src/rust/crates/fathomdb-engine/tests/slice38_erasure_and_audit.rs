//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 4 RED/GREEN suite for:
//! - `ExportClass` + `sqlite_master` completeness audit (positive + negative)
//! - Inline/indexed property physical erasure (Autonomous & ExplicitMaintenance),
//!   `physical_erasure::admission::envelope` `properties` byte accounting, and
//!   `data_plane_integrity` `canonical_property_values` verification (node + edge)
//! - W12 Ask 3a (Autonomous two-truncate ordering)
//! - W12 Ask 3b (`completed_pending_erasure` & cross-source audit guard)
//! - W12 Ask 3c (release-mode lock-free WAL attribution without env)

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_physical_erasure_hook_for_test, audit_schema_38_sqlite_master, classify_schema_38_table,
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, CleanupResult,
    DataPlaneIntegrityCheckV1, DataPlaneIntegrityFindingCodeV1, DataPlaneIntegrityRequestV1,
    EmbedderChoice, Engine, EngineConfig, EngineError, ExportClass, InitialState, LifecycleState,
    PhysicalErasurePolicy, PhysicalErasureSelector, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ProjectionVector, PropertyEntityClass, PropertyMap, PropertyValue,
    ProvenancedNodeV1, QueueResult, ReadView, SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct AuditEmbedder;

impl Embedder for AuditEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice38-audit-embedder", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vec = vec![0.0_f32; 8];
        for (i, b) in text.bytes().enumerate() {
            vec[i % 8] += f32::from(b) / 255.0;
        }
        if vec.iter().all(|v| *v == 0.0) {
            vec[0] = 1.0;
        }
        Ok(vec)
    }
}

fn props(entries: &[(&str, PropertyValue)]) -> Option<PropertyMap> {
    let mut map = PropertyMap::new();
    for (k, v) in entries {
        map.insert((*k).to_string(), v.clone());
    }
    Some(map)
}

fn node(
    id: &str,
    source: &str,
    kind: &str,
    body: &str,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties,
    }
}

fn edge(
    id: &str,
    source: &str,
    from: &str,
    to: &str,
    body: Option<&str>,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "relates".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        body: body.map(ToOwned::to_owned),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties,
    }
}

fn occurrences_across_db_files(db_path: &Path, needle: &str) -> usize {
    let bytes = needle.as_bytes();
    let mut total = 0usize;
    for path in [
        db_path.to_path_buf(),
        Path::new(&format!("{}-wal", db_path.display())).to_path_buf(),
        Path::new(&format!("{}-shm", db_path.display())).to_path_buf(),
    ] {
        if let Ok(buf) = std::fs::read(&path) {
            total += buf.windows(bytes.len()).filter(|w| *w == bytes).count();
        }
    }
    total
}

#[test]
fn slice38_sqlite_master_and_export_class_completeness_audit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("audit{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(AuditEmbedder)).expect("open");
    let engine = opened.engine;

    // Declare Filterable node + edge attributes AND Searchable+Vector so sqlite-vec
    // materializes `metadatachunks*` / `metadatatext*` and FTS5 shadows are active.
    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "tenant_tag".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable, ProjectionRole::Searchable]),
                    fts: Some(ProjectionFts { tokenizer: None }),
                    vector: Some(ProjectionVector {
                        embedder: None,
                        dense_readiness: None,
                        kinds: Some(vec!["custom:audit".into()]),
                    }),
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "edge_weight".into(),
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

    engine
        .write(&[
            node(
                "audit-node-1",
                "src-audit",
                "custom:audit",
                "audit vector body content",
                props(&[("tenant_tag", PropertyValue::String("alpha".into()))]),
            ),
            edge(
                "audit-edge-1",
                "src-audit",
                "audit-node-1",
                "audit-node-1",
                None,
                props(&[("edge_weight", PropertyValue::Float(0.75))]),
            ),
        ])
        .expect("write");
    engine.drain(5_000).expect("drain");
    engine.close().expect("close");

    let conn = Connection::open(&path).expect("open sqlite");
    let manifest: BTreeMap<String, ExportClass> =
        audit_schema_38_sqlite_master(&conn).expect("all schema-38 tables must classify cleanly");

    // Verify key Schema-38 classifications explicitly.
    assert_eq!(manifest.get("_fathomdb_vector_kinds"), Some(&ExportClass::DurableAuthority));
    assert_eq!(manifest.get("_fathomdb_commit_timestamps"), Some(&ExportClass::DurableAuthority));
    assert_eq!(manifest.get("canonical_nodes"), Some(&ExportClass::DurableAuthority));
    assert_eq!(manifest.get("canonical_edges"), Some(&ExportClass::DurableAuthority));
    assert_eq!(manifest.get("operational_collections"), Some(&ExportClass::DurableAuthority));
    assert_eq!(manifest.get("canonical_property_values"), Some(&ExportClass::DerivedProjection));
    assert_eq!(manifest.get("vector_default"), Some(&ExportClass::OptInVector));
    assert_eq!(manifest.get("_fathomdb_actuation_receipts"), Some(&ExportClass::OptInAudit));
    assert_eq!(manifest.get("_fathomdb_erasure_requests"), Some(&ExportClass::InternalRuntime));

    // Confirm dynamic sqlite-vec metadata shadows AND FTS5 shadows actually materialized.
    assert!(
        manifest.keys().any(|k| k.starts_with("vector_default_metadatachunks")),
        "expected materialized sqlite-vec metadatachunks* table in manifest: {:?}",
        manifest.keys().collect::<Vec<_>>()
    );
    assert!(
        manifest.keys().any(|k| k.starts_with("vector_default_metadatatext")),
        "expected materialized sqlite-vec metadatatext* table in manifest"
    );
    assert!(
        manifest.keys().any(|k| k == "search_index_v2_data"),
        "expected materialized FTS5 shadow table in manifest"
    );

    // Negative test: injecting an unclassified dummy table fails the audit loudly.
    assert_eq!(classify_schema_38_table("_fathomdb_unclassified_dummy"), None);
    assert_eq!(classify_schema_38_table("vector_default_bogus_dummy"), None);
    conn.execute_batch("CREATE TABLE _fathomdb_unclassified_dummy(id INTEGER PRIMARY KEY)")
        .expect("inject dummy table");
    let unclassified =
        audit_schema_38_sqlite_master(&conn).expect_err("unclassified table must fail audit");
    assert_eq!(unclassified, vec!["_fathomdb_unclassified_dummy".to_string()]);
}

#[test]
fn slice38_properties_physical_erasure_autonomous_and_explicit() {
    const NODE_KEY: &str = "s38_secret_node_prop_key";
    const NODE_VAL: &str = "s38_secret_node_prop_val_991";
    const EDGE_KEY: &str = "s38_secret_edge_prop_key";
    const EDGE_VAL: &str = "s38_secret_edge_prop_val_772";
    const PURGE_KEY: &str = "s38_purge_prop_key";
    const PURGE_VAL: &str = "s38_purge_prop_val_553";

    // (1) Autonomous mode: erase_source + purge over inline properties, nullable-body edge,
    //     custom-kind vectors, telemetry sink, and data_plane_integrity.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("erasure-auto{SQLITE_SUFFIX}"));
        let sink = dir.path().join("telemetry-auto.jsonl");
        let opened =
            Engine::open_with_embedder_for_test(&path, Arc::new(AuditEmbedder)).expect("open");
        let engine = opened.engine;
        engine.enable_telemetry(sink.to_str().unwrap()).expect("telemetry");

        engine
            .configure_projections(
                &[
                    ProjectionSpec {
                        name: NODE_KEY.into(),
                        roles: BTreeSet::from([
                            ProjectionRole::Filterable,
                            ProjectionRole::Searchable,
                        ]),
                        fts: Some(ProjectionFts { tokenizer: None }),
                        vector: Some(ProjectionVector {
                            embedder: None,
                            dense_readiness: None,
                            kinds: Some(vec!["custom:secret".into()]),
                        }),
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: PURGE_KEY.into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: EDGE_KEY.into(),
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

        engine
            .write(&[
                node(
                    "secret-n1",
                    "src-secret",
                    "custom:secret",
                    "autonomous custom vector node body",
                    props(&[(NODE_KEY, PropertyValue::String(NODE_VAL.into()))]),
                ),
                edge(
                    "secret-e1",
                    "src-secret",
                    "secret-n1",
                    "survivor-n1",
                    None, // nullable-body edge carrying properties
                    props(&[(EDGE_KEY, PropertyValue::String(EDGE_VAL.into()))]),
                ),
                node(
                    "purge-n1",
                    "src-purge",
                    "custom:secret",
                    "node to be purged",
                    props(&[(PURGE_KEY, PropertyValue::String(PURGE_VAL.into()))]),
                ),
                node(
                    "survivor-n1",
                    "src-keep",
                    "custom:secret",
                    "survivor node body",
                    props(&[(NODE_KEY, PropertyValue::String("kept_value".into()))]),
                ),
                edge(
                    "survivor-e1",
                    "src-keep",
                    "survivor-n1",
                    "survivor-n1",
                    None,
                    props(&[(EDGE_KEY, PropertyValue::String("kept_edge_value".into()))]),
                ),
            ])
            .expect("write");
        engine.drain(5_000).expect("drain");

        // Trigger a telemetry event referencing the erased node IDs.
        let _ = engine.search_text_only("autonomous").expect("search");

        // Data-plane integrity passes cleanly on populated node + edge canonical_property_values.
        let clean = engine
            .check_data_plane_integrity(DataPlaneIntegrityRequestV1::all(10_000, 100).unwrap())
            .expect("integrity clean");
        assert!(
            clean.findings.is_empty(),
            "expected zero integrity findings, got {:?}",
            clean.findings
        );

        // Erase `src-secret` (node + nullable-body edge + custom-kind vector).
        let report = engine.erase_source("src-secret").expect("erase_source");
        assert_eq!(report.nodes_excised, 1);
        assert_eq!(report.edges_excised, 1);
        assert!(!report.completed_pending_erasure);

        // Purge `purge-n1`.
        engine.transition("purge-n1", LifecycleState::Deleted, None).expect("delete");
        engine.purge("purge-n1").expect("purge");

        assert_eq!(
            engine
                .query_i64_col_for_test(&format!(
                    "SELECT COUNT(*) FROM canonical_property_values WHERE text_val IN ('{NODE_VAL}','{EDGE_VAL}','{PURGE_VAL}')"
                ))
                .unwrap(),
            vec![0]
        );
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE kind='custom:secret'"
                )
                .unwrap(),
            vec![1],
            "only survivor custom-kind vector row should remain"
        );
        assert_eq!(
            engine.query_i64_col_for_test("SELECT COUNT(*) FROM vector_default").unwrap(),
            vec![1]
        );

        // Data-plane integrity still clean after erasure.
        let post_clean = engine
            .check_data_plane_integrity(DataPlaneIntegrityRequestV1::all(10_000, 100).unwrap())
            .expect("integrity post-erase");
        assert!(
            post_clean.findings.is_empty(),
            "expected zero findings post-erase, got {:?}",
            post_clean.findings
        );

        // Inject synthetic canonical_property_values orphans for BOTH `node` and `edge` and verify detection.
        engine
            .execute_for_test(
                "INSERT INTO canonical_property_values(entity_class,write_cursor,attr_name,value_type,text_val) \
                 VALUES('node',99901,'s38_secret_node_prop_key','string','orphan_node'), \
                       ('edge',99902,'s38_secret_edge_prop_key','string','orphan_edge')",
            )
            .expect("inject cpv orphans");
        let orphan_report = engine
            .check_data_plane_integrity(
                DataPlaneIntegrityRequestV1::new(
                    vec![DataPlaneIntegrityCheckV1::ActiveSearchableOrphans],
                    10_000,
                    100,
                )
                .unwrap(),
            )
            .expect("integrity orphan check");
        let orphan_cursors: Vec<(DataPlaneIntegrityFindingCodeV1, Option<u64>)> =
            orphan_report.findings.iter().map(|f| (f.code, f.write_cursor)).collect();
        assert!(
            orphan_cursors.contains(&(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing,
                Some(99901)
            )),
            "missing node cpv orphan finding: {orphan_cursors:?}"
        );
        assert!(
            orphan_cursors.contains(&(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing,
                Some(99902)
            )),
            "missing edge cpv orphan finding: {orphan_cursors:?}"
        );
        // Clean up synthetic orphans before raw-byte check.
        engine
            .execute_for_test(
                "DELETE FROM canonical_property_values WHERE write_cursor IN (99901,99902)",
            )
            .unwrap();

        engine.close().expect("close");

        // Zero raw byte substrings across main, -wal, -shm, and telemetry sink.
        for needle in [NODE_VAL, EDGE_VAL, PURGE_VAL] {
            assert_eq!(
                occurrences_across_db_files(&path, needle),
                0,
                "erased property value `{needle}` leaked in DB files"
            );
            if let Ok(sink_text) = std::fs::read_to_string(&sink) {
                assert!(!sink_text.contains(needle), "erased value `{needle}` leaked in sink");
            }
        }
        if let Ok(sink_text) = std::fs::read_to_string(&sink) {
            assert!(!sink_text.contains("l:secret-n1"), "erased logical id leaked in sink");
            assert!(!sink_text.contains("l:purge-n1"), "purged logical id leaked in sink");
        }
    }

    // (2) ExplicitMaintenance mode: queue + complete_physical_erasure over nullable-body edge & node properties.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("erasure-explicit{SQLITE_SUFFIX}"));
        let engine = Arc::new(
            Engine::open_with_choice_and_config(
                &path,
                EmbedderChoice::None,
                EngineConfig {
                    physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                    ..Default::default()
                },
            )
            .expect("open explicit")
            .engine,
        );
        engine
            .configure_projections(
                &[
                    ProjectionSpec {
                        name: NODE_KEY.into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: EDGE_KEY.into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Edge,
                    },
                ],
                &[],
            )
            .expect("configure explicit projections");
        engine
            .write(&[
                node(
                    "exp-n1",
                    "src-exp",
                    "doc",
                    "explicit mode secret node",
                    props(&[(NODE_KEY, PropertyValue::String(NODE_VAL.into()))]),
                ),
                edge(
                    "exp-e1",
                    "src-exp",
                    "exp-n1",
                    "exp-keep",
                    None,
                    props(&[(EDGE_KEY, PropertyValue::String(EDGE_VAL.into()))]),
                ),
                node("exp-keep", "src-keep", "doc", "explicit survivor", None),
            ])
            .expect("write explicit");

        let mut handle = engine.begin_erasure_maintenance(1_000).expect("begin maintenance");
        let ticket = match handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "src-exp".into() },
                "11111111111111111111111111111111",
            )
            .expect("queue")
        {
            QueueResult::Accepted { ticket, .. } => ticket,
            other => panic!("expected Accepted, got {other:?}"),
        };
        assert!(matches!(
            handle.complete_physical_erasure(&ticket).expect("complete"),
            CleanupResult::Complete { .. }
        ));
        handle.close().expect("close handle");
        assert_eq!(
            engine
                .query_i64_col_for_test("SELECT COUNT(*) FROM canonical_property_values")
                .unwrap(),
            vec![0]
        );
        engine.close().expect("close engine");
        for needle in [NODE_VAL, EDGE_VAL] {
            assert_eq!(
                occurrences_across_db_files(&path, needle),
                0,
                "explicit mode leaked `{needle}`"
            );
        }
    }

    // (3) `physical_erasure::admission::envelope` counts `properties` bytes on nullable-body edges.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("envelope-props{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).expect("open").engine;
        let huge_prop = "p".repeat((2 << 20) + 64);
        engine
            .write(&[edge(
                "huge-edge",
                "src-huge",
                "a",
                "b",
                None, // body is NULL — only `properties` exceeds the 2 MiB single-row cap!
                props(&[("blob_prop", PropertyValue::String(huge_prop))]),
            )])
            .expect("write huge nullable-body edge");
        let err =
            engine.erase_source("src-huge").expect_err("envelope must refuse >2 MiB properties");
        assert!(
            matches!(err, EngineError::ErasureIncomplete { ref stage, .. } if stage == "physical_admission"),
            "expected physical_admission refusal, got {err:?}"
        );
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM canonical_edges WHERE source_id='src-huge'"
                )
                .unwrap(),
            vec![1],
            "envelope refusal must leave rows untouched"
        );
        engine.close().expect("close");
    }
}

#[test]
fn slice38_w12_3a_autonomous_checkpoint_busy_keeps_plain_writes_open() {
    // (1) Autonomous mode: checkpoint BUSY keeps `debt.redacted == false`,
    //     unrelated plain Node/Edge writes succeed and advance `safe_boundary`,
    //     denied identities/sources and actuation/admin/dependency writes fail closed,
    //     and retry finishes cleanly once snapshot releases.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("w12-3a-auto{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).expect("open").engine;
        engine
            .write(&[
                node("victim-n", "src-victim", "doc", "victim payload", None),
                node("keep-n", "src-keep", "doc", "survivor payload", None),
            ])
            .expect("seed");

        let pin = Connection::open(&path).expect("open pin");
        pin.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").expect("pin WAL");

        let err = engine.erase_source("src-victim").expect_err("checkpoint must fail BUSY");
        assert!(
            matches!(err, EngineError::ErasureIncomplete { ref stage, .. } if stage == "wal_checkpoint"),
            "expected wal_checkpoint BUSY, got {err:?}"
        );

        // Wait past one PhysicalMaintenance poll interval (100ms) to confirm the
        // background worker ALSO respects Autonomous two-truncate ordering.
        std::thread::sleep(Duration::from_millis(180));

        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT json_extract(value,'$.redacted') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
                )
                .unwrap(),
            vec![0],
            "Autonomous checkpoint BUSY must leave debt.redacted == false"
        );
        let boundary_before = engine
            .query_i64_col_for_test(
                "SELECT json_extract(value,'$.safe_boundary') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
            )
            .unwrap()[0];

        // Unrelated plain Node and Edge writes MUST succeed and advance `safe_boundary`.
        let w1 = engine
            .write(&[
                node("unrelated-n", "src-other", "doc", "unrelated plain write", None),
                edge("unrelated-e", "src-other", "keep-n", "unrelated-n", Some("links"), None),
            ])
            .expect("unrelated plain writes must succeed while debt.redacted == false");
        let boundary_after = engine
            .query_i64_col_for_test(
                "SELECT json_extract(value,'$.safe_boundary') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
            )
            .unwrap()[0];
        assert!(
            boundary_after as u64 >= w1.cursor && boundary_after > boundary_before,
            "safe_boundary must advance on admitted plain writes: before={boundary_before} after={boundary_after} cursor={}",
            w1.cursor
        );

        // Denied source, denied logical_id, and non-plain writes MUST fail closed.
        assert!(matches!(
            engine.write(&[node("reinsert", "src-victim", "doc", "nope", None)]),
            Err(EngineError::ErasureIncomplete { .. })
        ));
        assert!(matches!(
            engine.write(&[node("victim-n", "src-other", "doc", "nope", None)]),
            Err(EngineError::ErasureIncomplete { .. })
        ));
        assert!(matches!(
            engine.write(&[PreparedWrite::AdminSchema {
                name: "fenced_col".into(),
                kind: "latest_state".into(),
                schema_json: "{}".into(),
                retention_json: "{}".into(),
            }]),
            Err(EngineError::ErasureIncomplete { .. })
        ));
        let act_batch = ActuationBatchV1::new(
            "fenced-op-1",
            vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: "fenced actuation".into(),
                source_id: SourceId::new("src-other").unwrap(),
                logical_id: Some("act-node".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("rev-act-1").unwrap(),
                    SourceVersionId::new("ver-1").unwrap(),
                ),
                properties: None,
            })],
        )
        .unwrap();
        assert!(matches!(engine.actuate(act_batch), Err(EngineError::ErasureIncomplete { .. })));

        // Release snapshot -> explicit retry completes cleanly with zero debt residue.
        pin.execute_batch("ROLLBACK").expect("release pin");
        drop(pin);
        let retry_report = engine.erase_source("src-victim").expect("retry succeeds");
        assert!(retry_report.completed_pending_erasure);
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
                )
                .unwrap(),
            vec![0]
        );
        assert!(engine.read_get("unrelated-n", &ReadView::default()).unwrap().is_some());
        engine.close().expect("close");
    }

    // (2) Background `PhysicalMaintenance` finishes autonomously once external snapshot releases.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("w12-3a-bg{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).expect("open").engine;
        engine.write(&[node("bg-victim", "src-bg", "doc", "bg victim", None)]).expect("seed");

        let pin = Connection::open(&path).expect("open pin");
        pin.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").expect("pin");
        assert!(matches!(
            engine.erase_source("src-bg"),
            Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "wal_checkpoint"
        ));
        engine
            .write(&[node("bg-plain", "src-ok", "doc", "allowed plain", None)])
            .expect("plain ok");
        pin.execute_batch("ROLLBACK").expect("release pin");
        drop(pin);

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let owed = engine
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'",
                )
                .unwrap()[0];
            if owed == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
                )
                .unwrap(),
            vec![0],
            "background PhysicalMaintenance must finish cleanly after snapshot releases"
        );
        engine.close().expect("close");
    }

    // (3) ExplicitMaintenance preserves existing `prepare_redacted`-before-checkpoint order.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("w12-3a-explicit{SQLITE_SUFFIX}"));
        let engine = Arc::new(
            Engine::open_with_choice_and_config(
                &path,
                EmbedderChoice::None,
                EngineConfig {
                    physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                    ..Default::default()
                },
            )
            .expect("open explicit")
            .engine,
        );
        engine.write(&[node("exp-v", "src-exp", "doc", "explicit victim", None)]).expect("seed");
        let pin = Connection::open(&path).expect("open pin");
        pin.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").expect("pin");
        let mut handle = engine.begin_erasure_maintenance(1_000).expect("maintenance");
        let ticket = match handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "src-exp".into() },
                "22222222222222222222222222222222",
            )
            .expect("queue")
        {
            QueueResult::Accepted { ticket, .. } => ticket,
            other => panic!("expected Accepted, got {other:?}"),
        };
        assert!(matches!(
            handle.complete_physical_erasure(&ticket).expect("incomplete"),
            CleanupResult::Incomplete { .. }
        ));
        handle.close().expect("close handle");
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT json_extract(value,'$.redacted') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
                )
                .unwrap(),
            vec![1],
            "ExplicitMaintenance must retain prepare_redacted-before-checkpoint order"
        );
        pin.execute_batch("ROLLBACK").expect("release");
        engine.close().expect("close");
    }
}

#[test]
fn slice38_w12_3b_completed_pending_erasure_and_cross_source_audit_guard() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("w12-3b{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).expect("open").engine;

    engine
        .write(&[
            node("fresh-n", "src-fresh", "doc", "fresh body", None),
            node("retry-n", "src-retry", "doc", "retry body", None),
            node("redacted-a", "src-a", "doc", "source a body", None),
            node("redacted-b", "src-b", "doc", "source b body", None),
            node("redacted-same", "src-same", "doc", "source same body", None),
        ])
        .expect("seed");

    // (1) Normal fresh erasure and true already-erased no-op => completed_pending_erasure == false.
    let fresh = engine.erase_source("src-fresh").expect("fresh erase");
    assert_eq!(fresh.nodes_excised, 1);
    assert!(!fresh.completed_pending_erasure);
    let noop = engine.erase_source("src-fresh").expect("already-erased noop");
    assert_eq!(noop.nodes_excised, 0);
    assert!(!noop.completed_pending_erasure);

    // (2) `root_retry` path (unredacted BUSY retry): completed_pending_erasure == true,
    //     zero duplicate 0-row `excise_source_audit` row.
    let pin = Connection::open(&path).expect("pin");
    pin.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").expect("pin");
    assert!(matches!(
        engine.erase_source("src-retry"),
        Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "wal_checkpoint"
    ));
    pin.execute_batch("ROLLBACK").expect("release");
    drop(pin);
    let audit_before_retry = engine
        .query_i64_col_for_test(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name='excise_source_audit' AND record_key='src-retry'",
        )
        .unwrap()[0];
    assert_eq!(audit_before_retry, 1);
    let retried = engine.erase_source("src-retry").expect("root_retry complete");
    assert!(retried.completed_pending_erasure);
    assert_eq!(retried.nodes_excised, 0);
    let audit_after_retry = engine
        .query_i64_col_for_test(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name='excise_source_audit' AND record_key='src-retry'",
        )
        .unwrap()[0];
    assert_eq!(audit_after_retry, 1, "root_retry must not append duplicate 0-row audit row");

    // (3) `finish_redacted_before_request` on the SAME source:
    //     Pin WAL inside `after_redacted_commit` so `debt.redacted == true` and second truncate hits BUSY.
    let held_pin: Arc<Mutex<Option<Connection>>> = Arc::new(Mutex::new(None));
    {
        let pin_slot = Arc::clone(&held_pin);
        let pin_path = path.clone();
        arm_physical_erasure_hook_for_test(
            "after_redacted_commit",
            Box::new(move || {
                let c = Connection::open(&pin_path).unwrap();
                c.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").unwrap();
                *pin_slot.lock().unwrap() = Some(c);
                Ok(())
            }),
        );
    }
    assert!(matches!(
        engine.erase_source("src-same"),
        Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "wal_checkpoint"
    ));
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT json_extract(value,'$.redacted') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
            )
            .unwrap(),
        vec![1]
    );
    if let Some(c) = held_pin.lock().unwrap().take() {
        c.execute_batch("ROLLBACK").unwrap();
    }
    let same_retry = engine.erase_source("src-same").expect("same-source redacted retry");
    assert!(same_retry.completed_pending_erasure);
    assert_eq!(same_retry.nodes_excised, 0);
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT COUNT(*) FROM operational_mutations WHERE collection_name='excise_source_audit' AND record_key='src-same'"
            )
            .unwrap(),
        vec![1],
        "completing retry of same source via finish_redacted_before_request must suppress duplicate 0-row audit row"
    );

    // (4) Cross-source via `finish_redacted_before_request`:
    //     Source A leaves `debt.redacted == true`; caller calls `erase_source("src-b")`.
    //     Must finish Source A AND excise Source B AND RETAIN Source B's `excise_source_audit` row!
    {
        let pin_slot = Arc::clone(&held_pin);
        let pin_path = path.clone();
        arm_physical_erasure_hook_for_test(
            "after_redacted_commit",
            Box::new(move || {
                let c = Connection::open(&pin_path).unwrap();
                c.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").unwrap();
                *pin_slot.lock().unwrap() = Some(c);
                Ok(())
            }),
        );
    }
    assert!(matches!(
        engine.erase_source("src-a"),
        Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "wal_checkpoint"
    ));
    if let Some(c) = held_pin.lock().unwrap().take() {
        c.execute_batch("ROLLBACK").unwrap();
    }
    let cross = engine.erase_source("src-b").expect("cross-source erase succeeds");
    assert!(cross.completed_pending_erasure, "finished prior Source A closure");
    assert_eq!(cross.nodes_excised, 1, "excised real Source B row");
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT COUNT(*) FROM operational_mutations WHERE collection_name='excise_source_audit' AND record_key='src-b'"
            )
            .unwrap(),
        vec![1],
        "cross-source erasure MUST retain excise_source_audit row for Source B"
    );

    engine.close().expect("close");
}

#[test]
fn slice38_w12_3c_release_mode_wal_attribution_without_env() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("w12-3c{SQLITE_SUFFIX}"));
    let engine = Arc::new(Engine::open(&path).expect("open").engine);

    // Simulate release mode without FATHOMDB_WAL_ATTRIBUTION.
    engine.set_wal_attribution_enabled_for_test(false);

    // (1) Verify lock-free idempotency (duplicate set(true)/set(false) never double-counts or underflows)
    //     and accurate classification across all 4 classes while enabled == false.
    let probe = engine.probe_release_wal_attribution_idempotency_for_test();
    assert_eq!(probe.after_double_reader_set_true, 1, "duplicate set(true) must not double-count");
    assert_eq!(probe.reader_classification, "owned_reader_snapshot");
    assert_eq!(probe.after_double_reader_set_false, 0, "duplicate set(false) must not underflow");
    assert_eq!(
        probe.after_double_worker_set_true, 1,
        "duplicate worker set(true) must not double-count"
    );
    assert_eq!(probe.worker_classification, "owned_runtime_transaction");
    assert_eq!(
        probe.after_double_worker_set_false, 0,
        "duplicate worker set(false) must not underflow"
    );
    assert_eq!(probe.overlap_classification, "local_checkpoint_overlap");
    assert_eq!(probe.idle_classification, "no_owned_snapshot");

    // (2) Real checkpoint BUSY caused by an EXTERNAL SQLite connection while enabled == false:
    //     Must classify as `unclassified_external`, keep required substrings, and NOT falsely blame caller reader!
    engine.write(&[node("ext-v", "src-ext", "doc", "external blocker", None)]).expect("seed");
    let pin = Connection::open(&path).expect("external pin");
    pin.execute_batch("BEGIN; SELECT COUNT(*) FROM canonical_nodes;").expect("pin WAL");
    let err = engine.erase_source("src-ext").expect_err("external pin causes BUSY");
    pin.execute_batch("ROLLBACK").expect("release pin");
    drop(pin);

    let EngineError::ErasureIncomplete { stage, detail } = err else {
        panic!("expected ErasureIncomplete, got {err:?}");
    };
    assert_eq!(stage, "wal_checkpoint");
    assert!(detail.contains("wal_checkpoint(TRUNCATE)"), "{detail}");
    assert!(detail.contains("BUSY"), "{detail}");
    assert!(detail.contains("frames still in the log)"), "{detail}");
    assert!(detail.contains("unclassified_external"), "expected unclassified_external: {detail}");
    assert!(
        !detail.contains("a concurrent reader is pinning a WAL snapshot"),
        "must not falsely blame caller reader on external blocker: {detail}"
    );

    // Finish pending erasure cleanly.
    let done = engine.erase_source("src-ext").expect("finish ext");
    assert!(done.completed_pending_erasure);

    // (3) Real checkpoint BUSY caused by an OWNED reader worker while enabled == false:
    engine
        .write(&[node("owned-v", "src-owned", "doc", "owned blocker", None)])
        .expect("seed owned");
    let (ready, release) = engine.hold_owned_reader_snapshot_for_test();
    ready.wait();
    let owned_err = engine.erase_source("src-owned").expect_err("owned reader causes BUSY");
    release.wait();

    let EngineError::ErasureIncomplete { stage, detail } = owned_err else {
        panic!("expected ErasureIncomplete, got {owned_err:?}");
    };
    assert_eq!(stage, "wal_checkpoint");
    assert!(detail.contains("wal_checkpoint(TRUNCATE)"), "{detail}");
    assert!(detail.contains("BUSY"), "{detail}");
    assert!(detail.contains("frames still in the log)"), "{detail}");
    assert!(
        detail.contains("owned_reader_snapshot"),
        "release mode must classify owned reader truthfully: {detail}"
    );

    engine.erase_source("src-owned").expect("finish owned");
    engine.close().expect("close");
}
