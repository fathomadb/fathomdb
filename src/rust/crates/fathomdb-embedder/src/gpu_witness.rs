//! Current method-specific, in-process CUDA allocation proof.
//!
//! The actual held context's hardware and allocator select the method before
//! measurements. Integrated private pools use checked current USED; discrete
//! non-private allocators retain the bounded raw-control/global-free method.
//! No result selects a fallback. Identity, the model floor and a real forward
//! remain mandatory. The technical contract is
//! `dev/adr/ADR-0.8.28-attributable-cuda-allocation-witness.md`.
//!
//! Pure observation/evaluation/serialization is available without CUDA. Only
//! concrete driver operations and the installed-artifact runner are gated by
//! `embed-cuda`; CPU tests never manufacture GPU evidence.

use std::fmt;

use crate::device_policy::{DeviceResolutionReason, EffectiveEmbedDevice};
use crate::gpu_witness_measurement::{
    GpuAllocationMeasurement, PRIVATE_POOL_EXCLUSIVITY_PRECONDITION,
};

/// Current method-tagged record schema, separate from the CUDA envelope schema.
pub const TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA: &str = "fathomdb.tegra-gpu-allocation-witness/v2";

/// Minimum model allocation growth, 64 MiB, retained by both witness methods.
///
/// The pinned model's measured live footprint is approximately 127 MiB. The
/// current method-specific proof never weakens this floor to obtain a pass.
pub const DEFAULT_DELTA_FLOOR_BYTES: u64 = 67_108_864;

/// Default raw control block for the discrete global-free method, 1 GiB.
///
/// The integrated private-pool method uses one exact 64 MiB same-pool control
/// instead; a raw allocation cannot calibrate that pool's USED counter.
pub const DEFAULT_CONTROL_ALLOCATION_BYTES: u64 = 1_073_741_824;

/// Maximum resident raw blocks for the discrete global-free control.
///
/// An uncharged bounded control is a refusal, never an allocation retry loop.
pub const MAX_CONTROL_BLOCKS: usize = 16;

/// The precondition D-80.5-3 states rather than assumes, retained inside the
/// record so a later reader judges the evidence instead of inheriting a silent
/// assumption.
pub const SOLE_GPU_CONSUMER_PRECONDITION: &str =
    "the witness process must be the sole GPU consumer of the selected discrete device";

/// `CUDA_ERROR_INVALID_CONTEXT`, measured in § 2.7 as the status
/// `cuMemGetInfo` returns with no current context.
pub const CUDA_ERROR_INVALID_CONTEXT: u32 = 201;

/// The dimension of the pinned default embedder, asserted by the forward pass.
pub const WITNESS_VECTOR_DIM: usize = 384;

/// One `cuMemGetInfo` observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuMemorySample {
    /// Free device memory in bytes at sample time.
    pub free_bytes: u64,
    /// Total device memory in bytes; unified system memory on an iGPU.
    pub total_bytes: u64,
}

/// The injected GPU memory-counter boundary.
///
/// The production implementation is [`CudaDriverMemorySampler`]; tests inject a
/// scripted one, exactly as `FixtureCudaProvider` stands in for
/// `CandleCudaProvider` on the `CudaProvider` seam.
pub trait GpuMemorySampler {
    /// Read free/total device memory for the current CUDA context.
    ///
    /// Implementations must surface a missing context as
    /// [`GpuWitnessError::NoCudaContext`] and never as a zero sample.
    fn sample(&mut self) -> Result<GpuMemorySample, GpuWitnessError>;
}

/// The injected control-allocation boundary (D-80.5-3).
pub trait GpuControlAllocator {
    /// Allocate one more block of `bytes` on the witnessed device and keep
    /// every block allocated so far resident until
    /// [`GpuControlAllocator::release`] is called.
    fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError>;

    /// Release every block this allocator is holding. Infallible on purpose:
    /// a failure to release is not evidence about the load.
    fn release(&mut self);
}

/// The bracketed control allocation, retained raw so its verdict is
/// re-derivable (R80-13).
///
/// `free_before_bytes`/`free_after_bytes` bracket the **decisive** block — the
/// one the counter charged for in full — and `block_count` says how many
/// blocks it took to get there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlAllocationObservation {
    /// Bytes deliberately requested per block on the witnessed device.
    pub requested_bytes: u64,
    /// How many blocks were allocated, the last of which was charged in full.
    pub block_count: usize,
    /// Free bytes immediately before the decisive block.
    pub free_before_bytes: u64,
    /// Free bytes while the decisive block is still resident.
    pub free_after_bytes: u64,
}

impl ControlAllocationObservation {
    /// How far the shared counter moved for the control allocation.
    #[must_use]
    pub const fn delta_bytes(&self) -> i128 {
        self.free_before_bytes as i128 - self.free_after_bytes as i128
    }
}

/// Which ordered observation of D-80.5-1 is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessStage {
    /// Step 3 — the sample taken after the device exists and before the load.
    LoadBefore,
    /// Step 5 — the sample taken immediately after the load.
    LoadAfter,
    /// The bracketed control allocation of D-80.5-3.
    ControlAllocation,
    /// Step 6 — the real forward pass and its vector dimension.
    ForwardPass,
}

impl WitnessStage {
    /// Stable lower-snake-case stage tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LoadBefore => "load_before",
            Self::LoadAfter => "load_after",
            Self::ControlAllocation => "control_allocation",
            Self::ForwardPass => "forward_pass",
        }
    }
}

