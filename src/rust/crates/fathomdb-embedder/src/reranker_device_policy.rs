//! Strict runtime policy for the Candle cross-encoder reranker.
//!
//! This is intentionally separate from the embedding resolver: a successful
//! embedding CUDA selection must never be represented as reranker engagement.

use std::{fmt, str::FromStr};

use crate::{CudaDeviceInfo, CudaPoolFailure, CudaProbeError, CudaProvider, CudaVisibleDevice};

/// The sole cross-SDK transport for reranker policy selection.
pub const ENV_RERANK_DEVICE: &str = "FATHOMDB_RERANK_DEVICE";

/// The exact supported value of `FATHOMDB_RERANK_DEVICE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RerankerDevicePolicy {
    /// Select a compatible CUDA device when possible, otherwise CPU with a reason.
    Auto,
    /// Select CPU without enumerating or initializing CUDA.
    Cpu,
    /// Require one process-visible CUDA ordinal; never retry on CPU.
    Cuda(usize),
}

impl FromStr for RerankerDevicePolicy {
    type Err = RerankerDevicePolicyParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            _ => raw
                .strip_prefix("cuda:")
                .and_then(|ordinal| {
                    (!ordinal.is_empty() && ordinal.bytes().all(|byte| byte.is_ascii_digit()))
                        .then_some(ordinal)
                })
                .and_then(|ordinal| ordinal.parse().ok())
                .map(Self::Cuda)
                .ok_or_else(|| RerankerDevicePolicyParseError::InvalidPolicy {
                    raw: raw.to_owned(),
                }),
        }
    }
}

/// A malformed or retired reranker device policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RerankerDevicePolicyParseError {
    /// The setting is not exactly `auto`, `cpu`, or `cuda:N`.
    InvalidPolicy { raw: String },
}

impl fmt::Display for RerankerDevicePolicyParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy { raw } => write!(
                formatter,
                "invalid FATHOMDB_RERANK_DEVICE={raw:?}; expected auto, cpu, or cuda:N"
            ),
        }
    }
}

impl std::error::Error for RerankerDevicePolicyParseError {}

/// A typed reranker-policy error for bindings and callers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RerankerDevicePolicyError {
    /// The ambient policy did not use the exact supported grammar.
    InvalidPolicy(RerankerDevicePolicyParseError),
    /// A forced CUDA policy could not select its required device.
    Resolution(RerankerDeviceResolutionError),
    /// The private CUDA memory pool of device `ordinal` reached its cap of
    /// `max_size_bytes`. The device stays usable for the next call.
    CudaPoolExhausted { ordinal: usize, max_size_bytes: u64, message: String },
    /// The CUDA context recorded as `recorded_context_id` is gone
    /// (`current_context_id` is `None` when the primary context is
    /// inactive); `driver_error` exposed it during `operation`.
    CudaContextLost {
        recorded_context_id: u64,
        current_context_id: Option<u64>,
        driver_error: String,
        operation: String,
    },
    /// Building another private-pool CUDA context on device `ordinal` failed;
    /// no context on another allocator is built in its place.
    CudaPrivateBuildRefused { ordinal: usize, message: String },
}

