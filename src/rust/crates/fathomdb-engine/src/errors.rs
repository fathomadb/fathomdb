use super::*;
use fathomdb_schema::MigrationError as SchemaMigrationError;

/// Stable corruption-on-open detail carried by
/// [`EngineOpenError::Corruption`].
///
/// Layout owned by `dev/design/errors.md` § Corruption detail owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorruptionDetail {
    pub kind: CorruptionKind,
    pub stage: OpenStage,
    pub locator: CorruptionLocator,
    pub recovery_hint: RecoveryHint,
}

/// Open-path corruption category.
///
/// Per `dev/design/errors.md` § Engine.open corruption table, doctor-only
/// finding codes are not represented here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorruptionKind {
    WalReplayFailure,
    HeaderMalformed,
    SchemaInconsistent,
    EmbedderIdentityDrift,
    ProjectionGenerationDrift,
}

/// `Engine.open` stage at which corruption was detected.
///
/// Per ADR-0.6.0-corruption-open-behavior, `LockAcquisition` is intentionally
/// not a member here; lock contention is surfaced via
/// [`EngineOpenError::DatabaseLocked`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenStage {
    WalReplay,
    HeaderProbe,
    SchemaProbe,
    EmbedderIdentity,
    ProjectionGeneration,
}

/// Locator pointing at the corrupted region of the database file.
///
/// Variant set owned by `dev/design/errors.md` § CorruptionLocator
/// ownership. `OpaqueSqliteError` is the required fallback when SQLite
/// surfaces corruption without a usable structured locator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorruptionLocator {
    FileOffset { offset: u64 },
    PageId { page: u32 },
    TableRow { table: &'static str, rowid: i64 },
    Vec0ShadowRow { partition: &'static str, rowid: i64 },
    MigrationStep { from: u32, to: u32 },
    OpaqueSqliteError { sqlite_extended_code: i32 },
}

/// Recovery dispatch surface attached to a corruption detail.
///
/// `code` is the stable dispatch key used by bindings and doctor output;
/// `doc_anchor` points at the documentation section that explains the
/// remediation path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryHint {
    pub code: &'static str,
    pub doc_anchor: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineOpenError {
    RuntimeConfiguration(RuntimeConfigurationError),
    /// Invalid per-engine settings, distinct from process-wide SQLite setup.
    EngineConfiguration(EngineConfigurationError),
    DatabaseLocked {
        holder_pid: Option<u32>,
    },
    Corruption(CorruptionDetail),
    IncompatibleSchemaVersion {
        seen: u32,
        supported: u32,
    },
    MigrationError {
        schema_version_before: u32,
        schema_version_current: u32,
        step_id: u32,
    },
    EmbedderIdentityMismatch {
        stored: EmbedderIdentity,
        supplied: EmbedderIdentity,
    },
    EmbedderDimensionMismatch {
        stored: u32,
        supplied: u32,
    },
    /// Embedder runtime returned a typed error during `Engine::open`.
    Embedder(RuntimeEmbedderError),
    /// The default embedder's explicit CPU/CUDA policy could not be honored.
    EmbedDevicePolicy(EmbedDevicePolicyError),
    /// The cross-encoder's independent CPU/CUDA policy could not be honored.
    RerankerDevicePolicy(RerankerDevicePolicyError),
    Io {
        message: String,
    },
}

impl Display for EngineOpenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuntimeConfiguration(error) => error.fmt(f),
            Self::EngineConfiguration(error) => error.fmt(f),
            Self::DatabaseLocked { holder_pid } => match holder_pid {
                Some(pid) => write!(f, "database is locked by process {pid}"),
                None => write!(f, "database is locked by another engine instance"),
            },
            Self::Corruption(detail) => {
                write!(
                    f,
                    "engine corruption at {:?} stage: {}",
                    detail.stage, detail.recovery_hint.code
                )
            }
            Self::IncompatibleSchemaVersion { seen, supported } => write!(
                f,
                "database schema version {seen} is incompatible with supported version {supported}"
            ),
            Self::MigrationError {
                schema_version_before,
                schema_version_current,
                step_id,
            } => write!(
                f,
                "schema migration failed at step {step_id}; schema version remained between {schema_version_before} and {schema_version_current}"
            ),
            Self::EmbedderIdentityMismatch { stored, supplied } => write!(
                f,
                "embedder identity mismatch: stored {}@{}, supplied {}@{}",
                stored.name, stored.revision, supplied.name, supplied.revision,
            ),
            Self::EmbedderDimensionMismatch { stored, supplied } => write!(
                f,
                "embedder vector dimension mismatch: stored {stored}, supplied {supplied}",
            ),
            Self::Embedder(err) => match err {
                RuntimeEmbedderError::Timeout => write!(f, "embedder timeout during open"),
                RuntimeEmbedderError::Failed { message } => {
                    write!(f, "embedder failure during open: {message}")
                }
            },
            Self::EmbedDevicePolicy(error) => error.fmt(f),
            Self::RerankerDevicePolicy(error) => error.fmt(f),
            Self::Io { message } => write!(f, "database I/O error: {message}"),
        }
    }
}

