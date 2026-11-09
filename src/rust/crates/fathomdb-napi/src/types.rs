use super::*;

#[napi(object)]
pub struct CounterSnapshot {
    pub queries: i64,
    pub writes: i64,
    pub write_rows: i64,
    pub admin_ops: i64,
    pub cache_hit: i64,
    pub cache_miss: i64,
}

/// 0.8.8 EXP-OBS (Slice 10) — the explanation sidecar (mirror of engine
/// `Explanation`): a query-level [`QueryTrace`] + a per-hit breakdown parallel to
/// (and in the same order as) `SearchResult.results`.
#[napi(object)]
pub struct Explanation {
    pub trace: QueryTrace,
    pub per_hit: Vec<PerHitExplain>,
    pub correlation_id: String,
}

impl Explanation {
    pub(crate) fn from_rust(e: &RustExplanation) -> Self {
        Self {
            trace: QueryTrace::from_rust(&e.trace),
            per_hit: e.per_hit.iter().map(PerHitExplain::from_rust).collect(),
            correlation_id: e.correlation_id.clone(),
        }
    }
}

#[napi(object)]
pub struct MigrationStepReport {
    pub step_id: u32,
    pub duration_ms: Option<i64>,
    pub failed: bool,
}

impl MigrationStepReport {
    pub(crate) fn from_rust(r: &RustMigrationStepReport) -> Self {
        Self { step_id: r.step_id, duration_ms: r.duration_ms.map(|v| v as i64), failed: r.failed }
    }
}

#[napi(object)]
pub struct EmbedderIdentity {
    pub name: String,
    pub revision: String,
    pub dimension: u32,
}

impl EmbedderIdentity {
    pub(crate) fn from_rust(id: &RustEmbedderIdentity) -> Self {
        Self { name: id.name.clone(), revision: id.revision.clone(), dimension: id.dimension }
    }
}

/// EU-6 — discriminated-union shape for `OpenReport.embedderEvents`.
///
/// `kind` carries the variant name (`"DefaultEmbedderDownload"`,
/// `"DefaultEmbedderCacheHit"`, `"MeanVecPinned"`); the remaining
/// optional fields carry the variant payload in camelCase. We pick a
/// flat object (rather than a per-variant `#[napi]` class) so callers
/// can pattern-match on `event.kind` without importing leaf classes.
#[napi(object)]
pub struct EmbedderEvent {
    pub kind: String,
    pub file: Option<String>,
    pub url: Option<String>,
    pub bytes: Option<i64>,
    pub sha256: Option<String>,
    pub cache_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub dim: Option<u32>,
    pub doc_count: Option<i64>,
    /// 0.7.2 PR-2b — `"manual"` on `MeanVecRecomputed` (the automatic
    /// `"drift_auto"` trigger was carved out / deferred to 0.8.x).
    pub trigger: Option<String>,
    /// Reserved (always `None` as of 0.7.2 PR-2bc — the `MeanRecomputeDeferred`
    /// event that carried this was removed with the automatic drift path).
    pub drift_cos: Option<f64>,
}

impl EmbedderEvent {
    pub(crate) fn from_rust(ev: &RustEmbedderEvent) -> Self {
        match ev {
            RustEmbedderEvent::DefaultEmbedderDownload {
                file,
                url,
                bytes,
                sha256,
                cache_path,
                duration_ms,
            } => Self {
                kind: "DefaultEmbedderDownload".to_string(),
                file: Some(file.clone()),
                url: Some(url.clone()),
                bytes: Some(*bytes as i64),
                sha256: Some(sha256.clone()),
                cache_path: Some(cache_path.display().to_string()),
                duration_ms: Some(*duration_ms as i64),
                dim: None,
                doc_count: None,
                trigger: None,
                drift_cos: None,
            },
            RustEmbedderEvent::DefaultEmbedderCacheHit { file, sha256, cache_path } => Self {
                kind: "DefaultEmbedderCacheHit".to_string(),
                file: Some(file.clone()),
                url: None,
                bytes: None,
                sha256: Some(sha256.clone()),
                cache_path: Some(cache_path.display().to_string()),
                duration_ms: None,
                dim: None,
                doc_count: None,
                trigger: None,
                drift_cos: None,
            },
            RustEmbedderEvent::MeanVecPinned { dim, doc_count } => Self {
                kind: "MeanVecPinned".to_string(),
                file: None,
                url: None,
                bytes: None,
                sha256: None,
                cache_path: None,
                duration_ms: None,
                dim: Some(*dim),
                doc_count: Some(*doc_count as i64),
                trigger: None,
                drift_cos: None,
            },
            RustEmbedderEvent::MeanVecRecomputed { dim, doc_count, trigger } => Self {
                kind: "MeanVecRecomputed".to_string(),
                file: None,
                url: None,
                bytes: None,
                sha256: None,
                cache_path: None,
                duration_ms: None,
                dim: Some(*dim),
                doc_count: Some(*doc_count as i64),
                trigger: Some(trigger.as_str().to_string()),
                drift_cos: None,
            },
        }
    }
}

