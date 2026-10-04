//! Shared imported vocabulary for the private binding owners.

pub(crate) use std::panic::{catch_unwind, AssertUnwindSafe};
pub(crate) use std::sync::{Arc, Mutex};

pub(crate) use fathomdb_embedder::{
    CudaDeviceInfo as RustCudaDeviceInfo, CudaVisibleDevice as RustCudaVisibleDevice,
    DeviceResolution as RustDeviceResolution, EffectiveEmbedDevice as RustEffectiveEmbedDevice,
    EffectiveRerankerDevice as RustEffectiveRerankerDevice,
    EmbedDevicePolicy as RustEmbedDevicePolicy, EmbedderEvent as RustEmbedderEvent,
    GpuAllocationWitness as RustGpuAllocationWitness,
    RerankerDevicePolicy as RustRerankerDevicePolicy,
    RerankerDeviceResolution as RustRerankerDeviceResolution, SOLE_GPU_CONSUMER_PRECONDITION,
    TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA,
};
pub(crate) use fathomdb_embedder_api::EmbedderIdentity as RustEmbedderIdentity;
pub(crate) use fathomdb_engine::{
    decode_graph_expand_request_v1, encode_dependency_trace_result_v1,
    encode_graph_expand_result_v1, encode_resolved_graph_evidence_v1,
    rerank_passages as rust_rerank_passages, ActuationBatchV1, ActuationOperationV1,
    ActuationOutcomeV1, ActuationReceiptV1 as RustActuationReceiptV1, ArtifactRevisionId,
    BoundaryCrossing as RustBoundaryCrossing, CanonicalHash, ClosureLookupV1, ClosureRootV1,
    ClosureStatusV1 as RustClosureStatusV1, ComparisonOp as RustComparisonOp,
    ConsolidateAxis as RustConsolidateAxis, ConsolidateReceipt as RustConsolidateReceipt,
    CorruptionDetail, CorruptionKind, DenseReadiness as RustDenseReadiness,
    DependencyDerivedLookupV1, DependencyListV1 as RustDependencyListV1, DependencySourceLookupV1,
    DependencyTraceDirectionV1 as RustDependencyTraceDirectionV1,
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
    ProjectionFts as RustProjectionFts, ProjectionGenerationId as RustProjectionGenerationId,
    ProjectionGenerationStatusV1 as RustProjectionGenerationStatusV1,
    ProjectionRole as RustProjectionRole, ProjectionRuntimeStatus as RustProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry as RustProjectionRuntimeStatusEntry,
    ProjectionSpec as RustProjectionSpec, ProjectionVector as RustProjectionVector,
    ProvenancedEdgeV1, ProvenancedNodeV1, QueryTrace as RustQueryTrace,
    ReadContextV1 as RustReadContextV1, ReadView as RustReadView,
    ResolvedEvidenceV1 as RustResolvedEvidenceV1,
    RuntimeConfigurationError as RustRuntimeConfigurationError,
    RuntimeSqliteMode as RustRuntimeSqliteMode, ScalarValue as RustScalarValue,
    SearchExpandResult as RustSearchExpandResult, SearchFilter as RustSearchFilter,
    SearchHit as RustSearchHit, SearchResult as RustSearchResult, SoftFallbackBranch,
    SourceDependencyRegistrationV1, SourceDependencyV1 as RustSourceDependencyV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId,
    StructuralDegradationCodeV1 as RustStructuralDegradationCodeV1,
    StructuralDependencyStateV1 as RustStructuralDependencyStateV1,
    StructuralInclusionStateV1 as RustStructuralInclusionStateV1,
    StructuralInclusionV1 as RustStructuralInclusionV1,
    StructuralLifecycleStateV1 as RustStructuralLifecycleStateV1,
    StructuralProjectionOriginV1 as RustStructuralProjectionOriginV1,
    TraversalDirection as RustTraversalDirection, WriteProvenanceV1,
    WriteReceipt as RustWriteReceipt,
};
pub(crate) use fathomdb_schema::MigrationStepReport as RustMigrationStepReport;
pub(crate) use napi::{Env, Error, JsFunction, JsObject, Result, Status};
pub(crate) use napi_derive::napi;
pub(crate) use serde::Serialize;
pub(crate) use serde_json::{json, Value as JsonValue};