impl Error for EngineOpenError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    Storage,
    Projection,
    Vector,
    Embedder,
    EmbedderNotConfigured,
    /// A forced cross-encoder CUDA policy could not be honored while loading or
    /// running the reranker. CPU fallback is forbidden for this request.
    RerankerDevicePolicy(RerankerDevicePolicyError),
    /// Pending projection work requires an embedder that this session did not
    /// configure. Unlike [`Self::EmbedderNotConfigured`], this is a drain-time
    /// configuration outcome: the accepted write remains durable and can be
    /// completed by a later session with a usable embedder.
    EmbedderRequired(EmbedderRequired),
    KindNotVectorIndexed,
    EmbedderDimensionMismatch {
        expected: u32,
        actual: u32,
    },
    Scheduler,
    OpStore,
    WriteValidation,
    SchemaValidation,
    /// Immutable revision/source provenance validation or ownership refusal.
    Provenance(ProvenanceError),
    /// Immutable source-dependency validation, conflict, or bounded-read refusal.
    Dependency(DependencyError),
    /// Dependency-closure lookup validation refusal.
    DependencyClosure(DependencyClosureError),
    /// Bounded caller-decided actuation request or idempotency refusal.
    Actuation(ActuationError),
    /// A frozen read context was malformed, unauthenticated, addressed to a
    /// different database, or no longer matches the database read state.
    FrozenRead(FrozenReadError),
    /// A governed page request or operational-state selector was invalid.
    Page(PageError),
    /// Projection-generation request or persisted-authority failure.
    ProjectionGeneration(ProjectionGenerationError),
    /// An evidence request was invalid, unavailable, incomplete, or corrupt.
    Evidence(EvidenceErrorV1),
    /// A governed dependency trace request was invalid, unavailable, bounded, or corrupt.
    DependencyTrace(DependencyTraceErrorV1),
    /// A governed constrained graph-expansion request was refused.
    GraphExpansion(GraphExpansionErrorV1),
    /// An operator-only bounded data-plane integrity request failed.
    #[cfg(feature = "operator")]
    DataPlaneIntegrity(DataPlaneIntegrityErrorV1),
    Overloaded,
    Closing,
    /// G11 (Slice 15) — BYO-LLM extractor subprocess error (protocol mismatch,
    /// spawn failure, or harness-returned error code).
    Extractor,
    /// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — BYO-LLM consolidation provider
    /// error (protocol mismatch, spawn/handshake failure, task not advertised in
    /// `supported_tasks`, or a malformed/out-of-cluster verdict). Rides the SAME
    /// `provider_session` transport as `Extractor`; this is the task-specific leaf.
    Consolidator,
    /// G4 (Slice 35) — filter predicate construction error: non-allowlisted
    /// path or invalid filter argument. NOT a panic — returned as a typed error
    /// from [`Predicate::json_path_eq`] / [`Predicate::json_path_compare`].
    InvalidFilter {
        reason: String,
    },
    /// Slice 20 (G5/G6) — an argument is out of the accepted range (e.g.
    /// `depth > 3` for graph traversal). The `msg` field carries a
    /// human-readable explanation; it is intentionally non-exhaustive so the
    /// binding layer can forward it as a `ValueError` / `TypeError`.
    InvalidArgument {
        msg: String,
    },
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe KEYSTONE) — the open-time
    /// self-check re-embedded the 45 committed probes with the live backend and
    /// found a divergence beyond the frozen D4 floor (a Phase-1 mean-centered
    /// `embedding_bin` sign flip, OR a Phase-2 un-centered L2 distance over
    /// `VECTOR_EQUIVALENCE_L2_EPSILON`). `Engine::open` succeeded into a degraded
    /// state (`dense_disabled = true`); this query-time error is raised at the
    /// single choke point [`Engine::search_inner_with_stats`] BEFORE any embedding
    /// / vector SQL / graph seeding / CE rerank, refusing EVERY vector-dependent
    /// arm (`search`, `search_expand`, explain/rerank, graph-arm). The explicit
    /// text-only/FTS-only path ([`Engine::search_text_only`]) stays serviceable.
    /// Sibling of the open-time `EngineOpenError::EmbedderIdentityMismatch`; per
    /// ADR-0.8.18 codex R2 U1-1 the refusal surfaces as an `EngineError` (queries
    /// never surface `EngineOpenError`). `reason` carries a human-readable summary.
    VectorEquivalenceMismatch {
        reason: String,
    },
    /// OPP-12 Phase-1 (0.8.19 Slice 10) — a lifecycle `transition`/`purge` move
    /// that the engine-enforced legal-transition table (design §2) forbids.
    /// Raised for an illegal `transition` target (`purged`/`pending` are never
    /// `transition` targets; self-loops; a from→to pair not in the table) AND for
    /// a `purge` precondition failure (purge is legal only from `deleted`).
    /// `from_state`/`to_state` use the FULL, parity-safe field names (S7 — `from`
    /// is a Python reserved word); `legal` enumerates the target states reachable
    /// from `from_state` in the full state machine.
    IllegalTransition {
        from_state: LifecycleState,
        to_state: LifecycleState,
        legal: Vec<LifecycleState>,
    },
    /// OPP-12 Phase-1 (0.8.19 Slice 10) — a lifecycle verb (`transition`/`purge`)
    /// was addressed with a non-`Logical` id space (a `Content`/`h:` doc-seeded or
    /// `Passage`/`p:` synthetic id). Only the `Logical` (`l:`) space is
    /// lifecycle-addressable (design §3); this is a typed refusal, never a panic
    /// or a silent no-op. `id_space` carries the offending [`IdSpaceKind`].
    NotLifecycleAddressable {
        id_space: IdSpaceKind,
    },
    /// 0.8.20 Slice 5b (R-20-E5, design `0.8.20-slice0-erasure-design.md` §4
    /// item 4) — an erasure verb (`purge` / `excise_source` /
    /// `excise_collection_record`) deleted its rows but could NOT complete the
    /// erasure **at rest**, so it refuses to report success.
    ///
    /// The motivating case is the write-ahead log. `PRAGMA secure_delete=ON`
    /// zeroes pages freed inside the database file, but the erased content also
    /// sits in the WAL as committed frames from the ORIGINAL insert: an erasure
    /// DELETE appends new frames, it never rewrites old ones. Only a
    /// `wal_checkpoint(TRUNCATE)` removes them, and a concurrent reader pinning a
    /// WAL snapshot makes that checkpoint return `busy`. After a bounded retry
    /// the verb raises THIS error rather than returning `Ok` over erased bytes
    /// that are still `grep`-able on disk.
    ///
    /// **Contract: an erasure verb must never report success on an incomplete
    /// erasure.** The row deletions are committed and durable when this is
    /// raised; what failed is the at-rest scrub. The remedy is to retry the verb
    /// (or `recover --truncate-wal`) once the blocking reader has finished.
    /// `stage` names the uncompleted step (e.g. `"wal_checkpoint"`,
    /// `"telemetry_redaction"`); `detail` is a human-readable summary.
    ErasureIncomplete {
        stage: String,
        detail: String,
    },
    /// 0.8.20 Slice 15d (R-20-PR) — `configure_projections` refused an
    /// incompatible/DESTRUCTIVE change to an existing projection `name` that was
    /// NOT accompanied by an explicit `drop`. Omission from the spec never drops
    /// (C3, `api-surface.md:27`); a role REMOVAL or a tokenizer/embedder change
    /// on a live projection would silently discard an expensive-to-rebuild
    /// resource, so it is refused with the destructive `delta` surfaced. The
    /// caller re-issues with `drop: [name]` to consciously rebuild.
    ProjectionDestructive {
        name: String,
        delta: String,
    },
}

