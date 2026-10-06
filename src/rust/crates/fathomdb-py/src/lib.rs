// PyO3's `create_exception!` and `#[pymodule]` macros emit
// `#[cfg(feature = "gil-refs")]` arms that reference an upstream
// feature this crate does not export; the resulting `unexpected_cfgs`
// warnings are noise on a clippy `-D warnings` gate. The
// `useless_conversion` allow covers `#[pymethods]`-generated PyResult
// wrappers that clippy flags as redundant `Into<PyErr>` calls.
#![allow(unexpected_cfgs)]
#![allow(clippy::useless_conversion)]

//! PyO3 binding from the Python SDK to `fathomdb-engine`.
//!
//! FFI safety contract (mirrored by Phase 11b napi-rs):
//!
//! 1. Every method that may block inside the engine wraps the call in
//!    `py.detach(...)` so the GIL is released for the duration.
//! 2. Engine entry points return typed errors via [`engine_error_to_py`] /
//!    [`engine_open_error_to_py`] — single-switch mapping with no
//!    catch-all arm; the binding fails to compile when the Rust variant
//!    set drifts from the Python class set (AC-060a).
//! 3. Every string crossing the FFI is checked by [`validate_ffi_string`]
//!    for embedded NUL or unpaired UTF-16 surrogates BEFORE the writer
//!    transaction opens (AC-068a / AC-068b).
//! 4. Panics inside engine code surface as Python `PanicException`
//!    instances (PyO3 `pyo3::panic::PanicException`); the host process
//!    is not aborted (AC-067). Engine calls are wrapped in
//!    `catch_unwind` so the panic is translated on the Rust side rather
//!    than relying on PyO3's implicit conversion at the FFI boundary.
//!    PanicException is intentionally NOT an `EngineError` subclass:
//!    panic is a contract bug, not a typed engine outcome, and callers
//!    that catch `EngineError` must not silently swallow it.

mod logging_subscriber;

use std::panic::{catch_unwind, AssertUnwindSafe};
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(feature = "test-hooks")]
use std::sync::Barrier;
#[cfg(feature = "test-hooks")]
use std::sync::Mutex;

