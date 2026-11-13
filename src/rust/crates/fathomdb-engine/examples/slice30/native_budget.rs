//! Private native phase-budget control using the real None-provider database route.

use fathomdb_engine::{EmbedderChoice, Engine};
use serde_json::{json, Value};
use std::path::Path;

#[cfg(target_os = "linux")]
pub(super) fn monotonic_ns() -> Result<u64, String> {
    let mut value = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // The initialized timespec remains valid and exclusively borrowed for this call.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let seconds = u64::try_from(value.tv_sec).map_err(|error| error.to_string())?;
    let nanos = u64::try_from(value.tv_nsec).map_err(|error| error.to_string())?;
    if nanos >= 1_000_000_000 {
        return Err("invalid monotonic subsecond value".into());
    }
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|seconds| seconds.checked_add(nanos))
        .ok_or_else(|| "monotonic timestamp overflow".into())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn monotonic_ns() -> Result<u64, String> {
    Err("native CLOCK_MONOTONIC budgets require Linux".into())
}

pub(super) fn run_control(config: &Value, out: &Path) {
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
