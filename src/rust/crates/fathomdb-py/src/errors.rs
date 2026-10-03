use super::*;

// ===== Exceptions =====================================================
//
// Root + concrete leaves per dev/design/errors.md § Binding-facing class
// matrix. All concrete leaves inherit from `EngineError`; `EngineError`
// inherits from Python `Exception` via `create_exception!`.

create_exception!(_fathomdb, EngineError, PyException);
create_exception!(_fathomdb, RuntimeConfigurationError, EngineError);
create_exception!(_fathomdb, StorageError, EngineError);
create_exception!(_fathomdb, ProjectionError, EngineError);
create_exception!(_fathomdb, ProjectionGenerationError, EngineError);
create_exception!(_fathomdb, VectorError, EngineError);
create_exception!(_fathomdb, KindNotVectorIndexedError, VectorError);
create_exception!(_fathomdb, EmbedderError, EngineError);
create_exception!(_fathomdb, EmbedDevicePolicyError, EmbedderError);
create_exception!(_fathomdb, RerankerDevicePolicyError, EmbedderError);
create_exception!(_fathomdb, EmbedderNotConfiguredError, EmbedderError);
create_exception!(_fathomdb, EmbedderRequiredError, EmbedderError);
create_exception!(_fathomdb, SchedulerError, EngineError);
create_exception!(_fathomdb, OpStoreError, EngineError);
create_exception!(_fathomdb, WriteValidationError, EngineError);
create_exception!(_fathomdb, SchemaValidationError, EngineError);
create_exception!(_fathomdb, ProvenanceError, EngineError);
create_exception!(_fathomdb, DependencyError, EngineError);
create_exception!(_fathomdb, DependencyClosureError, EngineError);
create_exception!(_fathomdb, ActuationError, EngineError);
create_exception!(_fathomdb, OverloadedError, EngineError);
create_exception!(_fathomdb, ClosingError, EngineError);
create_exception!(_fathomdb, DatabaseLockedError, EngineError);
create_exception!(_fathomdb, CorruptionError, EngineError);
create_exception!(_fathomdb, IncompatibleSchemaVersionError, EngineError);
create_exception!(_fathomdb, MigrationError, EngineError);
create_exception!(_fathomdb, EmbedderIdentityMismatchError, EngineError);
create_exception!(_fathomdb, EmbedderDimensionMismatchError, EngineError);
// G11 (Slice 15) — BYO-LLM extraction harness protocol error.
create_exception!(_fathomdb, ExtractorError, EngineError);
// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation harness protocol error.
create_exception!(_fathomdb, ConsolidatorError, EngineError);
// G4 (Slice 35) — filter predicate construction error (non-allowlisted path).
create_exception!(_fathomdb, InvalidFilterError, EngineError);
create_exception!(_fathomdb, FrozenReadError, EngineError);
create_exception!(_fathomdb, EvidenceError, EngineError);
create_exception!(_fathomdb, PageError, EngineError);
create_exception!(_fathomdb, DependencyTraceError, EngineError);
create_exception!(_fathomdb, GraphExpansionError, EngineError);
// 0.8.18 Slice 5 (#5 vector-equivalence probe) — query-time dense-refusal leaf.
create_exception!(_fathomdb, VectorEquivalenceMismatchError, EngineError);
// Slice 20 (G5/G6) — traversal depth > 3 or other out-of-range argument.
create_exception!(_fathomdb, InvalidArgumentError, EngineError);
// OPP-12 Phase-1 (0.8.19 Slice 10) — an illegal lifecycle `transition`/`purge`
// move (carries `from_state`/`to_state`/`legal`) and a non-`l:` lifecycle-verb id
// (carries `id_space`). Field names are parity-safe (S7 — `from` is reserved).
create_exception!(_fathomdb, IllegalTransitionError, EngineError);
create_exception!(_fathomdb, NotLifecycleAddressableError, EngineError);
// 0.8.20 Slice 5b (R-20-E5) — an erasure verb deleted its rows but could not
// complete the erasure AT REST (carries `stage`/`detail`). Raised instead of
// returning success: an erasure verb must never report success on an incomplete
// erasure.
create_exception!(_fathomdb, ErasureIncompleteError, EngineError);
// 0.8.20 Slice 15d (R-20-PR) — `configure_projections` refused a destructive
// change to a live projection without an explicit `drop` (carries `name` +
// `delta`). Omission never drops; a role removal / tokenizer / embedder change
// requires an explicit drop.
create_exception!(_fathomdb, ProjectionDestructiveError, EngineError);

