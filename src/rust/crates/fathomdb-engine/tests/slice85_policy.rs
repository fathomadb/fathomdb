use fathomdb_engine::Engine;

#[test]
fn fresh_store_uses_format36_before_policy_can_be_ignored_by_old_binary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fresh.sqlite");
    let engine = Engine::open(&path).unwrap().engine;
    assert_eq!(engine.query_i64_col_for_test("PRAGMA user_version").unwrap(), vec![36]);
    engine.close().unwrap();
}