impl fmt::Display for WitnessStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Every non-pass outcome of the witness (R80-12).
///
/// There is deliberately no warning-shaped variant: anything other than a
/// proven CUDA engagement is one of these, and each carries the numbers a
/// reader needs to re-derive the refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuWitnessError {
    /// Device policy resolved to CPU, so there is nothing to witness (AC80-16).
    CpuFallback {
        /// The typed resolution reason, or `unspecified` when policy asked for CPU.
        reason: String,
    },
    /// The memory counter was read with no current CUDA context (AC80-17).
    NoCudaContext {
        /// Driver-reported message.
        message: String,
    },
    /// Candle retained a different ordinal than the one requested.
    OrdinalMismatch {
        /// The ordinal the policy requested.
        requested: usize,
        /// The ordinal Candle actually retained.
        retained: usize,
    },
    /// The probe-time and retained-device UUIDs do not correlate after
    /// normalization (§ 2.7).
    UuidMismatch {
        /// UUID reported when the device was probed.
        probed: String,
        /// UUID read back for the ordinal Candle retained.
        retained: String,
    },
    /// A required ordered observation is absent; never treated as zero.
    MissingSample {
        /// Which observation is missing.
        stage: WitnessStage,
    },
    /// The load moved the counter by less than the declared floor (D-80.5-2).
    InsufficientDelta {
        /// Observed method-specific movement across the load.
        delta_bytes: i128,
        /// The declared floor this run was judged against.
        floor_bytes: u64,
    },
    /// The counter did not move by at least the deliberate control allocation,
    /// so the load delta is unattributable on this host (D-80.5-3).
    ControlAllocationNotObserved {
        /// Bytes deliberately requested.
        requested_bytes: u64,
        /// Bytes the counter actually moved.
        delta_bytes: i128,
    },
    /// Actual hardware/allocator ownership does not support a witness method.
    UnsupportedAllocationMethod {
        /// Checked actual INTEGRATED attribute.
        device_integrated: bool,
        /// Whether the actual retained allocator is Private.
        allocator_private: bool,
        /// Whether that context owns a private pool.
        pool_present: bool,
    },
    /// The driver, the device probe, or the forward pass failed.
    ProbeFailed {
        /// Diagnostic detail.
        message: String,
    },
}

impl GpuWitnessError {
    /// Stable lower-snake-case failure tag for machine-readable surfaces.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CpuFallback { .. } => "cpu_fallback",
            Self::NoCudaContext { .. } => "no_cuda_context",
            Self::OrdinalMismatch { .. } => "ordinal_mismatch",
            Self::UuidMismatch { .. } => "uuid_mismatch",
            Self::MissingSample { .. } => "missing_sample",
            Self::InsufficientDelta { .. } => "insufficient_delta",
            Self::ControlAllocationNotObserved { .. } => "control_allocation_not_observed",
            Self::UnsupportedAllocationMethod { .. } => "unsupported_allocation_method",
            Self::ProbeFailed { .. } => "probe_failed",
        }
    }

    /// Classify a raw CUDA driver status into a named witness failure.
    ///
    /// Pure on purpose: `CUDA_ERROR_INVALID_CONTEXT` is the § 2.7 measurement
    /// this slice is built on, so AC80-17 is assertable with no driver present.
    /// Mirrors `classify_cuda_driver_error` (`candle_bge.rs`) in shape.
    #[must_use]
    pub fn from_driver_status(status: u32, message: &str) -> Self {
        if status == CUDA_ERROR_INVALID_CONTEXT {
            Self::NoCudaContext { message: message.to_owned() }
        } else {
            Self::ProbeFailed { message: format!("CUDA driver status {status}: {message}") }
        }
    }
}

impl fmt::Display for GpuWitnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuFallback { reason } => {
                write!(formatter, "gpu witness resolved to CPU ({reason}); no witness was written")
            }
            Self::NoCudaContext { message } => {
                write!(formatter, "gpu memory was sampled with no current CUDA context: {message}")
            }
            Self::OrdinalMismatch { requested, retained } => {
                write!(formatter, "requested cuda:{requested} but Candle retained cuda:{retained}")
            }
            Self::UuidMismatch { probed, retained } => write!(
                formatter,
                "probed device UUID {probed} does not correlate with retained {retained}"
            ),
            Self::MissingSample { stage } => {
                write!(formatter, "the {stage} observation is missing")
            }
            Self::InsufficientDelta { delta_bytes, floor_bytes } => write!(
                formatter,
                "allocation delta {delta_bytes} is below the declared floor {floor_bytes}"
            ),
            Self::ControlAllocationNotObserved { requested_bytes, delta_bytes } => write!(
                formatter,
                "control allocation of {requested_bytes} bytes moved the counter by only \
                 {delta_bytes}; the load delta is unattributable"
            ),
            Self::UnsupportedAllocationMethod {
                device_integrated,
                allocator_private,
                pool_present,
            } => {
                write!(formatter, "unsupported witness allocation method: integrated={device_integrated}, private={allocator_private}, owned_pool={pool_present}")
            }
            Self::ProbeFailed { message } => {
                write!(formatter, "gpu witness probe failed: {message}")
            }
        }
    }
}

impl std::error::Error for GpuWitnessError {}

/// The one legitimate non-failure outcome: no device to witness (R80-3,
/// AC80-20). Named so a skip can never be mistaken for a pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuWitnessSkip {
    /// This artifact has no CUDA provider compiled in.
    CudaNotCompiled,
    /// CUDA is compiled in but no device is visible to this process.
    NoVisibleCudaDevice,
    /// The operator did not opt this host into the real-hardware arm.
    NotOptedIn,
}

impl GpuWitnessSkip {
    /// Stable lower-snake-case skip reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CudaNotCompiled => "cuda_not_compiled",
            Self::NoVisibleCudaDevice => "no_visible_cuda_device",
            Self::NotOptedIn => "not_opted_in",
        }
    }
}

impl fmt::Display for GpuWitnessSkip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SKIP tegra-gpu-allocation-witness: {}", self.as_str())
    }
}