// ===== Error mapping ==================================================

/// Translate every `EngineError` variant to its Python counterpart.
///
/// No catch-all arm: drift between the Rust enum and the Python class
/// set is a compile error.
pub(super) fn engine_error_to_py(err: RustEngineError) -> PyErr {
    match err {
        RustEngineError::Storage => StorageError::new_err("storage error"),
        RustEngineError::Projection => ProjectionError::new_err("projection error"),
        RustEngineError::ProjectionGeneration(error) => projection_generation_error_to_py(&error),
        RustEngineError::Vector => VectorError::new_err("vector error"),
        RustEngineError::Embedder => EmbedderError::new_err("embedder error"),
        RustEngineError::RerankerDevicePolicy(error) => {
            let exc = RerankerDevicePolicyError::new_err(error.to_string());
            Python::attach(|py| {
                let value = exc.value(py);
                let _ = value.setattr("kind", error.kind());
                let _ = value.setattr("ordinal", error.ordinal());
            });
            exc
        }
        RustEngineError::EmbedderNotConfigured => {
            EmbedderNotConfiguredError::new_err("embedder is not configured")
        }
        RustEngineError::EmbedderRequired(required) => {
            let exc =
                EmbedderRequiredError::new_err("embedder is required for pending projection work");
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("code", required.code);
                let _ = v.setattr("operation", required.operation.as_str());
                let _ = v.setattr("state", required.state.as_str());
                let _ = v.setattr("remediations", required.remediations);
                let _ = v.setattr("documentation_url", required.documentation_url);
            });
            exc
        }
        RustEngineError::KindNotVectorIndexed => {
            KindNotVectorIndexedError::new_err("kind is not configured for vector indexing")
        }
        RustEngineError::EmbedderDimensionMismatch { expected, actual } => {
            let exc = EmbedderDimensionMismatchError::new_err(format!(
                "embedder vector dimension mismatch: stored {expected}, supplied {actual}",
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("stored", expected);
                let _ = v.setattr("supplied", actual);
            });
            exc
        }
        RustEngineError::Scheduler => SchedulerError::new_err("scheduler error"),
        RustEngineError::OpStore => OpStoreError::new_err("op-store error"),
        RustEngineError::WriteValidation => WriteValidationError::new_err("write validation error"),
        RustEngineError::SchemaValidation => {
            SchemaValidationError::new_err("schema validation error")
        }
        RustEngineError::Provenance(error) => provenance_error_to_py(&error),
        RustEngineError::Dependency(error) => dependency_error_to_py(&error),
        RustEngineError::DependencyClosure(error) => dependency_closure_error_to_py(&error),
        RustEngineError::Actuation(error) => actuation_error_to_py(&error),
        RustEngineError::Overloaded => OverloadedError::new_err("engine overloaded"),
        RustEngineError::Closing => ClosingError::new_err("engine is closing"),
        RustEngineError::Extractor => ExtractorError::new_err("extractor error"),
        RustEngineError::Consolidator => ConsolidatorError::new_err("consolidator error"),
        RustEngineError::InvalidFilter { reason } => {
            InvalidFilterError::new_err(format!("invalid filter: {reason}"))
        }
        RustEngineError::FrozenRead(error) => frozen_read_error_to_py(&error),
        RustEngineError::Evidence(error) => {
            let exc = EvidenceError::new_err(format!(
                "{} at {}",
                error.reason.as_str(),
                error.field_path
            ));
            Python::attach(|py| {
                let value = exc.value(py);
                let _ = value.setattr("reason", error.reason.as_str());
                let _ = value.setattr("field_path", &error.field_path);
            });
            exc
        }
        RustEngineError::DependencyTrace(error) => {
            dependency_trace_error(error.reason.as_str(), &error.field_path)
        }
        RustEngineError::GraphExpansion(error) => graph_expansion_error_to_py(&error),
        RustEngineError::Page(error) => {
            let exc =
                PageError::new_err(format!("{} at {}", error.reason.as_str(), error.field_path));
            Python::attach(|py| {
                let value = exc.value(py);
                let _ = value.setattr("reason", error.reason.as_str());
                let _ = value.setattr("field_path", error.field_path);
            });
            exc
        }
        RustEngineError::InvalidArgument { msg } => InvalidArgumentError::new_err(msg),
        RustEngineError::VectorEquivalenceMismatch { reason } => {
            let exc = VectorEquivalenceMismatchError::new_err(format!(
                "vector-equivalence self-check failed; dense retrieval refused: {reason}"
            ));
            Python::attach(|py| {
                let _ = exc.value(py).setattr("reason", reason);
            });
            exc
        }
        RustEngineError::IllegalTransition { from_state, to_state, legal } => {
            let legal_str: Vec<&'static str> = legal.iter().map(|s| s.as_str()).collect();
            let exc = IllegalTransitionError::new_err(format!(
                "illegal lifecycle transition {} -> {}; legal targets: {:?}",
                from_state.as_str(),
                to_state.as_str(),
                legal_str,
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                // Parity-safe field names (S7): `from_state`/`to_state`, NOT `from`.
                let _ = v.setattr("from_state", from_state.as_str());
                let _ = v.setattr("to_state", to_state.as_str());
                let _ = v.setattr("legal", legal_str);
            });
            exc
        }
        RustEngineError::NotLifecycleAddressable { id_space } => {
            let exc = NotLifecycleAddressableError::new_err(format!(
                "id space {:?} is not lifecycle-addressable; only the logical (l:) space is",
                id_space.as_str(),
            ));
            Python::attach(|py| {
                let _ = exc.value(py).setattr("id_space", id_space.as_str());
            });
            exc
        }
        RustEngineError::ErasureIncomplete { stage, detail } => {
            let exc = ErasureIncompleteError::new_err(format!(
                "erasure incomplete at stage '{stage}': {detail}"
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("stage", stage.clone());
                let _ = v.setattr("detail", detail.clone());
            });
            exc
        }
        RustEngineError::ProjectionDestructive { name, delta } => {
            let exc = ProjectionDestructiveError::new_err(format!(
                "configure_projections refused a destructive change to '{name}': {delta}; \
                 re-issue with drop: [\"{name}\"]"
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("name", name.clone());
                let _ = v.setattr("delta", delta.clone());
            });
            exc
        }
        // Cargo unifies dependency features across a workspace build. The CLI
        // can therefore add the operator-only engine variant while this SDK
        // still has no operator surface capable of producing it.
        #[allow(unreachable_patterns)]
        operator_only => EngineError::new_err(operator_only.to_string()),
    }
}

