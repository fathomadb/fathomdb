use super::*;

// ===== Error-code constants ===========================================
//
// One per typed leaf class on the TS side. The single-switch translator
// below maps every engine variant to exactly one of these codes; the TS
// `rethrowTyped` table maps each code to exactly one leaf class. Drift
// fails to compile (Rust side) or fails an exhaustiveness check (TS).

pub(crate) const CODE_STORAGE: &str = "FDB_STORAGE";
pub(crate) const CODE_RUNTIME_CONFIGURATION: &str = "FDB_RUNTIME_CONFIGURATION";
pub(crate) const CODE_PROJECTION: &str = "FDB_PROJECTION";
pub(crate) const CODE_PROJECTION_GENERATION: &str = "FDB_PROJECTION_GENERATION";
pub(crate) const CODE_VECTOR: &str = "FDB_VECTOR";
pub(crate) const CODE_EMBEDDER: &str = "FDB_EMBEDDER";
pub(crate) const CODE_EMBED_DEVICE_POLICY: &str = "FDB_EMBED_DEVICE_POLICY";
pub(crate) const CODE_RERANKER_DEVICE_POLICY: &str = "FDB_RERANKER_DEVICE_POLICY";
/// 0.8.28 pool study only (ruling 15): a private CUDA pool hit its cap.
#[cfg(feature = "tegra-pool-experiment")]
pub(crate) const CODE_CUDA_POOL_EXHAUSTED: &str = "FDB_CUDA_POOL_EXHAUSTED";