/// The CUDA allocator decision behind one device, as stable `as_str()` names.
#[napi(object)]
pub struct CudaAllocatorReport {
    /// `private`, `default_pool` or `synchronous`; absent when unknown.
    pub path: Option<String>,
    pub reason: String,
    pub pool_max_size_bytes: Option<i64>,
    /// `"0"` or `"max"`; absent unless the path is private.
    pub release_threshold: Option<String>,
    pub module_load_init: String,
    /// The pool mode in force (`auto`, `on`, `off`); absent for a malformed
    /// `FATHOMDB_POOL_MODE`.
    pub pool_mode: Option<String>,
    /// Where the pool mode in force came from: `sdk`, `env` or `default`.
    pub pool_mode_source: Option<String>,
    /// Where the pool size in force came from: `sdk`, `env` or `default`.
    pub pool_max_size_source: Option<String>,
    /// Where the release threshold in force came from: `sdk`, `env` or `default`.
    pub pool_release_threshold_source: Option<String>,
}

impl CudaAllocatorReport {
    pub(crate) fn from_rust(report: &RustCudaAllocatorReport) -> Self {
        Self {
            path: report.path.map(|path| path.as_str().to_string()),
            reason: report.reason.as_str().to_string(),
            // A pool cap never approaches i64::MAX bytes.
            pool_max_size_bytes: report.pool_max_size_bytes.map(|bytes| bytes as i64),
            release_threshold: report.release_threshold.map(|t| t.as_str().to_string()),
            module_load_init: report.module_load_init.as_str().to_string(),
            pool_mode: report.pool_mode.map(|mode| mode.as_str().to_string()),
            pool_mode_source: report.pool_mode_source.map(|source| source.as_str().to_string()),
            pool_max_size_source: report
                .pool_max_size_source
                .map(|source| source.as_str().to_string()),
            pool_release_threshold_source: report
                .pool_release_threshold_source
                .map(|source| source.as_str().to_string()),
        }
    }
}

/// One CUDA library the process loaded (0.8.28 Slice 31, D-18).
#[napi(object)]
pub struct CudaLibraryReport {
    /// `driver`, `cublas`, `curand`, `cublasLt` or `nvrtc`.
    pub library: String,
    pub opened_as: String,
    /// Canonical path of the loaded file, comparable with `/proc/self/maps`.
    pub path: Option<String>,
    pub version: Option<String>,
    /// The CUDA major the build checked the library against.
    pub built_major: u32,
}

impl CudaLibraryReport {
    pub(crate) fn from_rust(report: &RustCudaLibraryReport) -> Self {
        Self {
            library: report.library.clone(),
            opened_as: report.opened_as.clone(),
            path: report.path.clone(),
            version: report.version.clone(),
            built_major: report.built_major,
        }
    }
}

/// Safe CUDA provider facts associated with an effective CUDA selection.
#[napi(object)]
pub struct CudaDeviceInfo {
    pub ordinal: i64,
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub driver_version: Option<String>,
    pub compute_capability: Option<String>,
    pub cuda_toolkit_version: Option<String>,
    pub cuda_allocator: Option<CudaAllocatorReport>,
    /// The CUDA libraries the probe checked and loaded; empty off CUDA.
    pub cuda_libraries: Vec<CudaLibraryReport>,
}

impl CudaDeviceInfo {
    pub(crate) fn from_rust(info: &RustCudaDeviceInfo) -> Self {
        Self {
            ordinal: info.ordinal as i64,
            uuid: info.uuid.clone(),
            name: info.name.clone(),
            driver_version: info.driver_version.clone(),
            compute_capability: info.compute_capability.clone(),
            cuda_toolkit_version: info.cuda_toolkit_version.clone(),
            cuda_allocator: info.cuda_allocator.as_ref().map(CudaAllocatorReport::from_rust),
            cuda_libraries: info.cuda_libraries.iter().map(CudaLibraryReport::from_rust).collect(),
        }
    }
}

