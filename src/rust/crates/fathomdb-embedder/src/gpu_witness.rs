//! 0.8.23 Slice 80.5 — the Tegra-portable GPU allocation witness.
//!
//! The x86_64 evidence lane proves GPU engagement by correlating the process
//! PID against `nvidia-smi --query-compute-apps`. That query reports nothing on
//! an integrated Tegra GPU (`dev/design/0.8.23-aarch64-tegra.md` § 2.5), so
//! this module supplies the portable replacement § 7 80.5 specifies: an
//! **allocation-delta plus device-identity** witness measured in-process
//! through the CUDA driver API, never through an external reporting tool
//! (R80-13).
//!
//! The module follows `device_policy.rs`'s split. Everything here — the sample
//! ordering contract, the verdict, the typed failures, the floor comparison,
//! the UUID normalization and the canonical serialization — is **pure and
//! compiled unconditionally**, so every arm is testable on a host with no GPU
//! and no `embed-cuda` feature (AC80-18, AC80-20). Only the concrete driver
//! sampler/allocator and the end-to-end runner at the bottom of the file sit
//! behind `#[cfg(feature = "embed-cuda")]`, exactly as `CandleCudaProvider`
//! does for the `CudaProvider` trait.
//!
//! ## The ordering invariant (D-80.5-1)
//!
//! § 2.7 measured that `cuMemGetInfo` returns `CUDA_ERROR_INVALID_CONTEXT`
//! (201) when no CUDA context is current, and succeeds once one exists. The
//! sample order is therefore a contract, not a convention:
//!
//! 1. construct the Candle CUDA device (this creates the context),
//! 2. assert the ordinal Candle actually retained matches the requested one,
//! 3. sample free/total — the **before**,
//! 4. load the model,
//! 5. sample free/total — the **after**,
//! 6. run one real forward pass and record its vector dimension.
//!
//! A witness that samples "before" too early gets [`GpuWitnessError::NoCudaContext`];
//! it must never treat that as a zero sample, which would manufacture a false
//! delta (AC80-17).
//!
//! D-80.5-3's control allocation sits between steps 2 and 3, and its blocks
//! stay resident until after step 5. That position is forced by hardware:
//! Tegra's kernel page pool absorbs allocations without charging system
//! memory until it is drained, so the control both proves the counter is live
//! *and* leaves no pooled memory for the model load to hide inside. See
//! [`observe_control_allocation`] for the measurements.

use std::fmt;

use crate::device_policy::{DeviceResolutionReason, EffectiveEmbedDevice};

/// Schema string of the retained witness record.
///
/// D-80.5-5: this is a **separate** record, not a bump of the shared
/// `fathomdb.cuda-device-observation/v1` the x86_64 preflight seals. The two
/// merge in 80.6, where the artifact plumbing to do so exists.
pub const TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA: &str = "fathomdb.tegra-gpu-allocation-witness/v1";

/// The declared model floor, 64 MiB (D-80.5-2).
///
/// Justification, from measurement on this Orin rather than from any estimate
/// of the model's size:
///
/// * measured idle jitter is **0 bytes** across five consecutive samples with
///   no allocation in between (§ 2.7, reproduced during implementation), so
///   64 MiB is not "a bit above noise" — it is orders of magnitude above the
///   only noise ever observed here;
/// * the pinned `model.safetensors` is **133_466_304 bytes** as measured from
///   the warmed cache, and three consecutive witnessed loads on this host
///   charged the counter **143_364_096 / 143_622_144 / 143_880_192 bytes** —
///   the weights plus a stable ~10 MB of runtime state. The floor is therefore
///   just under half of a *measured* device-resident footprint, and no
///   allocation smaller than a substantial fraction of the real weights can
///   reach it;
/// * `cuMemGetInfo` on an integrated GPU reports a shared, system-wide counter,
///   so "nonzero" would be satisfiable by one unrelated page allocated during
///   the load window (D-80.5-2). 64 MiB is far beyond incidental traffic;
/// * it stays below the true footprint on purpose: the run-to-run spread above
///   is ~0.4%, but a host whose allocator rounds differently must not fail
///   spuriously, and the raw samples are retained so a reader can re-derive a
///   stricter verdict without re-running (R80-13).
pub const DEFAULT_DELTA_FLOOR_BYTES: u64 = 67_108_864;

