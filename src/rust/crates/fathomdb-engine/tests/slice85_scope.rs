use fathomdb_engine::*;
use proptest::prelude::*;
use std::{collections::BTreeSet, sync::Arc};
fn node(id: &str, source: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body,
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn open(path: &std::path::Path) -> Arc<Engine> {
    Arc::new(
        Engine::open_with_choice_and_config(
            path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    )
}
fn accepted(result: QueueResult) -> String {
    match result {
        QueueResult::Accepted { ticket, .. } => ticket,
        _ => panic!("heavy admission refused"),
    }
}
#[test]
fn bounded_queue_capacity_refuses_seventeenth_intent_without_installing_fence() {
    let dir = tempfile::tempdir().unwrap();
    let engine = open(&dir.path().join("capacity.sqlite"));
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    for index in 0..16 {
        handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: format!("source-{index}") },
                &format!("{index:032x}"),
            )
            .unwrap();
    }
    assert!(matches!(
        handle.queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "unaccepted-source".into() },
            "fc12d1f8ef8d45f9b311b2d767d4909b"
        ),
        Err(EngineError::Maintenance(MaintenanceError::Capacity))
    ));
    handle.close().unwrap();
    engine.write(&[node("unaccepted", "unaccepted-source", "permitted".into())]).unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM _fathomdb_erasure_requests WHERE phase='queued'"
            )
            .unwrap(),
        vec![16]
    );
}
#[test]
fn unsupported_target_count_refuses_before_acceptance() {
    let dir = tempfile::tempdir().unwrap();
    let engine = open(&dir.path().join("cap.sqlite"));
    let writes = (0..16385)
        .map(|i| node(&format!("n{i}"), "oversized", "small body".into()))
        .collect::<Vec<_>>();
    engine.write(&writes).unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        handle.queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "oversized".into() },
            "517f25cc3651418a837065ab467e21bb"
        ),
        Err(EngineError::Maintenance(MaintenanceError::UnsupportedScope))
    ));
    handle.close().unwrap();
    assert_eq!(
        engine.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_erasure_requests").unwrap(),
        vec![0]
    );
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn queued_writer_withdraw_reopen_histories_preserve_visibility_and_scope(steps in prop::collection::vec(0u8..5,1..12)){
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("history.sqlite");let mut engine=open(&path);engine.write(&[node("selected","target","original".into())]).unwrap();let mut pending=None;let mut sequence=1u64;
        for step in steps{match step{
            0 if pending.is_none()=>{let mut handle=engine.begin_erasure_maintenance(1000).unwrap();pending=Some(accepted(handle.queue_physical_erasure(PhysicalErasureSelector::SourceAllAtCut{source_id:"target".into()},&format!("{sequence:032x}")).unwrap()));sequence+=1;handle.close().unwrap();},
            1=>{if let Some(ticket)=pending.take(){prop_assert_eq!(engine.withdraw_queued_erasure(&ticket).unwrap(),WithdrawResult::WithdrawnBeforeRemoval);}},
            2=>{let outcome=engine.write(&[node("selected","target","replacement".into())]);prop_assert_eq!(outcome.is_err(),pending.is_some());},
            3=>{engine.write(&[node("unrelated","other","survivor".into())]).unwrap();},
            4=>{engine.close().unwrap();engine=Arc::new(Engine::open(&path).unwrap().engine);},
            _=>{}
        }prop_assert!(engine.read_get("selected",&ReadView::default()).unwrap().is_some());if let Some(ticket)=&pending{let status=engine.physical_erasure_status(ticket).unwrap().unwrap();prop_assert_eq!(status.phase,ErasurePhase::Queued);prop_assert_eq!(status.logical_state,ErasureLogicalState::UnchangedByThisRequest);}}
    }
}

struct Fixed;
impl fathomdb_embedder_api::Embedder for Fixed {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("slice85", "fixed", 8)
    }
    fn embed(&self, _: &str) -> Result<Vec<f32>, fathomdb_embedder_api::EmbedderError> {
        Ok(vec![1.; 8])
    }
}
const MARKER: &str = "qzxvphysicalscopeeightyfive";
const TABLES: [&str; 4] =
    ["search_index", "search_index_v2", "search_index_edges", "property_search_index"];
