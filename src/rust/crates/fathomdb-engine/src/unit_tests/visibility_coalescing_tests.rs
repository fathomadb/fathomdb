use rusqlite::{Connection, StatementStatus, Transaction, TransactionBehavior};

use crate::write_commit::{advance_read_visibility, VisibilityCoalescingGuard};

fn fixture(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TABLE _fathomdb_read_visibility_state(
               singleton INTEGER PRIMARY KEY CHECK(singleton=1),
               generation INTEGER NOT NULL,
               state_nonce TEXT NOT NULL,
               coalescing INTEGER NOT NULL DEFAULT 0
                 CHECK(typeof(coalescing)='integer' AND coalescing IN(0,1))
             );
             INSERT INTO _fathomdb_read_visibility_state VALUES(1,7,'original',0);
             CREATE TABLE sentinel(value INTEGER NOT NULL);
             INSERT INTO sentinel VALUES(42);",
        )
        .unwrap();
}

fn state(connection: &Connection) -> (i64, String, i64) {
    connection
        .query_row(
            "SELECT generation,state_nonce,coalescing
             FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

#[test]
fn coalescing_preserves_a_held_statement_without_reprepare() {
    let connection = Connection::open_in_memory().unwrap();
    fixture(&connection);
    let mut sentinel = connection.prepare("SELECT value FROM sentinel").unwrap();
    assert_eq!(sentinel.query_row([], |row| row.get::<_, i64>(0)).unwrap(), 42);
    sentinel.reset_status(StatementStatus::RePrepare);
    let tx = Transaction::new_unchecked(&connection, TransactionBehavior::Immediate).unwrap();
    let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
    assert_eq!(sentinel.query_row([], |row| row.get::<_, i64>(0)).unwrap(), 42);
    let armed_reprepares = sentinel.get_status(StatementStatus::RePrepare);
    guard.restore().unwrap();
    drop(guard);
    assert_eq!(sentinel.query_row([], |row| row.get::<_, i64>(0)).unwrap(), 42);
    assert_eq!(armed_reprepares, 0, "arming invalidated a held SQLite program");
    assert_eq!(sentinel.get_status(StatementStatus::RePrepare), 0);
    tx.commit().unwrap();
}

#[test]
fn coalescing_arms_and_checked_advance_disarms_atomically() {
    let mut connection = Connection::open_in_memory().unwrap();
    fixture(&connection);
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
    let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
    assert_eq!(state(&tx), (7, "original".into(), 1));
    advance_read_visibility(&tx).unwrap();
    let (generation, nonce, flag) = state(&tx);
    assert_eq!((generation, flag), (8, 0));
    assert_ne!(nonce, "original");
    guard.restore().unwrap();
    drop(guard);
    tx.commit().unwrap();
    assert_eq!(state(&connection), (8, nonce, 0));
}

#[test]
fn coalescing_no_change_restore_retains_generation_and_nonce() {
    let mut connection = Connection::open_in_memory().unwrap();
    fixture(&connection);
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
    let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
    assert_eq!(state(&tx).2, 1);
    guard.restore().unwrap();
    drop(guard);
    tx.commit().unwrap();
    assert_eq!(state(&connection), (7, "original".into(), 0));
}

#[test]
fn coalescing_rechecks_clean_state_under_the_writer_lock() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("admission.sqlite");
    let mut connection = Connection::open(&path).unwrap();
    fixture(&connection);
    assert_eq!(state(&connection).2, 0, "clean pretransaction observation");
    let external = Connection::open(&path).unwrap();
    external.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1", []).unwrap();
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
    assert!(VisibilityCoalescingGuard::arm(&tx).is_err(), "committed armed state was admitted");
    assert_eq!(
        tx.query_row("SELECT value FROM sentinel", [], |row| row.get::<_, i64>(0)).unwrap(),
        42
    );
}

#[test]
fn coalescing_restore_after_sqlite_rollback_never_opens_a_write() {
    let connection = Connection::open_in_memory().unwrap();
    fixture(&connection);
    connection
        .execute_batch(
            "CREATE TRIGGER abort_sentinel BEFORE UPDATE ON sentinel
         BEGIN SELECT RAISE(ROLLBACK,'rollback the owned transaction'); END;",
        )
        .unwrap();
    let tx = Transaction::new_unchecked(&connection, TransactionBehavior::Immediate).unwrap();
    let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
    assert_eq!(state(&tx).2, 1);
    assert!(tx.execute("UPDATE sentinel SET value=99", []).is_err());
    assert!(connection.is_autocommit());
    let changes = connection.total_changes();
    guard.restore().unwrap();
    drop(guard);
    drop(tx);
    assert_eq!(connection.total_changes(), changes);
    assert_eq!(state(&connection), (7, "original".into(), 0));
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 24, failure_persistence: None, ..Default::default()
    })]
    #[test]
    fn coalescing_rollback_sequences_preserve_the_committed_binding(
        commits in proptest::collection::vec(proptest::bool::ANY, 1..12)
    ) {
        let mut connection = Connection::open_in_memory().unwrap();
        fixture(&connection);
        let mut expected = state(&connection);
        for commit in commits {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
            let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
            proptest::prop_assert_eq!(state(&tx).2, 1);
            advance_read_visibility(&tx).unwrap();
            let advanced = state(&tx);
            guard.restore().unwrap();
            drop(guard);
            if commit {
                tx.commit().unwrap();
                expected = advanced;
            } else {
                tx.rollback().unwrap();
            }
            proptest::prop_assert_eq!(state(&connection), expected.clone());
            proptest::prop_assert_eq!(expected.2, 0);
        }
    }
}

#[test]
fn coalescing_refuses_missing_null_noninteger_and_committed_armed_state() {
    for value in ["NULL", "'corrupt'", "0.5", "1", "2"] {
        let mut connection = Connection::open_in_memory().unwrap();
        fixture(&connection);
        connection
            .execute_batch(&format!(
                "DROP TABLE _fathomdb_read_visibility_state;
             CREATE TABLE _fathomdb_read_visibility_state AS
             SELECT 1 AS singleton,7 AS generation,'original' AS state_nonce,
                    {value} AS coalescing;"
            ))
            .unwrap();
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
        assert!(VisibilityCoalescingGuard::arm(&tx).is_err(), "admitted {value}");
    }
    let mut connection = Connection::open_in_memory().unwrap();
    fixture(&connection);
    connection.execute("DELETE FROM _fathomdb_read_visibility_state", []).unwrap();
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
    assert!(VisibilityCoalescingGuard::arm(&tx).is_err(), "admitted missing singleton");
}