/// The deliberate control allocation block, 1 GiB (D-80.5-3).
///
/// § 2.7 recorded a 256 MiB `cuMemAlloc` moving the counter by 274_161_664
/// bytes. That number **did not reproduce** during implementation: with a
/// Candle CUDA context already established, ten consecutive 256 MiB
/// `cuMemAlloc` + `cuMemsetD8` + `cuCtxSynchronize` cycles each moved the
/// counter by only ~1_032_192 bytes — page-table cost, not backing store —
/// while 2 GiB allocations sometimes moved ~2.15 GB and sometimes ~9 MB. The
/// explanation, consistent with every sample taken: Tegra's `nvgpu`/`nvmap`
/// keeps a kernel-side **page pool** of previously freed pages, so an
/// allocation is charged against system memory only once that pool is empty.
/// The 274 MB in § 2.7 was the pool being filled at context creation, not the
/// allocation being charged.
///
/// A 1 GiB block drains that pool an order of magnitude faster than a 256 MiB
/// one and is still small relative to any host that can run this witness.
pub const DEFAULT_CONTROL_ALLOCATION_BYTES: u64 = 1_073_741_824;

/// How many control blocks the witness may hold before declaring the counter
/// unattributable — 16 × 1 GiB = 16 GiB.
///
/// Measured on this Orin: the kernel page pool absorbed 7 to 8 blocks before
/// the counter charged for one in full (three runs: 9, 8, 8), so the budget is
/// twice the observed requirement and still a quarter of this host's 61 GiB.
/// The budget exists because "the counter never charged for any block" is a
/// **failure** (R80-12), not a reason to keep allocating until the host is out
/// of memory.
pub const MAX_CONTROL_BLOCKS: usize = 16;

/// The precondition D-80.5-3 states rather than assumes, retained inside the
/// record so a later reader judges the evidence instead of inheriting a silent
/// assumption.
pub const SOLE_GPU_CONSUMER_PRECONDITION: &str = "the witness run must be the sole GPU consumer: cuMemGetInfo reports a shared, system-wide counter on an integrated GPU";

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
        /// Observed free-memory movement across the load.
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
    /// Step 3 — sampled after the device exists, before the load.
    pub before: Option<GpuMemorySample>,
    /// Step 5 — sampled immediately after the load.
    pub after: Option<GpuMemorySample>,
    /// The bracketed control allocation (D-80.5-3).
    pub control: Option<ControlAllocationObservation>,
    /// The declared floor this run is judged against (D-80.5-2).
    pub delta_floor_bytes: u64,
    /// Step 6 — the dimension of one real forward pass.
    pub embedded_vector_dim: Option<usize>,
}