pub(super) fn graph_expansion_error_to_py(error: &fathomdb_engine::GraphExpansionErrorV1) -> PyErr {
    let exc = GraphExpansionError::new_err(error.to_string());
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", &error.field_path);
    });
    exc
}

pub(super) fn projection_generation_error_to_py(error: &RustProjectionGenerationError) -> PyErr {
    let exc = ProjectionGenerationError::new_err(format!(
        "projection generation {} at {}",
        error.reason.as_str(),
        error.field_path
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", error.field_path.as_str());
    });
    exc
}

pub(super) fn provenance_error_to_py(error: &RustProvenanceError) -> PyErr {
    let exc = ProvenanceError::new_err(format!(
        "provenance {} at {}",
        error.reason.as_str(),
        error.field_path
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", error.field_path.as_str());
    });
    exc
}

pub(super) fn dependency_error_to_py(error: &RustDependencyError) -> PyErr {
    let exc = DependencyError::new_err(format!(
        "dependency {} at {}",
        error.reason.as_str(),
        error.field_path
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", error.field_path.as_str());
    });
    exc
}

pub(super) fn dependency_closure_error_to_py(error: &RustDependencyClosureError) -> PyErr {
    let exc = DependencyClosureError::new_err(format!(
        "dependency closure {} at {}",
        error.reason.as_str(),
        error.field_path
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", error.field_path.as_str());
    });
    exc
}