fn rich_fixture(path: &std::path::Path, targets: bool) -> Arc<Engine> {
    use sha2::Digest;
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            path,
            TestEmbedderChoice::Caller(Arc::new(Fixed)),
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "text".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .write(&[node("left", "kept", "left".into()), node("right", "kept", "right".into())])
        .unwrap();
    if targets {
        let body = serde_json::json!({"text":format!("{MARKER} canonical")}).to_string();
        let hash = sha2::Sha256::digest(body.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let provenance_node = |id: &str, source: &str, body: String, provenance| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body,
                source_id: SourceId::new(source).unwrap(),
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance,
            })
        };
        engine
            .write(&[
                provenance_node(
                    "root",
                    "target",
                    body,
                    WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("source-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                    ),
                ),
                provenance_node(
                    "dependent",
                    "target",
                    serde_json::json!({"text":format!("{MARKER} dependent")}).to_string(),
                    WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("derived-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(hash).unwrap(),
                    ),
                ),
            ])
            .unwrap();
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1")
                    .unwrap(),
            )
            .unwrap();
    }
    for version in 0..3 {
        let mut batch = vec![node(
            "kept-history",
            "kept",
            serde_json::json!({"text":format!("survivorneedle history{version}")}).to_string(),
        )];
        if targets {
            batch.push(node(
                "target-history",
                "target",
                serde_json::json!({"text":format!("{MARKER} history{version}")}).to_string(),
            ));
            batch.push(PreparedWrite::Edge {
                kind: "rel".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("target").unwrap(),
                logical_id: Some("target-edge".into()),
                body: Some(format!("{MARKER} edge{version}")),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            });
        }
        engine.write(&batch).unwrap();
    }
    engine.drain(10000).unwrap();
    engine
}
#[test]
fn exclusive_complete_covers_dependency_four_fts_families_mapped_vectors_and_survivor_statistics() {
    let dir = tempfile::tempdir().unwrap();
    let candidate = rich_fixture(&dir.path().join("candidate.sqlite"), true);
    let baseline = rich_fixture(&dir.path().join("baseline.sqlite"), false);
    let path = candidate.path().to_owned();
    let c = rusqlite::Connection::open(&path).unwrap();
    for table in TABLES {
        let count: i64 = c
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                [MARKER],
                |r| r.get(0),
            )
            .unwrap();
        assert!(count > 0, "positive marker in {table}");
    }
    let head=candidate.query_i64_col_for_test("SELECT v.rowid FROM _fathomdb_vector_rows v JOIN canonical_nodes n ON n.write_cursor=v.write_cursor WHERE n.source_id='target' ORDER BY v.rowid LIMIT 1").unwrap()[0];
    c.execute_batch(&format!("CREATE TEMP TABLE scope_vector_copy AS SELECT * FROM vector_default WHERE rowid={head}; INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status,lifecycle,valid_from,valid_until) SELECT 900000,embedding,vec_bit(embedding_bin),source_type,kind,created_at,status,lifecycle,valid_from,valid_until FROM scope_vector_copy; DROP TABLE scope_vector_copy; INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) SELECT 900000,kind,write_cursor FROM _fathomdb_vector_rows WHERE rowid={head}; INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length) SELECT 900000,write_cursor,(SELECT COALESCE(MAX(window_number),0)+1 FROM _fathomdb_vector_spans v WHERE v.write_cursor=s.write_cursor),byte_start,byte_end,body_length FROM _fathomdb_vector_spans s WHERE vector_rowid={head};")).unwrap();
    let ids=candidate.query_i64_col_for_test("SELECT v.rowid FROM _fathomdb_vector_rows v JOIN canonical_nodes n ON n.write_cursor=v.write_cursor WHERE n.source_id IN ('target','derived') ORDER BY v.rowid").unwrap();
    assert!(!ids.is_empty(), "positive mapped vector witness");
    let mut handle = candidate.begin_erasure_maintenance(1000).unwrap();
    let ticket = accepted(
        handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "target".into() },
                "b97eb7352eb747cf9bdd33375e0d0bb4",
            )
            .unwrap(),
    );
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    assert!(candidate.read_get("dependent", &ReadView::default()).unwrap().is_none());
    for table in TABLES {
        let count: i64 = c
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                [MARKER],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "exact zero in {table}");
    }
    for id in ids {
        assert_eq!(candidate.query_i64_col_for_test(&format!("SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid={id}) OR EXISTS(SELECT 1 FROM _fathomdb_vector_rows WHERE rowid={id})")).unwrap(),vec![0]);
    }
    let hits = |e: &Engine| {
        e.search_explained("survivorneedle", None, 0, false, 0.3, 0)
            .unwrap()
            .results
            .into_iter()
            .map(|hit| (hit.id.to_prefixed(), hit.score.to_bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(hits(&candidate), hits(&baseline), "survivor-only score/order oracle");
    for suffix in ["", "-wal"] {
        let bytes = std::fs::read(format!("{}{suffix}", path.display())).unwrap_or_default();
        assert!(!bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()));
    }
}

#[test]
fn logical_admission_freezes_and_removes_registered_dependency_closure() {
    let dir = tempfile::tempdir().unwrap();
    let engine = rich_fixture(&dir.path().join("logical.sqlite"), true);
    engine.transition("root", LifecycleState::Deleted, None).unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = accepted(
        handle
            .queue_physical_erasure(
                PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id: "root".into() },
                "b20bd7fa6dfa444db830b7d352ab9204",
            )
            .unwrap(),
    );
    handle.close().unwrap();
    assert!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().target_count >= 2,
        "root-only counting cannot certify dependency scope"
    );
    assert!(engine.write(&[node("dependent", "target", "replacement".into())]).is_err());
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    assert!(engine.read_get("root", &ReadView::default()).unwrap().is_none());
    assert!(engine.read_get("dependent", &ReadView::default()).unwrap().is_none());
    assert!(
        engine.read_get("target-history", &ReadView::default()).unwrap().is_some(),
        "unselected same-source root is an exact survivor of logical erasure"
    );
}