/// Everything the pure verdict needs, collected in the D-80.5-1 order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllocationWitnessInputs {
    /// Process that collected these observations.
    pub process_id: u32,
    /// Checked INTEGRATED attribute of the actual held device.
    pub device_integrated: bool,
    /// The CUDA ordinal the caller asked for.
    pub requested_ordinal: usize,
    /// The typed device resolution outcome.
    pub effective_device: EffectiveEmbedDevice,
    /// Why CPU was selected, when it was.
    pub cpu_reason: Option<DeviceResolutionReason>,
    /// The ordinal Candle actually retained (`device.location()`).
    pub retained_ordinal: usize,
    /// UUID read from the driver for the retained ordinal.
    pub retained_device_uuid: String,
    /// Actual global diagnostic sample before the load.
    pub before: Option<GpuMemorySample>,
    /// Actual global diagnostic sample after checked model completion.
    pub after: Option<GpuMemorySample>,
    /// One method-specific observation set; missing observations never become zero.
    pub measurement: Option<GpuAllocationMeasurement>,
    /// The declared floor this run is judged against (D-80.5-2).
    pub delta_floor_bytes: u64,
    /// Step 6 — the dimension of one real forward pass.
    pub embedded_vector_dim: Option<usize>,
}

/// The retained witness record. Every field the verdict used is present, so a
/// reader re-derives the verdict rather than trusting it (R80-13, AC80-15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuAllocationWitness {
    /// PID of this witness process.
    pub process_id: u32,
    /// Checked INTEGRATED attribute of the witnessed device.
    pub device_integrated: bool,
    /// The CUDA ordinal requested by policy.
    pub device_ordinal_requested: usize,
    /// The CUDA ordinal Candle actually retained.
    pub device_ordinal_actual: usize,
    /// Driver-API UUID, rendered in this repo's `GPU-`-prefixed form.
    pub device_uuid: String,
    /// Driver-API device name (`Orin` on this host).
    pub device_name: String,
    /// Driver-API compute capability (`8.7` on this host).
    pub compute_capability: String,
    /// Global free bytes before the load; diagnostic on the private method.
    pub free_before_bytes: u64,
    /// Global free bytes after the load; diagnostic on the private method.
    pub free_after_bytes: u64,
    /// Total device bytes; unified system memory on an iGPU.
    pub total_bytes: u64,
    /// `free_before_bytes - free_after_bytes`.
    pub delta_bytes: i128,
    /// The declared floor the delta was judged against.
    pub delta_floor_bytes: u64,
    /// Exactly one raw method-specific proof, never a bag of optional fields.
    pub measurement: GpuAllocationMeasurement,
    /// Dimension produced by one real forward pass on the device.
    pub embedded_vector_dim: usize,
}

impl GpuAllocationWitness {
    /// Serialize as canonical JSON: ASCII, compact separators, sorted keys,
    /// one trailing newline — byte-identical to
    /// `json.dumps(..., sort_keys=True, separators=(",", ":")) + "\n"`, which
    /// is what the verifier round-trips.
    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        let mut out = String::with_capacity(768);
        out.push('{');
        push_string_field(&mut out, "compute_capability", &self.compute_capability, true);
        push_i128_field(&mut out, "delta_bytes", self.delta_bytes, false);
        push_u64_field(&mut out, "delta_floor_bytes", self.delta_floor_bytes, false);
        out.push_str(",\"device_integrated\":");
        out.push_str(if self.device_integrated { "true" } else { "false" });
        push_string_field(&mut out, "device_name", &self.device_name, false);
        push_usize_field(&mut out, "device_ordinal_actual", self.device_ordinal_actual, false);
        push_usize_field(
            &mut out,
            "device_ordinal_requested",
            self.device_ordinal_requested,
            false,
        );
        push_string_field(&mut out, "device_uuid", &self.device_uuid, false);
        push_usize_field(&mut out, "embedded_vector_dim", self.embedded_vector_dim, false);
        push_u64_field(&mut out, "free_after_bytes", self.free_after_bytes, false);
        push_u64_field(&mut out, "free_before_bytes", self.free_before_bytes, false);
        out.push_str(",\"measurement\":{");
        match &self.measurement {
            GpuAllocationMeasurement::GlobalFree {
                control_allocation_request_bytes,
                control_block_count,
                control_delta_bytes,
                control_free_before_bytes,
                control_free_after_bytes,
                sole_gpu_consumer_precondition,
            } => {
                push_u64_field(
                    &mut out,
                    "control_allocation_request_bytes",
                    *control_allocation_request_bytes,
                    true,
                );
                push_usize_field(&mut out, "control_block_count", *control_block_count, false);
                push_i128_field(&mut out, "control_delta_bytes", *control_delta_bytes, false);
                push_u64_field(
                    &mut out,
                    "control_free_after_bytes",
                    *control_free_after_bytes,
                    false,
                );
                push_u64_field(
                    &mut out,
                    "control_free_before_bytes",
                    *control_free_before_bytes,
                    false,
                );
                push_string_field(&mut out, "kind", self.measurement.kind(), false);
                push_string_field(
                    &mut out,
                    "sole_gpu_consumer_precondition",
                    sole_gpu_consumer_precondition,
                    false,
                );
            }
            GpuAllocationMeasurement::PrivatePoolUsed {
                context_id,
                pool_identity,
                used_before_bytes,
                used_after_bytes,
                delta_bytes,
                control_request_bytes,
                control_before_bytes,
                control_live_bytes,
                control_released_bytes,
                pool_exclusivity_precondition,
            } => {
                push_string_field(&mut out, "context_id", context_id, true);
                push_u64_field(&mut out, "control_before_bytes", *control_before_bytes, false);
                push_u64_field(&mut out, "control_live_bytes", *control_live_bytes, false);
                push_u64_field(&mut out, "control_released_bytes", *control_released_bytes, false);
                push_u64_field(&mut out, "control_request_bytes", *control_request_bytes, false);
                push_i128_field(&mut out, "delta_bytes", *delta_bytes, false);
                push_string_field(&mut out, "kind", self.measurement.kind(), false);
                push_string_field(
                    &mut out,
                    "pool_exclusivity_precondition",
                    pool_exclusivity_precondition,
                    false,
                );
                push_string_field(&mut out, "pool_identity", pool_identity, false);
                push_u64_field(&mut out, "used_after_bytes", *used_after_bytes, false);
                push_u64_field(&mut out, "used_before_bytes", *used_before_bytes, false);
            }
        }
        out.push('}');
        push_u64_field(&mut out, "process_id", u64::from(self.process_id), false);
        push_string_field(&mut out, "schema", TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA, false);
        push_u64_field(&mut out, "total_bytes", self.total_bytes, false);
        out.push_str("}\n");
        out
    }
}