impl From<ProvenanceError> for EngineError {
    fn from(error: ProvenanceError) -> Self {
        Self::Provenance(error)
    }
}

impl From<DependencyError> for EngineError {
    fn from(error: DependencyError) -> Self {
        Self::Dependency(error)
    }
}

impl From<DependencyClosureError> for EngineError {
    fn from(error: DependencyClosureError) -> Self {
        Self::DependencyClosure(error)
    }
}

impl From<FrozenReadError> for EngineError {
    fn from(error: FrozenReadError) -> Self {
        Self::FrozenRead(error)
    }
}

impl From<PageError> for EngineError {
    fn from(error: PageError) -> Self {
        Self::Page(error)
    }
}

impl From<ProjectionGenerationError> for EngineError {
    fn from(error: ProjectionGenerationError) -> Self {
        Self::ProjectionGeneration(error)
    }
}

impl From<EvidenceErrorV1> for EngineError {
    fn from(error: EvidenceErrorV1) -> Self {
        Self::Evidence(error)
    }
}

impl From<DependencyTraceErrorV1> for EngineError {
    fn from(error: DependencyTraceErrorV1) -> Self {
        Self::DependencyTrace(error)
    }
}

impl From<GraphExpansionErrorV1> for EngineError {
    fn from(error: GraphExpansionErrorV1) -> Self {
        Self::GraphExpansion(error)
    }
}