/// The pool-exhaustion envelope: payload `{kind, ordinal, maxSizeBytes}`.
#[cfg(feature = "tegra-pool-experiment")]
pub(crate) fn cuda_pool_exhausted_error(
    message: String,
    ordinal: usize,
    max_size_bytes: u64,
) -> napi::Error {
    typed_error(
        CODE_CUDA_POOL_EXHAUSTED,
        message,
        json!({ "kind": "cuda_pool_exhausted", "ordinal": ordinal, "maxSizeBytes": max_size_bytes }),
    )
}
pub(crate) const CODE_EMBEDDER_NOT_CONFIGURED: &str = "FDB_EMBEDDER_NOT_CONFIGURED";
pub(crate) const CODE_EMBEDDER_REQUIRED: &str = "FDB_EMBEDDER_REQUIRED";
pub(crate) const CODE_KIND_NOT_VECTOR_INDEXED: &str = "FDB_KIND_NOT_VECTOR_INDEXED";
pub(crate) const CODE_EMBEDDER_DIMENSION_MISMATCH: &str = "FDB_EMBEDDER_DIMENSION_MISMATCH";
pub(crate) const CODE_SCHEDULER: &str = "FDB_SCHEDULER";
pub(crate) const CODE_OP_STORE: &str = "FDB_OP_STORE";
pub(crate) const CODE_WRITE_VALIDATION: &str = "FDB_WRITE_VALIDATION";
pub(crate) const CODE_SCHEMA_VALIDATION: &str = "FDB_SCHEMA_VALIDATION";
pub(crate) const CODE_PROVENANCE: &str = "FDB_PROVENANCE";
pub(crate) const CODE_DEPENDENCY: &str = "FDB_DEPENDENCY";
pub(crate) const CODE_DEPENDENCY_CLOSURE: &str = "FDB_DEPENDENCY_CLOSURE";
pub(crate) const CODE_ACTUATION: &str = "FDB_ACTUATION";
pub(crate) const CODE_OVERLOADED: &str = "FDB_OVERLOADED";
pub(crate) const CODE_CLOSING: &str = "FDB_CLOSING";
pub(crate) const CODE_DATABASE_LOCKED: &str = "FDB_DATABASE_LOCKED";
pub(crate) const CODE_CORRUPTION: &str = "FDB_CORRUPTION";
pub(crate) const CODE_INCOMPATIBLE_SCHEMA_VERSION: &str = "FDB_INCOMPATIBLE_SCHEMA_VERSION";
pub(crate) const CODE_MIGRATION: &str = "FDB_MIGRATION";
pub(crate) const CODE_EMBEDDER_IDENTITY_MISMATCH: &str = "FDB_EMBEDDER_IDENTITY_MISMATCH";
// G11 (Slice 15) — BYO-LLM extraction harness protocol error.
pub(crate) const CODE_EXTRACTOR: &str = "FDB_EXTRACTOR";
// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation harness protocol error.
pub(crate) const CODE_CONSOLIDATOR: &str = "FDB_CONSOLIDATOR";
// G4 (Slice 35) — filter predicate construction error (non-allowlisted path).
pub(crate) const CODE_INVALID_FILTER: &str = "FDB_INVALID_FILTER";
// Slice 20 — depth > 3 or invalid argument (G5/G6).
pub(crate) const CODE_INVALID_ARGUMENT: &str = "FDB_INVALID_ARGUMENT";
// 0.8.18 Slice 5 (#5 vector-equivalence probe) — query-time dense-refusal code.
pub(crate) const CODE_VECTOR_EQUIVALENCE_MISMATCH: &str = "FDB_VECTOR_EQUIVALENCE_MISMATCH";
// OPP-12 Phase-1 (0.8.19 Slice 10) — lifecycle-verb typed errors.
pub(crate) const CODE_ILLEGAL_TRANSITION: &str = "FDB_ILLEGAL_TRANSITION";
pub(crate) const CODE_NOT_LIFECYCLE_ADDRESSABLE: &str = "FDB_NOT_LIFECYCLE_ADDRESSABLE";
// 0.8.20 Slice 5b (R-20-E5) — an erasure verb deleted its rows but could not
// complete the erasure at rest.
pub(crate) const CODE_ERASURE_INCOMPLETE: &str = "FDB_ERASURE_INCOMPLETE";
// 0.8.20 Slice 15d (R-20-PR) — configure_projections refused a destructive
// change without an explicit drop.
pub(crate) const CODE_PROJECTION_DESTRUCTIVE: &str = "FDB_PROJECTION_DESTRUCTIVE";
pub(crate) const CODE_FROZEN_READ: &str = "FDB_FROZEN_READ";
pub(crate) const CODE_EVIDENCE: &str = "FDB_EVIDENCE";
pub(crate) const CODE_PAGE: &str = "FDB_PAGE";
pub(crate) const CODE_DEPENDENCY_TRACE: &str = "FDB_DEPENDENCY_TRACE";
pub(crate) const CODE_GRAPH_EXPANSION: &str = "FDB_GRAPH_EXPANSION";
pub(crate) const CODE_PANIC: &str = "FDB_PANIC";

// ===== Typed-error encoder ============================================

/// Encode a typed-error envelope as JSON in `err.message` so the
/// TS-side `rethrowTyped` can reconstitute the right leaf class with
/// the right payload. napi-rs 2.x has no public API for throwing a
/// Rust-defined JS class directly; the envelope-in-message pattern is
/// the canonical workaround.
#[derive(Serialize)]
pub(crate) struct TypedEnvelope<'a> {
    code: &'a str,
    message: String,
    payload: JsonValue,
}

pub(crate) fn typed_error(
    code: &'static str,
    message: impl Into<String>,
    payload: JsonValue,
) -> Error {
    let envelope = TypedEnvelope { code, message: message.into(), payload };
    // serde_json serialization is infallible for our envelope shape;
    // unwrap is safe.
    let reason = serde_json::to_string(&envelope).unwrap();
    Error::new(Status::GenericFailure, reason)
}

// ===== String validation (AC-068a / AC-068b) =========================

/// Reject strings carrying an embedded NUL or an unpaired UTF-16
/// surrogate codepoint (`U+D800..=U+DFFF`).
///
/// JavaScript strings are UTF-16 by spec, so lone surrogates are
/// representable on the JS side (`String.fromCharCode(0xD800)`). The
/// napi-rs string conversion translates UTF-16 → UTF-8 and accepts
/// some malformed inputs; this helper rejects them BEFORE the writer
/// transaction opens (no-row-written invariant).
pub fn validate_ffi_string(value: &str) -> std::result::Result<(), String> {
    if value.as_bytes().contains(&0) {
        return Err("embedded NUL byte in FFI string".to_string());
    }
    for ch in value.chars() {
        let cp = ch as u32;
        if (0xD800..=0xDFFF).contains(&cp) {
            return Err(format!("unpaired UTF-16 surrogate U+{cp:04X} in FFI string"));
        }
    }
    Ok(())
}

