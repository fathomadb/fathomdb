use std::sync::{mpsc, Arc};
use std::time::Duration;

use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};

fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn committed_sanitation_unit_releases_primary_for_native_read() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(Engine::open(dir.path().join("yield.sqlite")).unwrap().engine);
    engine
        .write(&[node("target", "target", "privateword"), node("survivor", "other", "publicword")])
        .unwrap();
    let other = Arc::clone(&engine);
    fathomdb_engine::arm_physical_erasure_hook_for_test(
        "after_sanitation_unit",
        Box::new(move || {
            let (tx, rx) = mpsc::channel();
            let reader = Arc::clone(&other);
            std::thread::spawn(move || {
                let _ = tx.send(reader.node_importance(2));
            });
            rx.recv_timeout(Duration::from_secs(2)).map_err(|_| EngineError::Scheduler)??;
            Ok(())
        }),
    );
    engine.erase_source("target").expect(
        "a committed sanitation unit must release the primary connection before its rendezvous",
    );
    engine.close().unwrap();
}

fn dependent_seed(path: &std::path::Path) -> Engine {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, ProvenancedNodeV1, SourceDependencyRegistrationV1,
        SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
    };
    use sha2::Digest;
    let engine = Engine::open(path).unwrap().engine;
    let body = "privateuniqueterm subject";
    let hash = sha2::Sha256::digest(body.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let make = |logical: &str, content: &str, provenance| {
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "doc".into(),
            body: content.into(),
            source_id: SourceId::new("target").unwrap(),
            logical_id: Some(logical.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance,
        })
    };
    engine
        .write(&[
            make(
                "subject",
                body,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            ),
            make(
                "dependent",
                "derived private text",
                WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(&hash).unwrap(),
                ),
            ),
            node("survivor", "survivor", "public content"),
        ])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    engine
}

#[test]
fn unrelated_survivor_updates_progress_while_registered_debt_is_sanitizing() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(dependent_seed(&dir.path().join("scoped.sqlite")));
    let other = Arc::clone(&engine);
    fathomdb_engine::arm_physical_erasure_hook_for_test(
        "after_sanitation_unit",
        Box::new(move || {
            assert_eq!(other.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_dependency_closures WHERE phase!='complete'").unwrap(), vec![1]);
            for write in
                [node("new", "other", "new public"), node("survivor", "survivor", "updated public")]
            {
                other
                    .write(&[write])
                    .expect("unrelated plain survivor writes must progress during sanitation");
            }
            for write in [
                node("replacement", "target", "new payload"),
                node("dependent", "other", "recreated derived payload"),
            ] {
                assert!(
                    matches!(other.write(&[write]), Err(EngineError::ErasureIncomplete { .. })),
                    "affected identities remain fenced"
                );
            }
            Ok(())
        }),
    );
    engine.erase_source("target").unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes WHERE source_id='target'")
            .unwrap(),
        vec![0]
    );
    assert_eq!(engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes WHERE logical_id='survivor' AND superseded_at IS NULL AND body='updated public'").unwrap(), vec![1]);
    engine.close().unwrap();
}

#[test]
fn unsupported_native_envelope_refuses_before_deletion_or_debt() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("envelope.sqlite")).unwrap().engine;
    engine.write(&[node("target", "target", &"x".repeat((1 << 20) + 1))]).unwrap();
    assert!(
        matches!(engine.erase_source("target"), Err(EngineError::ErasureIncomplete { stage, .. }) if stage == "physical_admission")
    );
    assert_eq!(
        engine
            .query_i64_col_for_test("SELECT count(*) FROM canonical_nodes WHERE source_id='target'")
            .unwrap(),
        vec![1]
    );
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'"
            )
            .unwrap(),
        vec![0]
    );
    engine.close().unwrap();
}

#[test]
fn missing_v2_denial_authority_refuses_open() {
    for field in ["denied_sources", "denied_identities", "safe_boundary", "redacted"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid-authority.sqlite");
        let engine = Engine::open(&path).unwrap().engine;
        engine.write(&[node("target", "target", "privateword")]).unwrap();
        fathomdb_engine::arm_physical_erasure_hook_for_test(
            "after_delete_commit",
            Box::new(|| Err(EngineError::Storage)),
        );
        assert!(engine.erase_source("target").is_err());
        engine.close().unwrap();
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute("UPDATE _fathomdb_open_state SET value=json_remove(value,?1) WHERE key='_fathomdb_physical_erasure'", [format!("$.{field}")]).unwrap();
        drop(c);
        assert!(Engine::open(&path).is_err(), "v2 journal missing {field} cannot authorize writes");
    }
}

fn edge(id: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "rel".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("other").unwrap(),
        logical_id: Some(id.into()),
        body: Some("public relationship".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

#[test]
fn survivor_edge_supersession_cannot_touch_an_old_denied_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(dependent_seed(&dir.path().join("old-endpoint.sqlite")));
    engine.write(&[edge("reference", "subject", "survivor")]).unwrap();
    let other = Arc::clone(&engine);
    fathomdb_engine::arm_physical_erasure_hook_for_test(
        "after_sanitation_unit",
        Box::new(move || {
            assert!(
                matches!(
                    other.write(&[edge("reference", "survivor", "survivor")]),
                    Err(EngineError::ErasureIncomplete { .. })
                ),
                "old affected endpoints remain fenced even when replacement endpoints are clean"
            );
            assert_eq!(other.query_i64_col_for_test("SELECT count(*) FROM canonical_edges WHERE logical_id='reference' AND from_id='subject' AND superseded_at IS NULL").unwrap(), vec![1]);
            Ok(())
        }),
    );
    engine.erase_source("target").unwrap();
    engine.close().unwrap();
}

#[test]
fn busy_finalization_redacts_denial_metadata_and_retry_processes_its_own_argument() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("redacted.sqlite");
    let engine = dependent_seed(&path);
    let pin = rusqlite::Connection::open(&path).unwrap();
    pin.execute_batch("BEGIN; SELECT * FROM canonical_nodes;").unwrap();
    assert!(matches!(engine.erase_source("target"), Err(EngineError::ErasureIncomplete { .. })));
    assert_eq!(engine.query_i64_col_for_test("SELECT json_extract(value,'$.redacted') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'").unwrap(), vec![1], "checkpoint waits on a durable identity-free finalization tombstone");
    assert_eq!(engine.query_i64_col_for_test("SELECT length(json_extract(value,'$.retry')) + json_array_length(value,'$.denied_sources') + json_array_length(value,'$.denied_identities') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'").unwrap(), vec![0]);
    assert!(matches!(
        engine.write(&[node("new", "other", "public")]),
        Err(EngineError::ErasureIncomplete { .. })
    ));
    pin.execute_batch("ROLLBACK").unwrap();
    engine.erase_source("survivor").unwrap();
    assert_eq!(
        engine.query_i64_col_for_test("SELECT count(*) FROM canonical_nodes").unwrap(),
        vec![0],
        "finishing another target is not success for this call's own argument"
    );
    engine.close().unwrap();
}
