use erasure_availability::{inventory, node, open, physical_markers, Result, MARKER};
use fathomdb_engine::{Engine, EngineError, ReadView};
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

fn event(file: &Mutex<File>, value: Value) {
    writeln!(file.lock().unwrap(), "{value}").unwrap();
}

fn offer_schedule(stream: &str, write_hz: usize) -> (usize, u64) {
    let hz = if stream == "read" { 100 } else { write_hz };
    (15 * hz, 1_000_000_000 / hz as u64)
}

fn offers(
    engine: Arc<Engine>,
    family: String,
    events: Arc<Mutex<File>>,
    start: Instant,
    stream: &'static str,
    client: usize,
    write_hz: usize,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let stride = if stream == "read" { 8 } else { 1 };
        let mut last_end = start;
        let (count, interval_ns) = offer_schedule(stream, write_hz);
        for id in (client..count).step_by(stride) {
            let due_ns = id as u64 * interval_ns;
            let due = start + Duration::from_nanos(due_ns);
            if last_end > due {
                event(
                    &events,
                    json!({"kind":"offer","stream":stream,"id":id,"state":"generator_missed","due_ns":due_ns}),
                );
                continue;
            }
            thread::sleep(due.saturating_duration_since(Instant::now()));
            let issued = Instant::now();
            event(
                &events,
                json!({"kind":"offer","stream":stream,"id":id,"state":"issued","due_ns":due_ns,"issued_ns":issued.duration_since(start).as_nanos() as u64}),
            );
            let call_start = Instant::now();
            let outcome = if stream == "write" {
                engine
                    .write(&[node(
                        &format!("foreground-{}", id % 8),
                        "foreground",
                        "unrelated service",
                    )])
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
            let state = match &outcome {
                Ok(()) => "completed",
                Err(EngineError::ErasureIncomplete { .. }) => "engine_refused",
                Err(_) => "failed",
            };
            event(
                &events,
                json!({"kind":"finish","stream":stream,"id":id,"state":state,"finished_ns":last_end.duration_since(start).as_nanos() as u64,"call_start_ns":call_start.duration_since(start).as_nanos() as u64,"service_ns":last_end.duration_since(call_start).as_nanos() as u64,"offer_latency_ns":last_end.duration_since(due).as_nanos() as u64,"error":outcome.err().map(|error|error.to_string())}),
            );
        }
    })
}