fn push_separator(out: &mut String, first: bool) {
    if !first {
        out.push(',');
    }
}

fn push_key(out: &mut String, key: &str, first: bool) {
    push_separator(out, first);
    push_json_string(out, key);
    out.push(':');
}

fn push_string_field(out: &mut String, key: &str, value: &str, first: bool) {
    push_key(out, key, first);
    push_json_string(out, value);
}

fn push_u64_field(out: &mut String, key: &str, value: u64, first: bool) {
    push_key(out, key, first);
    out.push_str(&value.to_string());
}

fn push_usize_field(out: &mut String, key: &str, value: usize, first: bool) {
    push_key(out, key, first);
    out.push_str(&value.to_string());
}

fn push_i128_field(out: &mut String, key: &str, value: i128, first: bool) {
    push_key(out, key, first);
    out.push_str(&value.to_string());
}

/// Minimal RFC 8259 string escaping with `ensure_ascii` semantics; the witness
/// only ever carries driver-reported ASCII identifiers, and anything else is
/// escaped rather than emitted raw so the canonical round-trip holds.
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 || (control as u32) > 0x7e => {
                let mut buffer = [0_u16; 2];
                for unit in control.encode_utf16(&mut buffer) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Normalize a CUDA UUID for comparison.
///
/// § 2.7 measured that Tegra's `nvidia-smi --query-gpu=uuid` omits the `GPU-`
/// prefix that x86_64 reports and that this repo's `cuda_uuid_string` renders.
/// The driver bytes are identical, so identity comparison normalizes rather
/// than string-equals — the exact assumption
/// `verify-cuda-preflight-witness.py:335` makes and that cannot hold here.
#[must_use]
pub fn normalize_cuda_uuid(raw: &str) -> String {
    let trimmed = raw.trim();
    let lowered = trimmed.to_ascii_lowercase();
    lowered.strip_prefix("gpu-").unwrap_or(&lowered).to_owned()
}

/// Allocate deliberate blocks of a known size until the counter charges for
/// one in full (D-80.5-3).
///
/// The design says "one control allocation of a known size … confirm the
/// counter moves by at least that much". Implementing 80.5 on this Orin
/// measured why a single block is not enough: Tegra's kernel-side page pool
/// absorbs allocations without charging system memory until it is drained
/// (ten consecutive 256 MiB allocations each moved the counter by ~1 MB).
/// A single absorbed block would report "counter not live" on hardware where
/// it very much is. Allocating blocks until one is charged in full keeps the
/// design's assertion exactly as written — the decisive block moved the
/// counter by at least its own size — and additionally leaves the pool empty,
/// so the model load that follows cannot hide inside it either. Both effects
/// are necessary: with a spare pool, the measured load delta on this host was
/// 6_189_056 bytes for a 133_466_304-byte model; with the pool drained it is
/// ~143_600_000 bytes.
///
/// On success the blocks stay **resident**: the caller releases them after the
/// post-load sample, so the load cannot be satisfied out of memory this
/// witness itself returned. On failure they are released here.
pub fn observe_control_allocation(
    sampler: &mut dyn GpuMemorySampler,
    allocator: &mut dyn GpuControlAllocator,
    block_bytes: u64,
    max_blocks: usize,
) -> Result<ControlAllocationObservation, GpuWitnessError> {
    let mut previous = match sampler.sample() {
        Ok(sample) => sample,
        Err(error) => {
            allocator.release();
            return Err(error);
        }
    };
    let mut largest_delta = 0_i128;
    for block_count in 1..=max_blocks {
        if let Err(error) = allocator.allocate(block_bytes) {
            allocator.release();
            return Err(error);
        }
        let current = match sampler.sample() {
            Ok(sample) => sample,
            Err(error) => {
                allocator.release();
                return Err(error);
            }
        };
        let delta = i128::from(previous.free_bytes) - i128::from(current.free_bytes);
        if delta >= i128::from(block_bytes) {
            return Ok(ControlAllocationObservation {
                requested_bytes: block_bytes,
                block_count,
                free_before_bytes: previous.free_bytes,
                free_after_bytes: current.free_bytes,
            });
        }
        largest_delta = largest_delta.max(delta);
        previous = current;
    }
    allocator.release();
    Err(GpuWitnessError::ControlAllocationNotObserved {
        requested_bytes: block_bytes,
        delta_bytes: largest_delta,
    })
}

/// The pure verdict: ordered observations in, a witness or a named failure out.
///
/// Check common identity/observations, then re-derive the selected method
/// without substituting global readings for private-pool allocation proof.
pub fn evaluate_allocation_witness(
    inputs: AllocationWitnessInputs,
) -> Result<GpuAllocationWitness, GpuWitnessError> {
    let device = match &inputs.effective_device {
        EffectiveEmbedDevice::Cpu => {
            return Err(GpuWitnessError::CpuFallback {
                reason: inputs
                    .cpu_reason
                    .map_or_else(|| "unspecified".to_owned(), |reason| reason.as_str().to_owned()),
            });
        }
        EffectiveEmbedDevice::Cuda(info) => info,
    };

    if device.ordinal != inputs.requested_ordinal
        || inputs.retained_ordinal != inputs.requested_ordinal
    {
        return Err(GpuWitnessError::OrdinalMismatch {
            requested: inputs.requested_ordinal,
            retained: inputs.retained_ordinal,
        });
    }

    let probed_uuid = device.uuid.clone().ok_or_else(|| GpuWitnessError::ProbeFailed {
        message: "the CUDA probe reported no device UUID".to_owned(),
    })?;
    if normalize_cuda_uuid(&probed_uuid) != normalize_cuda_uuid(&inputs.retained_device_uuid) {
        return Err(GpuWitnessError::UuidMismatch {
            probed: probed_uuid,
            retained: inputs.retained_device_uuid,
        });
    }
    let device_name = device.name.clone().ok_or_else(|| GpuWitnessError::ProbeFailed {
        message: "the CUDA probe reported no device name".to_owned(),
    })?;
    let compute_capability =
        device.compute_capability.clone().ok_or_else(|| GpuWitnessError::ProbeFailed {
            message: "the CUDA probe reported no compute capability".to_owned(),
        })?;

    let before =
        inputs.before.ok_or(GpuWitnessError::MissingSample { stage: WitnessStage::LoadBefore })?;
    let after =
        inputs.after.ok_or(GpuWitnessError::MissingSample { stage: WitnessStage::LoadAfter })?;
    let measurement = inputs
        .measurement
        .ok_or(GpuWitnessError::MissingSample { stage: WitnessStage::ControlAllocation })?;
    let dimension = inputs
        .embedded_vector_dim
        .ok_or(GpuWitnessError::MissingSample { stage: WitnessStage::ForwardPass })?;

    if before.total_bytes != after.total_bytes {
        return Err(GpuWitnessError::ProbeFailed {
            message: format!(
                "total device memory changed between samples: {} then {}",
                before.total_bytes, after.total_bytes
            ),
        });
    }
    if dimension != WITNESS_VECTOR_DIM {
        return Err(GpuWitnessError::ProbeFailed {
            message: format!(
                "the forward pass produced {dimension} dimensions, expected {WITNESS_VECTOR_DIM}"
            ),
        });
    }

    let delta = i128::from(before.free_bytes) - i128::from(after.free_bytes);
    if inputs.process_id == 0 || inputs.delta_floor_bytes < DEFAULT_DELTA_FLOOR_BYTES {
        return Err(GpuWitnessError::ProbeFailed {
            message: "witness requires a process ID and at least the 64 MiB model floor".into(),
        });
    }
    if before.total_bytes == 0
        || before.free_bytes > before.total_bytes
        || after.free_bytes > after.total_bytes
    {
        return Err(GpuWitnessError::ProbeFailed {
            message: "global diagnostic samples exceed total memory".into(),
        });
    }
    let model_delta = validate_measurement(&measurement, inputs.device_integrated, delta)?;
    if model_delta < i128::from(inputs.delta_floor_bytes) {
        return Err(GpuWitnessError::InsufficientDelta {
            delta_bytes: model_delta,
            floor_bytes: inputs.delta_floor_bytes,
        });
    }

    Ok(GpuAllocationWitness {
        process_id: inputs.process_id,
        device_integrated: inputs.device_integrated,
        device_ordinal_requested: inputs.requested_ordinal,
        device_ordinal_actual: inputs.retained_ordinal,
        device_uuid: probed_uuid,
        device_name,
        compute_capability,
        free_before_bytes: before.free_bytes,
        free_after_bytes: after.free_bytes,
        total_bytes: before.total_bytes,
        delta_bytes: delta,
        delta_floor_bytes: inputs.delta_floor_bytes,
        measurement,
        embedded_vector_dim: dimension,
    })
}

fn validate_measurement(
    measurement: &GpuAllocationMeasurement,
    integrated: bool,
    global_delta: i128,
) -> Result<i128, GpuWitnessError> {
    let invalid = |message: &str| GpuWitnessError::ProbeFailed { message: message.into() };
    match measurement {
        GpuAllocationMeasurement::GlobalFree {
            control_allocation_request_bytes: requested,
            control_block_count,
            control_free_before_bytes: before,
            control_free_after_bytes: after,
            control_delta_bytes: delta,
            sole_gpu_consumer_precondition,
        } => {
            if integrated || sole_gpu_consumer_precondition != SOLE_GPU_CONSUMER_PRECONDITION {
                return Err(invalid("global_free requires a discrete device and its canonical exclusivity precondition"));
            }
            if *requested == 0
                || !(1..=MAX_CONTROL_BLOCKS).contains(control_block_count)
                || *before == 0
                || *after == 0
                || *delta != i128::from(*before) - i128::from(*after)
            {
                return Err(invalid("global control observations are invalid or not re-derivable"));
            }
            if *delta < i128::from(*requested) {
                return Err(GpuWitnessError::ControlAllocationNotObserved {
                    requested_bytes: *requested,
                    delta_bytes: *delta,
                });
            }
            Ok(global_delta)
        }
        GpuAllocationMeasurement::PrivatePoolUsed {
            context_id,
            pool_identity,
            used_before_bytes,
            used_after_bytes,
            delta_bytes,
            control_request_bytes,
            control_before_bytes,
            control_live_bytes,
            control_released_bytes,
            pool_exclusivity_precondition,
        } => {
            if !integrated || pool_exclusivity_precondition != PRIVATE_POOL_EXCLUSIVITY_PRECONDITION
            {
                return Err(invalid("private_pool_used requires an integrated device and its canonical pool precondition"));
            }
            if context_id.parse::<u64>().map(|id| id.to_string()).ok().as_ref() != Some(context_id)
                || pool_identity.is_empty()
            {
                return Err(invalid("private pool identity requires a canonical uint64 context ID string and a pool identity"));
            }
            if *delta_bytes != i128::from(*used_after_bytes) - i128::from(*used_before_bytes) {
                return Err(invalid("private model delta does not follow from USED observations"));
            }
            if *control_request_bytes != DEFAULT_DELTA_FLOOR_BYTES {
                return Err(invalid("private control request must be exactly 64 MiB"));
            }
            let control_delta = i128::from(*control_live_bytes) - i128::from(*control_before_bytes);
            if control_delta < i128::from(*control_request_bytes) {
                return Err(GpuWitnessError::ControlAllocationNotObserved {
                    requested_bytes: *control_request_bytes,
                    delta_bytes: control_delta,
                });
            }
            if control_released_bytes != control_before_bytes {
                return Err(invalid("private control release did not restore exact USED baseline"));
            }
            Ok(*delta_bytes)
        }
    }
}

#[cfg(any(feature = "embed-cuda", test))]
fn observe_model_load<Model>(
    sampler: &mut dyn GpuMemorySampler,
    load: impl FnOnce() -> Result<Model, GpuWitnessError>,
    complete: impl FnOnce() -> Result<(), GpuWitnessError>,
) -> Result<(GpuMemorySample, Model, GpuMemorySample), GpuWitnessError> {
    let before = sampler.sample()?;
    let model = load()?;
    complete()?;
    let after = sampler.sample()?;
    Ok((before, model, after))
}

// ---------------------------------------------------------------------------
// The driver boundary. Only this section needs a GPU, mirroring how
// `CandleCudaProvider` is the sole CUDA-touching impl of `CudaProvider`.
// ---------------------------------------------------------------------------

/// `cuMemGetInfo` through the pinned Candle fork's re-exported cudarc driver
/// API — the same path `candle_bge.rs` already uses for UUID and
/// compute-capability queries, so this adds no dependency (D-80.5-1).
#[cfg(feature = "embed-cuda")]
#[derive(Clone, Copy, Debug, Default)]
pub struct CudaDriverMemorySampler;

#[cfg(feature = "embed-cuda")]
impl GpuMemorySampler for CudaDriverMemorySampler {
    fn sample(&mut self) -> Result<GpuMemorySample, GpuWitnessError> {
        use candle_core::cuda::cudarc::driver::result;

        let (free, total) = result::mem_get_info().map_err(|error| {
            GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
        })?;
        Ok(GpuMemorySample { free_bytes: free as u64, total_bytes: total as u64 })
    }
}

/// Initialize the CUDA driver and sample the memory counter **without**
/// creating a context.
///
/// This exists so AC80-17 is assertable against the real driver rather than
/// only against the § 2.7 constant: on this host the call must return
/// [`GpuWitnessError::NoCudaContext`], proving that a witness which sampled
/// before constructing the Candle device would fail loudly instead of
/// recording a zero sample. Call it before any device is constructed on the
/// calling thread; afterwards a context is current and it simply succeeds.
#[cfg(feature = "embed-cuda")]
pub fn sample_with_driver_initialized_only() -> Result<GpuMemorySample, GpuWitnessError> {
    use candle_core::cuda::cudarc::driver::result;

    result::init()
        .map_err(|error| GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string()))?;
    CudaDriverMemorySampler.sample()
}

