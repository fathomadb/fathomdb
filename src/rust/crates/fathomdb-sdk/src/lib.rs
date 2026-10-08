//! # fathomdb-sdk
//!
//! The FathomDB Rust application SDK. It carries the same surface as the
//! Python and TypeScript SDKs: an [`Engine`] with the governed engine
//! operations, the [`read`], [`graph`] and [`admin`] namespaces, and the
//! standalone [`rerank`] and [`embed_batch_cls`] operations. Calls are
//! synchronous and return [`Result`]; optional arguments are option structs
//! whose `Default` equals the Python/TypeScript defaults; [`Error::kind`]
//! names the shared error class. The contract and its translation rules are
//! `dev/interfaces/rust-sdk.md`.
//!
//! ```no_run
//! use fathomdb_sdk::{read, Engine, InitialState, OpenOptions, PreparedWrite, SearchOptions, SourceId};
//!
//! # fn main() -> fathomdb_sdk::Result<()> {
//! let engine = Engine::open("notes.sqlite", OpenOptions::default())?;
//! engine.write(&[PreparedWrite::Node {
//!     kind: "note".into(),
//!     body: "alpha river crossing".into(),
//!     source_id: SourceId::new("import-1").expect("valid source id"),
//!     logical_id: Some("note:1".into()),
//!     state: InitialState::Active,
//!     reason: None,
//!     valid_from: None,
//!     valid_until: None,
//! }])?;
//! let note = read::get(&engine, "note:1", None)?;
//! let hits = engine.search("alpha", SearchOptions::default())?;
//! engine.close()?;
//! # let _ = (note, hits);
//! # Ok(())
//! # }
//! ```
//!
//! Custom embedder injection, the operator/recovery seam, raw SQL and test
//! hooks are deliberately not reachable from this crate.

mod engine;
mod error;
mod guard;
mod options;
mod standalone;

pub mod admin;
pub mod graph;
pub mod read;

pub use engine::Engine;
pub use error::{CudaErrorDetails, Error, ErrorKind, Result};
pub use options::{
    FrozenSearchOptions, ListOptions, NeighborsOptions, OpenOptions, ProjectedTextSearchOptions,
    RerankOptions, SearchExpandOptions, SearchFilterArg, SearchOptions, TextSearchOptions,
};
pub use standalone::{embed_batch_cls, rerank, RerankPassage, RerankResult};

