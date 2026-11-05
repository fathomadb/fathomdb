use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::ffi;
use std::sync::atomic::{AtomicUsize, Ordering};

static SCANS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn observe(
    code: u32,
    _: *mut std::ffi::c_void,
    statement: *mut std::ffi::c_void,
    _: *mut std::ffi::c_void,
) -> i32 {
    if code == ffi::SQLITE_TRACE_STMT {
        // SQLite owns the statement and its SQL during this callback.
        let sql = unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_sql(statement.cast())) }
            .to_string_lossy();
        if sql.starts_with("DELETE FROM ")
            && sql.contains("search_index")
            && sql.contains("write_cursor")
        {
            SCANS.fetch_add(1, Ordering::SeqCst);
        }
    }
    0
}

#[test]
fn root_erasure_scans_each_fts_family_once_instead_of_once_per_revision() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("batch.sqlite")).unwrap().engine;
    let writes = (0..20)
        .map(|i| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("term{i} common"),
            source_id: SourceId::new("target").unwrap(),
            logical_id: Some(format!("id{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    engine.write(&writes).unwrap();
    engine
        .maintenance_transaction_for_test(|tx| {
            unsafe {
                ffi::sqlite3_trace_v2(
                    tx.handle(),
                    ffi::SQLITE_TRACE_STMT,
                    Some(observe),
                    std::ptr::null_mut(),
                );
            }
            Ok(())
        })
        .unwrap();
    SCANS.store(0, Ordering::SeqCst);
    let report = engine.erase_source("target").unwrap();
    let scans = SCANS.load(Ordering::SeqCst);
    engine
        .maintenance_transaction_for_test(|tx| {
            unsafe {
                ffi::sqlite3_trace_v2(tx.handle(), 0, None, std::ptr::null_mut());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(report.nodes_excised, 20);
    assert!(
        scans <= 4,
        "at most one cursor scan/delete per registered FTS family, observed {scans}"
    );
    engine.close().unwrap();
}
