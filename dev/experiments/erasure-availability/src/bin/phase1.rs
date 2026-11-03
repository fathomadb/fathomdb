use erasure_availability::{open, physical_markers, Result, MARKER};
use rusqlite::ffi;
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex, time::Instant};

static PROFILE: Mutex<BTreeMap<String, (u64, u64, u64)>> = Mutex::new(BTreeMap::new());

unsafe extern "C" fn observe(
    code: u32,
    _: *mut std::ffi::c_void,
    statement: *mut std::ffi::c_void,
    nanos: *mut std::ffi::c_void,
) -> i32 {
    if code == ffi::SQLITE_TRACE_PROFILE as u32 {
        let statement = statement.cast::<ffi::sqlite3_stmt>();
        // SQLite owns both pointers for the duration of its PROFILE callback.
        let sql =
            unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_sql(statement)) }.to_string_lossy();
        // Keep only top-level product statements; FTS shadow statements overlap.
        if sql.contains("search_index") || sql.contains("canonical_") || sql.contains("_fathomdb_")
        {
            let mut profile = PROFILE.lock().unwrap();
            let row = profile.entry(sql.into_owned()).or_default();
            row.0 += 1;
            row.1 += unsafe { *nanos.cast::<u64>() };
            row.2 += unsafe {
                ffi::sqlite3_stmt_status(statement, ffi::SQLITE_STMTSTATUS_FULLSCAN_STEP, 1)
            }
            .max(0) as u64;
        }
    }
    0
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(
            "usage: phase1 MASTER DISPOSABLE_OUTPUT (copies master, profiles native erase)".into(),
        );
    }
    let master = PathBuf::from(&args[0]);
    let path = PathBuf::from(&args[1]);
    if path.exists() {
        return Err("output already exists".into());
    }
    std::fs::copy(&master, &path)?;
    let engine = open(&path)?;
    engine.maintenance_transaction_for_test(|tx| {
        unsafe {
            ffi::sqlite3_trace_v2(
                tx.handle(),
                ffi::SQLITE_TRACE_PROFILE as u32,
                Some(observe),
                std::ptr::null_mut(),
            );
        }
        Ok(())
    })?;
    let start = Instant::now();
    let report = engine.erase_source("target");
    let elapsed = start.elapsed();
    engine.maintenance_transaction_for_test(|tx| {
        unsafe {
            ffi::sqlite3_trace_v2(tx.handle(), 0, None, std::ptr::null_mut());
        }
        Ok(())
    })?;
    let mut statements = PROFILE.lock().unwrap().iter().map(|(sql, (calls, ns, scans))| {
        json!({"sql":sql,"calls":calls,"profile_ns":ns,"fullscan_steps":scans})
    }).collect::<Vec<_>>();
    statements.sort_by_key(|row| std::cmp::Reverse(row["profile_ns"].as_u64().unwrap()));
    println!(
        "{}",
        json!({"native_ns":elapsed.as_nanos() as u64,"error":report.as_ref().err().map(ToString::to_string),"report":report.as_ref().ok().map(|r|json!({"nodes":r.nodes_excised,"edges":r.edges_excised,"projections":r.projections_invalidated})),"physical_markers":physical_markers(&path,MARKER)?,"statements":statements})
    );
    engine.close()?;
    report?;
    Ok(())
}