// Shared request, result and payload types: the `fathomdb` facade's
// non-operator types minus `Engine`, `OpenedEngine` and `Subscription`, plus
// the names SDK signatures and public fields need.
pub use fathomdb_embedder::{
    CudaDeviceInfo, CudaVisibleDevice, DeviceResolution, EffectiveEmbedDevice,
    EffectiveRerankerDevice, EmbedDevicePolicyError, EmbedderEvent, GpuAllocationWitness,
    RerankerDevicePolicyError, RerankerDeviceResolution,
};
pub use fathomdb_embedder_api::{EmbedderError as RuntimeEmbedderError, EmbedderIdentity};
pub use fathomdb_engine::lifecycle::{
    Event as SubscriberEvent, EventCategory, EventSource, Phase, ProfileRecord, ProjectionStatus,
    SlowStatement, StressFailureContext, Subscriber,
};
pub use fathomdb_engine::{
    ActuationBatchV1, ActuationError, ActuationErrorReason, ActuationOperationV1,
    ActuationOutcomeV1, ActuationReceiptV1, ActuationRefusalReasonV1, ArtifactRevisionId,
    BoundaryCrossing, CanonicalHash, ClosureCauseV1, ClosureLookupV1, ClosureOperationId,
    ClosurePhaseV1, ClosureProofV1, ClosureRootV1, ClosureStatusV1, ComparisonOp, ConsolidateAxis,
    ConsolidateReceipt, CorruptionDetail, CorruptionKind, CorruptionLocator, CounterSnapshot,
    DenseReadiness, DependencyClosureError, DependencyClosureErrorReason,
    DependencyDerivedLookupV1, DependencyError, DependencyErrorReason, DependencyId,
    DependencyListV1, DependencySourceLookupV1, DependencyTraceDirectionV1, DependencyTraceEdgeV1,
    DependencyTraceErrorReasonV1, DependencyTraceErrorV1, DependencyTraceNodeV1,
    DependencyTraceRequestV1, DependencyTraceResultV1, EmbedderRequired, EmbeddingOperation,
    EmbeddingReadiness, EmbeddingReadinessState, EngineConfig, EngineConfigurationError,
    EngineError, EngineOpenError, EvidenceArmV1, EvidenceArtifactClassV1,
    EvidenceArtifactLifecycleV1, EvidenceContributionV1, EvidenceErrorReasonV1, EvidenceErrorV1,
    EvidenceGraphOriginV1, EvidenceProjectionOriginV1, EvidenceRefV1, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, EvidenceSearchResultV1, EvidenceSidecarEntryV1,
    ExciseReport as EraseReport, Explanation, ExtractDocument, Filter, FilterTerm,
    FrozenReadContextV1, FrozenReadError, FrozenReadErrorReason, GraphEvidenceArtifactV1,
    GraphEvidenceRefV1, GraphEvidenceResolveRequestV1, GraphEvidenceSidecarEntryV1,
    GraphEvidenceSidecarV1, GraphExpandRequestV1, GraphExpandResultV1,
    GraphExpansionDegradationCodeV1, GraphExpansionErrorReasonV1, GraphExpansionErrorV1,
    GraphExpansionExplanationV1, GraphOriginV1, GraphProjectionOriginV1,
    GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1, GraphSeedSourceV1,
    GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, IdSpace, IdSpaceKind,
    IngestWithExtractorReceipt, InitialState, LifecycleActuationV1, LifecycleState,
    MutationProjectionStatusRequestV1, MutationProjectionStatusV1, NodeRecord, OpStoreRow,
    OpenReport, OpenStage, OperationalStateRecordV1, PageCursor, PageError, PageErrorReason,
    PageRequestV1, PageV1, PerHitExplain, Predicate, PreparedWrite, ProjectionDelta, ProjectionFts,
    ProjectionGenerationError, ProjectionGenerationErrorReason, ProjectionGenerationId,
    ProjectionGenerationOriginV1, ProjectionGenerationStatusV1, ProjectionReadinessV1,
    ProjectionRole, ProjectionRuntimeStateV1, ProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry, ProjectionRuntimeUnavailabilityReason, ProjectionSpec,
    ProjectionStatusDenseReadiness, ProjectionVector, ProvenanceCompleteness, ProvenanceError,
    ProvenanceErrorReason, ProvenancedEdgeV1, ProvenancedNodeV1, QueryTrace, ReadContextV1,
    ReadView, RecoveryHint, ResolvedEvidenceV1, ResolvedGraphEvidenceV1, ResolvedGraphSeedV1,
    RuntimeConfiguration, RuntimeConfigurationError, RuntimeSqliteMode, ScalarValue,
    SearchExpandResult, SearchFilter, SearchHit, SearchResult, SoftFallback, SoftFallbackBranch,
    SourceDependencyRegistrationV1, SourceDependencyV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, StructuralDegradationCodeV1, StructuralDependencyStateV1,
    StructuralInclusionStateV1, StructuralInclusionV1, StructuralLifecycleStateV1,
    StructuralProjectionOriginV1, TraceArtifactClassV1, TraceArtifactRoleV1, TraceNodeLifecycleV1,
    TraceReadBoundaryV1, TraversalDirection, WriteProvenanceV1, WriteReceipt,
};
pub use fathomdb_schema::MigrationStepReport;

/// Absence proofs: each core-only route below must not resolve through
/// `fathomdb_sdk`. The first block compiles, so the `compile_fail` blocks
/// fail for the named member, not for an unrelated error.
///
/// ```
/// fn reachable(engine: &fathomdb_sdk::Engine) {
///     let _ = fathomdb_sdk::read::get(engine, "note:1", None);
/// }
/// ```
///
/// ```compile_fail,E0432
/// use fathomdb_sdk::EmbedderChoice;
/// ```
///
/// ```compile_fail,E0432
/// use fathomdb_sdk::OpenedEngine;
/// ```
///
/// ```compile_fail,E0599
/// let _ = fathomdb_sdk::Engine::open_with_choice;
/// ```
///
/// ```compile_fail,E0599
/// fn leak(engine: &fathomdb_sdk::Engine) {
///     let _ = engine.read_get("note:1", &fathomdb_sdk::ReadView::default());
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn leak(engine: &fathomdb_sdk::Engine) {
///     let _ = engine.check_integrity();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn leak(engine: &fathomdb_sdk::Engine) {
///     let _ = engine.execute_for_test("SELECT 1");
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn leak(engine: &fathomdb_sdk::Engine) {
///     let _ = engine.search_with_limit("alpha", 3);
/// }
/// ```
#[doc(hidden)]
pub mod core_surface_absence_proof {}
