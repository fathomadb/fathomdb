//! Slice 132 AC27-132B/C — the `fathomdb_sdk` public surface, pinned by type.
//!
//! Every governed operation is bound to an explicit function-pointer type, so a
//! missing operation, a changed argument, or a changed return type fails to
//! compile. Signatures follow `dev/interfaces/rust-sdk.md`. This consumer
//! depends on `fathomdb_sdk` alone; it never names `fathomdb_engine`.

use std::sync::Arc;

use fathomdb_sdk::{
    admin, embed_batch_cls, graph, read, rerank, ActuationBatchV1, ActuationReceiptV1,
    BoundaryCrossing, ClosureLookupV1, ClosureStatusV1, ConsolidateAxis, ConsolidateReceipt,
    CounterSnapshot, DependencyDerivedLookupV1, DependencyListV1, DependencySourceLookupV1,
    DependencyTraceRequestV1, DependencyTraceResultV1, DeviceResolution, EmbedderEvent,
    EmbedderIdentity, EmbeddingReadiness, Engine, EngineConfig, EngineError, EngineOpenError,
    EraseReport, Error, ErrorKind, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    EvidenceSearchResultV1, ExtractDocument, Filter, FilterTerm, FrozenReadContextV1,
    FrozenSearchOptions, GpuAllocationWitness, GraphEvidenceResolveRequestV1, GraphExpandRequestV1,
    GraphExpandResultV1, IdSpace, IdSpaceKind, IngestWithExtractorReceipt, LifecycleState,
    ListOptions, MigrationStepReport, MutationProjectionStatusRequestV1,
    MutationProjectionStatusV1, NeighborsOptions, NodeRecord, OpStoreRow, OpenOptions, OpenReport,
    OperationalStateRecordV1, PageRequestV1, PageV1, Predicate, PreparedWrite,
    ProjectedTextSearchOptions, ProjectionDelta, ProjectionGenerationStatusV1,
    ProjectionRuntimeStatus, ProjectionSpec, ReadContextV1, ReadView, RerankOptions, RerankPassage,
    RerankResult, RerankerDeviceResolution, ResolvedEvidenceV1, ResolvedGraphEvidenceV1, Result,
    RuntimeConfiguration, RuntimeConfigurationError, RuntimeSqliteMode, SearchExpandOptions,
    SearchExpandResult, SearchFilter, SearchFilterArg, SearchHit, SearchOptions, SearchResult,
    SourceDependencyRegistrationV1, SourceDependencyV1, SourceId, Subscriber, SubscriberEvent,
    TextSearchOptions, TraversalDirection, WriteReceipt,
};

#[test]
fn engine_root_and_lifecycle_signatures() {
    let _: fn(&std::path::Path, OpenOptions) -> Result<Engine> = Engine::open::<&std::path::Path>;
    let _: fn(&Engine) -> &OpenReport = Engine::open_report;
    let _: fn(&Engine) -> &EngineConfig = Engine::config;
    let _: fn(&Engine) -> Result<()> = Engine::close;
    let _: fn(&Engine, u64) -> Result<()> = Engine::drain;
}