/// Process-visible CUDA identity facts preserved in an open report.
#[napi(object)]
pub struct CudaVisibleDevice {
    pub visible_ordinal: i64,
    pub uuid: String,
    pub name: String,
    pub compute_capability: Option<String>,
}

impl CudaVisibleDevice {
    pub(crate) fn from_rust(device: &RustCudaVisibleDevice) -> Self {
        Self {
            visible_ordinal: device.visible_ordinal as i64,
            uuid: device.uuid.clone(),
            name: device.name.clone(),
            compute_capability: device.compute_capability.clone(),
        }
    }
}

/// The CPU or CUDA backend selected for one embedder device policy.
#[napi(object)]
pub struct EffectiveEmbedDevice {
    /// Either `"cpu"` or `"cuda"`.
    pub kind: String,
    /// Present exactly when `kind == "cuda"`.
    pub cuda_device: Option<CudaDeviceInfo>,
}

impl EffectiveEmbedDevice {
    pub(crate) fn from_rust(device: &RustEffectiveEmbedDevice) -> Self {
        match device {
            RustEffectiveEmbedDevice::Cpu => Self { kind: "cpu".to_string(), cuda_device: None },
            RustEffectiveEmbedDevice::Cuda(info) => Self {
                kind: "cuda".to_string(),
                cuda_device: Some(CudaDeviceInfo::from_rust(info)),
            },
        }
    }
}

/// The strict CPU/CUDA policy outcome captured when an embedder was constructed.
#[napi(object)]
pub struct EmbedderDeviceResolution {
    pub requested_policy: String,
    pub cuda_compiled: bool,
    pub effective_device: EffectiveEmbedDevice,
    pub visible_cuda_devices: Vec<CudaVisibleDevice>,
    pub selected_cuda_uuid: Option<String>,
    pub reason: Option<String>,
}

impl EmbedderDeviceResolution {
    pub(crate) fn from_rust(resolution: &RustDeviceResolution) -> Self {
        let requested_policy = match resolution.requested_policy {
            RustEmbedDevicePolicy::Auto => "auto".to_string(),
            RustEmbedDevicePolicy::Cpu => "cpu".to_string(),
            RustEmbedDevicePolicy::Cuda(ordinal) => format!("cuda:{ordinal}"),
        };
        Self {
            requested_policy,
            cuda_compiled: resolution.cuda_compiled,
            effective_device: EffectiveEmbedDevice::from_rust(&resolution.effective_device),
            visible_cuda_devices: resolution
                .visible_cuda_devices
                .iter()
                .map(CudaVisibleDevice::from_rust)
                .collect(),
            selected_cuda_uuid: resolution.selected_cuda_uuid.clone(),
            reason: resolution.reason.map(|reason| reason.as_str().to_string()),
        }
    }

    pub(crate) fn from_reranker(resolution: &RustRerankerDeviceResolution) -> Self {
        let requested_policy = match resolution.requested_policy {
            RustRerankerDevicePolicy::Auto => "auto".to_string(),
            RustRerankerDevicePolicy::Cpu => "cpu".to_string(),
            RustRerankerDevicePolicy::Cuda(ordinal) => format!("cuda:{ordinal}"),
        };
        let effective_device = match &resolution.effective_device {
            RustEffectiveRerankerDevice::Cpu => {
                EffectiveEmbedDevice { kind: "cpu".to_string(), cuda_device: None }
            }
            RustEffectiveRerankerDevice::Cuda(info) => EffectiveEmbedDevice {
                kind: "cuda".to_string(),
                cuda_device: Some(CudaDeviceInfo::from_rust(info)),
            },
        };
        Self {
            requested_policy,
            cuda_compiled: resolution.cuda_compiled,
            effective_device,
            visible_cuda_devices: resolution
                .visible_cuda_devices
                .iter()
                .map(CudaVisibleDevice::from_rust)
                .collect(),
            selected_cuda_uuid: resolution.selected_cuda_uuid.clone(),
            reason: resolution.reason.map(|reason| reason.as_str().to_string()),
        }
    }
}

