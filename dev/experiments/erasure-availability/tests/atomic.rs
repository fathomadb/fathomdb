use fathomdb_engine::Engine;

#[test]
fn maintenance_commits_value_and_rolls_back_failure_before_writer_release() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("atomic.sqlite")).unwrap().engine;
    engine.execute_for_test("CREATE TABLE poc_atomic(value INTEGER)").unwrap();
    let (value, _wait, held) = engine
        .maintenance_transaction_for_test(|tx| {
            tx.execute("INSERT INTO poc_atomic VALUES(7)", [])?;
            Ok(7)
        })
        .expect("successful maintenance commits");
    assert_eq!(value, 7);
    assert!(!held.is_zero());
    let failed = engine.maintenance_transaction_for_test(|tx| {
        tx.execute("INSERT INTO poc_atomic VALUES(9)", [])?;
        tx.execute("INSERT INTO missing_table VALUES(1)", [])
    });
    assert!(failed.is_err());
    assert_eq!(engine.query_i64_col_for_test("SELECT value FROM poc_atomic").unwrap(), vec![7]);
    engine
        .maintenance_transaction_for_test(|tx| tx.execute("INSERT INTO poc_atomic VALUES(11)", []))
        .unwrap();
    assert_eq!(
        engine.query_i64_col_for_test("SELECT SUM(value) FROM poc_atomic").unwrap(),
        vec![18]
    );
    engine.close().unwrap();
}