use fathomdb_embedder::{
    CudaDeviceInfo as RustCudaDeviceInfo, CudaVisibleDevice as RustCudaVisibleDevice,
    DeviceResolution as RustDeviceResolution, EffectiveEmbedDevice as RustEffectiveEmbedDevice,
    EffectiveRerankerDevice as RustEffectiveRerankerDevice,
    EmbedDevicePolicy as RustEmbedDevicePolicy, EmbedderEvent as RustEmbedderEvent,
    GpuAllocationWitness as RustGpuAllocationWitness,
    RerankerDevicePolicy as RustRerankerDevicePolicy,
    RerankerDeviceResolution as RustRerankerDeviceResolution, SOLE_GPU_CONSUMER_PRECONDITION,
    TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA,
};
use fathomdb_embedder_api::EmbedderIdentity as RustEmbedderIdentity;
use fathomdb_engine::{
    decode_graph_expand_request_v1, encode_dependency_trace_result_v1,
    encode_graph_expand_result_v1, encode_resolved_graph_evidence_v1,
    rerank_passages as rust_rerank_passages, ActuationBatchV1,
    ActuationError as RustActuationError, ActuationOperationV1, ActuationOutcomeV1,
    ActuationReceiptV1 as RustActuationReceiptV1, ArtifactRevisionId,
    BoundaryCrossing as RustBoundaryCrossing, CanonicalHash, ClosureLookupV1, ClosureRootV1,
    ClosureStatusV1 as RustClosureStatusV1, ComparisonOp as RustComparisonOp,
    ConsolidateAxis as RustConsolidateAxis, ConsolidateReceipt as RustConsolidateReceipt,
    CorruptionDetail, CorruptionKind, DenseReadiness as RustDenseReadiness,
    DependencyClosureError as RustDependencyClosureError, DependencyDerivedLookupV1,
    DependencyError as RustDependencyError, DependencyListV1 as RustDependencyListV1,
    DependencySourceLookupV1, DependencyTraceDirectionV1 as RustDependencyTraceDirectionV1,
    DependencyTraceRequestV1 as RustDependencyTraceRequestV1, EmbedderChoice,
    EmbeddingReadiness as RustEmbeddingReadiness, Engine as RustEngine,
    EngineConfig as RustEngineConfig, EngineError as RustEngineError, EngineOpenError,
    EvidenceArtifactLifecycleV1 as RustEvidenceArtifactLifecycleV1,
    EvidenceContributionV1 as RustEvidenceContributionV1,
    EvidenceGraphOriginV1 as RustEvidenceGraphOriginV1,
    EvidenceProjectionOriginV1 as RustEvidenceProjectionOriginV1,
    EvidenceRefV1 as RustEvidenceRefV1, EvidenceResolveRequestV1 as RustEvidenceResolveRequestV1,
    EvidenceSearchRequestV1 as RustEvidenceSearchRequestV1,
    EvidenceSearchResultV1 as RustEvidenceSearchResultV1,
    EvidenceSidecarEntryV1 as RustEvidenceSidecarEntryV1, ExciseReport as RustExciseReport,
    Explanation as RustExplanation, ExtractDocument as RustExtractDocument, Filter as RustFilter,
    FilterTerm as RustFilterTerm, FrozenReadContextV1 as RustFrozenReadContextV1,
    GraphEvidenceRefV1 as RustGraphEvidenceRefV1,
    GraphEvidenceResolveRequestV1 as RustGraphEvidenceResolveRequestV1, IdSpace as RustIdSpace,
    IngestWithExtractorReceipt as RustIngestWithExtractorReceipt, InitialState,
    LifecycleActuationV1, LifecycleState as RustLifecycleState,
    MutationProjectionStatusRequestV1 as RustMutationProjectionStatusRequestV1,
    MutationProjectionStatusV1 as RustMutationProjectionStatusV1, NodeRecord as RustNodeRecord,
    OpStoreRow as RustOpStoreRow, OpenReport as RustOpenReport, OpenStage,
    OperationalStateRecordV1 as RustOperationalStateRecordV1, PageCursor as RustPageCursor,
    PageRequestV1 as RustPageRequestV1, PageV1 as RustPageV1, PerHitExplain as RustPerHitExplain,
    Predicate as RustPredicate, PreparedWrite, ProjectionDelta as RustProjectionDelta,
    ProjectionFts as RustProjectionFts, ProjectionGenerationError as RustProjectionGenerationError,
    ProjectionGenerationId as RustProjectionGenerationId,
    ProjectionGenerationStatusV1 as RustProjectionGenerationStatusV1,
    ProjectionRole as RustProjectionRole, ProjectionRuntimeStatus as RustProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry as RustProjectionRuntimeStatusEntry,
    ProjectionSpec as RustProjectionSpec, ProjectionVector as RustProjectionVector,
    ProvenanceError as RustProvenanceError, ProvenancedEdgeV1, ProvenancedNodeV1,
    QueryTrace as RustQueryTrace, ReadContextV1 as RustReadContextV1, ReadView as RustReadView,
    ResolvedEvidenceV1 as RustResolvedEvidenceV1, RuntimeConfiguration as RustRuntimeConfiguration,
    RuntimeConfigurationError as RustRuntimeConfigurationError,
    RuntimeSqliteMode as RustRuntimeSqliteMode, ScalarValue as RustScalarValue,
    SearchExpandResult as RustSearchExpandResult, SearchFilter as RustSearchFilter,
    SearchHit as RustSearchHit, SearchResult as RustSearchResult, SoftFallback as RustSoftFallback,
    SoftFallbackBranch, SourceDependencyRegistrationV1,
    SourceDependencyV1 as RustSourceDependencyV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, StructuralDegradationCodeV1 as RustStructuralDegradationCodeV1,
    StructuralDependencyStateV1 as RustStructuralDependencyStateV1,
    StructuralInclusionStateV1 as RustStructuralInclusionStateV1,
    StructuralInclusionV1 as RustStructuralInclusionV1,
    StructuralLifecycleStateV1 as RustStructuralLifecycleStateV1,
    StructuralProjectionOriginV1 as RustStructuralProjectionOriginV1,
    TraversalDirection as RustTraversalDirection, WriteProvenanceV1,
    WriteReceipt as RustWriteReceipt,
};
use fathomdb_schema::MigrationStepReport as RustMigrationStepReport;
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
use pyo3::panic::PanicException;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyList};

