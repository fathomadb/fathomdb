use current_history_followups::current::CurrentStore;
use fathomdb_engine::{EngineError, InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};

fn write(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("needle {id}"),
        source_id: SourceId::new(id).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn count(read: &Connection, sql: &str) -> i64 {
    read.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn current_cache_erasure_is_synchronous_secure_and_transactional() {
    for pinned in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("current.sqlite");
        let store = CurrentStore::open(&path).unwrap();
        store.write(&[write("target"), write("survivor")]).unwrap();
        store.bootstrap(50).unwrap();
        let marker = "qzxvplmnbvcxzasdfghjklqwertx";
        store.with_writer(|e|e.execute_for_test(&format!("UPDATE poc_current SET body='{marker}' WHERE rowid=(SELECT write_cursor FROM canonical_nodes WHERE source_id='target')"))).unwrap();
        let read = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let shadow = format!(
            "SELECT COUNT(*) FROM poc_current_data WHERE instr(block,CAST('{marker}' AS BLOB))>0"
        );
        assert!(count(&read, &shadow) > 0, "pre-erasure raw term must be witnessed");
        store
            .with_writer(|e| {
                e.execute_for_test(
                    "BEGIN; DELETE FROM canonical_nodes WHERE source_id='target'; ROLLBACK;",
                )
            })
            .unwrap();
        assert_eq!(
            count(&read, "SELECT COUNT(*) FROM poc_current"),
            2,
            "rollback restores candidate projection too"
        );
        let pinned_tx = if pinned {
            let tx = read.unchecked_transaction().unwrap();
            assert_eq!(count(&tx, "SELECT COUNT(*) FROM poc_current"), 2);
            Some(tx)
        } else {
            None
        };
        let erased = store.with_writer(|e| e.erase_source("target"));
        if pinned {
            assert!(
                matches!(erased,Err(EngineError::ErasureIncomplete{ref stage,..}) if stage=="wal_checkpoint")
            );
        } else {
            erased.unwrap();
        }
        let fresh = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            count(&fresh, &format!("SELECT COUNT(*) FROM poc_current WHERE body='{marker}'")),
            0,
            "erase commit must not wait for a subsequent candidate search"
        );
        assert_eq!(
            count(
                &fresh,
                &format!("SELECT COUNT(*) FROM poc_current WHERE poc_current MATCH '{marker}'")
            ),
            0
        );
        assert_eq!(
            count(&fresh, &shadow),
            0,
            "candidate FTS live shadow must not retain the erased term"
        );
        assert_eq!(
            count(&fresh, "SELECT COUNT(*) FROM poc_current WHERE poc_current MATCH 'survivor'"),
            1
        );
        if let Some(tx) = pinned_tx {
            assert_eq!(count(&tx, "SELECT COUNT(*) FROM poc_current"), 2);
            drop(tx);
            store.with_writer(|e| e.erase_source("target")).unwrap();
        }
        assert_eq!(
            store.search("needle", 50, 10).unwrap().rows.len(),
            1,
            "refresh cannot resurrect erased owners"
        );
        store.close().unwrap();
    }
}
