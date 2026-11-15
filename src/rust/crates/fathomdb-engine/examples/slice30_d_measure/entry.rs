use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;

pub(super) fn run_cell(
    identity: &Value,
    _output: &Path,
    _origin: Instant,
    _deadline: Instant,
) -> Result<Value, String> {
    Ok(json!({"cell":identity,"complete":false,"state":"not_run",
        "reason":"D native entry routing is not implemented","calls":[]}))
}
