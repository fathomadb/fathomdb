use erasure_availability::{
    build_campaign, node, open, physical_markers, prepare_eager, Result, MARKER,
};
use fathomdb_engine::{ProjectionFts, ProjectionRole, ProjectionSpec};
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
    if code == ffi::SQLITE_TRACE_PROFILE {
        let statement = statement.cast::<ffi::sqlite3_stmt>();
        // SQLite owns both pointers for the duration of its PROFILE callback.
        let sql =
            unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_sql(statement)) }.to_string_lossy();
        // Shadow statement times overlap their parent; never sum both as stage time.
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
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "fixture") && args.len() == 4 {
        let started = Instant::now();
        build_campaign(&PathBuf::from(&args[1]), &args[2], false, args[3].parse()?)?;
        println!("{}", json!({"preparation_ns":started.elapsed().as_nanos() as u64}));
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "mutations") && args.len() == 4 {
        let path = PathBuf::from(&args[1]);
        if path.exists() {
            return Err("output already exists".into());
        }
        let engine = open(&path)?;
        if args[3] == "secure" {
            prepare_eager(&engine)?;
        }
        if args[2] == "property" {
            engine.configure_projections(
                &[ProjectionSpec {
                    name: "p1".into(),
                    roles: std::collections::BTreeSet::from([ProjectionRole::Searchable]),
                    fts: Some(ProjectionFts { tokenizer: None }),
                    vector: None,
                    source: None,
                }],
                &[],
            )?;
        }
        let mut times = Vec::new();
        for revision in 0..5 {
            let started = Instant::now();
            let writes = (0..1000)
                .map(|i| {
                    let body = format!(
                        "common owner{i} revision{revision} {}",
                        "alpha beta gamma delta ".repeat(40)
                    );
                    let body =
                        if args[2] == "property" { json!({"p1":body}).to_string() } else { body };
                    node(&format!("owner{i}"), "survivor", &body)
                })
                .collect::<Vec<_>>();
            engine.write(&writes)?;
            engine.drain(600_000)?;
            times.push(started.elapsed().as_nanos() as u64);
        }
        println!(
            "{}",
            json!({"family":args[2],"policy":args[3],"owners":1000,"revisions":5,"round_ns":times})
        );
        engine.close()?;
        return Ok(());
    }
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
                ffi::SQLITE_TRACE_PROFILE,
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
