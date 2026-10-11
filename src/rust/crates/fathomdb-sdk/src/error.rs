use std::fmt;

use fathomdb_engine::{EngineError, EngineOpenError, RuntimeConfigurationError};

/// The single error type returned by every `fathomdb_sdk` call.
///
/// Core failures keep their typed engine payloads (`holder_pid`, `legal`,
/// field paths, ...). [`Error::kind`] names the error class shared with the
/// Python and TypeScript SDKs.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A core engine failure.
    Engine(EngineError),
    /// A failure while opening a database.
    Open(EngineOpenError),
    /// A process-level runtime configuration refusal.
    RuntimeConfiguration(RuntimeConfigurationError),
    /// A failure raised by the SDK itself (argument validation, the string
    /// transport guard) or by a component outside the engine (reranker,
    /// CLS embedder).
    Sdk { kind: ErrorKind, message: String },
}

/// `Result` alias used by every `fathomdb_sdk` call.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn sdk(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::Sdk { kind, message: message.into() }
    }

    pub(crate) fn invalid_argument(message: impl Into<String>) -> Self {
        Self::sdk(ErrorKind::InvalidArgument, message)
    }

    /// The shared Python/TypeScript error class of this error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Engine(error) => engine_kind(error),
            Self::Open(error) => open_kind(error),
            Self::RuntimeConfiguration(_) => ErrorKind::RuntimeConfiguration,
            Self::Sdk { kind, .. } => *kind,
        }
    }
}

// Mirrors `engine_error_to_py` in `fathomdb-py/src/errors.rs`.
fn engine_kind(error: &EngineError) -> ErrorKind {
    #[allow(unreachable_patterns)]
    match error {
        EngineError::Storage => ErrorKind::Storage,
        EngineError::Projection => ErrorKind::Projection,
        EngineError::ProjectionGeneration(_) => ErrorKind::ProjectionGeneration,
        EngineError::Vector => ErrorKind::Vector,
        EngineError::Embedder => ErrorKind::Embedder,
        EngineError::RerankerDevicePolicy(_) => ErrorKind::RerankerDevicePolicy,
        EngineError::EmbedderNotConfigured => ErrorKind::EmbedderNotConfigured,
        EngineError::EmbedderRequired(_) => ErrorKind::EmbedderRequired,
        EngineError::KindNotVectorIndexed => ErrorKind::KindNotVectorIndexed,
        EngineError::EmbedderDimensionMismatch { .. } => ErrorKind::EmbedderDimensionMismatch,
        EngineError::Scheduler => ErrorKind::Scheduler,
        EngineError::OpStore => ErrorKind::OpStore,
        EngineError::WriteValidation => ErrorKind::WriteValidation,
        EngineError::SchemaValidation => ErrorKind::SchemaValidation,
        EngineError::Provenance(_) => ErrorKind::Provenance,
        EngineError::Dependency(_) => ErrorKind::Dependency,
        EngineError::DependencyClosure(_) => ErrorKind::DependencyClosure,
        EngineError::Actuation(_) => ErrorKind::Actuation,
        EngineError::Overloaded => ErrorKind::Overloaded,
        EngineError::Closing => ErrorKind::Closing,
        EngineError::Extractor => ErrorKind::Extractor,
        EngineError::Consolidator => ErrorKind::Consolidator,
        EngineError::InvalidFilter { .. } => ErrorKind::InvalidFilter,
        EngineError::FrozenRead(_) => ErrorKind::FrozenRead,
        EngineError::Evidence(_) => ErrorKind::Evidence,
        EngineError::DependencyTrace(_) => ErrorKind::DependencyTrace,
        EngineError::GraphExpansion(_) => ErrorKind::GraphExpansion,
        EngineError::Page(_) => ErrorKind::Page,
        EngineError::InvalidArgument { .. } => ErrorKind::InvalidArgument,
        EngineError::VectorEquivalenceMismatch { .. } => ErrorKind::VectorEquivalenceMismatch,
        EngineError::IllegalTransition { .. } => ErrorKind::IllegalTransition,
        EngineError::NotLifecycleAddressable { .. } => ErrorKind::NotLifecycleAddressable,
        EngineError::ErasureIncomplete { .. } => ErrorKind::ErasureIncomplete,
        EngineError::ProjectionDestructive { .. } => ErrorKind::ProjectionDestructive,
        // Variants compiled only with the engine's `operator` feature (which
        // workspace builds unify on) surface as the base class, as in Python.
        _ => ErrorKind::Engine,
    }
}