#[test]
fn engine_write_and_lifecycle_command_signatures() {
    let _: fn(&Engine, &[PreparedWrite]) -> Result<WriteReceipt> = Engine::write;
    let _: fn(&Engine, ActuationBatchV1) -> Result<ActuationReceiptV1> = Engine::actuate;
    let _: fn(&Engine, SourceDependencyRegistrationV1) -> Result<SourceDependencyV1> =
        Engine::register_source_dependency;
    let _: fn(&Engine, DependencySourceLookupV1) -> Result<DependencyListV1> =
        Engine::dependencies_for_source;
    let _: fn(&Engine, DependencyDerivedLookupV1) -> Result<Option<SourceDependencyV1>> =
        Engine::dependency_for_derived;
    let _: fn(&Engine, ClosureLookupV1) -> Result<Option<ClosureStatusV1>> =
        Engine::read_dependency_closure;
    let _: fn(&Engine, DependencyTraceRequestV1) -> Result<DependencyTraceResultV1> =
        Engine::trace_dependency;
    let _: fn(&Engine, &str, LifecycleState, Option<&str>) -> Result<()> = Engine::transition;
    let _: fn(&Engine, &str) -> Result<()> = Engine::purge;
    let _: fn(&Engine, &str) -> Result<EraseReport> = Engine::erase_source;
    let _: fn(&Engine, &[ProjectionSpec], &[String]) -> Result<ProjectionDelta> =
        Engine::configure_projections;
    let _: fn(&Engine, &[&str], &[ExtractDocument]) -> Result<IngestWithExtractorReceipt> =
        Engine::ingest_with_extractor;
    let _: fn(&Engine, &[&str], &[ConsolidateAxis]) -> Result<ConsolidateReceipt> =
        Engine::consolidate_with_provider;
    let _: fn(&Engine, &str) -> Result<Vec<f32>> = Engine::embed;
}

#[test]
fn engine_search_signatures() {
    let _: fn(&Engine, &str, SearchOptions) -> Result<SearchResult> = Engine::search;
    let _: fn(&Engine, &str, TextSearchOptions) -> Result<SearchResult> = Engine::search_text_only;
    let _: fn(&Engine, &str, &str, ProjectedTextSearchOptions) -> Result<SearchResult> =
        Engine::search_projected_text;
    let _: fn(&Engine, &ReadContextV1) -> Result<FrozenReadContextV1> = Engine::freeze_read_context;
    let _: fn(&Engine, &str, &FrozenReadContextV1, FrozenSearchOptions) -> Result<SearchResult> =
        Engine::search_frozen;
    let _: fn(
        &Engine,
        &str,
        &FrozenReadContextV1,
        u32,
        SearchExpandOptions,
    ) -> Result<SearchExpandResult> = Engine::search_expand_frozen;
    let _: fn(&Engine, &EvidenceSearchRequestV1) -> Result<EvidenceSearchResultV1> =
        Engine::search_with_evidence;
    let _: fn(&Engine, &EvidenceResolveRequestV1) -> Result<ResolvedEvidenceV1> =
        Engine::resolve_evidence;
    let _: fn(&Engine, &GraphEvidenceResolveRequestV1) -> Result<ResolvedGraphEvidenceV1> =
        Engine::resolve_graph_evidence;
}

#[test]
fn engine_instrumentation_signatures() {
    let _: fn(&Engine) -> bool = Engine::dense_disabled;
    let _: fn(&Engine) -> Option<String> = Engine::dense_disabled_reason;
    let _: fn(&Engine) -> u64 = Engine::vector_equivalence_refusal_count;
    let _: fn(&Engine, &str) -> Result<()> = Engine::enable_telemetry;
    let _: fn(&Engine) -> Option<String> = Engine::last_telemetry_query_id;
    let _: fn(&Engine, &str, &[u64], &[u64], &str) -> Result<()> = Engine::record_feedback;
    let _: fn(&Engine) -> CounterSnapshot = Engine::counters;
    let _: fn(&Engine, bool) = Engine::set_profiling;
    let _: fn(&Engine, u64) = Engine::set_slow_threshold_ms;
    let _: fn(&Engine, Arc<dyn Subscriber>) -> Result<()> = Engine::attach_subscriber;
}

