//! Strict, typed runtime policy for the default embedder's CPU/CUDA device.
//!
//! This module deliberately does not construct a Candle device.  The resolver
//! accepts an injected provider so parsing and selection can be tested without
//! CUDA hardware, a driver, or model assets.  A later construction seam uses
//! the resulting [`DeviceResolution`] exactly once to create the actual
//! embedder backend.

use std::{fmt, str::FromStr};

/// The supported value of `FATHOMDB_EMBED_DEVICE`.
///
/// `auto` permits CPU fallback only after a CUDA probe reports no usable
/// device. `cpu` is an explicit off switch and never initializes CUDA.
/// `cuda:N` is an explicit request that fails rather than falling back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbedDevicePolicy {
    /// Probe CUDA ordinal zero and otherwise use CPU.
    Auto,
    /// Use CPU without initializing or probing CUDA.
    Cpu,
    /// Require the specified CUDA ordinal.
    Cuda(usize),
}

impl FromStr for EmbedDevicePolicy {
    type Err = EmbedDevicePolicyParseError;

    /// Parse the exact public `FATHOMDB_EMBED_DEVICE` grammar.
    ///
    /// Accepted values are exactly `auto`, `cpu`, and `cuda:N`, where `N` is
    /// a non-negative base-10 ordinal. In particular, legacy bare `cuda`,
    /// other providers, whitespace, and case variants are configuration
    /// errors rather than implicit fallbacks.
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
                .and_then(|ordinal| ordinal.parse::<usize>().ok())
                .map(Self::Cuda)
                .ok_or_else(|| EmbedDevicePolicyParseError::InvalidPolicy { raw: raw.to_owned() }),
        }
    }
}

/// A malformed `FATHOMDB_EMBED_DEVICE` setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbedDevicePolicyParseError {
    /// The raw setting is not one of the supported policy values.
    InvalidPolicy { raw: String },
}

impl fmt::Display for EmbedDevicePolicyParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy { raw } => write!(
                formatter,
                "invalid FATHOMDB_EMBED_DEVICE={raw:?}; expected auto, cpu, or cuda:N"
            ),
        }
    }
}

impl std::error::Error for EmbedDevicePolicyParseError {}

/// A typed failure while resolving the public embedder-device setting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbedDevicePolicyError {
    /// `FATHOMDB_EMBED_DEVICE` is malformed or uses a retired spelling.
    InvalidPolicy(EmbedDevicePolicyParseError),
    /// A syntactically valid policy could not select its required device.
    Resolution(DeviceResolutionError),
}

impl fmt::Display for EmbedDevicePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy(error) => error.fmt(formatter),
            Self::Resolution(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EmbedDevicePolicyError {}

/// Safe metadata returned after a compatible CUDA device has been initialized
/// and minimally probed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaDeviceInfo {
    /// CUDA ordinal used to initialize the provider.
    pub ordinal: usize,
    /// UUID returned by the CUDA driver for the process-visible ordinal.
    pub uuid: Option<String>,
    /// Provider-reported device name, when available.
    pub name: Option<String>,
    /// Provider-reported NVIDIA driver version, when available.
    pub driver_version: Option<String>,
    /// Provider-reported device compute capability, when available.
    pub compute_capability: Option<String>,
    /// CUDA toolkit version used by the loaded provider, when available.
    pub cuda_toolkit_version: Option<String>,
}

/// One CUDA device visible to this process.
///
/// `visible_ordinal` is relative to `CUDA_VISIBLE_DEVICES`; it is never an
/// inferred host or PCI ordinal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CudaVisibleDevice {
    /// Process-visible CUDA ordinal.
    pub visible_ordinal: usize,
    /// CUDA driver UUID for this process-visible device.
    pub uuid: String,
    /// Driver-reported device name.
    pub name: String,
    /// Driver-reported CUDA compute capability, if available.
    pub compute_capability: Option<String>,
}

/// The injected CUDA probe boundary.
///
/// A production implementation initializes the chosen CUDA ordinal and runs a
/// minimal provider probe. Implementations must not report success until that
/// work has completed.
pub trait CudaProvider {
    /// Enumerate all CUDA devices visible to this process before selection.
    ///
    /// A successful empty vector means the driver initialized but no device is
    /// visible. An error means visibility itself could not be established.
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError>;