/// Deliberate `cuMemAlloc` blocks of a known size, held resident together.
///
/// It allocates through the **driver API on the witnessed context**, not
/// through a Candle tensor. Measured on this Orin while implementing 80.5: a
/// 268_435_456-byte Candle tensor requested immediately after the model load
/// moved the counter by only 167_395_328 bytes, because Candle reaches the
/// device through cudarc's stream-ordered pool and satisfied part of the
/// request from memory the forward pass had already returned to that pool. A
/// pooled allocator cannot answer the question D-80.5-3 asks — "is this
/// counter live and attributable right now?" — whereas `cuMemAlloc` is the
/// instrument § 2.7 measured.
#[cfg(feature = "embed-cuda")]
#[derive(Debug, Default)]
pub struct CudaDriverControlAllocator {
    /// Every block still resident, in allocation order. Kept as a list rather
    /// than a single pointer because the control step holds several blocks at
    /// once and none of them may leak.
    held: Vec<u64>,
}

#[cfg(feature = "embed-cuda")]
impl GpuControlAllocator for CudaDriverControlAllocator {
    fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError> {
        use candle_core::cuda::cudarc::driver::result;

        let bytes = usize::try_from(bytes).map_err(|error| GpuWitnessError::ProbeFailed {
            message: format!("control allocation size is not addressable: {error}"),
        })?;
        // SAFETY: allocation only; the returned pointer is never dereferenced
        // by Rust, and it is freed exactly once in `release` on this context.
        let pointer = unsafe { result::malloc_sync(bytes) }.map_err(|error| {
            GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
        })?;
        self.held.push(pointer);
        // The block is TOUCHED, not merely reserved: a reservation the counter
        // never charged for would prove nothing about the counter. The pattern
        // is deliberately nonzero so no zero-page sharing can serve the write.
        // SAFETY: `pointer` is the live allocation just returned above and
        // `bytes` is exactly its length.
        unsafe { result::memset_d8_sync(pointer, 0xa5, bytes) }.map_err(|error| {
            GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
        })?;
        // `cuMemsetD8` is asynchronous with respect to the host, so the write
        // — and with it the page commit the counter charges for — is not
        // guaranteed to have happened when the next sample is taken. Sample
        // ordering is this slice's whole contract, so the context is
        // synchronized rather than trusted.
        result::ctx::synchronize().map_err(|error| {
            GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
        })?;
        Ok(())
    }