#[test]
fn read_namespace_signatures() {
    let _: fn(&Engine, &str, Option<&ReadView>) -> Result<Option<NodeRecord>> = read::get;
    let _: fn(&Engine, &[String], Option<&ReadView>) -> Result<Vec<Option<NodeRecord>>> =
        read::get_many;
    let _: fn(&Engine, &str, Option<i64>, usize) -> Result<Vec<OpStoreRow>> = read::collection;
    let _: fn(&Engine, &str, Option<i64>, usize) -> Result<Vec<OpStoreRow>> = read::mutations;
    let _: fn(&Engine, &str, ListOptions) -> Result<Vec<NodeRecord>> = read::list;
    let _: fn(&Engine, &str, &FrozenReadContextV1, &PageRequestV1) -> Result<PageV1<NodeRecord>> =
        read::canonical_page;
    let _: fn(
        &Engine,
        &str,
        &str,
        Option<&FrozenReadContextV1>,
    ) -> Result<Option<OperationalStateRecordV1>> = read::operational_state;
    let _: fn(
        &Engine,
        &str,
        &FrozenReadContextV1,
        &PageRequestV1,
    ) -> Result<PageV1<OperationalStateRecordV1>> = read::operational_state_page;
    let _: fn(&Engine, i64, Option<&ReadView>) -> Result<Vec<BoundaryCrossing>> =
        read::crossed_boundary_since;
    let _: fn(&Engine) -> Result<Vec<ProjectionSpec>> = read::projections;
    let _: fn(&Engine) -> Result<ProjectionRuntimeStatus> = read::projection_status;
    let _: fn(&Engine) -> Result<ProjectionGenerationStatusV1> = read::projection_generation_status;
    let _: fn(&Engine, MutationProjectionStatusRequestV1) -> Result<MutationProjectionStatusV1> =
        read::mutation_projection_status;
    let _: fn(&Engine) -> Result<EmbeddingReadiness> = read::embedding_readiness;
}

#[test]
fn graph_admin_and_standalone_signatures() {
    let _: fn(&Engine, &GraphExpandRequestV1) -> Result<GraphExpandResultV1> = graph::expand;
    let _: fn(&Engine, &str, u32, NeighborsOptions) -> Result<Vec<NodeRecord>> = graph::neighbors;
    let _: fn(
        &Engine,
        &str,
        u32,
        Option<SearchFilter>,
        SearchExpandOptions,
    ) -> Result<SearchExpandResult> = graph::search_expand;
    let _: fn(&Engine, &str, &str) -> Result<WriteReceipt> = admin::configure;
    let _: fn(RuntimeSqliteMode) -> Result<RuntimeConfiguration> = admin::configure_runtime;
    let _: fn(&str, &[RerankPassage], usize, RerankOptions) -> Result<Vec<RerankResult>> = rerank;
    let _: fn(&[&str]) -> Result<Vec<Vec<f32>>> = embed_batch_cls;
}

/// Defaults equal the Python/TypeScript documented defaults.
#[test]
fn option_defaults_match_python_and_typescript() {
    let search = SearchOptions::default();
    assert_eq!(search.filter, None);
    assert_eq!(search.rerank_depth, 0);
    assert!(!search.use_graph_arm);
    assert_eq!(search.alpha, None);
    assert_eq!(search.pool_n, None);
    assert!(!search.explain);
    assert_eq!(search.view, ReadView::default());
    assert_eq!(search.limit, 10);

    let text = TextSearchOptions::default();
    assert_eq!((text.view, text.limit), (ReadView::default(), 10));

    let projected = ProjectedTextSearchOptions::default();
    assert_eq!(projected.filter, None);
    assert_eq!(projected.limit, 10);

    let frozen = FrozenSearchOptions::default();
    assert_eq!(frozen.rerank_depth, 0);
    assert!(!frozen.use_graph_arm);
    assert_eq!(frozen.alpha, None);
    assert_eq!(frozen.pool_n, None);
    assert!(!frozen.explain);
    assert_eq!(frozen.limit, 10);

    assert_eq!(SearchExpandOptions::default().search_limit, 10);

    let list = ListOptions::default();
    assert!(list.predicates.is_empty());
    assert_eq!(list.filter, None);
    assert_eq!(list.limit, 100);
    assert_eq!(list.view, None);

    let neighbors = NeighborsOptions::default();
    assert_eq!(neighbors.direction, TraversalDirection::Both);
    assert_eq!(neighbors.view, None);

    let rerank = RerankOptions::default();
    assert_eq!((rerank.alpha, rerank.pool_n), (None, None));

    let open = OpenOptions::default();
    assert_eq!(open.config, EngineConfig::default());
    assert!(!open.use_default_embedder);
}