impl RerankerDevicePolicyError {
    /// Stable error classification for SDK error envelopes.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::InvalidPolicy(_) => "invalid_policy",
            Self::Resolution(RerankerDeviceResolutionError::CudaNotCompiled { .. }) => {
                "cuda_not_compiled"
            }
            Self::Resolution(RerankerDeviceResolutionError::ForcedCudaUnavailable {
                reason,
                ..
            }) => reason.as_str(),
            Self::Resolution(RerankerDeviceResolutionError::CudaPool(failure)) => failure.kind(),
            Self::CudaPoolExhausted { .. } => "cuda_pool_exhausted",
            Self::CudaContextLost { .. } => "cuda_context_lost",
            Self::CudaPrivateBuildRefused { .. } => "cuda_private_build_refused",
        }
    }

    /// The forced ordinal, where present.
    #[must_use]
    pub const fn ordinal(&self) -> Option<usize> {
        match self {
            Self::InvalidPolicy(_) | Self::CudaContextLost { .. } => None,
            Self::CudaPoolExhausted { ordinal, .. }
            | Self::CudaPrivateBuildRefused { ordinal, .. } => Some(*ordinal),
            Self::Resolution(RerankerDeviceResolutionError::CudaNotCompiled { ordinal })
            | Self::Resolution(RerankerDeviceResolutionError::ForcedCudaUnavailable {
                ordinal,
                ..
            }) => Some(*ordinal),
            Self::Resolution(RerankerDeviceResolutionError::CudaPool(failure)) => failure.ordinal(),
        }
    }
}

impl fmt::Display for RerankerDevicePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy(error) => error.fmt(formatter),
            Self::Resolution(error) => error.fmt(formatter),
            Self::CudaPoolExhausted { ordinal, max_size_bytes, message } => write!(
                formatter,
                "the private CUDA memory pool of device {ordinal} reached its cap of \
                 {max_size_bytes} bytes: {message}"
            ),
            Self::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => {
                write!(
                    formatter,
                    "CUDA context {recorded_context_id} is gone (current: \
                     {current_context_id:?}) at {operation}: {driver_error}"
                )
            }
            Self::CudaPrivateBuildRefused { ordinal, message } => write!(
                formatter,
                "could not build a private-pool CUDA context on device {ordinal}: {message}"
            ),
        }
    }
}

impl std::error::Error for RerankerDevicePolicyError {}

/// The actual reranker backend selected by policy resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectiveRerankerDevice {
    /// Cross-encoder inference runs on CPU.
    Cpu,
    /// Cross-encoder inference runs on the initialized CUDA device.
    Cuda(CudaDeviceInfo),
}

/// Stable reason for automatic CPU fallback or forced CUDA refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RerankerDeviceResolutionReason {
    /// The artifact does not include the CUDA reranker provider.
    CudaNotCompiled,
    /// No CUDA device is visible to this process.
    NoVisibleCudaDevice,
    /// The visible device cannot satisfy the loaded CUDA provider.
    CudaIncompatible,
    /// CUDA initialization, session construction, or probe failed unexpectedly.
    CudaProbeFailed,
}

impl RerankerDeviceResolutionReason {
    /// Stable lower-snake-case binding value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CudaNotCompiled => "cuda_not_compiled",
            Self::NoVisibleCudaDevice => "no_visible_cuda_device",
            Self::CudaIncompatible => "cuda_incompatible",
            Self::CudaProbeFailed => "cuda_probe_failed",
        }
    }
}

/// A forced CUDA request that could not select the requested device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RerankerDeviceResolutionError {
    /// The artifact lacks CUDA reranker support.
    CudaNotCompiled { ordinal: usize },
    /// CUDA was requested and could not be engaged; CPU must not be tried.
    ForcedCudaUnavailable { ordinal: usize, reason: RerankerDeviceResolutionReason },
    /// The probe failed with a private CUDA memory pool kind. Raised under
    /// `auto` as well as forced CUDA: a private-pool process never moves to
    /// CPU. [`resolve_reranker_device_policy_from_env`] returns it as the
    /// matching [`RerankerDevicePolicyError`] pool variant.
    CudaPool(CudaPoolFailure),
}

impl fmt::Display for RerankerDeviceResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CudaNotCompiled { ordinal } => write!(
                formatter,
                "cuda:{ordinal} requested for reranking but this artifact was built without CUDA"
            ),
            Self::ForcedCudaUnavailable { ordinal, reason } => write!(
                formatter,
                "cuda:{ordinal} requested for reranking but unavailable: {reason:?}"
            ),
            Self::CudaPool(failure) => failure.fmt(formatter),
        }
    }
}

