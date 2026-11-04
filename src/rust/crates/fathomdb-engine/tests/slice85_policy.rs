use fathomdb_engine::Engine;

#[test]
fn fresh_store_uses_format36_before_policy_can_be_ignored_by_old_binary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fresh.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    assert_eq!(engine.query_i64_col_for_test("PRAGMA user_version").unwrap(), vec![36]);
    engine.close().unwrap();
}

#[test]
fn explicit_policy_is_persisted_honored_and_mismatch_refused_without_mutation() {
    use fathomdb_engine::{EmbedderChoice, EngineConfig, PhysicalErasurePolicy};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("explicit.sqlite");
    let engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
            ..Default::default()
        },
    )
    .unwrap()
    .engine;
    assert_eq!(engine.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    engine.close().unwrap();
    let reopened = Engine::open(&path).unwrap().engine;
    assert_eq!(reopened.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    reopened.close().unwrap();
    let before = std::fs::read(&path).unwrap();
    let mismatch = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::Autonomous),
            ..Default::default()
        },
    );
    assert!(mismatch.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn format36_policy_is_mandatory_and_unknown_policy_refuses() {
    for replacement in [None, Some("unknown")] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.sqlite");
        let engine = Engine::open(&path).unwrap().engine;
        assert_eq!(engine.query_text_col_for_test("SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure_policy'").unwrap(), vec!["autonomous"]);
        engine
            .execute_for_test(
                "DELETE FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure_policy'",
            )
            .unwrap();
        if replacement.is_some() {
            engine.execute_for_test("INSERT INTO _fathomdb_open_state(key,value) VALUES('_fathomdb_physical_erasure_policy','unknown')").unwrap();
        }
        engine.close().unwrap();
        assert!(Engine::open(&path).is_err());
    }
}