fn traffic_mode(mode: &str) -> Result<(bool, bool)> {
    Ok((mode == "control", true))
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(4..=5).contains(&args.len()) {
        return Err("usage: phase2 MASTER OUTPUT FAMILY erase|control|idle [WRITE_HZ]".into());
    }
    let write_hz: usize = args.get(4).map(|rate| rate.parse()).transpose()?.unwrap_or(100);
    if !(1..=100).contains(&write_hz) {
        return Err("WRITE_HZ must be between 1 and 100".into());
    }
    let master = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    let family = &args[2];
    let (control, traffic) = traffic_mode(&args[3])?;
    if out.exists() {
        return Err("output must not exist".into());
    }
    fs::create_dir_all(out)?;
    let path = out.join("replica.sqlite");
    fs::copy(master, &path)?;
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)?;
    let engine = Arc::new(open(&path)?);
    #[cfg(feature = "phase2-metrics")]
    engine.enable_physical_cleanup_metrics_for_test();
    let before = inventory(&path, MARKER)?;
    if before.iter().all(|item| item.matches == 0 || item.shadow_markers == 0) {
        return Err("positive target witness missing".into());
    }
    let events = Arc::new(Mutex::new(File::create(out.join("events.jsonl"))?));
    let start = Instant::now() + Duration::from_millis(50);
    let mut workers = (0..if traffic { 8 } else { 0 })
        .map(|client| {
            offers(
                Arc::clone(&engine),
                family.clone(),
                Arc::clone(&events),
                start,
                "read",
                client,
                write_hz,
            )
        })
        .collect::<Vec<_>>();
    if traffic {
        workers.push(offers(
            Arc::clone(&engine),
            family.clone(),
            Arc::clone(&events),
            start,
            "write",
            0,
            write_hz,
        ));
    }
    thread::sleep(start.saturating_duration_since(Instant::now()));
    let begun = Instant::now();
    let native = if control { Ok(None) } else { engine.erase_source("target").map(Some) };
    let native_ns = begun.elapsed().as_nanos() as u64;
    let native_error = native.as_ref().err().map(ToString::to_string);
    event(
        &events,
        json!({"kind":"native_result","elapsed_ns":start.elapsed().as_nanos() as u64,"native_ns":native_ns,"error":native_error}),
    );
    for worker in workers {
        worker.join().map_err(|_| "foreground worker panicked")?;
    }
    thread::sleep((start + Duration::from_secs(15)).saturating_duration_since(Instant::now()));
    let owed_before_retry = engine.query_i64_col_for_test("SELECT EXISTS(SELECT 1 FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure') OR EXISTS(SELECT 1 FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased'))")?[0] != 0;
    let retry_start = Instant::now();
    let retry = if !control && owed_before_retry {
        engine.erase_source("target").map(|_| ())
    } else {
        Ok(())
    };
    let retry_ns = retry_start.elapsed().as_nanos() as u64;
    let owed_after = engine.query_i64_col_for_test("SELECT EXISTS(SELECT 1 FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure') OR EXISTS(SELECT 1 FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased'))")?[0] != 0;
    let final_markers = physical_markers(&path, MARKER)?;
    let after = inventory(&path, MARKER)?;
    let survivors = inventory(&path, "survivorneedle")?;
    let expected = match family.as_str() {
        "node" => [45000, 0, 0, 45000],
        "edge" => [0, 45000, 0, 0],
        "property" => [45000, 0, 18000, 45000],
        _ => return Err("unknown family".into()),
    };
    let survivors_ok = survivors.iter().map(|row| row.matches).collect::<Vec<_>>() == expected;
    #[cfg(feature = "phase2-metrics")]
    let metrics = engine.physical_cleanup_metrics_for_test();
    #[cfg(not(feature = "phase2-metrics"))]
    let metrics = Value::Null;
    let verified = if control {
        survivors_ok && final_markers > 0 && after.iter().any(|row| row.matches > 0)
    } else {
        !owed_after
            && retry.is_ok()
            && final_markers == 0
            && after.iter().all(|row| row.matches == 0)
            && survivors_ok
            && matches!(&native, Ok(_) | Err(EngineError::ErasureIncomplete { .. }))
    };
    let result = json!({"family":family,"write_hz":write_hz,"control":control,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"native_ns":native_ns,"native_error":native_error,"native_report":native.ok().flatten().map(|report|json!({"nodes":report.nodes_excised,"edges":report.edges_excised,"projections":report.projections_invalidated})),"owed_before_retry":owed_before_retry,"retry_ns":retry_ns,"retry_error":retry.err().map(|error|error.to_string()),"owed_after":owed_after,"physical_markers":final_markers,"survivors_ok":survivors_ok,"verified":verified,"elapsed_ns":start.elapsed().as_nanos() as u64,"metrics":metrics});
    engine.close()?;
    fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result)?)?;
    println!("{result}");
    if !verified {
        return Err("physical completion or survivor qualification failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_mode_has_no_offer_schedule_and_unknown_modes_refuse() {
        assert_eq!(traffic_mode("idle").unwrap(), (false, false));
        assert_eq!(traffic_mode("erase").unwrap(), (false, true));
        assert_eq!(traffic_mode("control").unwrap(), (true, true));
        assert!(traffic_mode("typo").is_err());
    }

    #[test]
    fn supplemental_write_rate_preserves_the_fifteen_second_read_schedule() {
        assert_eq!(offer_schedule("read", 20), (1500, 10_000_000));
        assert_eq!(offer_schedule("write", 100), (1500, 10_000_000));
        assert_eq!(offer_schedule("write", 20), (300, 50_000_000));
    }
}