/// The retained witness record. Every field the verdict used is present, so a
/// reader re-derives the verdict rather than trusting it (R80-13, AC80-15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuAllocationWitness {
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
    /// Free bytes before the load.
    pub free_before_bytes: u64,
    /// Free bytes after the load.
    pub free_after_bytes: u64,
    /// Total device bytes; unified system memory on an iGPU.
    pub total_bytes: u64,
    /// `free_before_bytes - free_after_bytes`.
    pub delta_bytes: i128,
    /// The declared floor the delta was judged against.
    pub delta_floor_bytes: u64,
    /// Bytes deliberately requested per control block.
    pub control_allocation_request_bytes: u64,
    /// How many control blocks it took before one was charged in full.
    pub control_block_count: usize,
    /// Free bytes before the control allocation.
    pub control_free_before_bytes: u64,
    /// Free bytes while the control allocation was resident.
    pub control_free_after_bytes: u64,
    /// How far the counter moved for the control allocation.
    pub control_delta_bytes: i128,
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
        push_u64_field(
            &mut out,
            "control_allocation_request_bytes",
            self.control_allocation_request_bytes,
            false,
        );
        push_usize_field(&mut out, "control_block_count", self.control_block_count, false);
        push_i128_field(&mut out, "control_delta_bytes", self.control_delta_bytes, false);
        push_u64_field(&mut out, "control_free_after_bytes", self.control_free_after_bytes, false);
        push_u64_field(
            &mut out,
            "control_free_before_bytes",
            self.control_free_before_bytes,
            false,
        );
        push_i128_field(&mut out, "delta_bytes", self.delta_bytes, false);
        push_u64_field(&mut out, "delta_floor_bytes", self.delta_floor_bytes, false);
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
        push_string_field(&mut out, "schema", TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA, false);
        push_string_field(
            &mut out,
            "sole_gpu_consumer_precondition",
            SOLE_GPU_CONSUMER_PRECONDITION,
            false,
        );
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
/// Checks run in the order D-80.5-1 collects the evidence, so the first thing
/// that is wrong is what the caller is told.
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
    let control = inputs
        .control
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

    let control_delta = control.delta_bytes();
    if control_delta < i128::from(control.requested_bytes) {
        return Err(GpuWitnessError::ControlAllocationNotObserved {
            requested_bytes: control.requested_bytes,
            delta_bytes: control_delta,
        });
    }

    let delta = i128::from(before.free_bytes) - i128::from(after.free_bytes);
    if delta < i128::from(inputs.delta_floor_bytes) {
        return Err(GpuWitnessError::InsufficientDelta {
            delta_bytes: delta,
            floor_bytes: inputs.delta_floor_bytes,
        });
    }

    Ok(GpuAllocationWitness {
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
        control_allocation_request_bytes: control.requested_bytes,
        control_block_count: control.block_count,
        control_free_before_bytes: control.free_before_bytes,
        control_free_after_bytes: control.free_after_bytes,
        control_delta_bytes: control_delta,
        embedded_vector_dim: dimension,
    })
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
    use fathomdb_embedder_api::Embedder;

    use crate::candle_bge::{attest_retained_cuda_device, CandleBgeEmbedder};
    use crate::loader::load_pinned_default_embedder;

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

    // Reading the cache is pure host IO; doing it before the "before" sample
    // brackets the delta as tightly as possible around the device upload.
    let weights = load_pinned_default_embedder()
        .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;

    let mut sampler = CudaDriverMemorySampler;

    // D-80.5-3, before the load: prove the counter charges for a deliberate
    // allocation right now, and — because it is proven by a block the counter
    // charged in FULL — leave the driver with no spare pre-reserved arena that
    // could absorb the model load and hide its delta. The blocks stay resident
    // until after the post-load sample.
    let mut allocator = CudaDriverControlAllocator::default();
    let control = observe_control_allocation(
        &mut sampler,
        &mut allocator,
        config.control_allocation_bytes,
        config.max_control_blocks,
    )?;

    // 3. before
    let before = sampler.sample()?;
    // 4. load the model onto the witnessed device
    let embedder = CandleBgeEmbedder::new_from_weights_on_device(weights, device.clone())
        .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;
    // 5. after
    let after = sampler.sample()?;
    // The control memory has done its work; hold it no longer than the bracket
    // it protects.
    allocator.release();
    // 6. one real forward pass on the device
    let dimension = embedder
        .embed("fathomdb tegra gpu allocation witness")
        .map(|vector| vector.len())
        .map_err(|error| GpuWitnessError::ProbeFailed { message: format!("{error:?}") })?;

    evaluate_allocation_witness(AllocationWitnessInputs {
        requested_ordinal: config.ordinal,
        effective_device: EffectiveEmbedDevice::Cuda(info),
        cpu_reason: resolution.reason,
        retained_ordinal,
        retained_device_uuid: retained_uuid,
        before: Some(before),
        after: Some(after),
        control: Some(control),
        delta_floor_bytes: config.delta_floor_bytes,
        embedded_vector_dim: Some(dimension),
    })
}