    /// Initialize and minimally probe `ordinal`, returning safe identity facts.
    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError>;
}

/// A classified failure while initializing or minimally probing CUDA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CudaProbeError {
    /// No CUDA device is visible to this process.
    NoVisibleDevice,
    /// The visible device or driver cannot satisfy the loaded CUDA provider.
    Incompatible { message: String },
    /// Provider initialization or the minimal probe failed for another reason.
    ProbeFailed { message: String },
}

/// The device ultimately selected by a successful policy resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectiveEmbedDevice {
    /// The default embedder must construct its CPU backend.
    Cpu,
    /// The default embedder must construct its CUDA backend using this probe.
    Cuda(CudaDeviceInfo),
}

/// A machine-readable reason associated with CPU fallback or CUDA failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceResolutionReason {
    /// The artifact was built without CUDA support.
    CudaNotCompiled,
    /// CUDA support is compiled in but no device is visible.
    NoVisibleCudaDevice,
    /// A visible device cannot run the loaded CUDA provider.
    CudaIncompatible,
    /// CUDA initialization/probing failed before compatibility was known.
    CudaProbeFailed,
    /// The platform is ARM64 SBSA, whose CUDA userspace is incompatible with
    /// the Tegra-linked artifact before any CUDA provider is loaded.
    Arm64SbsaUnsupported,
}

impl DeviceResolutionReason {
    /// Stable lower-snake-case reason for bindings and diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CudaNotCompiled => "cuda_not_compiled",
            Self::NoVisibleCudaDevice => "no_visible_cuda_device",
            Self::CudaIncompatible => "cuda_incompatible",
            Self::CudaProbeFailed => "cuda_probe_failed",
            Self::Arm64SbsaUnsupported => "arm64_sbsa_unsupported",
        }
    }
}

/// The immutable result of resolving one embedder device policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceResolution {
    /// The policy requested by the caller or environment.
    pub requested_policy: EmbedDevicePolicy,
    /// Whether this artifact includes the CUDA embedder provider.
    pub cuda_compiled: bool,
    /// The backend the default embedder must construct.
    pub effective_device: EffectiveEmbedDevice,
    /// Ordered process-visible CUDA inventory observed while resolving.
    pub visible_cuda_devices: Vec<CudaVisibleDevice>,
    /// UUID of the effective CUDA selection, when CUDA was selected.
    pub selected_cuda_uuid: Option<String>,
    /// Why CUDA was unavailable when CPU was selected automatically.
    pub reason: Option<DeviceResolutionReason>,
}

/// Stable status emitted by the CLI-only `doctor gpu` diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoctorGpuStatus {
    /// CUDA was selected and its minimal allocation/provider probe succeeded.
    SelectedCuda,
    /// Explicit CPU was selected without CUDA activity.
    SelectedCpuNoCuda,
    /// The artifact has no CUDA provider.
    CudaNotCompiled,
    /// No requested CUDA device was visible.
    CudaUnavailable,
    /// A visible CUDA device could not satisfy the provider.
    CudaIncompatible,
    /// The ambient policy was not part of the supported grammar.
    InvalidPolicy,
    /// Driver enumeration or a minimal provider probe failed unexpectedly.
    ProbeFailed,
}

impl DoctorGpuStatus {
    /// Stable lower-snake-case diagnostic status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelectedCuda => "selected_cuda",
            Self::SelectedCpuNoCuda => "selected_cpu_no_cuda",
            Self::CudaNotCompiled => "cuda_not_compiled",
            Self::CudaUnavailable => "cuda_unavailable",
            Self::CudaIncompatible => "cuda_incompatible",
            Self::InvalidPolicy => "invalid_policy",
            Self::ProbeFailed => "probe_failed",
        }
    }
}

