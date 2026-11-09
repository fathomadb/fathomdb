use std::sync::Arc;

use rusqlite::Connection;

use crate::*;

fn armed(connection: &Connection) {
    connection.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1", []).unwrap();
}

#[test]
fn boot_reconciliation_refuses_committed_armed_state_before_any_effect() {
    let dir = tempfile::tempdir().unwrap();
    let opened = Engine::open(dir.path().join("boot.sqlite")).unwrap();
    let guard = opened.engine.connection.lock().unwrap();
    let connection = guard.as_ref().unwrap();
    armed(connection);
    let changes = connection.total_changes();
    assert!(
        crate::projection_registry::reconcile_inert_vector_enrolments_on_boot(connection).is_err()
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn physical_transaction_refuses_armed_state_without_running_the_operation() {
    let dir = tempfile::tempdir().unwrap();
    let opened = Engine::open(dir.path().join("physical.sqlite")).unwrap();
    let guard = opened.engine.connection.lock().unwrap();
    let connection = guard.as_ref().unwrap();
    armed(connection);
    let executed = std::cell::Cell::new(false);
    assert!(crate::physical_erasure::transaction(connection, || {
        executed.set(true);
        Ok(())
    })
    .is_err());
    assert!(!executed.get());
    assert!(connection.is_autocommit());
    assert!(crate::dependency_closure::maintain_before_writer(connection).is_err());
}

#[test]
fn projection_and_noncoalesced_writers_refuse_external_armed_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outer.sqlite");
    let opened = Engine::open(&path).unwrap();
    let external = Connection::open(&path).unwrap();
    assert_eq!(
        external
            .query_row("SELECT coalescing FROM _fathomdb_read_visibility_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    armed(&external);
    {
        let mut guard = opened.engine.connection.lock().unwrap();
        let connection = guard.as_mut().unwrap();
        let changes = connection.total_changes();
        assert!(crate::projection_commit::commit_projection_outcomes(
            connection,
            &[],
            &opened.engine.projection_runtime.shared,
            0
        )
        .is_err());
        assert_eq!(connection.total_changes(), changes);
    }
    assert!(matches!(opened.engine.write_node_importance(1, 0.5), Err(EngineError::Storage)));
    assert!(matches!(opened.engine.configure_projections(&[], &[]), Err(EngineError::Storage)));
    assert_eq!(
        external
            .query_row("SELECT coalescing FROM _fathomdb_read_visibility_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn maintenance_admission_refusal_releases_runtime_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("maintenance.sqlite");
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    let external = Connection::open(&path).unwrap();
    armed(&external);
    assert!(matches!(engine.begin_erasure_maintenance(1000), Err(EngineError::Storage)));
    external.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=0", []).unwrap();
    engine.begin_erasure_maintenance(1000).unwrap().close().unwrap();
    engine.close().unwrap();
}

#[test]
fn vector_mapping_cleanup_remains_active_while_visibility_is_coalesced() {
    let dir = tempfile::tempdir().unwrap();
    let opened = Engine::open(dir.path().join("mapping.sqlite")).unwrap();
    let mut primary = opened.engine.connection.lock().unwrap();
    let connection = primary.as_mut().unwrap();
    let tx =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
    let mut guard = VisibilityCoalescingGuard::arm(&tx).unwrap();
    tx.execute(
        "INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(42,'doc',42)",
        [],
    )
    .unwrap();
    tx.execute("INSERT INTO _fathomdb_vector_spans VALUES(42,42,0,0,NULL,0)", []).unwrap();
    tx.execute("DELETE FROM _fathomdb_vector_rows WHERE rowid=42", []).unwrap();
    assert_eq!(
        tx.query_row("SELECT COUNT(*) FROM _fathomdb_vector_spans", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    advance_read_visibility(&tx).unwrap();
    guard.restore().unwrap();
    drop(guard);
    tx.commit().unwrap();
}