pub(crate) fn validate_ffi_string_napi(value: &str) -> Result<()> {
    validate_ffi_string(value)
        .map_err(|msg| typed_error(CODE_WRITE_VALIDATION, msg, JsonValue::Null))
}

/// codex §9 [P2] — convert a napi `i64` id list to the engine's `u64`, rejecting
/// any negative id (which `as u64` would silently wrap). Mirrors the TS/Python
/// wrapper `validateIdArray`/`_validate_id_list` guards so a raw-napi caller
/// cannot smuggle a wrapped id past the boundary.
pub(crate) fn checked_ids_napi(name: &str, ids: &[i64]) -> Result<Vec<u64>> {
    ids.iter()
        .map(|&x| {
            u64::try_from(x).map_err(|_| {
                typed_error(
                    CODE_WRITE_VALIDATION,
                    format!("{name} must contain only non-negative integers, got {x}"),
                    JsonValue::Null,
                )
            })
        })
        .collect()
}

// ===== Error mapping ==================================================

/// Translate every `EngineError` variant to its typed JS counterpart.
///
/// No catch-all arm: drift between the Rust enum and the TS class set
/// is a compile error.
pub(crate) fn engine_error_to_napi(err: RustEngineError) -> Error {
    match err {
        RustEngineError::Storage => typed_error(CODE_STORAGE, "storage error", JsonValue::Null),
        RustEngineError::Projection => {
            typed_error(CODE_PROJECTION, "projection error", JsonValue::Null)
        }
        RustEngineError::ProjectionGeneration(error) => typed_error(
            CODE_PROJECTION_GENERATION,
            format!("projection generation {} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::Vector => typed_error(CODE_VECTOR, "vector error", JsonValue::Null),
        RustEngineError::Embedder => typed_error(CODE_EMBEDDER, "embedder error", JsonValue::Null),
        #[cfg(feature = "tegra-pool-experiment")]
        error @ RustEngineError::CudaPoolExhausted { ordinal, max_size_bytes } => {
            cuda_pool_exhausted_error(error.to_string(), ordinal, max_size_bytes)
        }
        RustEngineError::RerankerDevicePolicy(error) => reranker_device_policy_error_to_napi(error),
        RustEngineError::EmbedderNotConfigured => {
            typed_error(CODE_EMBEDDER_NOT_CONFIGURED, "embedder is not configured", JsonValue::Null)
        }
        RustEngineError::EmbedderRequired(required) => typed_error(
            CODE_EMBEDDER_REQUIRED,
            "embedder is required for pending projection work",
            json!({
                "operation": required.operation.as_str(),
                "state": required.state.as_str(),
                "remediations": required.remediations,
                "documentationUrl": required.documentation_url,
            }),
        ),
        RustEngineError::KindNotVectorIndexed => typed_error(
            CODE_KIND_NOT_VECTOR_INDEXED,
            "kind is not configured for vector indexing",
            JsonValue::Null,
        ),
        RustEngineError::EmbedderDimensionMismatch { expected, actual } => typed_error(
            CODE_EMBEDDER_DIMENSION_MISMATCH,
            format!("embedder vector dimension mismatch: stored {expected}, supplied {actual}"),
            json!({ "stored": expected, "supplied": actual }),
        ),
        RustEngineError::Scheduler => {
            typed_error(CODE_SCHEDULER, "scheduler error", JsonValue::Null)
        }
        RustEngineError::OpStore => typed_error(CODE_OP_STORE, "op-store error", JsonValue::Null),
        RustEngineError::WriteValidation => {
            typed_error(CODE_WRITE_VALIDATION, "write validation error", JsonValue::Null)
        }
        RustEngineError::SchemaValidation => {
            typed_error(CODE_SCHEMA_VALIDATION, "schema validation error", JsonValue::Null)
        }
        RustEngineError::Provenance(error) => typed_error(
            CODE_PROVENANCE,
            format!("provenance {} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::Dependency(error) => typed_error(
            CODE_DEPENDENCY,
            format!("dependency {} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::DependencyClosure(error) => typed_error(
            CODE_DEPENDENCY_CLOSURE,
            format!("dependency closure {} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::Actuation(error) => typed_error(
            CODE_ACTUATION,
            format!("actuation {} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::FrozenRead(error) => typed_error(
            CODE_FROZEN_READ,
            format!("{} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::Evidence(error) => typed_error(
            CODE_EVIDENCE,
            format!("{} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::Page(error) => typed_error(
            CODE_PAGE,
            format!("{} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::DependencyTrace(error) => typed_error(
            CODE_DEPENDENCY_TRACE,
            format!("{} at {}", error.reason.as_str(), error.field_path),
            json!({
                "reason": error.reason.as_str(),
                "fieldPath": error.field_path,
            }),
        ),
        RustEngineError::GraphExpansion(error) => graph_expansion_error_to_napi(&error),
        RustEngineError::Overloaded => {
            typed_error(CODE_OVERLOADED, "engine overloaded", JsonValue::Null)
        }
        RustEngineError::Closing => typed_error(CODE_CLOSING, "engine is closing", JsonValue::Null),
        RustEngineError::Extractor => {
            typed_error(CODE_EXTRACTOR, "extractor error", JsonValue::Null)
        }
        RustEngineError::Consolidator => {
            typed_error(CODE_CONSOLIDATOR, "consolidator error", JsonValue::Null)
        }
        RustEngineError::InvalidFilter { reason } => {
            typed_error(CODE_INVALID_FILTER, format!("invalid filter: {reason}"), JsonValue::Null)
        }
        RustEngineError::InvalidArgument { msg } => {
            typed_error(CODE_INVALID_ARGUMENT, msg, JsonValue::Null)
        }
        RustEngineError::IllegalTransition { from_state, to_state, legal } => {
            let legal_str: Vec<&'static str> = legal.iter().map(|s| s.as_str()).collect();
            typed_error(
                CODE_ILLEGAL_TRANSITION,
                format!(
                    "illegal lifecycle transition {} -> {}; legal targets: {:?}",
                    from_state.as_str(),
                    to_state.as_str(),
                    legal_str,
                ),
                // Parity-safe field names (S7): `fromState`/`toState`, never `from`.
                json!({
                    "fromState": from_state.as_str(),
                    "toState": to_state.as_str(),
                    "legal": legal_str,
                }),
            )
        }
        RustEngineError::NotLifecycleAddressable { id_space } => typed_error(
            CODE_NOT_LIFECYCLE_ADDRESSABLE,
            format!(
                "id space {:?} is not lifecycle-addressable; only the logical (l:) space is",
                id_space.as_str(),
            ),
            json!({ "idSpace": id_space.as_str() }),
        ),
        RustEngineError::VectorEquivalenceMismatch { reason } => typed_error(
            CODE_VECTOR_EQUIVALENCE_MISMATCH,
            format!("vector-equivalence self-check failed; dense retrieval refused: {reason}"),
            json!({ "reason": reason }),
        ),
        RustEngineError::ErasureIncomplete { stage, detail } => typed_error(
            CODE_ERASURE_INCOMPLETE,
            format!("erasure incomplete at stage '{stage}': {detail}"),
            json!({ "stage": stage, "detail": detail }),
        ),
        RustEngineError::ProjectionDestructive { name, delta } => typed_error(
            CODE_PROJECTION_DESTRUCTIVE,
            format!(
                "configure_projections refused a destructive change to '{name}': {delta}; \
                 re-issue with drop: [\"{name}\"]"
            ),
            json!({ "name": name, "delta": delta }),
        ),
        // Cargo unifies dependency features across a workspace build. The CLI
        // can therefore add the operator-only engine variant while this SDK
        // still has no operator surface capable of producing it.
        #[allow(unreachable_patterns)]
        operator_only => typed_error(CODE_STORAGE, operator_only.to_string(), JsonValue::Null),
    }
}

pub(crate) fn graph_expansion_error_to_napi(
    error: &fathomdb_engine::GraphExpansionErrorV1,
) -> Error {
    typed_error(
        CODE_GRAPH_EXPANSION,
        error.to_string(),
        json!({
            "reason": error.reason.as_str(),
            "fieldPath": error.field_path,
        }),
    )
}

pub(crate) fn corruption_kind_str(kind: CorruptionKind) -> &'static str {
    match kind {
        CorruptionKind::WalReplayFailure => "WalReplayFailure",
        CorruptionKind::HeaderMalformed => "HeaderMalformed",
        CorruptionKind::SchemaInconsistent => "SchemaInconsistent",
        CorruptionKind::EmbedderIdentityDrift => "EmbedderIdentityDrift",
        CorruptionKind::ProjectionGenerationDrift => "ProjectionGenerationDrift",
    }
}

pub(crate) fn open_stage_str(stage: OpenStage) -> &'static str {
    match stage {
        OpenStage::HeaderProbe => "HeaderProbe",
        OpenStage::WalReplay => "WalReplay",
        OpenStage::SchemaProbe => "SchemaProbe",
        OpenStage::EmbedderIdentity => "EmbedderIdentity",
        OpenStage::ProjectionGeneration => "ProjectionGeneration",
    }
}

pub(crate) fn corruption_to_napi(detail: CorruptionDetail) -> Error {
    let kind = corruption_kind_str(detail.kind);
    let stage = open_stage_str(detail.stage);
    let recovery_hint_code = detail.recovery_hint.code;
    let doc_anchor = detail.recovery_hint.doc_anchor;
    typed_error(
        CODE_CORRUPTION,
        format!("corruption {kind} at stage {stage} ({recovery_hint_code})"),
        json!({
            "kind": kind,
            "stage": stage,
            "recoveryHintCode": recovery_hint_code,
            "docAnchor": doc_anchor,
        }),
    )
}

pub(crate) fn embed_device_policy_error_to_napi(
    error: fathomdb_embedder::EmbedDevicePolicyError,
) -> Error {
    let mut payload = serde_json::Map::new();
    payload.insert("kind".to_string(), json!(error.kind()));
    if let Some(ordinal) = error.ordinal() {
        payload.insert("ordinal".to_string(), json!(ordinal));
    }
    let message = crate::cuda_early_init::device_policy_refusal_message(
        crate::cuda_early_init::RefusingComponent::Embedder,
        error.kind(),
        error.to_string(),
    );
    typed_error(CODE_EMBED_DEVICE_POLICY, message, JsonValue::Object(payload))
}

pub(crate) fn reranker_device_policy_error_to_napi(
    error: fathomdb_embedder::RerankerDevicePolicyError,
) -> Error {
    // 0.8.28 pool study only (ruling 15): a cross-encoder forward that
    // exhausted the private pool uses the pool-exhaustion envelope.
    #[cfg(feature = "tegra-pool-experiment")]
    if let fathomdb_embedder::RerankerDevicePolicyError::CudaPoolExhausted {
        ordinal,
        max_size_bytes,
    } = error
    {
        return cuda_pool_exhausted_error(error.to_string(), ordinal, max_size_bytes);
    }
    let mut payload = serde_json::Map::new();
    payload.insert("kind".to_string(), json!(error.kind()));
    if let Some(ordinal) = error.ordinal() {
        payload.insert("ordinal".to_string(), json!(ordinal));
    }
    let message = crate::cuda_early_init::device_policy_refusal_message(
        crate::cuda_early_init::RefusingComponent::Reranker,
        error.kind(),
        error.to_string(),
    );
    typed_error(CODE_RERANKER_DEVICE_POLICY, message, JsonValue::Object(payload))
}

pub(crate) fn engine_open_error_to_napi(err: EngineOpenError) -> Error {
    match err {
        EngineOpenError::RuntimeConfiguration(error) => runtime_configuration_error_to_napi(error),
        EngineOpenError::EngineConfiguration(error) => {
            typed_error(CODE_INVALID_ARGUMENT, error.to_string(), JsonValue::Null)
        }
        EngineOpenError::DatabaseLocked { holder_pid } => typed_error(
            CODE_DATABASE_LOCKED,
            match holder_pid {
                Some(pid) => format!("database is locked by process {pid}"),
                None => "database is locked by another engine instance".to_string(),
            },
            json!({ "holderPid": holder_pid }),
        ),
        EngineOpenError::Corruption(detail) => corruption_to_napi(detail),
        EngineOpenError::IncompatibleSchemaVersion { seen, supported } => typed_error(
            CODE_INCOMPATIBLE_SCHEMA_VERSION,
            format!(
                "database schema version {seen} is incompatible with supported version {supported}"
            ),
            json!({ "seen": seen, "supported": supported }),
        ),
        EngineOpenError::MigrationError {
            schema_version_before,
            schema_version_current,
            step_id,
        } => typed_error(
            CODE_MIGRATION,
            format!(
                "schema migration failed at step {step_id}; schema version remained between {schema_version_before} and {schema_version_current}"
            ),
            json!({
                "schemaVersionBefore": schema_version_before,
                "schemaVersionCurrent": schema_version_current,
                "stepId": step_id,
            }),
        ),
        EngineOpenError::EmbedderIdentityMismatch { stored, supplied } => typed_error(
            CODE_EMBEDDER_IDENTITY_MISMATCH,
            format!(
                "embedder identity mismatch: stored {}@{}, supplied {}@{}",
                stored.name, stored.revision, supplied.name, supplied.revision,
            ),
            json!({
                "storedName": stored.name,
                "storedRevision": stored.revision,
                "suppliedName": supplied.name,
                "suppliedRevision": supplied.revision,
            }),
        ),
        EngineOpenError::EmbedderDimensionMismatch { stored, supplied } => typed_error(
            CODE_EMBEDDER_DIMENSION_MISMATCH,
            format!(
                "embedder vector dimension mismatch: stored {stored}, supplied {supplied}"
            ),
            json!({ "stored": stored, "supplied": supplied }),
        ),
        EngineOpenError::Embedder(err) => typed_error(
            CODE_EMBEDDER,
            format!("embedder error during open: {err:?}"),
            JsonValue::Null,
        ),
        EngineOpenError::EmbedDevicePolicy(error) => embed_device_policy_error_to_napi(error),
        EngineOpenError::RerankerDevicePolicy(error) => reranker_device_policy_error_to_napi(error),
        EngineOpenError::Io { message } => typed_error(
            CODE_STORAGE,
            format!("database I/O error: {message}"),
            JsonValue::Null,
        ),
    }
}

pub(crate) fn runtime_configuration_error_to_napi(error: RustRuntimeConfigurationError) -> Error {
    let mode = |value| match value {
        RustRuntimeSqliteMode::Performance => "performance",
        RustRuntimeSqliteMode::Diagnostics => "diagnostics",
    };
    let data = match error {
        RustRuntimeConfigurationError::TooLate => json!({
            "reason": "too_late",
            "requestedMode": null,
            "effectiveMode": null,
            "sqliteCode": null
        }),
        RustRuntimeConfigurationError::Conflict { requested, effective } => json!({
            "reason": "conflict",
            "requestedMode": mode(requested),
            "effectiveMode": mode(effective),
            "sqliteCode": null
        }),
        RustRuntimeConfigurationError::SqliteFailure { code } => json!({
            "reason": "sqlite_failure",
            "requestedMode": null,
            "effectiveMode": null,
            "sqliteCode": code
        }),
    };
    typed_error(CODE_RUNTIME_CONFIGURATION, error.to_string(), data)
}

pub(crate) fn panic_error() -> Error {
    typed_error(CODE_PANIC, "engine panic (see logs)", JsonValue::Null)
}