/// Isolated CUDA diagnostic result for `fathomdb doctor gpu`.
///
/// This deliberately differs from [`DeviceResolution`]: an automatic CUDA
/// probe failure is an error in the diagnostic (exit 70), even though normal
/// open may use CPU and retain `CudaProbeFailed` as its fallback reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorGpuDiagnosticResult {
    /// Raw policy string read by the diagnostic.
    pub policy: String,
    /// Whether this artifact includes the CUDA embedder provider.
    pub cuda_compiled: bool,
    /// Classified diagnostic outcome.
    pub status: DoctorGpuStatus,
    /// Effective CPU/CUDA device, if policy resolution selected one.
    pub effective_device: Option<EffectiveEmbedDevice>,
    /// Ordered process-visible CUDA inventory.
    pub devices: Vec<CudaVisibleDevice>,
    /// UUID of the selected device, when CUDA was selected.
    pub selected_uuid: Option<String>,
    /// Optional compatibility/fallback reason.
    pub reason: Option<DeviceResolutionReason>,
}

impl DoctorGpuDiagnosticResult {
    /// Construct the no-provider invalid-policy result.
    #[must_use]
    pub fn from_invalid_policy(policy: impl Into<String>, cuda_compiled: bool) -> Self {
        Self {
            policy: policy.into(),
            cuda_compiled,
            status: DoctorGpuStatus::InvalidPolicy,
            effective_device: None,
            devices: Vec::new(),
            selected_uuid: None,
            reason: None,
        }
    }

    /// Stable status accessor for bindings and the CLI serializer.
    #[must_use]
    pub const fn status(&self) -> DoctorGpuStatus {
        self.status
    }

    /// The literal selected device (`cpu` or `cuda:N`), if any.
    #[must_use]
    pub fn effective_device(&self) -> Option<String> {
        match &self.effective_device {
            Some(EffectiveEmbedDevice::Cpu) => Some("cpu".to_owned()),
            Some(EffectiveEmbedDevice::Cuda(info)) => Some(format!("cuda:{}", info.ordinal)),
            None => None,
        }
    }

    /// Ordered process-visible inventory.
    #[must_use]
    pub fn devices(&self) -> &[CudaVisibleDevice] {
        &self.devices
    }

    /// UUID of the selected CUDA device, if any.
    #[must_use]
    pub fn selected_uuid(&self) -> Option<&str> {
        self.selected_uuid.as_deref()
    }

    /// CLI process exit code required by the diagnostic contract.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self.status {
            DoctorGpuStatus::SelectedCuda | DoctorGpuStatus::SelectedCpuNoCuda => 0,
            DoctorGpuStatus::CudaNotCompiled
            | DoctorGpuStatus::CudaUnavailable
            | DoctorGpuStatus::CudaIncompatible => {
                if matches!(&self.effective_device, Some(EffectiveEmbedDevice::Cpu)) {
                    0
                } else {
                    65
                }
            }
            DoctorGpuStatus::InvalidPolicy | DoctorGpuStatus::ProbeFailed => 70,
        }
    }
}

/// A policy that required CUDA could not be satisfied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceResolutionError {
    /// A forced CUDA policy was requested from a CPU-only artifact.
    CudaNotCompiled { ordinal: usize },
    /// A forced CUDA policy could not initialize/probe the requested ordinal.
    ForcedCudaUnavailable { ordinal: usize, reason: DeviceResolutionReason },
}

impl EmbedDevicePolicyError {
    /// Stable lower-snake-case error kind for bindings and diagnostics.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::InvalidPolicy(_) => "invalid_policy",
            Self::Resolution(DeviceResolutionError::CudaNotCompiled { .. }) => "cuda_not_compiled",
            Self::Resolution(DeviceResolutionError::ForcedCudaUnavailable { reason, .. }) => {
                reason.as_str()
            }
        }
    }

    /// The forced CUDA ordinal when one was part of the failed policy.
    #[must_use]
    pub const fn ordinal(&self) -> Option<usize> {
        match self {
            Self::InvalidPolicy(_) => None,
            Self::Resolution(DeviceResolutionError::CudaNotCompiled { ordinal })
            | Self::Resolution(DeviceResolutionError::ForcedCudaUnavailable { ordinal, .. }) => {
                Some(*ordinal)
            }
        }
    }
}