mod errors;
use errors::*;
mod ffi;
pub use ffi::*;
mod types;
use types::*;
mod engine;
use engine::*;
mod admin;
use admin::*;
mod projection;
use projection::*;
mod read_search;
use read_search::*;
mod write;
use write::*;
mod graph_evidence;
use graph_evidence::*;
mod embedding;
use embedding::*;
#[cfg(any(test, feature = "test-hooks"))]
mod test_support;
#[cfg(any(test, feature = "test-hooks"))]
use test_support::*;

// ===== Module =========================================================

// `gil_used = true` preserves current GIL semantics: PyO3 makes
// `#[pymodule]` free-threaded by default, but this binding is `abi3-py310`
// and the whole FFI contract assumes the GIL is held. Opting into
// free-threading (`gil_used = false`) is a separate, larger correctness
// campaign — see dev/design/free-threaded-python-value-lift-and-experiments.md.
#[pymodule(gil_used = true)]
fn _fathomdb(py: Python<'_>, m: Bound<'_, PyModule>) -> PyResult<()> {
    // 0.8.28 pool study only (ruling 1): early `cuInit` at import, as the
    // Node addon does at registration, so the driver's address reservation
    // exists before the application's heap grows. `FATHOMDB_CUDA_EARLY_INIT=off`
    // opts out. A panic must not fail the import; open records the outcome.
    #[cfg(all(
        feature = "tegra-pool-experiment",
        target_os = "linux",
        target_arch = "aarch64",
        any(feature = "embed-cuda", feature = "rerank-cuda")
    ))]
    if std::env::var("FATHOMDB_CUDA_EARLY_INIT").ok().as_deref() != Some("off") {
        let _ = std::panic::catch_unwind(fathomdb_embedder::initialize_cuda_driver);
    }
    m.add_class::<PyEngine>()?;
    #[cfg(feature = "test-hooks")]
    m.add_class::<PyWalSnapshotPause>()?;
    m.add_class::<PyWriteReceipt>()?;
    m.add_class::<PyEraseReport>()?;
    m.add_class::<PyIngestWithExtractorReceipt>()?;
    // 0.8.12 Slice 15 (OPP-2) — consolidation receipt.
    m.add_class::<PyConsolidateReceipt>()?;
    m.add_class::<PySoftFallback>()?;
    // C-2 (0.8.19 / TC-8) — typed IdSpace id carrier for SearchHit.id.
    m.add_class::<PyIdSpace>()?;
    m.add_class::<PySearchHit>()?;
    m.add_class::<PySearchResult>()?;
    m.add_class::<PyEvidenceSidecarEntryV1>()?;
    m.add_class::<PyEvidenceSearchResultV1>()?;
    m.add_class::<PyEvidenceContributionV1>()?;
    m.add_class::<PyEvidenceProjectionOriginV1>()?;
    m.add_class::<PyResolvedEvidenceV1>()?;
    // 0.8.8 EXP-OBS (Slice 10) — explanation sidecar types.
    m.add_class::<PyQueryTrace>()?;
    m.add_class::<PyPerHitExplain>()?;
    m.add_class::<PyStructuralInclusionV1>()?;
    m.add_class::<PyExplanation>()?;
    m.add_class::<PyCounterSnapshot>()?;
    m.add_class::<PyMigrationStepReport>()?;
    m.add_class::<PyEmbedderIdentity>()?;
    m.add_class::<PyCudaDeviceInfo>()?;
    m.add_class::<PyCudaVisibleDevice>()?;
    m.add_class::<PyEffectiveEmbedDevice>()?;
    m.add_class::<PyDeviceResolution>()?;
    m.add_class::<PyGpuAllocationWitness>()?;
    m.add_class::<PyOpenReport>()?;
    m.add_class::<PyNodeRecord>()?;
    m.add_class::<PyOpStoreRow>()?;
    m.add_class::<PyOperationalStateRecordV1>()?;
    m.add_class::<PyNodePageV1>()?;
    m.add_class::<PyOperationalStatePageV1>()?;
    m.add_class::<PySourceDependencyV1>()?;
    m.add_class::<PyActuationReceiptV1>()?;
    m.add_class::<PyProjectionGenerationStatusV1>()?;
    m.add_class::<PyMutationProjectionStatusV1>()?;
    m.add_class::<PyDependencyListV1>()?;
    m.add_class::<PyClosureProofV1>()?;
    m.add_class::<PyClosureStatusV1>()?;
    // Slice 20 — graph traversal result types.
    m.add_class::<PyExpandedNode>()?;
    m.add_class::<PySearchExpandResult>()?;
    m.add_class::<PyRuntimeConfiguration>()?;
    m.add_function(wrap_pyfunction!(admin_configure_runtime, &m)?)?;
    m.add_function(wrap_pyfunction!(admin_configure, &m)?)?;
    // OPP-12 Phase-1 (0.8.19 Slice 10) — lifecycle verbs.
    m.add_function(wrap_pyfunction!(transition, &m)?)?;
    m.add_function(wrap_pyfunction!(purge, &m)?)?;
    m.add_function(wrap_pyfunction!(erase_source, &m)?)?;
    // 0.8.20 Slice 15d — projection registry (R-20-PR).
    m.add_function(wrap_pyfunction!(configure_projections, &m)?)?;
    m.add_function(wrap_pyfunction!(read_projections, &m)?)?;
    m.add_function(wrap_pyfunction!(read_projection_status, &m)?)?;
    m.add_function(wrap_pyfunction!(read_projection_generation_status, &m)?)?;
    m.add_function(wrap_pyfunction!(read_mutation_projection_status, &m)?)?;
    m.add_function(wrap_pyfunction!(read_embedding_readiness, &m)?)?;
    m.add_class::<PyProjectionSpec>()?;
    m.add_class::<PyProjectionDelta>()?;
    m.add_class::<PyProjectionRuntimeStatusEntry>()?;
    m.add_class::<PyProjectionRuntimeStatus>()?;
    m.add_class::<PyEmbeddingReadiness>()?;
    // Slice 30 — governed read.* native fns (G2/G3).
    m.add_function(wrap_pyfunction!(read_get, &m)?)?;
    m.add_function(wrap_pyfunction!(read_get_many, &m)?)?;
    m.add_function(wrap_pyfunction!(read_collection, &m)?)?;
    m.add_function(wrap_pyfunction!(read_mutations, &m)?)?;
    // Slice 35 — G4 read.list with Predicate filter.
    m.add_function(wrap_pyfunction!(read_list, &m)?)?;
    // 0.8.11 Slice 40 — unified Filter → read.list backend (#17).
    m.add_function(wrap_pyfunction!(read_list_filter, &m)?)?;
    m.add_function(wrap_pyfunction!(read_canonical_page, &m)?)?;
    m.add_function(wrap_pyfunction!(read_operational_state, &m)?)?;
    m.add_function(wrap_pyfunction!(read_operational_state_page, &m)?)?;
    // Slice 20 — G5/G6 graph traversal fns.
    m.add_function(wrap_pyfunction!(graph_neighbors, &m)?)?;
    m.add_function(wrap_pyfunction!(crossed_boundary_since, &m)?)?;
    m.add_class::<PyReadView>()?;
    m.add_class::<PyReadContextV1>()?;
    m.add_class::<PyFrozenReadContextV1>()?;
    m.add_class::<PyBoundaryCrossing>()?;
    m.add_function(wrap_pyfunction!(search_expand, &m)?)?;
    // 0.8.2 Slice E2 — standalone rerank over an arbitrary passage list.
    m.add_function(wrap_pyfunction!(rerank, &m)?)?;
    m.add_function(wrap_pyfunction!(embed_batch_cls, &m)?)?;

    #[cfg(any(test, feature = "test-hooks"))]
    m.add_function(wrap_pyfunction!(force_panic_for_test, &m)?)?;
    #[cfg(feature = "test-hooks")]
    m.add_function(wrap_pyfunction!(native_raw_wal_checkpoint_for_test, &m)?)?;

    m.add("EngineError", py.get_type::<EngineError>())?;
    m.add("RuntimeConfigurationError", py.get_type::<RuntimeConfigurationError>())?;
    m.add("StorageError", py.get_type::<StorageError>())?;
    m.add("ProjectionError", py.get_type::<ProjectionError>())?;
    m.add("VectorError", py.get_type::<VectorError>())?;
    m.add("KindNotVectorIndexedError", py.get_type::<KindNotVectorIndexedError>())?;
    m.add("EmbedderError", py.get_type::<EmbedderError>())?;
    m.add("EmbedDevicePolicyError", py.get_type::<EmbedDevicePolicyError>())?;
    m.add("RerankerDevicePolicyError", py.get_type::<RerankerDevicePolicyError>())?;
    m.add("EmbedderNotConfiguredError", py.get_type::<EmbedderNotConfiguredError>())?;
    m.add("EmbedderRequiredError", py.get_type::<EmbedderRequiredError>())?;
    m.add("SchedulerError", py.get_type::<SchedulerError>())?;
    m.add("OpStoreError", py.get_type::<OpStoreError>())?;
    m.add("WriteValidationError", py.get_type::<WriteValidationError>())?;
    m.add("SchemaValidationError", py.get_type::<SchemaValidationError>())?;
    m.add("ProvenanceError", py.get_type::<ProvenanceError>())?;
    m.add("DependencyError", py.get_type::<DependencyError>())?;
    m.add("DependencyClosureError", py.get_type::<DependencyClosureError>())?;
    m.add("ActuationError", py.get_type::<ActuationError>())?;
    m.add("OverloadedError", py.get_type::<OverloadedError>())?;
    m.add("ClosingError", py.get_type::<ClosingError>())?;
    m.add("DatabaseLockedError", py.get_type::<DatabaseLockedError>())?;
    m.add("CorruptionError", py.get_type::<CorruptionError>())?;
    m.add("IncompatibleSchemaVersionError", py.get_type::<IncompatibleSchemaVersionError>())?;
    m.add("MigrationError", py.get_type::<MigrationError>())?;
    m.add("EmbedderIdentityMismatchError", py.get_type::<EmbedderIdentityMismatchError>())?;
    m.add("EmbedderDimensionMismatchError", py.get_type::<EmbedderDimensionMismatchError>())?;
    m.add("ExtractorError", py.get_type::<ExtractorError>())?;
    m.add("ConsolidatorError", py.get_type::<ConsolidatorError>())?;
    m.add("InvalidFilterError", py.get_type::<InvalidFilterError>())?;
    m.add("FrozenReadError", py.get_type::<FrozenReadError>())?;
    m.add("EvidenceError", py.get_type::<EvidenceError>())?;
    m.add("PageError", py.get_type::<PageError>())?;
    m.add("DependencyTraceError", py.get_type::<DependencyTraceError>())?;
    m.add("GraphExpansionError", py.get_type::<GraphExpansionError>())?;
    m.add("InvalidArgumentError", py.get_type::<InvalidArgumentError>())?;
    m.add("VectorEquivalenceMismatchError", py.get_type::<VectorEquivalenceMismatchError>())?;
    m.add("IllegalTransitionError", py.get_type::<IllegalTransitionError>())?;
    m.add("NotLifecycleAddressableError", py.get_type::<NotLifecycleAddressableError>())?;
    m.add("ErasureIncompleteError", py.get_type::<ErasureIncompleteError>())?;
    m.add("ProjectionDestructiveError", py.get_type::<ProjectionDestructiveError>())?;
    m.add("ProjectionGenerationError", py.get_type::<ProjectionGenerationError>())?;
    Ok(())
}
