use super::*;

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

/// Safe CUDA provider facts associated with an effective CUDA selection.
#[napi(object)]
pub struct CudaDeviceInfo {
    pub ordinal: i64,
    pub uuid: Option<String>,
    pub name: Option<String>,
    pub driver_version: Option<String>,
    pub compute_capability: Option<String>,
    pub cuda_toolkit_version: Option<String>,
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
#[napi(object)]
pub struct GpuAllocationWitness {
    /// Schema string of the retained record.
    pub schema: String,
    /// The precondition the witness run states rather than assumes.
    pub sole_gpu_consumer_precondition: String,
    pub device_ordinal_requested: u32,
    pub device_ordinal_actual: u32,
    pub device_uuid: String,
    pub device_name: String,
    pub compute_capability: String,
    pub free_before_bytes: i64,
    pub free_after_bytes: i64,
    pub total_bytes: i64,
    pub delta_bytes: i64,
    pub delta_floor_bytes: i64,
    pub control_allocation_request_bytes: i64,
    pub control_block_count: u32,
    pub control_free_before_bytes: i64,
    pub control_free_after_bytes: i64,
    pub control_delta_bytes: i64,
    pub embedded_vector_dim: u32,
}

impl GpuAllocationWitness {
    pub(crate) fn from_rust(witness: &RustGpuAllocationWitness) -> Self {
        Self {
            schema: TEGRA_GPU_ALLOCATION_WITNESS_SCHEMA.to_string(),
            sole_gpu_consumer_precondition: SOLE_GPU_CONSUMER_PRECONDITION.to_string(),
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
            control_allocation_request_bytes: clamp_witness_bytes(i128::from(
                witness.control_allocation_request_bytes,
            )),
            control_block_count: witness.control_block_count as u32,
            control_free_before_bytes: clamp_witness_bytes(i128::from(
                witness.control_free_before_bytes,
            )),
            control_free_after_bytes: clamp_witness_bytes(i128::from(
                witness.control_free_after_bytes,
            )),
            control_delta_bytes: clamp_witness_bytes(witness.control_delta_bytes),
            embedded_vector_dim: witness.embedded_vector_dim as u32,
        }
    }
}

/// JavaScript carries these as numbers, so the widest exact type available is
/// `i64`. Every witness quantity is a device-memory byte count bounded by
/// installed RAM, so the saturation below is unreachable in practice; it exists
/// so an impossible value clamps to an obviously-wrong extreme rather than
/// wrapping into a plausible-looking one.
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
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — `true` iff the
    /// open-time #5 self-check found a vector-equivalence divergence beyond the
    /// D4 floor and every vector-dependent arm now refuses at query time with a
    /// `FDB_VECTOR_EQUIVALENCE_MISMATCH` error. The text-only/FTS-only path
    /// (`searchTextOnly`) stays serviceable.
    pub dense_disabled: bool,
    /// R-VEQ-6 — human-readable reason for `denseDisabled`, or `null` when healthy.
    pub dense_disabled_reason: Option<String>,
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
            dense_disabled: r.dense_disabled,
            dense_disabled_reason: r.dense_disabled_reason.clone(),
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
        }
    }
}
