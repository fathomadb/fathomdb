use super::*;

// ===== admin.configure ================================================

#[napi(js_name = "adminConfigureRuntime")]
pub fn admin_configure_runtime(options: RuntimeConfigureOptions) -> Result<RuntimeConfiguration> {
    let mode = match options.sqlite_mode.as_str() {
        "performance" => RustRuntimeSqliteMode::Performance,
        "diagnostics" => RustRuntimeSqliteMode::Diagnostics,
        _ => {
            return Err(Error::new(
                Status::InvalidArg,
                "sqliteMode must be 'performance' or 'diagnostics'",
            ));
        }
    };
    fathomdb_engine::configure_runtime(mode)
        .map(|configuration| RuntimeConfiguration {
            sqlite_mode: match configuration.sqlite_mode {
                RustRuntimeSqliteMode::Performance => "performance",
                RustRuntimeSqliteMode::Diagnostics => "diagnostics",
            }
            .to_string(),
        })
        .map_err(runtime_configuration_error_to_napi)
}

#[napi(js_name = "adminConfigure")]
pub async fn admin_configure(
    engine: &Engine,
    options: AdminConfigureOptions,
) -> Result<WriteReceipt> {
    validate_ffi_string_napi(&options.name)?;
    validate_ffi_string_napi(&options.body)?;
    if options.name.is_empty() {
        return Err(typed_error(
            CODE_WRITE_VALIDATION,
            "admin.configure requires a non-empty name",
            JsonValue::Null,
        ));
    }
    // why: `dev/interfaces/typescript.md` § Runtime surface pins the
    // admin.configure({ name, body }) signature; the engine's
    // `PreparedWrite::AdminSchema` requires `kind ∈ {latest_state,
    // append_only_log}`. The TS verb is sugar over latest-state
    // collection registration in 0.6.0.
    let batch = vec![PreparedWrite::AdminSchema {
        name: options.name,
        kind: "latest_state".to_string(),
        schema_json: options.body,
        retention_json: "{}".to_string(),
    }];
    let inner = Arc::clone(&engine.inner);
    let receipt = call_engine(move || inner.write(&batch)).await?;
    Ok(WriteReceipt::from_rust(receipt))
}

#[napi(object)]
pub struct CounterSnapshot {
    pub queries: i64,
    pub writes: i64,
    pub write_rows: i64,
    pub admin_ops: i64,
    pub cache_hit: i64,
    pub cache_miss: i64,
}

#[napi(object)]
pub struct EngineConfig {
    pub embedder_pool_size: Option<f64>,
    pub scheduler_runtime_threads: Option<f64>,
    pub provenance_row_cap: Option<f64>,
    pub embedder_call_timeout_ms: Option<f64>,
    pub slow_threshold_ms: Option<f64>,
}

pub(crate) fn checked_config_number(
    field: &str,
    value: Option<f64>,
    min: u64,
    max: u64,
) -> Result<Option<u64>> {
    value
        .map(|number| {
            if !number.is_finite()
                || number.fract() != 0.0
                || number < min as f64
                || number > max as f64
                || number > ((1_u64 << 53) - 1) as f64
            {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!("engine configuration {field} must be an integer in {min}..={max}"),
                    JsonValue::Null,
                ));
            }
            Ok(number as u64)
        })
        .transpose()
}

impl EngineConfig {
    pub(crate) fn into_rust(self) -> Result<RustEngineConfig> {
        const MAX_SAFE: u64 = (1_u64 << 53) - 1;
        Ok(RustEngineConfig {
            embedder_pool_size: checked_config_number(
                "embedder_pool_size",
                self.embedder_pool_size,
                1,
                64,
            )?,
            scheduler_runtime_threads: checked_config_number(
                "scheduler_runtime_threads",
                self.scheduler_runtime_threads,
                1,
                64,
            )?,
            provenance_row_cap: checked_config_number(
                "provenance_row_cap",
                self.provenance_row_cap,
                0,
                MAX_SAFE,
            )?,
            embedder_call_timeout_ms: checked_config_number(
                "embedder_call_timeout_ms",
                self.embedder_call_timeout_ms,
                1,
                u64::from(u32::MAX),
            )?,
            slow_threshold_ms: checked_config_number(
                "slow_threshold_ms",
                self.slow_threshold_ms,
                0,
                MAX_SAFE,
            )?,
        })
    }
}

#[napi(object)]
pub struct EngineOpenOptions {
    pub engine_config: Option<EngineConfig>,
    /// EU-6: opt-in to the engine's pinned default embedder
    /// (`fathomdb-bge-small-en-v1.5`). On first use, weights are
    /// downloaded from HuggingFace and cached under
    /// `~/.cache/fathomdb/embedders/`. `false` (the default) opens
    /// without an embedder; vector writes then fail with
    /// `EmbedderNotConfigured`.
    pub use_default_embedder: Option<bool>,
}

#[napi(object)]
pub struct AdminConfigureOptions {
    pub name: String,
    pub body: String,
}

#[napi(object)]
pub struct RuntimeConfigureOptions {
    pub sqlite_mode: String,
}

#[napi(object)]
pub struct RuntimeConfiguration {
    pub sqlite_mode: String,
}
