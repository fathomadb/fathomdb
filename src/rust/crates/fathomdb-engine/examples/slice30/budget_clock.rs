//! Shared private absolute phase clock; no engine timeout or public API changes.

use serde_json::{json, Value};
use std::path::Path;

#[cfg(target_os = "linux")]
pub(crate) fn monotonic_ns() -> Result<u64, String> {
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
pub(crate) fn monotonic_ns() -> Result<u64, String> {
    Err("native CLOCK_MONOTONIC budgets require Linux".into())
}

pub(crate) fn bounds(config: &Value) -> Result<Option<(u64, u64)>, String> {
    if !config["work_deadline_ns"].is_u64() {
        return Ok(None);
    }
    let limit = ["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"]
        .into_iter()
        .map(|key| config[key].as_u64().unwrap())
        .min()
        .unwrap();
    Ok(Some((monotonic_ns()?, limit)))
}

pub(crate) fn phase(config: &Value, out: &Path, phase: &str) -> Result<(), ()> {
    let mut report = match bounds(config) {
        Ok(None) => return Ok(()),
        Ok(Some((now, limit))) if now < limit => return Ok(()),
        Ok(Some((now, _))) => {
            json!({"phase":phase,"disposition":"deadline_refused","observed_at_ns":now})
        }
        Err(error) => json!({"phase":phase,"disposition":"clock_unavailable","error":error}),
    };
    if phase == "entry" {
        report["open_calls"] = json!(0);
        report["sqlite_created"] = json!(false);
    }
    std::fs::create_dir_all(out).unwrap();
    std::fs::write(out.join("budget.json"), serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    Err(())
}
