use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, EmbedderChoice, Engine, EngineConfig,
    EngineError, EngineOpenError, InitialState, LatestStateActuationV1, PreparedWrite,
    ProvenancedNodeV1, ScopePolicy, SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn lock_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(".lock");
    std::path::PathBuf::from(s)
}

fn canonical_node(revision: &str, logical_id: &str, body: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "note".into(),
        body: body.into(),
        source_id: SourceId::new("src-pin-c").unwrap(),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("ver-{revision}")).unwrap(),
        ),
    }
}

fn register_collection(engine: &Engine, name: &str, kind: &str, schema_json: &str) {
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: name.into(),
            kind: kind.into(),
            schema_json: schema_json.into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
}

#[test]
fn put_latest_state_constructor_and_request_revalidation() {
    let err = LatestStateActuationV1::new("", "k1", "{}").unwrap_err();
    assert_eq!(err.reason, ActuationErrorReason::NestedRequestInvalid);
    assert_eq!(err.field_path, "/collection");

    let err = LatestStateActuationV1::new("   ", "k1", "{}").unwrap_err();
    assert_eq!(err.reason, ActuationErrorReason::NestedRequestInvalid);
    assert_eq!(err.field_path, "/collection");

    let err = LatestStateActuationV1::new("ckpt", "", "{}").unwrap_err();
    assert_eq!(err.reason, ActuationErrorReason::NestedRequestInvalid);
    assert_eq!(err.field_path, "/recordKey");

    let err = LatestStateActuationV1::new("ckpt", "k1", "{not-json").unwrap_err();
    assert_eq!(err.reason, ActuationErrorReason::NestedRequestInvalid);
    assert_eq!(err.field_path, "/payloadJson");

    let dir = TempDir::new().unwrap();
    let engine = Engine::open(db_path(&dir, "reval")).unwrap().engine;

    let mut mutated = LatestStateActuationV1::new("ckpt", "k1", r#"{"step":1}"#).unwrap();
    mutated.payload_json = "not-json".into();
    let req = ActuationBatchV1::new(
        "op-mutated-payload",
        vec![ActuationOperationV1::PutLatestState(mutated)],
    )
    .unwrap();
    let err = engine.actuate(req).unwrap_err();
    match err {
        EngineError::Actuation(ae) => {
            assert_eq!(ae.reason, ActuationErrorReason::NestedRequestInvalid);
            assert_eq!(ae.field_path, "/operations/0/payloadJson");
        }
        other => panic!("expected Actuation error, got {other:?}"),
    }

    let mut mutated_key = LatestStateActuationV1::new("ckpt", "k1", r#"{"step":1}"#).unwrap();
    mutated_key.record_key = "   ".into();
    let req = ActuationBatchV1::new(
        "op-mutated-key",
        vec![ActuationOperationV1::PutLatestState(mutated_key)],
    )
    .unwrap();
    let err = engine.actuate(req).unwrap_err();
    match err {
        EngineError::Actuation(ae) => {
            assert_eq!(ae.reason, ActuationErrorReason::NestedRequestInvalid);
            assert_eq!(ae.field_path, "/operations/0/recordKey");
        }
        other => panic!("expected Actuation error, got {other:?}"),
    }

    engine.close().unwrap();
}

#[test]
fn put_latest_state_commits_atomically_with_graph_node_and_replays() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "atomic");
    let schema =
        r#"{"type":"object","required":["step"],"properties":{"step":{"type":"integer"}}}"#;

    let first_receipt = {
        let engine = Engine::open(&path).unwrap().engine;
        register_collection(&engine, "checkpoints", "latest_state", schema);

        let state_op =
            LatestStateActuationV1::new("checkpoints", "run-1", r#"{"step":42}"#).unwrap();
        let node_op = canonical_node("rev-node-1", "node-1", "checkpointed memory");
        let batch = ActuationBatchV1::new(
            "op-atomic-1",
            vec![
                ActuationOperationV1::PutCanonicalNode(node_op),
                ActuationOperationV1::PutLatestState(state_op),
            ],
        )
        .unwrap();

        let receipt = engine.actuate(batch.clone()).unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
        assert_eq!(receipt.affected_revision_ids, vec!["rev-node-1".to_string()]);
        assert!(receipt.resulting_write_boundary.is_some());
        // PutLatestState itself is immediately terminal; only the node may be pending (if any).
        let boundary = receipt.resulting_write_boundary.unwrap();
        assert!(!receipt.pending_projection_write_cursors.contains(&boundary));

        let stored = engine.read_operational_state("checkpoints", "run-1", None).unwrap().unwrap();
        assert_eq!(stored.payload, r#"{"step":42}"#);
        assert_eq!(stored.schema_id.as_deref(), Some("checkpoints"));
        assert_eq!(stored.write_cursor, boundary);

        // Exact replay returns identical receipt.
        let replay = engine.actuate(batch).unwrap();
        assert_eq!(replay, receipt);

        // Conflicting payload under same operation_id fails with OperationIdConflict.
        let conflict_batch = ActuationBatchV1::new(
            "op-atomic-1",
            vec![
                ActuationOperationV1::PutCanonicalNode(canonical_node(
                    "rev-node-1",
                    "node-1",
                    "checkpointed memory",
                )),
                ActuationOperationV1::PutLatestState(
                    LatestStateActuationV1::new("checkpoints", "run-1", r#"{"step":99}"#).unwrap(),
                ),
            ],
        )
        .unwrap();
        let err = engine.actuate(conflict_batch).unwrap_err();
        match err {
            EngineError::Actuation(ae) => {
                assert_eq!(ae.reason, ActuationErrorReason::OperationIdConflict);
            }
            other => panic!("expected OperationIdConflict, got {other:?}"),
        }

        engine.close().unwrap();
        receipt
    };

    // Reopen and verify persistence + post-reopen write does not regress cursor.
    let engine = Engine::open(&path).unwrap().engine;
    let stored = engine.read_operational_state("checkpoints", "run-1", None).unwrap().unwrap();
    assert_eq!(stored.payload, r#"{"step":42}"#);

    let batch_only_state = ActuationBatchV1::new(
        "op-atomic-2",
        vec![ActuationOperationV1::PutLatestState(
            LatestStateActuationV1::new("checkpoints", "run-1", r#"{"step":43}"#).unwrap(),
        )],
    )
    .unwrap();
    let second = engine.actuate(batch_only_state).unwrap();
    assert_eq!(second.outcome, ActuationOutcomeV1::Committed);
    assert!(second.affected_revision_ids.is_empty());
    assert!(second.pending_projection_write_cursors.is_empty());
    assert!(
        second.resulting_write_boundary.unwrap() > first_receipt.resulting_write_boundary.unwrap()
    );

    let stored2 = engine.read_operational_state("checkpoints", "run-1", None).unwrap().unwrap();
    assert_eq!(stored2.payload, r#"{"step":43}"#);
    engine.close().unwrap();
}

#[test]
fn put_latest_state_refusals_use_collection_and_payload_paths_and_rollback_batch() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(db_path(&dir, "refusals")).unwrap().engine;
    let schema =
        r#"{"type":"object","required":["step"],"properties":{"step":{"type":"integer"}}}"#;
    register_collection(&engine, "checkpoints", "latest_state", schema);
    register_collection(&engine, "events", "append_only_log", "{}");

    // 1. Unregistered collection -> /operations/0/collection
    let unregistered = ActuationBatchV1::new(
        "op-unreg",
        vec![ActuationOperationV1::PutLatestState(
            LatestStateActuationV1::new("missing_col", "k1", "{}").unwrap(),
        )],
    )
    .unwrap();
    let r1 = engine.actuate(unregistered).unwrap();
    assert_eq!(r1.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(r1.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(r1.refused_operation_index, Some(0));
    assert_eq!(r1.refused_field_path.as_deref(), Some("/operations/0/collection"));

    // 2. Wrong kind (append_only_log) -> /operations/0/collection
    let wrong_kind = ActuationBatchV1::new(
        "op-wrong-kind",
        vec![ActuationOperationV1::PutLatestState(
            LatestStateActuationV1::new("events", "k1", "{}").unwrap(),
        )],
    )
    .unwrap();
    let r2 = engine.actuate(wrong_kind).unwrap();
    assert_eq!(r2.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(r2.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(r2.refused_operation_index, Some(0));
    assert_eq!(r2.refused_field_path.as_deref(), Some("/operations/0/collection"));

    // 3. Schema mismatch paired with PutCanonicalNode -> /operations/1/payloadJson and node rolled back
    let bad_schema = ActuationBatchV1::new(
        "op-bad-schema",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical_node(
                "rev-rollback-1",
                "rollback-node",
                "must not commit",
            )),
            ActuationOperationV1::PutLatestState(
                LatestStateActuationV1::new("checkpoints", "k1", r#"{"step":"wrong-type"}"#)
                    .unwrap(),
            ),
        ],
    )
    .unwrap();
    let r3 = engine.actuate(bad_schema).unwrap();
    assert_eq!(r3.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(r3.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(r3.refused_operation_index, Some(1));
    assert_eq!(r3.refused_field_path.as_deref(), Some("/operations/1/payloadJson"));
    assert!(engine
        .read_get("rollback-node", &fathomdb_engine::ReadView::default())
        .unwrap()
        .is_none());
    assert!(engine.read_operational_state("checkpoints", "k1", None).unwrap().is_none());

    engine.close().unwrap();
}

#[test]
fn put_latest_state_inherits_bound_caller_scope() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "scoped");
    {
        let admin = Engine::open(&path).unwrap().engine;
        register_collection(&admin, "checkpoints", "latest_state", r#"{"type":"object"}"#);
        admin.close().unwrap();
    }

    let alpha_cfg = EngineConfig {
        scope_policy: ScopePolicy::read_write(["alpha"], "alpha"),
        ..EngineConfig::default()
    };
    {
        let alpha = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, alpha_cfg)
            .unwrap()
            .engine;
        let receipt = alpha
            .actuate(
                ActuationBatchV1::new(
                    "shared-op-id",
                    vec![ActuationOperationV1::PutLatestState(
                        LatestStateActuationV1::new("checkpoints", "k1", r#"{"owner":"alpha"}"#)
                            .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
        let row = alpha.read_operational_state("checkpoints", "k1", None).unwrap().unwrap();
        assert_eq!(row.payload, r#"{"owner":"alpha"}"#);
        alpha.close().unwrap();
    }

    let beta_cfg = EngineConfig {
        scope_policy: ScopePolicy::read_write(["beta"], "beta"),
        ..EngineConfig::default()
    };
    {
        let beta = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, beta_cfg)
            .unwrap()
            .engine;
        // Alpha's row is invisible to beta.
        assert!(beta.read_operational_state("checkpoints", "k1", None).unwrap().is_none());
        // Same operation_id in beta scope commits independently without conflict!
        let receipt = beta
            .actuate(
                ActuationBatchV1::new(
                    "shared-op-id",
                    vec![ActuationOperationV1::PutLatestState(
                        LatestStateActuationV1::new("checkpoints", "k1", r#"{"owner":"beta"}"#)
                            .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
        let row = beta.read_operational_state("checkpoints", "k1", None).unwrap().unwrap();
        assert_eq!(row.payload, r#"{"owner":"beta"}"#);
        beta.close().unwrap();
    }
}

#[test]
fn ep07_create_if_missing_false_refuses_missing_zero_len_and_dangling_symlink_cleanly() {
    let dir = TempDir::new().unwrap();

    let expect_open_err = |res: Result<_, EngineOpenError>, msg: &str| match res {
        Err(err) => err,
        Ok(_) => panic!("{msg}"),
    };

    // 1. Missing file -> DatabaseNotFound, zero artifacts created.
    let missing = db_path(&dir, "does_not_exist");
    let err = expect_open_err(
        Engine::open_with_choice_config_and_create(
            &missing,
            EmbedderChoice::None,
            EngineConfig::default(),
            false,
        ),
        "missing DB must fail",
    );
    assert!(matches!(err, EngineOpenError::DatabaseNotFound { .. }), "got {err:?}");
    assert!(!missing.exists(), "must not create sqlite file");
    assert!(!lock_path(&missing).exists(), "must not leave newly created .lock file");

    // 2. Missing file with EmbedderChoice::Default -> DatabaseNotFound BEFORE embedder load!
    let missing_default = db_path(&dir, "missing_default_embedder");
    let err = expect_open_err(
        Engine::open_with_choice_config_and_create(
            &missing_default,
            EmbedderChoice::Default,
            EngineConfig::default(),
            false,
        ),
        "missing DB with Default embedder must fail with DatabaseNotFound",
    );
    assert!(
        matches!(err, EngineOpenError::DatabaseNotFound { .. }),
        "expected DatabaseNotFound before embedder load, got {err:?}"
    );
    assert!(!missing_default.exists());
    assert!(!lock_path(&missing_default).exists());

    // 3. Zero-length file -> DatabaseNotFound, file untouched, no .lock left.
    let zero_len = db_path(&dir, "empty_zero");
    std::fs::write(&zero_len, b"").unwrap();
    let err = expect_open_err(
        Engine::open_with_choice_config_and_create(
            &zero_len,
            EmbedderChoice::None,
            EngineConfig::default(),
            false,
        ),
        "zero-length DB must fail",
    );
    assert!(matches!(err, EngineOpenError::DatabaseNotFound { .. }), "got {err:?}");
    assert_eq!(std::fs::metadata(&zero_len).unwrap().len(), 0);
    assert!(!lock_path(&zero_len).exists());

    // 4. Pre-existing .lock on missing path -> DatabaseNotFound, pre-existing .lock preserved.
    let pre_locked_missing = db_path(&dir, "pre_locked_missing");
    let pre_lock = lock_path(&pre_locked_missing);
    std::fs::write(&pre_lock, b"stale").unwrap();
    let err = expect_open_err(
        Engine::open_with_choice_config_and_create(
            &pre_locked_missing,
            EmbedderChoice::None,
            EngineConfig::default(),
            false,
        ),
        "missing DB with pre-existing lock must fail",
    );
    assert!(matches!(err, EngineOpenError::DatabaseNotFound { .. }), "got {err:?}");
    assert!(pre_lock.exists(), "must not unlink a pre-existing .lock file");

    // 5. Dangling symlink -> Io error precedence, zero .lock created.
    #[cfg(unix)]
    {
        let dangling = dir.path().join("dangling.sqlite");
        std::os::unix::fs::symlink(dir.path().join("nonexistent_target.sqlite"), &dangling)
            .unwrap();
        let err = expect_open_err(
            Engine::open_with_choice_config_and_create(
                &dangling,
                EmbedderChoice::None,
                EngineConfig::default(),
                false,
            ),
            "dangling symlink must fail",
        );
        assert!(matches!(err, EngineOpenError::Io { .. }), "expected Io, got {err:?}");
        assert!(!lock_path(&dangling).exists());
    }

    // 6. Existing database opens normally with create_if_missing = false.
    let existing = db_path(&dir, "existing");
    {
        let opened = Engine::open(&existing).unwrap();
        opened.engine.close().unwrap();
    }
    let reopened = Engine::open_with_choice_config_and_create(
        &existing,
        EmbedderChoice::None,
        EngineConfig::default(),
        false,
    )
    .unwrap();
    reopened.engine.close().unwrap();
}

#[test]
fn ep01_read_visibility_state_tracks_writes_reopen_restore_and_close() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "vis");

    let s0 = {
        let engine = Engine::open(&path).unwrap().engine;
        let a = engine.read_visibility_state().unwrap();
        let b = engine.read_visibility_state().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.schema_version, 1);
        assert_eq!(a.database_id.len(), 32);
        assert_eq!(a.nonce.len(), 64);
        assert_eq!(a.canonical_write_boundary, 0);
        assert_eq!(a.projection_cursor, 0);
        engine.close().unwrap();
        a
    };

    // Reopen with no writes: visibility state is completely stable.
    let (s_after_write, snapshot_bytes) = {
        let engine = Engine::open(&path).unwrap().engine;
        let reopened = engine.read_visibility_state().unwrap();
        assert_eq!(reopened, s0, "Engine::open must not spuriously advance generation");

        // Copy DB before write for branch-restore test.
        engine.close().unwrap();
        let bytes = std::fs::read(&path).unwrap();

        let engine = Engine::open(&path).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "note".into(),
                body: "hello visibility".into(),
                source_id: SourceId::new("src-vis").unwrap(),
                logical_id: Some("n1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap();
        let after = engine.read_visibility_state().unwrap();
        assert_eq!(after.database_id, s0.database_id);
        assert!(after.generation > s0.generation);
        assert_ne!(after.nonce, s0.nonce);
        assert_eq!(after.canonical_write_boundary, 1);

        // Append-only operational_mutations advances canonical_write_boundary (even without visibility trigger).
        register_collection(&engine, "logs", "append_only_log", "{}");
        let after_schema = engine.read_visibility_state().unwrap();
        engine
            .write(&[PreparedWrite::OpStore {
                collection: "logs".into(),
                record_key: "r1".into(),
                schema_id: Some("logs".into()),
                body: r#"{"ok":true}"#.into(),
            }])
            .unwrap();
        let after_append = engine.read_visibility_state().unwrap();
        assert!(after_append.canonical_write_boundary > after_schema.canonical_write_boundary);

        engine.close().unwrap();
        assert!(matches!(engine.read_visibility_state(), Err(EngineError::Closing)));
        (after, bytes)
    };

    // Restore backup into a new file, perform one write -> nonce must differ even if generation repeats.
    let restored_path = db_path(&dir, "vis_restored");
    std::fs::write(&restored_path, snapshot_bytes).unwrap();
    let restored = Engine::open(&restored_path).unwrap().engine;
    restored
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "diverged branch".into(),
            source_id: SourceId::new("src-vis").unwrap(),
            logical_id: Some("n1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
    let diverged = restored.read_visibility_state().unwrap();
    assert_eq!(diverged.database_id, s_after_write.database_id);
    assert_ne!(diverged.nonce, s_after_write.nonce);
    restored.close().unwrap();
}