#[test]
fn queued_effect_guards_cover_batch_old_new_endpoints_provenance_and_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let engine = rich_fixture(&dir.path().join("effects.sqlite"), true);
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "target".into() },
            "9f4e7cb2ac094246a95bb0d1d9b42f17",
        )
        .unwrap();
    handle.close().unwrap();
    let before = engine
        .query_i64_col_for_test(
            "SELECT count(*) FROM canonical_nodes UNION ALL SELECT count(*) FROM canonical_edges",
        )
        .unwrap();
    let edge = |logical: &str, from: &str, to: &str| PreparedWrite::Edge {
        kind: "rel".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("other").unwrap(),
        logical_id: Some(logical.into()),
        body: Some("replacement".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    };
    for effect in [edge("new-edge", "left", "root"), edge("target-edge", "left", "right")] {
        assert!(engine
            .write(&[node("unrelated-prefix", "other", "must rollback".into()), effect])
            .is_err());
        assert!(engine.read_get("unrelated-prefix", &ReadView::default()).unwrap().is_none());
    }
    let provenance = PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "replacement".into(),
        source_id: SourceId::new("target").unwrap(),
        logical_id: Some("root".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("new-r1").unwrap(),
            SourceVersionId::new("v2").unwrap(),
        ),
    });
    assert!(matches!(
        engine.write(&[provenance]),
        Err(EngineError::Maintenance(MaintenanceError::QueuedScopeFenced))
    ));
    assert!(matches!(
        engine.register_source_dependency(
            SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1").unwrap()
        ),
        Err(EngineError::Maintenance(MaintenanceError::QueuedScopeFenced))
    ));
    assert_eq!(engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes UNION ALL SELECT count(*) FROM canonical_edges").unwrap(),before);
}