    fn release(&mut self) {
        use candle_core::cuda::cudarc::driver::result;

        for pointer in std::mem::take(&mut self.held) {
            // SAFETY: each pointer came from `malloc_sync` above, was never
            // dereferenced or copied, and the list is taken so no pointer can
            // be freed twice.
            let _ = unsafe { result::free_sync(pointer) };
        }
    }
}

#[cfg(feature = "embed-cuda")]
impl Drop for CudaDriverControlAllocator {
    fn drop(&mut self) {
        self.release();
    }
}

/// How one real-hardware witness run is parameterized.
#[cfg(feature = "embed-cuda")]
#[derive(Clone, Copy, Debug)]
pub struct AllocationWitnessConfig {
    /// The CUDA ordinal to witness.
    pub ordinal: usize,
    /// The declared floor (D-80.5-2).
    pub delta_floor_bytes: u64,
    /// The deliberate control block size (D-80.5-3).
    pub control_allocation_bytes: u64,
    /// How many control blocks may be held before the counter is declared
    /// unattributable.
    pub max_control_blocks: usize,
}

#[cfg(feature = "embed-cuda")]
impl Default for AllocationWitnessConfig {
    fn default() -> Self {
        Self {
            ordinal: 0,
            delta_floor_bytes: DEFAULT_DELTA_FLOOR_BYTES,
            control_allocation_bytes: DEFAULT_CONTROL_ALLOCATION_BYTES,
            max_control_blocks: MAX_CONTROL_BLOCKS,
        }
    }
}

