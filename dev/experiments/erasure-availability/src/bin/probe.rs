use erasure_availability::*;
use fathomdb_engine::{Engine, ReadView};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn wall() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}
fn validate(path: &Path, family: &str, owners: i64, erased: bool) -> Result<Value> {
    let c = reader(path)?;
    let scalar = |sql: &str| c.query_row(sql, [], |r| r.get::<_, i64>(0));
    let n = if erased { owners * 9 / 10 } else { owners };
    let expected = match family {
        "edge" => [owners + 1, n * 5, 0, owners + 1],
        "property" => [n * 5, 0, n * 2, n * 5],
        "node" => [n * 5, 0, 0, n * 5],
        _ => return Err("unknown family".into()),
    };
    let inv = inventory(path, MARKER)?;
    let targets = if erased { 0 } else { owners / 10 };
    let expected_matches = match family {
        "edge" => [0, targets * 5, 0, 0],
        "property" => [targets * 5, 0, targets * 2, targets * 5],
        _ => [targets * 5, 0, 0, targets * 5],
    };
    for ((row, count), matches) in inv.iter().zip(expected).zip(expected_matches) {
        if row.rows != count || row.matches != matches {
            return Err(format!("fixture count mismatch {}: rows {} expected {count}, matches {} expected {matches}",row.table,row.rows,row.matches).into());
        }
        if matches > 0 && row.shadow_markers == 0 {
            return Err(format!("no positive shadow witness {}", row.table).into());
        }
    }
    let nodes = scalar("SELECT count(*) FROM canonical_nodes")?;
    let edges = scalar("SELECT count(*) FROM canonical_edges")?;
    if nodes != expected[0] || edges != expected[1] {
        return Err("canonical revision counts mismatch".into());
    }
    let owner_table = if family == "edge" { "canonical_edges" } else { "canonical_nodes" };
    let target_owners = scalar(&format!(
        "SELECT count(DISTINCT logical_id) FROM {owner_table} WHERE source_id='target'"
    ))?;
    let surviving_owners = scalar(&format!(
        "SELECT count(DISTINCT logical_id) FROM {owner_table} WHERE source_id='survivor'"
    ))?;
    if target_owners != targets || surviving_owners != owners * 9 / 10 {
        return Err("source ownership count mismatch".into());
    }
    let primary = match family {
        "edge" => "search_index_edges",
        "property" => "property_search_index",
        _ => "search_index",
    };
    let column = if family == "property" { "attr_value" } else { "body" };
    for bound in ["min", "max"] {
        if scalar(&format!("SELECT {bound}(length(CAST({column} AS BLOB))) FROM {primary}"))?
            != 1024
        {
            return Err("primary payload byte count mismatch".into());
        }
    }
    let sqlite: String = c.query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
    Ok(
        json!({"sqlite":sqlite,"families":inv,"canonical_nodes":nodes,"canonical_edges":edges,"target_owners":target_owners,"survivor_owners":surviving_owners}),
    )
}
fn emit(file: &Mutex<File>, event: Value) {
    writeln!(file.lock().unwrap(), "{event}").unwrap();
}
fn offers(
    engine: Arc<Engine>,
    family: String,
    events: Arc<Mutex<File>>,
    start: Instant,
    stream: &'static str,
    client: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let stride = if stream == "read" { 8 } else { 1 };
        let mut last_end = start;
        for id in (client..1500).step_by(stride) {
            let due = start + Duration::from_millis(id as u64 * 10);
            if last_end > due {
                emit(
                    &events,
                    json!({"kind":"offer","stream":stream,"id":id,"state":"refused","due_ns":id as u64*10_000_000}),
                );
                continue;
            }
            thread::sleep(due.saturating_duration_since(Instant::now()));
            emit(
                &events,
                json!({"kind":"offer","stream":stream,"id":id,"state":"started","due_ns":id as u64*10_000_000}),
            );
            let outcome = if stream == "write" {
                engine
                    .write(&[node(&format!("foreground-{id}"), "foreground", "unrelated service")])
                    .map(|_| ())
            } else if family == "property" {
                engine
                    .search_projected_text("survivorneedle", "p1", None, &ReadView::default())
                    .map(|_| ())
            } else {
                engine
                    .search_text_only(if family == "edge" { "stable" } else { "survivorneedle" })
                    .map(|_| ())
            };
            last_end = Instant::now();
            emit(
                &events,
                json!({"kind":"finish","stream":stream,"id":id,"outcome":if outcome.is_ok(){"success"}else{"error"},"latency_ns":last_end.duration_since(due).as_nanos() as u64,"error":outcome.err().map(|e|e.to_string())}),
            );
        }
    })
}
fn replica(args: &[String]) -> Result<Value> {
    let started = Instant::now();
    let master = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    let family = &args[2];
    let arm = &args[3];
    let workload = &args[4];
    let loaded = &args[5] == "loaded";
    fs::create_dir_all(out)?;
    let path = out.join("replica.sqlite");
    fs::copy(master, &path)?;
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)?;
    let engine = Arc::new(open(&path)?);
    let inventory_before = validate(&path, family, 10_000, workload == "legacy")?;
    if workload == "legacy" && physical_markers(&path, MARKER)? == 0 {
        return Err("legacy fixture lacks physical ghost".into());
    }
    if loaded && started.elapsed() > Duration::from_secs(15) {
        return Err("setup-incomplete: insufficient full offer window".into());
    }
    let events = Arc::new(Mutex::new(File::create(out.join("events.jsonl"))?));
    let offer_start = Instant::now() + Duration::from_millis(50);
    let mut workers = Vec::new();
    if loaded {
        for client in 0..8 {
            workers.push(offers(
                Arc::clone(&engine),
                family.clone(),
                Arc::clone(&events),
                offer_start,
                "read",
                client,
            ));
        }
        workers.push(offers(
            Arc::clone(&engine),
            family.clone(),
            Arc::clone(&events),
            offer_start,
            "write",
            0,
        ));
        thread::sleep(offer_start.saturating_duration_since(Instant::now()));
    }
    let mut native_ns = None;
    let mut sanitation_accepted_ns = None;
    let mut finalization_ns = None;
    let mut holds = Vec::new();
    let mut waits = Vec::new();
    let mut progress = Vec::new();
    let cleanup = (|| -> Result<()> {
        if arm == "control" {
            return Ok(());
        }
        if workload == "fresh" {
            emit(
                &events,
                json!({"kind":"phase","phase":"native_start","elapsed_ns":started.elapsed().as_nanos() as u64}),
            );
            let native = Instant::now();
            let outcome = engine.erase_source("target");
            native_ns = Some(native.elapsed().as_nanos() as u64);
            emit(
                &events,
                json!({"kind":"phase","phase":"native_end","elapsed_ns":started.elapsed().as_nanos() as u64,"native_call_ns":native_ns,"error":outcome.as_ref().err().map(|e|e.to_string())}),
            );
            outcome?;
        }
        if arm == "A" && workload == "legacy" {
            sanitation_accepted_ns = Some(started.elapsed().as_nanos() as u64);
            let (_, wait, held) = engine.maintenance_transaction_for_test(|tx| {
                for table in TABLES {
                    tx.execute(&format!("INSERT INTO {table}({table}) VALUES('rebuild')"), [])?;
                }
                Ok(())
            })?;
            emit(
                &events,
                json!({"kind":"phase","phase":"rebuild_commit","elapsed_ns":started.elapsed().as_nanos() as u64,"writer_wait_ns":wait.as_nanos() as u64,"writer_held_ns":held.as_nanos() as u64}),
            );
            holds.push(held.as_nanos() as u64);
            waits.push(wait.as_nanos() as u64);
        } else if arm == "B" {
            let now = wall();
            admit(&engine, now, now + 30)?;
            emit(
                &events,
                json!({"kind":"phase","phase":"sanitation_admitted","elapsed_ns":started.elapsed().as_nanos() as u64,"accepted_wall":now,"deadline_wall":now+30}),
            );
            sanitation_accepted_ns = Some(started.elapsed().as_nanos() as u64);
            for _ in 0..100_000 {
                if job(&engine)?.phase != "cleaning" {
                    break;
                }
                if started.elapsed() >= Duration::from_secs(30) {
                    return Err("replica budget exhausted during merging".into());
                }
                let (wait, held) = step(&engine, 64, wall(), false)?;
                emit(
                    &events,
                    json!({"kind":"phase","phase":"merge_commit","elapsed_ns":started.elapsed().as_nanos() as u64,"writer_wait_ns":wait,"writer_held_ns":held}),
                );
                waits.push(wait);
                holds.push(held);
                progress.push(started.elapsed().as_nanos() as u64);
                thread::sleep(Duration::from_millis(1));
            }
            if job(&engine)?.phase != "proof_pending" {
                return Err("merge step ceiling exhausted".into());
            }
        }
        Ok(())
    })();
    for worker in workers {
        worker.join().map_err(|_| "foreground worker panicked")?;
    }
    if loaded {
        thread::sleep(
            (offer_start + Duration::from_secs(15)).saturating_duration_since(Instant::now()),
        );
    }
    let mut physical = None;
    let mut after = None;
    let cleanup = cleanup.and_then(|()| -> Result<()> {
        if arm == "control" {
            return Ok(());
        }
        emit(&events,json!({"kind":"phase","phase":"drain_start","elapsed_ns":started.elapsed().as_nanos() as u64}));
        let remaining=Duration::from_secs(30).saturating_sub(started.elapsed()).as_millis() as u64;
        if remaining==0 {return Err("replica exhausted before final drain".into());}
        engine.drain(remaining)?;
        emit(&events,json!({"kind":"phase","phase":"finalization_start","elapsed_ns":started.elapsed().as_nanos() as u64}));
        let begun = Instant::now();
        let complete = if arm == "B" {
            finalize(&engine, &path, MARKER)?
        } else {
            checkpoint(&engine)? && physical_markers(&path, MARKER)? == 0
        };
        finalization_ns = Some(begun.elapsed().as_nanos() as u64);
        emit(&events,json!({"kind":"phase","phase":"physical_proof_end","elapsed_ns":started.elapsed().as_nanos() as u64,"complete":complete,"finalization_ns":finalization_ns}));
        physical = Some(physical_markers(&path, MARKER)?);
        after = Some(inventory(&path, MARKER)?);
        if !complete {
            return Err("physical cleanup incomplete".into());
        }
        let survivors = inventory(&path, "survivorneedle")?;
        let expected = match family.as_str() {
            "node" => [45000, 0, 0, 45000],
            "edge" => [0, 45000, 0, 0],
            _ => [45000, 0, 18000, 45000],
        };
        if survivors.iter().map(|r| r.matches).collect::<Vec<_>>() != expected {
            return Err("survivor population changed".into());
        }
        Ok(())
    });
    let complete_at = started.elapsed().as_nanos() as u64;
    let job_state =
        if arm == "B" && sanitation_accepted_ns.is_some() { Some(job(&engine)?) } else { None };
    engine.close()?;
    let elapsed = started.elapsed().as_secs_f64();
    Ok(
        json!({"status":if cleanup.is_ok(){"complete"}else{"failed"},"error":cleanup.err().map(|e|e.to_string()),"arm":arm,"family":family,"workload":workload,"loaded":loaded,"inventory_before":inventory_before,"inventory_after":after,"native_call_ns":native_ns,"sanitation_accepted_ns":sanitation_accepted_ns,"progress_ns":progress,"writer_wait_ns":waits,"writer_held_ns":holds,"writer_max_ns":holds.iter().max(),"finalization_ns":finalization_ns,"physical_markers":physical,"sanitation_complete_ns":complete_at,"elapsed_s":elapsed,"job":job_state,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"offer_start_ns":offer_start.duration_since(started).as_nanos() as u64}),
    )
}
fn run() -> Result<Value> {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("fixture")=>{let path=PathBuf::from(&args[1]);build_campaign(&path,&args[2],args[3]=="secure",10_000)?;validate(&path,&args[2],10_000,false)},
        Some("legacy")=>{let path=PathBuf::from(&args[2]);fs::copy(&args[1],&path)?;let engine=open(&path)?;let started=Instant::now();engine.erase_source("target")?;let native_ns=started.elapsed().as_nanos() as u64;engine.close()?;if physical_markers(&path,MARKER)?==0{return Err("legacy ghost missing".into());}Ok(json!({"inventory":validate(&path,&args[3],10_000,true)?,"native_call_ns":native_ns}))},
        Some("replica")=>replica(&args[1..]),
        _=>Err("usage: probe fixture PATH FAMILY secure|default; legacy MASTER PATH FAMILY; replica MASTER OUT FAMILY ARM fresh|legacy idle|loaded".into()),
    }
}
fn main() {
    match run() {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!("{}", json!({"status":"error","error":error.to_string()}));
            std::process::exit(1);
        }
    }
}