#[cfg(feature = "operator")]
impl From<DataPlaneIntegrityErrorV1> for EngineError {
    fn from(error: DataPlaneIntegrityErrorV1) -> Self {
        Self::DataPlaneIntegrity(error)
    }
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage => write!(f, "storage error"),
            Self::Projection => write!(f, "projection error"),
            Self::Vector => write!(f, "vector error"),
            Self::Embedder => write!(f, "embedder error"),
            Self::EmbedderNotConfigured => write!(f, "embedder is not configured"),
            Self::RerankerDevicePolicy(error) => error.fmt(f),
            Self::EmbedderRequired(required) => write!(
                f,
                "{} requires a configured embedder; see {}",
                required.operation.as_str(),
                required.documentation_url
            ),
            Self::KindNotVectorIndexed => write!(f, "kind is not configured for vector indexing"),
            Self::EmbedderDimensionMismatch { expected, actual } => {
                write!(f, "embedder dimension mismatch: expected {expected}, actual {actual}")
            }
            Self::Scheduler => write!(f, "scheduler error"),
            Self::OpStore => write!(f, "op-store error"),
            Self::WriteValidation => write!(f, "write validation error"),
            Self::SchemaValidation => write!(f, "schema validation error"),
            Self::Provenance(error) => write!(f, "provenance: {error}"),
            Self::Dependency(error) => write!(f, "dependency: {error}"),
            Self::DependencyClosure(error) => write!(f, "dependency closure: {error}"),
            Self::Actuation(error) => write!(f, "actuation: {error}"),
            Self::FrozenRead(error) => write!(f, "frozen read: {error}"),
            Self::Page(error) => write!(f, "page: {error}"),
            Self::ProjectionGeneration(error) => write!(f, "projection generation: {error}"),
            Self::Evidence(error) => write!(f, "evidence: {error}"),
            Self::DependencyTrace(error) => write!(f, "dependency trace: {error}"),
            Self::GraphExpansion(error) => write!(f, "graph expansion: {error}"),
            #[cfg(feature = "operator")]
            Self::DataPlaneIntegrity(error) => write!(f, "data-plane integrity: {error}"),
            Self::Overloaded => write!(f, "engine overloaded"),
            Self::Closing => write!(f, "engine is closing"),
            Self::Extractor => write!(f, "extractor error"),
            Self::Consolidator => write!(f, "consolidator error"),
            Self::InvalidFilter { reason } => write!(f, "invalid filter: {reason}"),
            Self::InvalidArgument { msg } => write!(f, "invalid argument: {msg}"),
            Self::VectorEquivalenceMismatch { reason } => {
                write!(f, "vector-equivalence self-check failed; dense retrieval refused: {reason}")
            }
            Self::IllegalTransition { from_state, to_state, legal } => {
                let legal_list = legal.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                write!(
                    f,
                    "illegal lifecycle transition {} -> {}; legal targets from {}: [{}]",
                    from_state.as_str(),
                    to_state.as_str(),
                    from_state.as_str(),
                    legal_list,
                )
            }
            Self::NotLifecycleAddressable { id_space } => write!(
                f,
                "id space {:?} ({}) is not lifecycle-addressable; only the logical (l:) space is",
                id_space,
                id_space.prefix(),
            ),
            Self::ErasureIncomplete { stage, detail } => write!(
                f,
                "erasure incomplete at stage '{stage}': the rows were deleted but the erasure \
                 could not be completed at rest ({detail})",
            ),
            Self::ProjectionDestructive { name, delta } => write!(
                f,
                "configure_projections refused a destructive change to projection '{name}' \
                 without an explicit drop ({delta}); re-issue with drop: [\"{name}\"] to rebuild",
            ),
        }
    }
}

