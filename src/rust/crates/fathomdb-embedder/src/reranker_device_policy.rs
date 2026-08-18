//! Strict runtime policy for the Candle cross-encoder reranker.
//!
//! This is intentionally separate from the embedding resolver: a successful
//! embedding CUDA selection must never be represented as reranker engagement.

use std::{fmt, str::FromStr};

use crate::{CudaDeviceInfo, CudaProbeError, CudaProvider, CudaVisibleDevice};

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
        }
    }

    /// The forced ordinal, where present.
    #[must_use]
    pub const fn ordinal(&self) -> Option<usize> {
        match self {
            Self::InvalidPolicy(_) => None,
            Self::Resolution(RerankerDeviceResolutionError::CudaNotCompiled { ordinal })
            | Self::Resolution(RerankerDeviceResolutionError::ForcedCudaUnavailable {
                ordinal,
                ..
            }) => Some(*ordinal),
        }
    }
}

impl fmt::Display for RerankerDevicePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy(error) => error.fmt(formatter),
            Self::Resolution(error) => error.fmt(formatter),
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
            Ok(devices) => match devices.iter().min_by_key(|device| device.visible_ordinal) {
                Some(device) => match probe(provider, device) {
                    Ok(info) => {
                        Ok(selected_resolution(requested_policy, cuda_compiled, devices, info))
                    }
                    Err(reason) => {
                        Ok(cpu_resolution(requested_policy, cuda_compiled, devices, Some(reason)))
                    }
                },
                None => Ok(cpu_resolution(
                    requested_policy,
                    cuda_compiled,
                    devices,
                    Some(RerankerDeviceResolutionReason::NoVisibleCudaDevice),
                )),
            },
            Err(reason) => {
                Ok(cpu_resolution(requested_policy, cuda_compiled, vec![], Some(reason)))
            }
        },
        RerankerDevicePolicy::Cuda(ordinal) => match enumerate(provider) {
            Ok(devices) => match devices.iter().find(|device| device.visible_ordinal == ordinal) {
                Some(device) => probe(provider, device)
                    .map(|info| selected_resolution(requested_policy, cuda_compiled, devices, info))
                    .map_err(|reason| RerankerDeviceResolutionError::ForcedCudaUnavailable {
                        ordinal,
                        reason,
                    }),
                None => Err(RerankerDeviceResolutionError::ForcedCudaUnavailable {
                    ordinal,
                    reason: RerankerDeviceResolutionReason::NoVisibleCudaDevice,
                }),
            },
            Err(reason) => {
                Err(RerankerDeviceResolutionError::ForcedCudaUnavailable { ordinal, reason })
            }
        },
    }
}

/// Parse the one supported environment transport and resolve it once.
pub fn resolve_reranker_device_policy_from_env(
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> Result<RerankerDeviceResolution, RerankerDevicePolicyError> {
    let raw = std::env::var(ENV_RERANK_DEVICE).unwrap_or_else(|_| "auto".to_owned());
    let policy = raw.parse().map_err(RerankerDevicePolicyError::InvalidPolicy)?;
    resolve_reranker_device_policy(policy, cuda_compiled, provider)
        .map_err(RerankerDevicePolicyError::Resolution)
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

fn enumerate(
    provider: &mut dyn CudaProvider,
) -> Result<Vec<CudaVisibleDevice>, RerankerDeviceResolutionReason> {
    match provider.enumerate_visible_cuda_devices() {
        Ok(devices) => Ok(devices),
        Err(CudaProbeError::NoVisibleDevice) => Ok(vec![]),
        Err(CudaProbeError::Incompatible { .. }) => {
            Err(RerankerDeviceResolutionReason::CudaIncompatible)
        }
        Err(CudaProbeError::ProbeFailed { .. }) => {
            Err(RerankerDeviceResolutionReason::CudaProbeFailed)
        }
    }
}

fn probe(
    provider: &mut dyn CudaProvider,
    selected: &CudaVisibleDevice,
) -> Result<CudaDeviceInfo, RerankerDeviceResolutionReason> {
    match provider.probe_cuda(selected.visible_ordinal) {
        Ok(info)
            if info.ordinal == selected.visible_ordinal
                && info.uuid.as_deref() == Some(selected.uuid.as_str()) =>
        {
            Ok(info)
        }
        Ok(_) => Err(RerankerDeviceResolutionReason::CudaProbeFailed),
        Err(CudaProbeError::NoVisibleDevice) => {
            Err(RerankerDeviceResolutionReason::NoVisibleCudaDevice)
        }
        Err(CudaProbeError::Incompatible { .. }) => {
            Err(RerankerDeviceResolutionReason::CudaIncompatible)
        }
        Err(CudaProbeError::ProbeFailed { .. }) => {
            Err(RerankerDeviceResolutionReason::CudaProbeFailed)
        }
    }
}
