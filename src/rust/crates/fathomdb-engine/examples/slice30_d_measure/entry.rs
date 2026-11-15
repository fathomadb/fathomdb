use super::provider::VaryingEmbedder;
use super::schedule::{self, Mode, OrdinaryCell};
use super::{capture, workload};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(super) fn run_cell(
    identity: &Value,
    output: &Path,
    origin: Instant,
    deadline: Instant,
) -> Result<Value, String> {
    let n = usize::try_from(identity["n"].as_u64().ok_or("D cell n is unavailable")?)
        .map_err(|error| format!("D cell n: {error:?}"))?;
    let mode = match identity["mode"].as_str() {
        Some("held") => Mode::Held,
        Some("overlap") => Mode::Overlap,
        _ => return Err("D mode must be held or overlap".into()),
    };
    if deadline.checked_duration_since(Instant::now()).unwrap_or_default()
        <= Duration::from_secs(30)
    {
        return Err("D entry has no work allowance before its close reserve".into());
    }
    fs::create_dir_all(output).map_err(|error| format!("D output: {error:?}"))?;
    let store = output.join("store.sqlite");
    if store.exists() {
        return Err("D cell requires a fresh store".into());
    }
    let queries = workload::query_bodies();
    let cell = OrdinaryCell { path: &store, mode, queries: &queries, origin, deadline };
    let bodies = workload::corpus_bodies(n);
    let provider = Arc::new(VaryingEmbedder);
    let mut report = match identity["layer"].as_str() {
        Some("T") => schedule::run_ordinary(cell, bodies, provider)?,
        Some("C") => {
            capture::run_characterized(cell, bodies, provider, identity, &output.join("capture"))?
        }
        _ => return Err("D layer must be C or T".into()),
    };
    report["cell"] = identity.clone();
    report["entry_elapsed_ns"] = json!(schedule::elapsed(origin));
    if Instant::now() >= deadline {
        report["complete"] = json!(false);
        report["error"] = json!("D whole-cell allowance exhausted at native report boundary");
    }
    Ok(report)
}

pub(super) fn run_file(config: &Path, output: &Path, origin: Instant) -> Result<Value, String> {
    let config: Value = serde_json::from_slice(
        &fs::read(config).map_err(|error| format!("read D cell: {error:?}"))?,
    )
    .map_err(|error| format!("decode D cell: {error:?}"))?;
    let spec: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../dev/experiments/0.8.29-slice30-measurements/d-cells.json"
    )))
    .expect("committed fixed D cell table");
    let selected = spec["cells"]
        .as_array()
        .expect("fixed D cells")
        .iter()
        .find(|cell| {
            ["id", "n", "mode", "repeat", "layer", "cell_ns"]
                .iter()
                .all(|field| cell[*field] == config[*field])
        })
        .ok_or("D native CLI requires one unchanged fixed24 cell")?;
    let attempt = config["attempt_id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("D attempt identity is unavailable")?;
    let remaining_ms = config["remaining_cell_ms"]
        .as_u64()
        .filter(|remaining| *remaining > 0 && *remaining <= 900_000)
        .ok_or("D remaining whole-cell allowance must be within1..=900000ms")?;
    let identity = json!({"cell_id":selected["id"],"n":selected["n"],"mode":selected["mode"],
        "layer":selected["layer"],"repeat":selected["repeat"],"attempt_id":attempt});
    let deadline = origin + Duration::from_millis(remaining_ms);
    run_cell(&identity, output, origin, deadline)
}

#[cfg(test)]
/// Return inherited settings that change D default execution, when present.
pub(super) fn inherited_settings() -> Option<Value> {
    None
}