/// 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the retained
/// `fathomdb.tegra-gpu-allocation-witness/v1` record, measured in the
/// artifact's own process.
///
/// Every number the verdict used is carried, so a reader re-derives the
/// verdict instead of trusting it: the raw free-memory samples that bracket
/// the model load, the declared floor they were judged against, and the
/// deliberate control allocation that proves the shared iGPU counter was live
/// and attributable at the time. Dropping any of them would leave a consumer
/// able to report a pass it cannot check.
/// Discrete-device global counter and raw control.
#[napi(object)]
pub struct GpuGlobalFreeMeasurement {
    /// Exact global_free discriminant.
    pub kind: String,
    /// Requested raw bytes per control block.
    pub control_allocation_request_bytes: i64,
    /// Number of resident raw blocks.
    pub control_block_count: u32,
    /// Free bytes before the decisive block.
    pub control_free_before_bytes: i64,
    /// Free bytes while that block is live.
    pub control_free_after_bytes: i64,
    /// Signed raw-control before-minus-after delta.
    pub control_delta_bytes: i64,
    /// Canonical selected-device consumer precondition.
    pub sole_gpu_consumer_precondition: String,
}

/// Integrated-device private USED model and control proof.
#[napi(object)]
pub struct GpuPrivatePoolUsedMeasurement {
    /// Exact private_pool_used discriminant.
    pub kind: String,
    /// Checked uint64 context ID as lossless decimal text.
    pub context_id: String,
    /// Opaque retained private-pool identity.
    pub pool_identity: String,
    /// Current USED before model load.
    pub used_before_bytes: i64,
    /// Current USED after checked model load.
    pub used_after_bytes: i64,
    /// Signed USED after-minus-before model delta.
    pub delta_bytes: i64,
    /// Exact same-pool control request.
    pub control_request_bytes: i64,
    /// Post-forward USED control baseline.
    pub control_before_bytes: i64,
    /// USED while the completed control is live.
    pub control_live_bytes: i64,
    /// USED after checked control release.
    pub control_released_bytes: i64,
    /// Canonical same-pool exclusivity precondition.
    pub pool_exclusivity_precondition: String,
}

/// Current witness with exactly one method-specific native output object.
#[napi(object)]
pub struct GpuAllocationWitness {
    /// Current v2 schema.
    pub schema: String,
    /// PID of the witness process.
    pub process_id: u32,
    /// Checked actual INTEGRATED attribute.
    pub device_integrated: bool,
    /// Ordinal requested by policy.
    pub device_ordinal_requested: u32,
    /// Ordinal retained by the model device.
    pub device_ordinal_actual: u32,
    /// Driver UUID of the witnessed device.
    pub device_uuid: String,
    /// Driver device name.
    pub device_name: String,
    /// Driver compute capability.
    pub compute_capability: String,
    /// Actual global diagnostic free bytes before load.
    pub free_before_bytes: i64,
    /// Actual global diagnostic free bytes after load.
    pub free_after_bytes: i64,
    /// Actual global total bytes.
    pub total_bytes: i64,
    /// Signed global before-minus-after diagnostic delta.
    pub delta_bytes: i64,
    /// Declared method-specific model floor.
    pub delta_floor_bytes: i64,
    /// Dimension of the actual model forward.
    pub embedded_vector_dim: u32,
    /// Exact tagged measurement; no irrelevant optional fields.
    pub measurement:
        napi::bindgen_prelude::Either<GpuGlobalFreeMeasurement, GpuPrivatePoolUsedMeasurement>,
}

