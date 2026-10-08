use std::fmt;

use fathomdb_embedder::RerankerDevicePolicyError;
use fathomdb_embedder_api::EmbedderError as RuntimeEmbedderError;
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
    /// A CUDA private-memory-pool failure, from any path (engine, open,
    /// reranker, CLS embedder). [`Error::cuda_details`] returns the payload.
    /// Conversions from [`EngineError`] and [`EngineOpenError`] produce this
    /// variant for the three CUDA pool kinds.
    Cuda(CudaErrorDetails),
}

/// The payload of a CUDA private-memory-pool failure; see
/// [`Error::cuda_details`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CudaErrorDetails {
    /// [`ErrorKind::CudaPoolExhausted`]: the private pool of device `ordinal`
    /// reached its cap of `max_size_bytes`. The device stays usable; a
    /// following request runs normally once memory is free.
    PoolExhausted { ordinal: usize, max_size_bytes: u64, message: String },
    /// [`ErrorKind::CudaContextLost`]: the recorded CUDA context is gone
    /// (`current_context_id` is `None` when no context is current) or was
    /// replaced; `driver_error` exposed it during `operation`. CUDA cannot be
    /// used again in this process.
    ContextLost {
        recorded_context_id: u64,
        current_context_id: Option<u64>,
        driver_error: String,
        operation: String,
    },
    /// [`ErrorKind::CudaPrivateBuildRefused`]: building another private-pool
    /// context on device `ordinal` failed; no context on another allocator is
    /// built in its place.
    PrivateBuildRefused { ordinal: usize, message: String },
}

impl CudaErrorDetails {
    /// The error class of this payload.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::PoolExhausted { .. } => ErrorKind::CudaPoolExhausted,
            Self::ContextLost { .. } => ErrorKind::CudaContextLost,
            Self::PrivateBuildRefused { .. } => ErrorKind::CudaPrivateBuildRefused,
        }
    }

    fn from_engine(error: &EngineError) -> Option<Self> {
        match error {
            EngineError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
                Some(Self::PoolExhausted {
                    ordinal: *ordinal,
                    max_size_bytes: *max_size_bytes,
                    message: message.clone(),
                })
            }
            EngineError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => Some(Self::ContextLost {
                recorded_context_id: *recorded_context_id,
                current_context_id: *current_context_id,
                driver_error: driver_error.clone(),
                operation: operation.clone(),
            }),
            EngineError::CudaPrivateBuildRefused { ordinal, message } => {
                Some(Self::PrivateBuildRefused { ordinal: *ordinal, message: message.clone() })
            }
            EngineError::RerankerDevicePolicy(error) => Self::from_reranker(error),
            _ => None,
        }
    }

    fn from_reranker(error: &RerankerDevicePolicyError) -> Option<Self> {
        match error {
            RerankerDevicePolicyError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
                Some(Self::PoolExhausted {
                    ordinal: *ordinal,
                    max_size_bytes: *max_size_bytes,
                    message: message.clone(),
                })
            }
            RerankerDevicePolicyError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => Some(Self::ContextLost {
                recorded_context_id: *recorded_context_id,
                current_context_id: *current_context_id,
                driver_error: driver_error.clone(),
                operation: operation.clone(),
            }),
            RerankerDevicePolicyError::CudaPrivateBuildRefused { ordinal, message } => {
                Some(Self::PrivateBuildRefused { ordinal: *ordinal, message: message.clone() })
            }
            RerankerDevicePolicyError::InvalidPolicy(_)
            | RerankerDevicePolicyError::Resolution(_) => None,
        }
    }

    fn from_embedder(error: &RuntimeEmbedderError) -> Option<Self> {
        match error {
            RuntimeEmbedderError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
                Some(Self::PoolExhausted {
                    ordinal: *ordinal,
                    max_size_bytes: *max_size_bytes,
                    message: message.clone(),
                })
            }
            RuntimeEmbedderError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => Some(Self::ContextLost {
                recorded_context_id: *recorded_context_id,
                current_context_id: *current_context_id,
                driver_error: driver_error.clone(),
                operation: operation.clone(),
            }),
            RuntimeEmbedderError::CudaPrivateBuildRefused { ordinal, message } => {
                Some(Self::PrivateBuildRefused { ordinal: *ordinal, message: message.clone() })
            }
            RuntimeEmbedderError::Failed { .. } | RuntimeEmbedderError::Timeout => None,
            // `EmbedderError` is `#[non_exhaustive]`; every CUDA pool kind has
            // an arm above.
            _ => None,
        }
    }

    fn from_open(error: &EngineOpenError) -> Option<Self> {
        match error {
            EngineOpenError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
                Some(Self::PoolExhausted {
                    ordinal: *ordinal,
                    max_size_bytes: *max_size_bytes,
                    message: message.clone(),
                })
            }
            EngineOpenError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => Some(Self::ContextLost {
                recorded_context_id: *recorded_context_id,
                current_context_id: *current_context_id,
                driver_error: driver_error.clone(),
                operation: operation.clone(),
            }),
            EngineOpenError::CudaPrivateBuildRefused { ordinal, message } => {
                Some(Self::PrivateBuildRefused { ordinal: *ordinal, message: message.clone() })
            }
            EngineOpenError::Embedder(error) => Self::from_embedder(error),
            EngineOpenError::RerankerDevicePolicy(error) => Self::from_reranker(error),
            _ => None,
        }
    }
}