impl fmt::Display for DeviceResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CudaNotCompiled { ordinal } => {
                write!(
                    formatter,
                    "cuda:{ordinal} requested but this artifact was built without CUDA"
                )
            }
            Self::ForcedCudaUnavailable { ordinal, reason } => {
                write!(formatter, "cuda:{ordinal} requested but unavailable: {reason:?}")
            }
        }
    }
}

impl std::error::Error for DeviceResolutionError {}

/// Resolve one policy with a provider whose initialization/probe is injectable.
///
/// This function never parses ambient environment state. The caller parses
/// `FATHOMDB_EMBED_DEVICE` once, then passes its explicit policy here. `cpu`
/// and CPU-only `auto` do not call `provider`. `auto` can select CPU only for
/// an unavailable/unusable CUDA provider; forced `cuda:N` returns a typed error
/// for the same conditions and never resolves to CPU.
pub fn resolve_embed_device_policy(
    requested_policy: EmbedDevicePolicy,
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> Result<DeviceResolution, DeviceResolutionError> {
    match requested_policy {
        EmbedDevicePolicy::Cpu => Ok(DeviceResolution {
            requested_policy,
            cuda_compiled,
            effective_device: EffectiveEmbedDevice::Cpu,
            visible_cuda_devices: Vec::new(),
            selected_cuda_uuid: None,
            reason: None,
        }),
        EmbedDevicePolicy::Auto if !cuda_compiled => Ok(DeviceResolution {
            requested_policy,
            cuda_compiled,
            effective_device: EffectiveEmbedDevice::Cpu,
            visible_cuda_devices: Vec::new(),
            selected_cuda_uuid: None,
            reason: Some(DeviceResolutionReason::CudaNotCompiled),
        }),
        EmbedDevicePolicy::Cuda(ordinal) if !cuda_compiled => {
            Err(DeviceResolutionError::CudaNotCompiled { ordinal })
        }
        EmbedDevicePolicy::Auto => match enumerate(provider) {
            Ok(devices) => match devices.iter().min_by_key(|device| device.visible_ordinal) {
                Some(device) => match probe(provider, device) {
                    Ok(info) => {
                        Ok(selected_resolution(requested_policy, cuda_compiled, devices, info))
                    }
                    Err(reason) => {
                        Ok(cpu_resolution(requested_policy, cuda_compiled, devices, reason))
                    }
                },
                None => Ok(cpu_resolution(
                    requested_policy,
                    cuda_compiled,
                    devices,
                    DeviceResolutionReason::NoVisibleCudaDevice,
                )),
            },
            Err(reason) => Ok(cpu_resolution(requested_policy, cuda_compiled, Vec::new(), reason)),
        },
        EmbedDevicePolicy::Cuda(ordinal) => match enumerate(provider) {
            Ok(devices) => match devices.iter().find(|device| device.visible_ordinal == ordinal) {
                Some(device) => match probe(provider, device) {
                    Ok(info) => {
                        Ok(selected_resolution(requested_policy, cuda_compiled, devices, info))
                    }
                    Err(reason) => {
                        Err(DeviceResolutionError::ForcedCudaUnavailable { ordinal, reason })
                    }
                },
                None => Err(DeviceResolutionError::ForcedCudaUnavailable {
                    ordinal,
                    reason: DeviceResolutionReason::NoVisibleCudaDevice,
                }),
            },
            Err(reason) => Err(DeviceResolutionError::ForcedCudaUnavailable { ordinal, reason }),
        },
    }
}

fn cpu_resolution(
    requested_policy: EmbedDevicePolicy,
    cuda_compiled: bool,
    visible_cuda_devices: Vec<CudaVisibleDevice>,
    reason: DeviceResolutionReason,
) -> DeviceResolution {
    DeviceResolution {
        requested_policy,
        cuda_compiled,
        effective_device: EffectiveEmbedDevice::Cpu,
        visible_cuda_devices,
        selected_cuda_uuid: None,
        reason: Some(reason),
    }
}

fn selected_resolution(
    requested_policy: EmbedDevicePolicy,
    cuda_compiled: bool,
    visible_cuda_devices: Vec<CudaVisibleDevice>,
    info: CudaDeviceInfo,
) -> DeviceResolution {
    let selected_cuda_uuid = info.uuid.clone();
    DeviceResolution {
        requested_policy,
        cuda_compiled,
        effective_device: EffectiveEmbedDevice::Cuda(info),
        visible_cuda_devices,
        selected_cuda_uuid,
        reason: None,
    }
}

/// Parse and resolve `FATHOMDB_EMBED_DEVICE` exactly once for a default
/// embedder construction.
///
/// An unset setting means [`EmbedDevicePolicy::Auto`]. The injected provider
/// keeps the environment transport separate from actual CUDA initialization,
/// so `cpu` and CPU-only `auto` still make no provider call.
pub fn resolve_embed_device_policy_from_env(
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> Result<DeviceResolution, EmbedDevicePolicyError> {
    let raw = std::env::var("FATHOMDB_EMBED_DEVICE").unwrap_or_else(|_| "auto".to_string());
    let policy = raw.parse::<EmbedDevicePolicy>().map_err(EmbedDevicePolicyError::InvalidPolicy)?;
    resolve_embed_device_policy(policy, cuda_compiled, provider)
        .map_err(EmbedDevicePolicyError::Resolution)
}

fn enumerate(
    provider: &mut dyn CudaProvider,
) -> Result<Vec<CudaVisibleDevice>, DeviceResolutionReason> {
    match provider.enumerate_visible_cuda_devices() {
        Ok(devices) => Ok(devices),
        Err(CudaProbeError::NoVisibleDevice) => Ok(Vec::new()),
        Err(CudaProbeError::Incompatible { .. }) => Err(DeviceResolutionReason::CudaIncompatible),
        Err(CudaProbeError::ProbeFailed { .. }) => Err(DeviceResolutionReason::CudaProbeFailed),
    }
}

fn probe(
    provider: &mut dyn CudaProvider,
    selected: &CudaVisibleDevice,
) -> Result<CudaDeviceInfo, DeviceResolutionReason> {
    match provider.probe_cuda(selected.visible_ordinal) {
        Ok(info)
            if info.ordinal == selected.visible_ordinal
                && info.uuid.as_deref() == Some(selected.uuid.as_str()) =>
        {
            Ok(info)
        }
        Ok(_) | Err(CudaProbeError::ProbeFailed { .. }) => {
            Err(DeviceResolutionReason::CudaProbeFailed)
        }
        Err(CudaProbeError::NoVisibleDevice) => Err(DeviceResolutionReason::NoVisibleCudaDevice),
        Err(CudaProbeError::Incompatible { .. }) => Err(DeviceResolutionReason::CudaIncompatible),
    }
}

/// Diagnose one parsed policy from raw CUDA inventory/probe evidence.
///
/// Unlike normal open resolution, an automatic provider failure remains a
/// `probe_failed` diagnostic with exit 70. This function does not construct an
/// engine, open a database, load a model, or write configuration.
#[must_use]
pub fn diagnose_gpu(
    requested_policy: EmbedDevicePolicy,
    cuda_compiled: bool,
    provider: &mut dyn CudaProvider,
) -> DoctorGpuDiagnosticResult {
    let policy = policy_string(requested_policy);
    match requested_policy {
        EmbedDevicePolicy::Cpu => diagnostic(
            policy,
            cuda_compiled,
            DoctorGpuStatus::SelectedCpuNoCuda,
            Some(EffectiveEmbedDevice::Cpu),
            Vec::new(),
            None,
            None,
        ),
        EmbedDevicePolicy::Auto if !cuda_compiled => diagnostic(
            policy,
            cuda_compiled,
            DoctorGpuStatus::CudaNotCompiled,
            Some(EffectiveEmbedDevice::Cpu),
            Vec::new(),
            None,
            Some(DeviceResolutionReason::CudaNotCompiled),
        ),
        EmbedDevicePolicy::Cuda(_) if !cuda_compiled => diagnostic(
            policy,
            cuda_compiled,
            DoctorGpuStatus::CudaNotCompiled,
            None,
            Vec::new(),
            None,
            Some(DeviceResolutionReason::CudaNotCompiled),
        ),
        EmbedDevicePolicy::Auto | EmbedDevicePolicy::Cuda(_) => {
            let forced_ordinal = match requested_policy {
                EmbedDevicePolicy::Cuda(ordinal) => Some(ordinal),
                _ => None,
            };
            let devices = match provider.enumerate_visible_cuda_devices() {
                Ok(devices) => devices,
                Err(CudaProbeError::NoVisibleDevice) => {
                    return diagnostic(
                        policy,
                        cuda_compiled,
                        DoctorGpuStatus::CudaUnavailable,
                        forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                        Vec::new(),
                        None,
                        Some(DeviceResolutionReason::NoVisibleCudaDevice),
                    );
                }
                Err(CudaProbeError::Incompatible { .. }) => {
                    return diagnostic(
                        policy,
                        cuda_compiled,
                        DoctorGpuStatus::CudaIncompatible,
                        forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                        Vec::new(),
                        None,
                        Some(DeviceResolutionReason::CudaIncompatible),
                    );
                }
                Err(CudaProbeError::ProbeFailed { .. }) => {
                    return diagnostic(
                        policy,
                        cuda_compiled,
                        DoctorGpuStatus::ProbeFailed,
                        forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                        Vec::new(),
                        None,
                        Some(DeviceResolutionReason::CudaProbeFailed),
                    );
                }
            };
            let selected = match forced_ordinal {
                Some(ordinal) => {
                    devices.iter().find(|device| device.visible_ordinal == ordinal).cloned()
                }
                None => devices.iter().min_by_key(|device| device.visible_ordinal).cloned(),
            };
            let Some(selected) = selected else {
                return diagnostic(
                    policy,
                    cuda_compiled,
                    DoctorGpuStatus::CudaUnavailable,
                    forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                    devices,
                    None,
                    Some(DeviceResolutionReason::NoVisibleCudaDevice),
                );
            };
            match provider.probe_cuda(selected.visible_ordinal) {
                Ok(info)
                    if info.ordinal == selected.visible_ordinal
                        && info.uuid.as_deref() == Some(selected.uuid.as_str()) =>
                {
                    diagnostic(
                        policy,
                        cuda_compiled,
                        DoctorGpuStatus::SelectedCuda,
                        Some(EffectiveEmbedDevice::Cuda(info)),
                        devices,
                        Some(selected.uuid.clone()),
                        None,
                    )
                }
                Err(CudaProbeError::Incompatible { .. }) => diagnostic(
                    policy,
                    cuda_compiled,
                    DoctorGpuStatus::CudaIncompatible,
                    forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                    devices,
                    None,
                    Some(DeviceResolutionReason::CudaIncompatible),
                ),
                Err(CudaProbeError::NoVisibleDevice) => diagnostic(
                    policy,
                    cuda_compiled,
                    DoctorGpuStatus::CudaUnavailable,
                    forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                    devices,
                    None,
                    Some(DeviceResolutionReason::NoVisibleCudaDevice),
                ),
                Ok(_) | Err(CudaProbeError::ProbeFailed { .. }) => diagnostic(
                    policy,
                    cuda_compiled,
                    DoctorGpuStatus::ProbeFailed,
                    forced_ordinal.is_none().then_some(EffectiveEmbedDevice::Cpu),
                    devices,
                    None,
                    Some(DeviceResolutionReason::CudaProbeFailed),
                ),
            }
        }
    }
}

fn policy_string(policy: EmbedDevicePolicy) -> String {
    match policy {
        EmbedDevicePolicy::Auto => "auto".to_owned(),
        EmbedDevicePolicy::Cpu => "cpu".to_owned(),
        EmbedDevicePolicy::Cuda(ordinal) => format!("cuda:{ordinal}"),
    }
}

fn diagnostic(
    policy: String,
    cuda_compiled: bool,
    status: DoctorGpuStatus,
    effective_device: Option<EffectiveEmbedDevice>,
    devices: Vec<CudaVisibleDevice>,
    selected_uuid: Option<String>,
    reason: Option<DeviceResolutionReason>,
) -> DoctorGpuDiagnosticResult {
    DoctorGpuDiagnosticResult {
        policy,
        cuda_compiled,
        status,
        effective_device,
        devices,
        selected_uuid,
        reason,
    }
}