/// Run the full D-80.5-1 sequence against the real device and return the
/// witness, or the first named failure.
///
/// Reads the ambient `FATHOMDB_EMBED_DEVICE` policy exactly once through the
/// product's own resolver, so a run that resolves to CPU is reported as
/// [`GpuWitnessError::CpuFallback`] instead of silently witnessing nothing
/// (AC80-16). The ordinal assertion is the crate's single
/// `device.location()` check, shared with the TC-5 attested constructor.
#[cfg(feature = "embed-cuda")]
pub fn run_default_embedder_allocation_witness(
    config: AllocationWitnessConfig,
) -> Result<GpuAllocationWitness, GpuWitnessError> {
    crate::gpu_pool_diagnostic::run_optional(
        crate::gpu_pool_diagnostic::Session::enabled(),
        |diagnostic| run_default_embedder_allocation_witness_inner(config, diagnostic),
        |diagnostic, verdict| diagnostic.emit(verdict),
    )
}

#[cfg(feature = "embed-cuda")]
fn run_default_embedder_allocation_witness_inner(
    config: AllocationWitnessConfig,
    mut diagnostic: Option<&mut crate::gpu_pool_diagnostic::Session>,
) -> Result<GpuAllocationWitness, GpuWitnessError> {
    use crate::candle_bge::{attest_retained_cuda_device, CandleBgeEmbedder};
    use crate::gpu_pool_diagnostic::ObservedSampler;
    use crate::gpu_witness_private::{
        method_for_device, observe_private_model, HeldPrivatePool, WitnessMethod,
    };
    use crate::loader::load_pinned_default_embedder;
    use fathomdb_embedder_api::Embedder;

    // 1. Resolve the product policy and construct the Candle CUDA device.
    let resolution = crate::candle_bge::resolve_default_embedder_device_from_env()
        .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;
    let info = match &resolution.effective_device {
        EffectiveEmbedDevice::Cpu => {
            return Err(GpuWitnessError::CpuFallback {
                reason: resolution
                    .reason
                    .map_or_else(|| "unspecified".to_owned(), |reason| reason.as_str().to_owned()),
            });
        }
        EffectiveEmbedDevice::Cuda(info) => info.clone(),
    };

    // 2. Assert the ordinal Candle actually retained, and read its UUID back
    //    from the driver for the identity binding (R80-13).
    let (device, retained_ordinal, retained_uuid) = attest_retained_cuda_device(config.ordinal)
        .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;

    let (method, device_integrated) = method_for_device(&device)?;

    if let Some(diagnostic) = diagnostic.as_deref_mut() {
        diagnostic.attach(&device, config.ordinal, &retained_uuid);
    }

    // Reading the cache is pure host IO; doing it before the "before" sample
    // brackets the delta as tightly as possible around the device upload.
    let weights = load_pinned_default_embedder()
        .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;

    let load_model = || {
        CandleBgeEmbedder::new_from_weights_on_device(weights, device.clone())
            .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })
    };
    let forward_model = |embedder: &CandleBgeEmbedder| {
        embedder
            .embed("fathomdb tegra gpu allocation witness")
            .map(|vector| vector.len())
            .map_err(|error| GpuWitnessError::ProbeFailed { message: format!("{error:?}") })
    };
    let (before, embedder, after, dimension, measurement) = match method {
        WitnessMethod::GlobalFree => {
            let mut sampler = CudaDriverMemorySampler;
            let mut allocator = CudaDriverControlAllocator::default();
            let control = observe_control_allocation(
                &mut ObservedSampler::new(&mut sampler, diagnostic.as_deref_mut(), false),
                &mut allocator,
                config.control_allocation_bytes,
                config.max_control_blocks,
            )?;
            let (before, embedder, after) = observe_model_load(
                &mut ObservedSampler::new(&mut sampler, diagnostic.as_deref_mut(), true),
                load_model,
                || {
                    let cuda = device.as_cuda_device().map_err(|error| {
                        GpuWitnessError::ProbeFailed { message: error.to_string() }
                    })?;
                    cuda.cuda_stream().context().synchronize().map_err(|error| {
                        GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
                    })
                },
            )?;
            allocator.release();
            let dimension = forward_model(&embedder)?;
            (
                before,
                embedder,
                after,
                dimension,
                GpuAllocationMeasurement::from_global_control(control),
            )
        }
        WitnessMethod::PrivatePoolUsed => {
            let mut probe = HeldPrivatePool::new(&device, config.ordinal, &retained_uuid)?;
            let (embedder, observations) =
                observe_private_model(&mut probe, load_model, |embedder| {
                    let dimension = forward_model(embedder)?;
                    if let Some(diagnostic) = diagnostic.as_deref_mut() {
                        diagnostic.forward_completed(dimension);
                    }
                    Ok(dimension)
                })?;
            if let Some(diagnostic) = diagnostic.as_deref_mut() {
                diagnostic.retain_private_global_samples(
                    observations.before.global,
                    observations.after.global,
                );
            }
            let measurement = GpuAllocationMeasurement::PrivatePoolUsed {
                context_id: probe.context_id().to_string(),
                pool_identity: probe.pool_identity(),
                used_before_bytes: observations.before.used,
                used_after_bytes: observations.after.used,
                delta_bytes: i128::from(observations.after.used)
                    - i128::from(observations.before.used),
                control_request_bytes: crate::gpu_witness_private::CONTROL_BYTES,
                control_before_bytes: observations.control_before_bytes,
                control_live_bytes: observations.control_live_bytes,
                control_released_bytes: observations.control_released_bytes,
                pool_exclusivity_precondition: PRIVATE_POOL_EXCLUSIVITY_PRECONDITION.into(),
            };
            (
                observations.before.global,
                embedder,
                observations.after.global,
                observations.dimension,
                measurement,
            )
        }
    };

    if let Some(diagnostic) = diagnostic.as_deref_mut() {
        diagnostic.forward_completed(dimension);
    }

    let verdict = evaluate_allocation_witness(AllocationWitnessInputs {
        process_id: std::process::id(),
        device_integrated,
        requested_ordinal: config.ordinal,
        effective_device: EffectiveEmbedDevice::Cuda(info),
        cpu_reason: resolution.reason,
        retained_ordinal,
        retained_device_uuid: retained_uuid,
        before: Some(before),
        after: Some(after),
        measurement: Some(measurement),
        delta_floor_bytes: config.delta_floor_bytes,
        embedded_vector_dim: Some(dimension),
    });
    let verdict = match diagnostic {
        Some(diagnostic) => diagnostic.finish(verdict),
        None => verdict,
    };
    drop(embedder);
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct QueuedAllocation {
        visible: Cell<bool>,
        model_live: Cell<bool>,
        events: RefCell<Vec<&'static str>>,
    }

    struct Model<'a>(&'a QueuedAllocation);

    impl Drop for Model<'_> {
        fn drop(&mut self) {
            self.0.model_live.set(false);
        }
    }

    struct Sampler<'a>(&'a QueuedAllocation);

    impl GpuMemorySampler for Sampler<'_> {
        fn sample(&mut self) -> Result<GpuMemorySample, GpuWitnessError> {
            let after = !self.0.events.borrow().is_empty();
            self.0.events.borrow_mut().push(if after { "after" } else { "before" });
            if after {
                assert!(self.0.model_live.get(), "model must remain live through sampling");
            }
            Ok(GpuMemorySample {
                free_bytes: 4 * DEFAULT_DELTA_FLOOR_BYTES
                    - u64::from(self.0.visible.get()) * DEFAULT_DELTA_FLOOR_BYTES,
                total_bytes: 8 * DEFAULT_DELTA_FLOOR_BYTES,
            })
        }
    }

    impl QueuedAllocation {
        fn new() -> Self {
            Self {
                visible: Cell::new(false),
                model_live: Cell::new(false),
                events: RefCell::new(Vec::new()),
            }
        }

        fn load(&self) -> Result<Model<'_>, GpuWitnessError> {
            self.events.borrow_mut().push("load");
            self.model_live.set(true);
            Ok(Model(self))
        }

        fn complete(&self) -> Result<(), GpuWitnessError> {
            assert!(self.model_live.get());
            self.events.borrow_mut().push("complete");
            self.visible.set(true);
            Ok(())
        }
    }

    #[test]
    fn model_load_bracket_observes_completed_queued_allocation() {
        let queued = QueuedAllocation::new();
        let (before, model, after) =
            observe_model_load(&mut Sampler(&queued), || queued.load(), || queued.complete())
                .expect("completed load");
        assert_eq!(before.free_bytes - after.free_bytes, DEFAULT_DELTA_FLOOR_BYTES);
        assert_eq!(*queued.events.borrow(), ["before", "load", "complete", "after"]);
        assert!(queued.model_live.get());
        drop(model);
        assert!(!queued.model_live.get());
    }

    #[test]
    fn model_load_bracket_stops_before_sampling_on_completion_failure() {
        let queued = QueuedAllocation::new();
        let result = observe_model_load(
            &mut Sampler(&queued),
            || queued.load(),
            || {
                queued.events.borrow_mut().push("complete");
                Err(GpuWitnessError::from_driver_status(
                    CUDA_ERROR_INVALID_CONTEXT,
                    "completion failed",
                ))
            },
        );
        assert!(matches!(result, Err(GpuWitnessError::NoCudaContext { .. })));
        assert_eq!(*queued.events.borrow(), ["before", "load", "complete"]);
        assert!(!queued.model_live.get(), "failed load must release its model");
    }
}