impl fmt::Display for CudaErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PoolExhausted { ordinal, max_size_bytes, message } => write!(
                f,
                "the private CUDA memory pool of device {ordinal} reached its cap of \
                 {max_size_bytes} bytes: {message}"
            ),
            Self::ContextLost {
                recorded_context_id,
                current_context_id: Some(current),
                driver_error,
                operation,
            } => write!(
                f,
                "CUDA context {recorded_context_id} was replaced by context {current} at \
                 {operation}: {driver_error}"
            ),
            Self::ContextLost {
                recorded_context_id,
                current_context_id: None,
                driver_error,
                operation,
            } => write!(
                f,
                "CUDA context {recorded_context_id} is gone (no current context) at \
                 {operation}: {driver_error}"
            ),
            Self::PrivateBuildRefused { ordinal, message } => write!(
                f,
                "could not build a private-pool CUDA context on device {ordinal}: {message}"
            ),
        }
    }
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

    /// A CUDA pool failure from an embedder call outside the engine; every
    /// other embedder failure is `ErrorKind::Embedder` with `context`.
    pub(crate) fn embedder(error: &RuntimeEmbedderError, context: &str) -> Self {
        match CudaErrorDetails::from_embedder(error) {
            Some(details) => Self::Cuda(details),
            None => Self::sdk(ErrorKind::Embedder, format!("{context}: {error:?}")),
        }
    }

    /// The shared Python/TypeScript error class of this error.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Engine(error) => engine_kind(error),
            Self::Open(error) => open_kind(error),
            Self::RuntimeConfiguration(_) => ErrorKind::RuntimeConfiguration,
            Self::Sdk { kind, .. } => *kind,
            Self::Cuda(details) => details.kind(),
        }
    }

    /// The payload of a CUDA private-memory-pool failure
    /// ([`ErrorKind::CudaPoolExhausted`], [`ErrorKind::CudaContextLost`],
    /// [`ErrorKind::CudaPrivateBuildRefused`]); `None` for every other error.
    #[must_use]
    pub fn cuda_details(&self) -> Option<&CudaErrorDetails> {
        match self {
            Self::Cuda(details) => Some(details),
            Self::Engine(_) | Self::Open(_) | Self::RuntimeConfiguration(_) | Self::Sdk { .. } => {
                None
            }
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
        EngineError::RerankerDevicePolicy(error) => CudaErrorDetails::from_reranker(error)
            .map_or(ErrorKind::RerankerDevicePolicy, |details| details.kind()),
        EngineError::CudaPoolExhausted { .. } => ErrorKind::CudaPoolExhausted,
        EngineError::CudaContextLost { .. } => ErrorKind::CudaContextLost,
        EngineError::CudaPrivateBuildRefused { .. } => ErrorKind::CudaPrivateBuildRefused,
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
        EngineOpenError::Embedder(error) => CudaErrorDetails::from_embedder(error)
            .map_or(ErrorKind::Embedder, |details| details.kind()),
        EngineOpenError::EmbedDevicePolicy(_) => ErrorKind::EmbedDevicePolicy,
        EngineOpenError::RerankerDevicePolicy(error) => CudaErrorDetails::from_reranker(error)
            .map_or(ErrorKind::RerankerDevicePolicy, |details| details.kind()),
        EngineOpenError::CudaPoolExhausted { .. } => ErrorKind::CudaPoolExhausted,
        EngineOpenError::CudaContextLost { .. } => ErrorKind::CudaContextLost,
        EngineOpenError::CudaPrivateBuildRefused { .. } => ErrorKind::CudaPrivateBuildRefused,
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
            Self::Cuda(details) => write!(f, "{}: {details}", details.kind().name()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            Self::Open(error) => Some(error),
            Self::RuntimeConfiguration(error) => Some(error),
            Self::Sdk { .. } | Self::Cuda(_) => None,
        }
    }
}

/// The three CUDA pool kinds (including a reranker refusal of one of them)
/// become [`Error::Cuda`]; every other engine error is [`Error::Engine`].
impl From<EngineError> for Error {
    fn from(error: EngineError) -> Self {
        match CudaErrorDetails::from_engine(&error) {
            Some(details) => Self::Cuda(details),
            None => Self::Engine(error),
        }
    }
}

/// The three CUDA pool kinds (including an embedder or reranker failure of
/// one of them) become [`Error::Cuda`]; every other open error is
/// [`Error::Open`].
impl From<EngineOpenError> for Error {
    fn from(error: EngineOpenError) -> Self {
        match CudaErrorDetails::from_open(&error) {
            Some(details) => Self::Cuda(details),
            None => Self::Open(error),
        }
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
    CudaPoolExhausted => "CudaPoolExhaustedError",
    CudaContextLost => "CudaContextLostError",
    CudaPrivateBuildRefused => "CudaPrivateBuildRefusedError",
}

impl ErrorKind {
    /// The intermediate class this kind derives from, if any: `Embedder` for
    /// the embedder-policy family and the CUDA pool kinds, `Vector` for
    /// `KindNotVectorIndexed`.
    #[must_use]
    pub fn parent(self) -> Option<ErrorKind> {
        match self {
            Self::EmbedDevicePolicy
            | Self::RerankerDevicePolicy
            | Self::EmbedderNotConfigured
            | Self::EmbedderRequired
            | Self::CudaPoolExhausted
            | Self::CudaContextLost
            | Self::CudaPrivateBuildRefused => Some(Self::Embedder),
            Self::KindNotVectorIndexed => Some(Self::Vector),
            _ => None,
        }
    }
}