pub(super) fn actuation_error_to_py(error: &RustActuationError) -> PyErr {
    let exc = ActuationError::new_err(format!(
        "actuation {} at {}",
        error.reason.as_str(),
        error.field_path
    ));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", error.field_path.as_str());
    });
    exc
}

pub(super) fn frozen_read_error_to_py(error: &fathomdb_engine::FrozenReadError) -> PyErr {
    let exception =
        FrozenReadError::new_err(format!("{} at {}", error.reason.as_str(), error.field_path));
    Python::attach(|py| {
        let value = exception.value(py);
        let _ = value.setattr("reason", error.reason.as_str());
        let _ = value.setattr("field_path", &error.field_path);
    });
    exception
}

pub(super) fn dependency_trace_error(reason: &str, field_path: &str) -> PyErr {
    let exception = DependencyTraceError::new_err(format!("{reason} at {field_path}"));
    Python::attach(|py| {
        let value = exception.value(py);
        let _ = value.setattr("code", "FDB_DEPENDENCY_TRACE");
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exception
}

pub(super) fn page_error(reason: &'static str, field_path: &'static str) -> PyErr {
    let exception = PageError::new_err(format!("{reason} at {field_path}"));
    Python::attach(|py| {
        let value = exception.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exception
}

pub(super) fn corruption_kind_str(kind: CorruptionKind) -> &'static str {
    match kind {
        CorruptionKind::WalReplayFailure => "WalReplayFailure",
        CorruptionKind::HeaderMalformed => "HeaderMalformed",
        CorruptionKind::SchemaInconsistent => "SchemaInconsistent",
        CorruptionKind::EmbedderIdentityDrift => "EmbedderIdentityDrift",
        CorruptionKind::ProjectionGenerationDrift => "ProjectionGenerationDrift",
    }
}

pub(super) fn open_stage_str(stage: OpenStage) -> &'static str {
    match stage {
        OpenStage::HeaderProbe => "HeaderProbe",
        OpenStage::WalReplay => "WalReplay",
        OpenStage::SchemaProbe => "SchemaProbe",
        OpenStage::EmbedderIdentity => "EmbedderIdentity",
        OpenStage::ProjectionGeneration => "ProjectionGeneration",
    }
}

pub(super) fn engine_open_error_to_py(err: EngineOpenError) -> PyErr {
    match err {
        EngineOpenError::RuntimeConfiguration(error) => runtime_configuration_error_to_py(error),
        EngineOpenError::EngineConfiguration(error) => {
            InvalidArgumentError::new_err(error.to_string())
        }
        EngineOpenError::DatabaseLocked { holder_pid } => {
            let exc = DatabaseLockedError::new_err(match holder_pid {
                Some(pid) => format!("database is locked by process {pid}"),
                None => "database is locked by another engine instance".to_string(),
            });
            Python::attach(|py| {
                let _ = exc.value(py).setattr("holder_pid", holder_pid);
            });
            exc
        }
        EngineOpenError::Corruption(detail) => corruption_to_py(detail),
        EngineOpenError::IncompatibleSchemaVersion { seen, supported } => {
            IncompatibleSchemaVersionError::new_err(format!(
                "database schema version {seen} is incompatible with supported version {supported}"
            ))
        }
        EngineOpenError::MigrationError {
            schema_version_before,
            schema_version_current,
            step_id,
        } => MigrationError::new_err(format!(
            "schema migration failed at step {step_id}; schema version remained between {schema_version_before} and {schema_version_current}"
        )),
        EngineOpenError::EmbedderIdentityMismatch { stored, supplied } => {
            let exc = EmbedderIdentityMismatchError::new_err(format!(
                "embedder identity mismatch: stored {}@{}, supplied {}@{}",
                stored.name, stored.revision, supplied.name, supplied.revision,
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("stored_name", stored.name);
                let _ = v.setattr("stored_revision", stored.revision);
                let _ = v.setattr("supplied_name", supplied.name);
                let _ = v.setattr("supplied_revision", supplied.revision);
            });
            exc
        }
        EngineOpenError::EmbedderDimensionMismatch { stored, supplied } => {
            let exc = EmbedderDimensionMismatchError::new_err(format!(
                "embedder vector dimension mismatch: stored {stored}, supplied {supplied}",
            ));
            Python::attach(|py| {
                let v = exc.value(py);
                let _ = v.setattr("stored", stored);
                let _ = v.setattr("supplied", supplied);
            });
            exc
        }
        EngineOpenError::Embedder(err) => EmbedderError::new_err(format!("{err:?}")),
        EngineOpenError::EmbedDevicePolicy(error) => {
            let exc = EmbedDevicePolicyError::new_err(error.to_string());
            Python::attach(|py| {
                let value = exc.value(py);
                let _ = value.setattr("kind", error.kind());
                let _ = value.setattr("ordinal", error.ordinal());
            });
            exc
        }
        EngineOpenError::RerankerDevicePolicy(error) => {
            let exc = RerankerDevicePolicyError::new_err(error.to_string());
            Python::attach(|py| {
                let value = exc.value(py);
                let _ = value.setattr("kind", error.kind());
                let _ = value.setattr("ordinal", error.ordinal());
            });
            exc
        }
        EngineOpenError::Io { message } => {
            StorageError::new_err(format!("database I/O error: {message}"))
        }
    }
}

pub(super) fn corruption_to_py(detail: CorruptionDetail) -> PyErr {
    let kind = corruption_kind_str(detail.kind);
    let stage = open_stage_str(detail.stage);
    let recovery_hint_code = detail.recovery_hint.code;
    let doc_anchor = detail.recovery_hint.doc_anchor;
    let exc = CorruptionError::new_err(format!(
        "corruption {kind} at stage {stage} ({recovery_hint_code})"
    ));
    Python::attach(|py| {
        let v = exc.value(py);
        let _ = v.setattr("kind", kind);
        let _ = v.setattr("stage", stage);
        let _ = v.setattr("recovery_hint_code", recovery_hint_code);
        let _ = v.setattr("doc_anchor", doc_anchor);
    });
    exc
}

pub(super) fn runtime_configuration_error_to_py(error: RustRuntimeConfigurationError) -> PyErr {
    Python::attach(|py| {
        let exc = RuntimeConfigurationError::new_err(error.to_string());
        let value = exc.value(py);
        match error {
            RustRuntimeConfigurationError::TooLate => {
                let _ = value.setattr("reason", "too_late");
                let _ = value.setattr("requested_mode", py.None());
                let _ = value.setattr("effective_mode", py.None());
                let _ = value.setattr("sqlite_code", py.None());
            }
            RustRuntimeConfigurationError::Conflict { requested, effective } => {
                let name = |mode| match mode {
                    RustRuntimeSqliteMode::Performance => "performance",
                    RustRuntimeSqliteMode::Diagnostics => "diagnostics",
                };
                let _ = value.setattr("reason", "conflict");
                let _ = value.setattr("requested_mode", name(requested));
                let _ = value.setattr("effective_mode", name(effective));
                let _ = value.setattr("sqlite_code", py.None());
            }
            RustRuntimeConfigurationError::SqliteFailure { code } => {
                let _ = value.setattr("reason", "sqlite_failure");
                let _ = value.setattr("requested_mode", py.None());
                let _ = value.setattr("effective_mode", py.None());
                let _ = value.setattr("sqlite_code", code);
            }
        }
        exc
    })
}