impl GpuAllocationWitness {
    pub(crate) fn from_rust(witness: &RustGpuAllocationWitness) -> Self {
        let measurement = match &witness.measurement {
            fathomdb_embedder::GpuAllocationMeasurement::GlobalFree {
                control_allocation_request_bytes,
                control_block_count,
                control_free_before_bytes,
                control_free_after_bytes,
                control_delta_bytes,
                sole_gpu_consumer_precondition,
            } => napi::bindgen_prelude::Either::A(GpuGlobalFreeMeasurement {
                kind: "global_free".into(),
                control_allocation_request_bytes: clamp_witness_bytes(i128::from(
                    *control_allocation_request_bytes,
                )),
                control_block_count: *control_block_count as u32,
                control_free_before_bytes: clamp_witness_bytes(i128::from(
                    *control_free_before_bytes,
                )),
                control_free_after_bytes: clamp_witness_bytes(i128::from(
                    *control_free_after_bytes,
                )),
                control_delta_bytes: clamp_witness_bytes(*control_delta_bytes),
                sole_gpu_consumer_precondition: sole_gpu_consumer_precondition.clone(),
            }),
            fathomdb_embedder::GpuAllocationMeasurement::PrivatePoolUsed { .. } => {
                unimplemented!("private native witness conversion")
            }
        };
        Self {
            schema: TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA.to_owned(),
            process_id: witness.process_id,
            device_integrated: witness.device_integrated,
            device_ordinal_requested: witness.device_ordinal_requested as u32,
            device_ordinal_actual: witness.device_ordinal_actual as u32,
            device_uuid: witness.device_uuid.clone(),
            device_name: witness.device_name.clone(),
            compute_capability: witness.compute_capability.clone(),
            free_before_bytes: clamp_witness_bytes(i128::from(witness.free_before_bytes)),
            free_after_bytes: clamp_witness_bytes(i128::from(witness.free_after_bytes)),
            total_bytes: clamp_witness_bytes(i128::from(witness.total_bytes)),
            delta_bytes: clamp_witness_bytes(witness.delta_bytes),
            delta_floor_bytes: clamp_witness_bytes(i128::from(witness.delta_floor_bytes)),
            embedded_vector_dim: witness.embedded_vector_dim as u32,
            measurement,
        }
    }
}

/// Keep the existing saturating byte conversion instead of wrapping at i64.
/// Qualified device-memory counts fit JavaScript Number's exact 53-bit integer
/// range. Opaque context/pool identity strings never pass through this helper.
pub(crate) fn clamp_witness_bytes(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value.is_negative() { i64::MIN } else { i64::MAX })
}

#[napi(object)]
pub struct OpenReport {
    pub schema_version_before: u32,
    pub schema_version_after: u32,
    pub migration_steps: Vec<MigrationStepReport>,
    pub embedder_warmup_ms: i64,
    pub query_backend: String,
    pub default_embedder: EmbedderIdentity,
    // EU-5a1/5a2/5b — surfaced by EU-6.
    pub embedder_download_ms: Option<i64>,
    pub embedder_events: Vec<EmbedderEvent>,
    pub embedder_mean_centering_required: bool,
    pub embedder_mean_vec_pinned: bool,
    /// Strict CPU/CUDA selection used to construct the embedder, or `null` when
    /// no embedder was configured.
    pub embedder_device_resolution: Option<EmbedderDeviceResolution>,
    pub reranker_device_resolution: Option<EmbedderDeviceResolution>,
    /// 0.8.23 Slice 80.6 (D-80.6-6, AC80-6) — the in-process GPU allocation
    /// witness, or `null` when this open measured none.
    ///
    /// `null` means **no witness was taken**, never "a witness measured
    /// nothing": a zero, negative, or below-floor allocation delta is a typed
    /// failure inside the witness and fails the open, so a zero-valued record
    /// is not reachable here.
    pub embedder_gpu_allocation_witness: Option<GpuAllocationWitness>,
    /// SHA-256 (lowercase hex) of the engine's effective query stop words.
    pub stop_words_sha256: String,
}

impl OpenReport {
    pub(crate) fn from_rust(r: &RustOpenReport) -> Self {
        Self {
            schema_version_before: r.schema_version_before,
            schema_version_after: r.schema_version_after,
            migration_steps: r.migration_steps.iter().map(MigrationStepReport::from_rust).collect(),
            embedder_warmup_ms: r.embedder_warmup_ms as i64,
            query_backend: r.query_backend.to_string(),
            default_embedder: EmbedderIdentity::from_rust(&r.default_embedder),
            embedder_download_ms: r.embedder_download_ms.map(|v| v as i64),
            embedder_events: r.embedder_events.iter().map(EmbedderEvent::from_rust).collect(),
            embedder_mean_centering_required: r.embedder_mean_centering_required,
            embedder_mean_vec_pinned: r.embedder_mean_vec_pinned,
            embedder_device_resolution: r
                .embedder_device_resolution
                .as_ref()
                .map(EmbedderDeviceResolution::from_rust),
            reranker_device_resolution: r
                .reranker_device_resolution
                .as_ref()
                .map(EmbedderDeviceResolution::from_reranker),
            embedder_gpu_allocation_witness: r
                .embedder_gpu_allocation_witness
                .as_ref()
                .map(GpuAllocationWitness::from_rust),
            stop_words_sha256: r.stop_words_sha256.clone(),
        }
    }
}
