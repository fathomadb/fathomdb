//! Private native phase-budget control using the real None-provider database route.

use fathomdb_engine::{EmbedderChoice, Engine};
use serde_json::{json, Value};
use std::path::Path;

#[path = "budget_clock.rs"]
mod clock;

pub(super) fn monotonic_ns() -> Result<u64, String> {
    clock::monotonic_ns()
}

pub(super) fn bounds(config: &Value) -> Result<Option<(u64, u64)>, String> {
    clock::bounds(config)
}

pub(super) fn phase(config: &Value, out: &Path, phase: &str) -> Result<(), ()> {
    clock::phase(config, out, phase)
}

pub(super) fn query_open(config: &Value, out: &Path) -> Result<(), ()> {
    phase(config, out, "query_open")
}

pub(super) fn admit(config: &Value, out: &Path) -> bool {
    phase(config, out, "entry").is_ok()
}

pub(super) fn run_control(config: &Value, out: &Path) {
    if !admit(config, out) {
        return;
    }
    std::fs::create_dir_all(out).unwrap();
    let path = out.join("store.sqlite");
    let limit = ["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"]
        .into_iter()
        .map(|key| config[key].as_u64().unwrap())
        .min()
        .unwrap();
    let report = match monotonic_ns() {
        Ok(now) if now < limit => {
            let engine = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
            engine.close().unwrap();
            json!({"open_calls":1,"sqlite_created":path.exists(),"disposition":"executed","admitted_at_ns":now})
        }
        Ok(now) => {
            json!({"open_calls":0,"sqlite_created":false,"disposition":"deadline_refused","observed_at_ns":now})
        }
        Err(error) => {
            json!({"open_calls":0,"sqlite_created":false,"disposition":"clock_unavailable","error":error})
        }
    };
    std::fs::write(out.join("budget.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

pub(super) fn drain(engine: &Engine, config: &Value) -> Result<Value, String> {
    let bound = bounds(config)?;
    let started = bound.map(|(now, _)| now);
    let timeout_ms = if let Some((now, limit)) = bound {
        let remaining_ms = limit.saturating_sub(now) / 1_000_000;
        if remaining_ms == 0 {
            return Ok(
                json!({"drain_calls":0,"timeout_ms":0,"start_ns":now,"disposition":"deadline_refused"}),
            );
        }
        remaining_ms
    } else {
        5000
    };
    engine.drain(timeout_ms).map_err(|error| format!("{error:?}"))?;
    Ok(json!({"drain_calls":1,"timeout_ms":timeout_ms,"start_ns":started}))
}
