use crate::{fixtures::database, policy::Policy, session::Session};
#[test]
fn admitted_session_owns_engine_lock_and_releases_on_close() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let before = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row("SELECT group_concat(logical_id||':'||body) FROM canonical_nodes", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap();
    let session = Session::attach(&path, Policy::default()).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("{}.lock", path.display()))
        .unwrap();
    assert!(lock.try_lock().is_err());
    session.close().unwrap();
    assert!(lock.try_lock().is_ok());
    drop(lock);
    let after = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row("SELECT group_concat(logical_id||':'||body) FROM canonical_nodes", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap();
    assert_eq!(before, after);
}

#[test]
fn session_refuses_a_committed_dirty_coalesced_visibility_context() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let session = Session::attach(&path, Policy::default()).unwrap();
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer
        .execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1", [])
        .unwrap();
    assert!(matches!(session.connection(), Err("attachment_expired")));
    writer
        .execute("UPDATE _fathomdb_read_visibility_state SET coalescing=0 WHERE singleton=1", [])
        .unwrap();
    drop(writer);
    session.close().unwrap();
}
