//! The `admin` namespace: op-store schema configuration and the
//! process-level SQLite runtime mode.

use fathomdb_engine::{PreparedWrite, RuntimeConfiguration, RuntimeSqliteMode, WriteReceipt};

use crate::error::Result;
use crate::Engine;

/// Register a `latest_state` op-store collection `name` with JSON schema `body`.
pub fn configure(engine: &Engine, name: &str, body: &str) -> Result<WriteReceipt> {
    engine.write(&[PreparedWrite::AdminSchema {
        name: name.to_string(),
        kind: "latest_state".to_string(),
        schema_json: body.to_string(),
        retention_json: "{}".to_string(),
    }])
}

/// Select the process-wide SQLite runtime mode. Must precede the first open;
/// repeating the effective mode is a no-op.
///
/// # Errors
/// `RuntimeConfiguration` when called too late or with a conflicting mode.
pub fn configure_runtime(sqlite_mode: RuntimeSqliteMode) -> Result<RuntimeConfiguration> {
    Ok(fathomdb_engine::configure_runtime(sqlite_mode)?)
}