// Mirrors `engine_open_error_to_py` in `fathomdb-py/src/errors.rs`.
fn open_kind(error: &EngineOpenError) -> ErrorKind {
    match error {
        EngineOpenError::RuntimeConfiguration(_) => ErrorKind::RuntimeConfiguration,
        EngineOpenError::EngineConfiguration(_) => ErrorKind::InvalidArgument,
        EngineOpenError::DatabaseLocked { .. } => ErrorKind::DatabaseLocked,
        EngineOpenError::Corruption(_) => ErrorKind::Corruption,
        EngineOpenError::IncompatibleSchemaVersion { .. } => ErrorKind::IncompatibleSchemaVersion,
        EngineOpenError::MigrationError { .. } => ErrorKind::Migration,
        EngineOpenError::EmbedderIdentityMismatch { .. } => ErrorKind::EmbedderIdentityMismatch,
        EngineOpenError::EmbedderDimensionMismatch { .. } => ErrorKind::EmbedderDimensionMismatch,
        EngineOpenError::Embedder(_) => ErrorKind::Embedder,
        EngineOpenError::EmbedDevicePolicy(_) => ErrorKind::EmbedDevicePolicy,
        EngineOpenError::RerankerDevicePolicy(_) => ErrorKind::RerankerDevicePolicy,
        EngineOpenError::Io { .. } => ErrorKind::Storage,
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(error) => write!(f, "{}: {error}", self.kind().name()),
            Self::Open(error) => write!(f, "{}: {error}", self.kind().name()),
            Self::RuntimeConfiguration(error) => write!(f, "{}: {error}", self.kind().name()),
            Self::Sdk { kind, message } => write!(f, "{}: {message}", kind.name()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            Self::Open(error) => Some(error),
            Self::RuntimeConfiguration(error) => Some(error),
            Self::Sdk { .. } => None,
        }
    }
}

impl From<EngineError> for Error {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

impl From<EngineOpenError> for Error {
    fn from(error: EngineOpenError) -> Self {
        Self::Open(error)
    }
}

impl From<RuntimeConfigurationError> for Error {
    fn from(error: RuntimeConfigurationError) -> Self {
        Self::RuntimeConfiguration(error)
    }
}

macro_rules! error_kinds {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// One variant per error class shared by the Python and TypeScript
        /// SDKs (class name without the `Error` suffix), plus [`ErrorKind::Engine`]
        /// for the base class.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[non_exhaustive]
        pub enum ErrorKind {
            $(#[doc = concat!("`", $name, "`")] $variant,)+
        }

        impl ErrorKind {
            /// Every kind, in the shared class-matrix order.
            pub const ALL: &'static [ErrorKind] = &[$(ErrorKind::$variant,)+];

            /// The shared class name. The base is Python's `EngineError`
            /// (TypeScript: `FathomDbError`).
            #[must_use]
            pub fn name(self) -> &'static str {
                match self {
                    $(ErrorKind::$variant => $name,)+
                }
            }
        }
    };
}

error_kinds! {
    Engine => "EngineError",
    RuntimeConfiguration => "RuntimeConfigurationError",
    Storage => "StorageError",
    Projection => "ProjectionError",
    ProjectionGeneration => "ProjectionGenerationError",
    Vector => "VectorError",
    KindNotVectorIndexed => "KindNotVectorIndexedError",
    Embedder => "EmbedderError",
    EmbedDevicePolicy => "EmbedDevicePolicyError",
    RerankerDevicePolicy => "RerankerDevicePolicyError",
    EmbedderNotConfigured => "EmbedderNotConfiguredError",
    EmbedderRequired => "EmbedderRequiredError",
    Scheduler => "SchedulerError",
    OpStore => "OpStoreError",
    WriteValidation => "WriteValidationError",
    SchemaValidation => "SchemaValidationError",
    Provenance => "ProvenanceError",
    Dependency => "DependencyError",
    DependencyClosure => "DependencyClosureError",
    Actuation => "ActuationError",
    Overloaded => "OverloadedError",
    Closing => "ClosingError",
    DatabaseLocked => "DatabaseLockedError",
    Corruption => "CorruptionError",
    IncompatibleSchemaVersion => "IncompatibleSchemaVersionError",
    Migration => "MigrationError",
    EmbedderIdentityMismatch => "EmbedderIdentityMismatchError",
    EmbedderDimensionMismatch => "EmbedderDimensionMismatchError",
    Extractor => "ExtractorError",
    Consolidator => "ConsolidatorError",
    InvalidFilter => "InvalidFilterError",
    FrozenRead => "FrozenReadError",
    Evidence => "EvidenceError",
    Page => "PageError",
    DependencyTrace => "DependencyTraceError",
    GraphExpansion => "GraphExpansionError",
    VectorEquivalenceMismatch => "VectorEquivalenceMismatchError",
    InvalidArgument => "InvalidArgumentError",
    IllegalTransition => "IllegalTransitionError",
    NotLifecycleAddressable => "NotLifecycleAddressableError",
    ErasureIncomplete => "ErasureIncompleteError",
    ProjectionDestructive => "ProjectionDestructiveError",
}

impl ErrorKind {
    /// The intermediate class this kind derives from, if any: `Embedder` for
    /// the embedder-policy family, `Vector` for `KindNotVectorIndexed`.
    #[must_use]
    pub fn parent(self) -> Option<ErrorKind> {
        match self {
            Self::EmbedDevicePolicy
            | Self::RerankerDevicePolicy
            | Self::EmbedderNotConfigured
            | Self::EmbedderRequired => Some(Self::Embedder),
            Self::KindNotVectorIndexed => Some(Self::Vector),
            _ => None,
        }
    }
}