#[test]
fn search_filter_arg_accepts_both_filter_forms() {
    let search: SearchFilterArg = SearchFilter::default().into();
    assert!(matches!(search, SearchFilterArg::Search(_)));
    let unified: SearchFilterArg = Filter::default().into();
    assert!(matches!(unified, SearchFilterArg::Unified(_)));
}

/// The shared Python/TypeScript error classes, minus the `Error` suffix, plus
/// the base. Python: `fathomdb.errors`; TypeScript: `src/ts/src/errors.ts`.
const SHARED_ERROR_CLASSES: [&str; 42] = [
    "EngineError",
    "RuntimeConfigurationError",
    "StorageError",
    "ProjectionError",
    "ProjectionGenerationError",
    "VectorError",
    "KindNotVectorIndexedError",
    "EmbedderError",
    "EmbedDevicePolicyError",
    "RerankerDevicePolicyError",
    "EmbedderNotConfiguredError",
    "EmbedderRequiredError",
    "SchedulerError",
    "OpStoreError",
    "WriteValidationError",
    "SchemaValidationError",
    "ProvenanceError",
    "DependencyError",
    "DependencyClosureError",
    "ActuationError",
    "OverloadedError",
    "ClosingError",
    "DatabaseLockedError",
    "CorruptionError",
    "IncompatibleSchemaVersionError",
    "MigrationError",
    "EmbedderIdentityMismatchError",
    "EmbedderDimensionMismatchError",
    "ExtractorError",
    "ConsolidatorError",
    "InvalidFilterError",
    "FrozenReadError",
    "EvidenceError",
    "PageError",
    "DependencyTraceError",
    "GraphExpansionError",
    "VectorEquivalenceMismatchError",
    "InvalidArgumentError",
    "IllegalTransitionError",
    "NotLifecycleAddressableError",
    "ErasureIncompleteError",
    "ProjectionDestructiveError",
];

#[test]
fn error_kinds_equal_shared_error_classes() {
    let names: Vec<&str> = ErrorKind::ALL.iter().map(|kind| kind.name()).collect();
    assert_eq!(names, SHARED_ERROR_CLASSES);
}

#[test]
fn error_kind_hierarchy_matches_python_and_typescript() {
    for kind in ErrorKind::ALL {
        let expected = match kind {
            ErrorKind::EmbedDevicePolicy
            | ErrorKind::RerankerDevicePolicy
            | ErrorKind::EmbedderNotConfigured
            | ErrorKind::EmbedderRequired => Some(ErrorKind::Embedder),
            ErrorKind::KindNotVectorIndexed => Some(ErrorKind::Vector),
            _ => None,
        };
        assert_eq!(kind.parent(), expected, "{kind:?}");
    }
}

/// Typed payload carriers stay reachable for callers that inspect them.
#[test]
fn error_wraps_typed_core_payloads() {
    let error: Error = EngineError::Closing.into();
    assert_eq!(error.kind(), ErrorKind::Closing);
    let error: Error = EngineOpenError::DatabaseLocked { holder_pid: Some(7) }.into();
    assert_eq!(error.kind(), ErrorKind::DatabaseLocked);
    let error: Error = RuntimeConfigurationError::TooLate.into();
    assert_eq!(error.kind(), ErrorKind::RuntimeConfiguration);
    let _: &dyn std::error::Error = &error;
}

/// Shared DTO names a consumer must be able to name without the engine crate.
#[test]
fn shared_types_are_nameable() {
    fn named<T: ?Sized>() {}
    named::<SearchHit>();
    named::<IdSpace>();
    named::<IdSpaceKind>();
    named::<FilterTerm>();
    named::<Predicate>();
    named::<SourceId>();
    named::<SubscriberEvent>();
    named::<MigrationStepReport>();
    named::<EmbedderIdentity>();
    named::<EmbedderEvent>();
    named::<DeviceResolution>();
    named::<RerankerDeviceResolution>();
    named::<GpuAllocationWitness>();
}