impl EngineError {
    /// Stable machine-readable code for `errors_by_code` keys.
    ///
    /// Names match the binding-facing class stems in
    /// `dev/design/errors.md` § Binding-facing class matrix.
    pub(crate) fn stable_code(&self) -> &'static str {
        match self {
            Self::Storage => "StorageError",
            Self::Projection => "ProjectionError",
            Self::Vector => "VectorError",
            Self::Embedder => "EmbedderError",
            Self::EmbedderNotConfigured => "EmbedderNotConfiguredError",
            Self::RerankerDevicePolicy(_) => "RerankerDevicePolicyError",
            Self::EmbedderRequired(_) => "EmbedderRequiredError",
            Self::KindNotVectorIndexed => "KindNotVectorIndexedError",
            Self::EmbedderDimensionMismatch { .. } => "EmbedderDimensionMismatchError",
            Self::Scheduler => "SchedulerError",
            Self::OpStore => "OpStoreError",
            Self::WriteValidation => "WriteValidationError",
            Self::SchemaValidation => "SchemaValidationError",
            Self::Provenance(_) => "ProvenanceError",
            Self::Dependency(_) => "DependencyError",
            Self::DependencyClosure(_) => "DependencyClosureError",
            Self::Actuation(_) => "ActuationError",
            Self::FrozenRead(_) => "FrozenReadError",
            Self::Page(_) => "PageError",
            Self::ProjectionGeneration(_) => "ProjectionGenerationError",
            Self::Evidence(_) => "EvidenceError",
            Self::DependencyTrace(_) => "DependencyTraceError",
            Self::GraphExpansion(_) => "GraphExpansionError",
            #[cfg(feature = "operator")]
            Self::DataPlaneIntegrity(_) => "DataPlaneIntegrityError",
            Self::Overloaded => "OverloadedError",
            Self::Closing => "ClosingError",
            Self::Extractor => "ExtractorError",
            Self::Consolidator => "ConsolidatorError",
            Self::InvalidFilter { .. } => "InvalidFilterError",
            Self::InvalidArgument { .. } => "InvalidArgumentError",
            Self::VectorEquivalenceMismatch { .. } => "VectorEquivalenceMismatchError",
            Self::IllegalTransition { .. } => "IllegalTransitionError",
            Self::NotLifecycleAddressable { .. } => "NotLifecycleAddressableError",
            Self::ErasureIncomplete { .. } => "ErasureIncompleteError",
            Self::ProjectionDestructive { .. } => "ProjectionDestructiveError",
        }
    }
}