impl std::error::Error for RerankerDeviceResolutionError {}

/// Immutable selection evidence for one reranker construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RerankerDeviceResolution {
    /// The requested policy.
    pub requested_policy: RerankerDevicePolicy,
    /// Whether this artifact contains the CUDA reranker provider.
    pub cuda_compiled: bool,
    /// Actual inference device.
    pub effective_device: EffectiveRerankerDevice,
    /// Ordered process-visible CUDA inventory.
    pub visible_cuda_devices: Vec<CudaVisibleDevice>,
    /// UUID of the effective CUDA selection, when CUDA is selected.
    pub selected_cuda_uuid: Option<String>,
    /// Honest automatic CPU fallback reason, when CPU was selected by `auto`.
    pub reason: Option<RerankerDeviceResolutionReason>,
}

/// Resolve an explicit reranker policy through an injectable provider.
///
/// `cpu` does not invoke the provider. `auto` may use CPU only with an
/// explicit reason. Forced `cuda:N` returns an error and never resolves CPU.
/// A probe that fails with a private CUDA memory pool kind is
/// [`RerankerDeviceResolutionError::CudaPool`] under `auto` and forced CUDA
/// alike, and `auto` probes no further device.
pub fn resolve_reranker_device_policy(
    requested_policy: RerankerDevicePolicy,
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> Result<RerankerDeviceResolution, RerankerDeviceResolutionError> {
    match requested_policy {
        RerankerDevicePolicy::Cpu => {
            Ok(cpu_resolution(requested_policy, cuda_compiled, vec![], None))
        }
        RerankerDevicePolicy::Auto if !cuda_compiled => Ok(cpu_resolution(
            requested_policy,
            cuda_compiled,
            vec![],
            Some(RerankerDeviceResolutionReason::CudaNotCompiled),
        )),
        RerankerDevicePolicy::Cuda(ordinal) if !cuda_compiled => {
            Err(RerankerDeviceResolutionError::CudaNotCompiled { ordinal })
        }
        RerankerDevicePolicy::Auto => match enumerate(provider) {
            Err(Unavailable::Pool(failure)) => {
                Err(RerankerDeviceResolutionError::CudaPool(failure))
            }
            Ok(devices) if devices.is_empty() => Ok(cpu_resolution(
                requested_policy,
                cuda_compiled,
                devices,
                Some(RerankerDeviceResolutionReason::NoVisibleCudaDevice),
            )),
            Ok(devices) => {
                let mut ordered = devices.iter().collect::<Vec<_>>();
                ordered.sort_by_key(|device| device.visible_ordinal);
                let mut last_reason = RerankerDeviceResolutionReason::NoVisibleCudaDevice;
                for device in ordered {
                    match probe(provider, device) {
                        Ok(info) => {
                            return Ok(selected_resolution(
                                requested_policy,
                                cuda_compiled,
                                devices,
                                info,
                            ));
                        }
                        Err(Unavailable::Reason(reason)) => last_reason = reason,
                        Err(Unavailable::Pool(failure)) => {
                            return Err(RerankerDeviceResolutionError::CudaPool(failure));
                        }
                    }
                }
                Ok(cpu_resolution(requested_policy, cuda_compiled, devices, Some(last_reason)))
            }
            Err(Unavailable::Reason(reason)) => {
                Ok(cpu_resolution(requested_policy, cuda_compiled, vec![], Some(reason)))
            }
        },
        RerankerDevicePolicy::Cuda(ordinal) => {
            let forced = |unavailable| match unavailable {
                Unavailable::Reason(reason) => {
                    RerankerDeviceResolutionError::ForcedCudaUnavailable { ordinal, reason }
                }
                Unavailable::Pool(failure) => RerankerDeviceResolutionError::CudaPool(failure),
            };
            let devices = enumerate(provider).map_err(forced)?;
            match devices.iter().find(|device| device.visible_ordinal == ordinal) {
                Some(device) => probe(provider, device)
                    .map(|info| selected_resolution(requested_policy, cuda_compiled, devices, info))
                    .map_err(forced),
                None => Err(RerankerDeviceResolutionError::ForcedCudaUnavailable {
                    ordinal,
                    reason: RerankerDeviceResolutionReason::NoVisibleCudaDevice,
                }),
            }
        }
    }
}

/// Parse the one supported environment transport and resolve it once.
pub fn resolve_reranker_device_policy_from_env(
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> Result<RerankerDeviceResolution, RerankerDevicePolicyError> {
    let raw = std::env::var(ENV_RERANK_DEVICE).unwrap_or_else(|_| "auto".to_owned());
    let policy = raw.parse().map_err(RerankerDevicePolicyError::InvalidPolicy)?;
    resolve_reranker_device_policy(policy, cuda_compiled, provider).map_err(|error| match error {
        RerankerDeviceResolutionError::CudaPool(failure) => failure.into_reranker_policy_error(),
        error => RerankerDevicePolicyError::Resolution(error),
    })
}

fn cpu_resolution(
    requested_policy: RerankerDevicePolicy,
    cuda_compiled: bool,
    visible_cuda_devices: Vec<CudaVisibleDevice>,
    reason: Option<RerankerDeviceResolutionReason>,
) -> RerankerDeviceResolution {
    RerankerDeviceResolution {
        requested_policy,
        cuda_compiled,
        effective_device: EffectiveRerankerDevice::Cpu,
        visible_cuda_devices,
        selected_cuda_uuid: None,
        reason,
    }
}

fn selected_resolution(
    requested_policy: RerankerDevicePolicy,
    cuda_compiled: bool,
    visible_cuda_devices: Vec<CudaVisibleDevice>,
    info: CudaDeviceInfo,
) -> RerankerDeviceResolution {
    RerankerDeviceResolution {
        requested_policy,
        cuda_compiled,
        selected_cuda_uuid: info.uuid.clone(),
        effective_device: EffectiveRerankerDevice::Cuda(info),
        visible_cuda_devices,
        reason: None,
    }
}

/// Why CUDA could not be selected: a reason `auto` may fall back to CPU on,
/// or a private-pool failure it never falls back on.
enum Unavailable {
    Reason(RerankerDeviceResolutionReason),
    Pool(CudaPoolFailure),
}

fn unavailable(error: CudaProbeError) -> Unavailable {
    match error {
        CudaProbeError::NoVisibleDevice => {
            Unavailable::Reason(RerankerDeviceResolutionReason::NoVisibleCudaDevice)
        }
        CudaProbeError::Incompatible { .. } => {
            Unavailable::Reason(RerankerDeviceResolutionReason::CudaIncompatible)
        }
        CudaProbeError::ProbeFailed { .. } => {
            Unavailable::Reason(RerankerDeviceResolutionReason::CudaProbeFailed)
        }
        CudaProbeError::Pool(failure) => Unavailable::Pool(failure),
    }
}

fn enumerate(provider: &mut dyn CudaProvider) -> Result<Vec<CudaVisibleDevice>, Unavailable> {
    match provider.enumerate_visible_cuda_devices() {
        Ok(devices) => Ok(devices),
        Err(CudaProbeError::NoVisibleDevice) => Ok(vec![]),
        Err(error) => Err(unavailable(error)),
    }
}

fn probe(
    provider: &mut dyn CudaProvider,
    selected: &CudaVisibleDevice,
) -> Result<CudaDeviceInfo, Unavailable> {
    match provider.probe_cuda(selected.visible_ordinal) {
        Ok(info)
            if info.ordinal == selected.visible_ordinal
                && info.uuid.as_deref() == Some(selected.uuid.as_str()) =>
        {
            Ok(info)
        }
        Ok(_) => Err(Unavailable::Reason(RerankerDeviceResolutionReason::CudaProbeFailed)),
        Err(error) => Err(unavailable(error)),
    }
}