impl Error for EngineError {}

pub(crate) fn map_migration_error(err: SchemaMigrationError) -> EngineOpenError {
    match err {
        SchemaMigrationError::IncompatibleSchemaVersion { seen, supported } => {
            EngineOpenError::IncompatibleSchemaVersion { seen, supported }
        }
        SchemaMigrationError::MigrationError(report) => EngineOpenError::MigrationError {
            schema_version_before: report.schema_version_before,
            schema_version_current: report.schema_version_current,
            step_id: report.migration_steps.last().map_or(0, |step| step.step_id),
        },
        SchemaMigrationError::Storage { message } => {
            EngineOpenError::Io { message: message.to_string() }
        }
    }
}

pub(crate) fn map_open_sqlite_error(err: rusqlite::Error, stage: OpenStage) -> EngineOpenError {
    let Some(sqlite_error) = err.sqlite_error() else {
        return EngineOpenError::Io { message: "could not open database".to_string() };
    };
    match sqlite_error.extended_code {
        rusqlite::ffi::SQLITE_CORRUPT | rusqlite::ffi::SQLITE_NOTADB => {
            EngineOpenError::Corruption(CorruptionDetail {
                kind: match stage {
                    OpenStage::WalReplay => CorruptionKind::WalReplayFailure,
                    OpenStage::HeaderProbe => CorruptionKind::HeaderMalformed,
                    OpenStage::SchemaProbe => CorruptionKind::SchemaInconsistent,
                    OpenStage::EmbedderIdentity => CorruptionKind::EmbedderIdentityDrift,
                    OpenStage::ProjectionGeneration => CorruptionKind::ProjectionGenerationDrift,
                },
                stage,
                locator: CorruptionLocator::OpaqueSqliteError {
                    sqlite_extended_code: sqlite_error.extended_code,
                },
                recovery_hint: RecoveryHint {
                    code: match stage {
                        OpenStage::WalReplay => "E_CORRUPT_WAL_REPLAY",
                        OpenStage::HeaderProbe => "E_CORRUPT_HEADER",
                        OpenStage::SchemaProbe => "E_CORRUPT_SCHEMA",
                        OpenStage::EmbedderIdentity => "E_CORRUPT_EMBEDDER_IDENTITY",
                        OpenStage::ProjectionGeneration => "E_CORRUPT_PROJECTION_GENERATION",
                    },
                    doc_anchor: match stage {
                        OpenStage::WalReplay => "design/recovery.md#wal-replay-failures",
                        OpenStage::HeaderProbe => "design/recovery.md#header-malformed",
                        OpenStage::SchemaProbe => "design/recovery.md#schema-inconsistent",
                        OpenStage::EmbedderIdentity => "design/recovery.md#embedder-identity-drift",
                        OpenStage::ProjectionGeneration => {
                            "design/recovery-0.8.25.md#projection-generation"
                        }
                    },
                },
            })
        }
        _ => EngineOpenError::Io { message: "could not open database".to_string() },
    }
}
