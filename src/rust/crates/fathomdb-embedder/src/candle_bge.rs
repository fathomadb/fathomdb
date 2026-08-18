//! `CandleBgeEmbedder` — the default embedder shipped with fathomdb.
//!
//! Wraps `candle_transformers::models::bert::BertModel` loaded with the
//! pinned `BAAI/bge-small-en-v1.5` weights (revision
//! `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, dim 384) and implements
//! the `fathomdb_embedder_api::Embedder` trait.
//!
//! Contract sources:
//! - `dev/design/embedder.md` §0.4 — mean-pool over attention mask, then
//!   L2-normalize; store un-centered (centering is the engine's concern).
//! - `dev/design/embedder.md` §1 — pinned identity.
//! - `dev/design/embedder.md` §7 — embedder does NOT append to
//!   `embedder_events`; that is the engine's job.
//! - `dev/design/embedder.md` §8 — little-endian invariant; enforced as
//!   a `compile_error!` at the top of this module so BE builds fail at
//!   `cargo build` rather than at runtime.
//! - `ADR-0.6.0-embedder-protocol.md` Invariants 1–4 — unit-norm output,
//!   no re-entrancy into the engine, no log/tracing/println in `embed()`,
//!   trait method is synchronous.

// Design §8: safetensors weight format is little-endian; pinned
// workspace targets (x86_64, aarch64) are LE on all supported OSes.
// BE platforms are explicitly out of scope for 0.7.1 — they would need
// an additional byte-swap pass during weight load. Enforce at compile
// time so a BE build fails fast rather than silently producing garbage
// f32 vectors.
#[cfg(target_endian = "big")]
compile_error!("fathomdb-embedder default path requires a little-endian target");

use std::path::Path;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use tokenizers::{Tokenizer, TruncationParams};

use crate::{
    diagnose_gpu,
    loader::{
        load_pinned_default_embedder, load_pinned_default_embedder_from_local_asset,
        EmbedderLoadError, LoadedWeights, HF_REVISION,
    },
    resolve_embed_device_policy_from_env, CudaDeviceInfo, CudaProbeError, CudaProvider,
    CudaVisibleDevice, DeviceResolution, DoctorGpuDiagnosticResult, EffectiveEmbedDevice,
    EmbedDevicePolicyError,
};

/// Engine-facing identity name (per
/// `dev/plans/prompts/0.7.1-EMBEDDER-UNDEFER-HANDOFF.md` §0.5). EU-5 will
/// reconcile this with `fathomdb-engine::default_embedder_identity()` so
/// the engine stops returning the noop name.
pub const DEFAULT_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";

/// Output dimension for `bge-small-en-v1.5`. Must match the hidden_size
/// field of the pinned `config.json`; `new()` enforces this at runtime.
pub const DEFAULT_EMBEDDER_DIM: u32 = 384;

/// Maximum sequence length (in tokens, INCLUDING the `[CLS]`/`[SEP]`
/// specials) the tokenizer truncates to. BGE-small's learned position
/// embeddings have exactly 512 slots (`config.json` `max_position_
/// embeddings`); feeding a longer sequence trips an out-of-bounds
/// index-select inside `BertModel::forward`. Per
/// `dev/design/embedder-decision.md` §2.1 the tokenizer runs with
/// `truncation True` / max 512.
const MAX_SEQUENCE_TOKENS: usize = 512;

/// Sentence-vector pooling strategy. `bge-small-en-v1.5` ships
/// `1_Pooling/config.json` with `pooling_mode_cls_token: true`, i.e. it is a
/// CLS-pooled model; BGE's docs warn that mean-pooling causes "a significant
/// decrease in performance". `Mean` is the historical default (design §0.4);
/// `Cls` is the model-correct mode, under evaluation behind the 1-bit binary
/// recall-floor gate (it changes the embedding-space geometry that sign-bit
/// quantization is sensitive to). See `dev/notes/IR-C-embedder-options-research.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pooling {
    /// Mean over the attention mask (historical default; design §0.4).
    Mean,
    /// `[CLS]` token (position 0) — the mode bge-small was trained for.
    Cls,
}

/// Concrete device selected by the TC-5 benchmark preflight.
///
/// Unlike the ordinary constructors, this enum is never derived from an
/// environment variable and never permits a CPU fallback for a CUDA request.
/// It is available only with the `tc5-benchmark` feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExplicitCandleDevice {
    /// Use Candle's CPU backend.
    Cpu,
    /// Use the exact CUDA ordinal selected by benchmark preflight.
    Cuda(usize),
}

/// L2-normalize a `(1, D)` pooled tensor.
fn l2_normalize(pooled: &Tensor) -> candle_core::Result<Tensor> {
    let norm = pooled.sqr()?.sum_keepdim(1)?.sqrt()?;
    let norm = norm.clamp(1e-12_f32, f32::INFINITY)?;
    pooled.broadcast_div(&norm)
}

/// Mean-pool `(1, L, D)` hidden states over the attention mask → `(1, D)`. Pad
/// positions contribute zero; divide by the non-pad token count, not by L.
fn mean_pool(hidden: &Tensor, attn_mask_u32: &Tensor) -> candle_core::Result<Tensor> {
    let mask_f = attn_mask_u32.to_dtype(DType::F32)?.unsqueeze(2)?; // (1, L, 1)
    let mask_f = mask_f.broadcast_as(hidden.shape())?; // (1, L, D)
    let summed = (hidden * &mask_f)?.sum(1)?; // (1, D)
    let counts = mask_f.sum(1)?.clamp(1e-9_f32, f32::INFINITY)?; // (1, D)
    summed / counts
}

/// CLS-pool: take the `[CLS]` token at position 0 of `(1, L, D)` → `(1, D)`.
fn cls_pool(hidden: &Tensor) -> candle_core::Result<Tensor> {
    hidden.narrow(1, 0, 1)?.squeeze(1)
}

/// Default embedder backed by `candle-transformers` BERT.
pub struct CandleBgeEmbedder {
    identity: EmbedderIdentity,
    tokenizer: Tokenizer,
    model: BertModel,
    device: Device,
    pooling: Pooling,
}

/// Candle's concrete CUDA probe used by the default embedder.
///
/// A successful result means Candle initialized the requested ordinal and a
/// one-element allocation on it succeeded. The returned report intentionally
/// contains only metadata safe for product diagnostics.
struct CandleCudaProvider;

impl CudaProvider for CandleCudaProvider {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        #[cfg(feature = "embed-cuda")]
        {
            use candle_core::cuda::cudarc::driver::{result, sys};

            let dynamic_driver_present = unsafe {
                // SAFETY: this only attempts to locate the CUDA driver shared library;
                // no CUDA driver symbol is invoked before the subsequent `result::init`.
                sys::is_culib_present()
            };
            classify_cuda_driver_presence(dynamic_driver_present)?;
            result::init().map_err(classify_cuda_driver_error)?;
            let count = result::device::get_count().map_err(classify_cuda_driver_error)?;
            let count = usize::try_from(count).map_err(|_| CudaProbeError::ProbeFailed {
                message: "CUDA driver returned a negative device count".to_owned(),
            })?;
            (0..count)
                .map(|visible_ordinal| {
                    let device = result::device::get(visible_ordinal as i32)
                        .map_err(classify_cuda_driver_error)?;
                    let name = result::device::get_name(device).map_err(classify_cuda_driver_error)?;
                    let uuid = result::device::get_uuid(device).map_err(classify_cuda_driver_error)?;
                    let major = unsafe {
                        result::device::get_attribute(
                            device,
                            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
                        )
                    }
                    .map_err(classify_cuda_driver_error)?;
                    let minor = unsafe {
                        result::device::get_attribute(
                            device,
                            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
                        )
                    }
                    .map_err(classify_cuda_driver_error)?;
                    Ok(CudaVisibleDevice {
                        visible_ordinal,
                        uuid: cuda_uuid_string(uuid.bytes),
                        name,
                        compute_capability: Some(format!("{major}.{minor}")),
                    })
                })
                .collect()
        }

        #[cfg(not(feature = "embed-cuda"))]
        {
            Err(CudaProbeError::ProbeFailed {
                message: "the default embedder was built without CUDA".to_owned(),
            })
        }
    }

    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        #[cfg(feature = "embed-cuda")]
        {
            let device = Device::new_cuda(ordinal).map_err(classify_candle_cuda_error)?;
            Tensor::zeros(1, DType::F32, &device).map_err(classify_candle_cuda_error)?;
            let visible = self
                .enumerate_visible_cuda_devices()?
                .into_iter()
                .find(|visible| visible.visible_ordinal == ordinal)
                .ok_or(CudaProbeError::NoVisibleDevice)?;
            Ok(CudaDeviceInfo {
                ordinal,
                uuid: Some(visible.uuid),
                name: Some(visible.name),
                driver_version: None,
                compute_capability: visible.compute_capability,
                cuda_toolkit_version: None,
            })
        }

        #[cfg(not(feature = "embed-cuda"))]
        {
            let _ = ordinal;
            Err(CudaProbeError::ProbeFailed {
                message: "the default embedder was built without CUDA".to_string(),
            })
        }
    }
}

#[cfg(feature = "embed-cuda")]
fn classify_cuda_driver_presence(driver_present: bool) -> Result<(), CudaProbeError> {
    driver_present.then_some(()).ok_or(CudaProbeError::NoVisibleDevice)
}

#[cfg(feature = "embed-cuda")]
fn classify_cuda_driver_error(
    error: candle_core::cuda::cudarc::driver::result::DriverError,
) -> CudaProbeError {
    use candle_core::cuda::cudarc::driver::sys::CUresult;

    let message = error.to_string();
    match error.0 {
        CUresult::CUDA_ERROR_NO_DEVICE | CUresult::CUDA_ERROR_STUB_LIBRARY => {
            CudaProbeError::NoVisibleDevice
        }
        CUresult::CUDA_ERROR_SYSTEM_DRIVER_MISMATCH
        | CUresult::CUDA_ERROR_COMPAT_NOT_SUPPORTED_ON_DEVICE
        | CUresult::CUDA_ERROR_NO_BINARY_FOR_GPU
        | CUresult::CUDA_ERROR_UNSUPPORTED_PTX_VERSION => CudaProbeError::Incompatible { message },
        _ => CudaProbeError::ProbeFailed { message },
    }
}

#[cfg(feature = "embed-cuda")]
fn classify_candle_cuda_error(error: candle_core::Error) -> CudaProbeError {
    use candle_core::{
        cuda::{cudarc::cublas::sys::cublasStatus_t, CudaError},
        Error,
    };

    let message = error.to_string();
    match error {
        Error::Cuda(source) => match source.downcast_ref::<CudaError>() {
            Some(CudaError::Cuda(error)) => classify_cuda_driver_error(*error),
            Some(CudaError::Cublas(error))
                if matches!(
                    error.0,
                    cublasStatus_t::CUBLAS_STATUS_ARCH_MISMATCH
                        | cublasStatus_t::CUBLAS_STATUS_NOT_SUPPORTED
                ) =>
            {
                CudaProbeError::Incompatible { message }
            }
            _ => CudaProbeError::ProbeFailed { message },
        },
        _ => CudaProbeError::ProbeFailed { message },
    }
}

#[cfg(feature = "embed-cuda")]
fn cuda_uuid_string(bytes: [std::os::raw::c_char; 16]) -> String {
    let bytes = bytes.map(|byte| byte as u8);
    format!(
        "GPU-{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

/// Resolve the product `FATHOMDB_EMBED_DEVICE` policy for one default
/// embedder construction.
///
/// This is the only ambient-policy read on the Candle default path. Callers
/// receive a typed outcome; forced CUDA is never downgraded to CPU.
pub fn resolve_default_embedder_device_from_env() -> Result<DeviceResolution, EmbedDevicePolicyError>
{
    let mut provider = CandleCudaProvider;
    resolve_embed_device_policy_from_env(cfg!(feature = "embed-cuda"), &mut provider)
}

/// Run the isolated default-embedder CUDA diagnostic from the ambient policy.
///
/// This creates no engine, database, loader, or model. It only performs the
/// CUDA driver inventory and minimal allocation/provider probe required by
/// `fathomdb doctor gpu`.
#[must_use]
pub fn diagnose_default_embedder_gpu_from_env() -> DoctorGpuDiagnosticResult {
    let raw = std::env::var("FATHOMDB_EMBED_DEVICE").unwrap_or_else(|_| "auto".to_owned());
    let cuda_compiled = cfg!(feature = "embed-cuda");
    match raw.parse() {
        Ok(policy) => {
            let mut provider = CandleCudaProvider;
            diagnose_gpu(policy, cuda_compiled, &mut provider)
        }
        Err(_) => DoctorGpuDiagnosticResult::from_invalid_policy(raw, cuda_compiled),
    }
}

fn device_from_resolution(resolution: &DeviceResolution) -> Result<Device, EmbedderLoadError> {
    match &resolution.effective_device {
        EffectiveEmbedDevice::Cpu => Ok(Device::Cpu),
        EffectiveEmbedDevice::Cuda(info) => Device::new_cuda(info.ordinal).map_err(|error| {
            EmbedderLoadError::DeviceInitialization { message: error.to_string() }
        }),
    }
}

impl CandleBgeEmbedder {
    /// Fetch (or read from cache) the pinned weights and construct a ready
    /// embedder. First call on a cold cache downloads ~135 MB from
    /// huggingface.co; subsequent calls are local-IO.
    ///
    /// Per `dev/design/embedder.md` §8 this constructor asserts the host is
    /// little-endian (safetensors layout assumption).
    pub fn new() -> Result<Self, EmbedderLoadError> {
        let resolution = resolve_default_embedder_device_from_env().map_err(|error| {
            EmbedderLoadError::DeviceInitialization { message: error.to_string() }
        })?;
        let weights = load_pinned_default_embedder()?;
        Self::new_from_weights_with_device_resolution(weights, &resolution)
    }

    /// EU-5b — construct from already-fetched weights. The engine path
    /// invokes the loader directly (so it can splice the loader's
    /// `bytes_downloaded` + `events` into `OpenReport`), then hands the
    /// `LoadedWeights` to this constructor.
    pub fn new_from_weights(weights: LoadedWeights) -> Result<Self, EmbedderLoadError> {
        let resolution = resolve_default_embedder_device_from_env().map_err(|error| {
            EmbedderLoadError::DeviceInitialization { message: error.to_string() }
        })?;
        Self::new_from_weights_with_device_resolution(weights, &resolution)
    }

    /// Construct from already-fetched weights and one prior strict device resolution.
    pub fn new_from_weights_with_device_resolution(
        weights: LoadedWeights,
        resolution: &DeviceResolution,
    ) -> Result<Self, EmbedderLoadError> {
        Self::new_from_weights_on_device(weights, device_from_resolution(resolution)?)
    }

    /// Constructs a cache-only TC-5 embedder using the explicitly preflighted device.
    #[cfg(feature = "tc5-benchmark")]
    pub fn new_from_local_asset_on_device(
        asset_dir: &Path,
        requested_device: ExplicitCandleDevice,
    ) -> Result<Self, EmbedderLoadError> {
        let weights = load_pinned_default_embedder_from_local_asset(asset_dir)?;
        let device = match requested_device {
            ExplicitCandleDevice::Cpu => Device::Cpu,
            ExplicitCandleDevice::Cuda(ordinal) => {
                #[cfg(feature = "embed-cuda")]
                {
                    Device::new_cuda(ordinal).map_err(|error| {
                        EmbedderLoadError::DeviceUnavailable {
                            device: format!("cuda:{ordinal}"),
                            reason: error.to_string(),
                        }
                    })?
                }
                #[cfg(not(feature = "embed-cuda"))]
                {
                    return Err(EmbedderLoadError::DeviceUnavailable {
                        device: format!("cuda:{ordinal}"),
                        reason: "the benchmark binary lacks the `embed-cuda` feature".to_string(),
                    });
                }
            }
        };
        Self::new_from_weights_on_device(weights, device)
    }

    fn new_from_weights_on_device(
        weights: LoadedWeights,
        device: Device,
    ) -> Result<Self, EmbedderLoadError> {
        // Design §8: safetensors are little-endian; we never run on BE.
        // Tightened in EU-5d follow-up from a debug_assert into a
        // compile-time error so BE builds fail at `cargo build` rather
        // than panicking at runtime in debug mode.

        // 1. Parse config.json.
        let config_bytes = std::fs::read(&weights.config_json_path).map_err(|source| {
            EmbedderLoadError::CacheIoError { path: weights.config_json_path.clone(), source }
        })?;
        let config: BertConfig =
            serde_json::from_slice(&config_bytes).map_err(|e| EmbedderLoadError::CacheIoError {
                path: weights.config_json_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
            })?;

        // Runtime dim sanity (codex-checklist item): the pinned shape must
        // match our public dim constant. If a future revision bump changes
        // hidden_size, fail loudly here.
        if config.hidden_size != DEFAULT_EMBEDDER_DIM as usize {
            // Distinct from `ModelDeserialize`: the bytes parsed cleanly
            // but the pinned model's `hidden_size` disagrees with our
            // compile-time `DEFAULT_EMBEDDER_DIM`. This always points at
            // a deliberate model/version drift — see design §9 row
            // "DimensionMismatch".
            return Err(EmbedderLoadError::DimensionMismatch {
                expected: DEFAULT_EMBEDDER_DIM,
                actual: config.hidden_size as u32,
            });
        }

        // 2. Load tokenizer.json and pin truncation to the model's 512-slot
        // position-embedding window (design §2.1). `tokenizer.json` does not
        // always carry a truncation policy, so set it programmatically; the
        // tokenizer truncates AND re-applies `[SEP]` correctly (a raw token
        // truncate would drop the trailing special token). Without this a
        // >512-token document errors with "index-select invalid index 512".
        let mut tokenizer = Tokenizer::from_file(&weights.tokenizer_json_path)
            .map_err(|e| EmbedderLoadError::TokenizerLoad { source: e })?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQUENCE_TOKENS,
                ..Default::default()
            }))
            .map_err(|e| EmbedderLoadError::TokenizerLoad { source: e })?;

        // 3. mmap safetensors on the explicitly resolved device.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights.model_safetensors_path.as_path() as &Path],
                DType::F32,
                &device,
            )
        }
        .map_err(|source| EmbedderLoadError::ModelDeserialize { source })?;

        let model = BertModel::load(vb, &config)
            .map_err(|source| EmbedderLoadError::ModelDeserialize { source })?;

        let identity =
            EmbedderIdentity::new(DEFAULT_EMBEDDER_NAME, HF_REVISION, DEFAULT_EMBEDDER_DIM);

        Ok(Self { identity, tokenizer, model, device, pooling: Pooling::Mean })
    }

    /// Select the pooling strategy (default [`Pooling::Mean`]). Does NOT change
    /// the embedder identity (pooling is not part of the model identity), so
    /// stored vectors from a different pooling are incompatible — use only on a
    /// fresh workspace / in measurement harnesses until the CLS↔binary-floor gate
    /// is settled.
    pub fn with_pooling(mut self, pooling: Pooling) -> Self {
        self.pooling = pooling;
        self
    }

    /// The EFFECTIVE candle device selected by the strict resolution at construction,
    /// as a stable label (`"cpu"` / `"cuda:N"` / `"metal:N"`).
    ///
    /// A forced CUDA policy never produces this value as CPU: it returns a
    /// typed error before model construction. This accessor does not change the
    /// embedder identity (device is not part of `EmbedderIdentity`).
    #[must_use]
    pub fn device_label(&self) -> String {
        match self.device.location() {
            candle_core::DeviceLocation::Cpu => "cpu".to_string(),
            candle_core::DeviceLocation::Cuda { gpu_id } => format!("cuda:{gpu_id}"),
            candle_core::DeviceLocation::Metal { gpu_id } => format!("metal:{gpu_id}"),
        }
    }
}

impl Embedder for CandleBgeEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        // Invariant 3 (ADR-0.6.0-embedder-protocol.md): no log/tracing/
        // println/eprintln/dbg in this function. Errors travel up via
        // the return value only.

        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| EmbedderError::Failed { message: format!("tokenize: {e}") })?;

        let ids: Vec<u32> = encoding.get_ids().to_vec();
        let attn: Vec<u32> = encoding.get_attention_mask().to_vec();
        let len = ids.len();

        let embed_impl = || -> candle_core::Result<Vec<f32>> {
            let input_ids = Tensor::from_vec(ids, (1, len), &self.device)?;
            let attn_mask_u32 = Tensor::from_vec(attn, (1, len), &self.device)?;
            let token_type_ids = input_ids.zeros_like()?;

            // BertModel::forward takes the raw (B, L) attention mask and
            // internally builds the additive mask. (B, L, D) f32 out.
            let hidden = self.model.forward(&input_ids, &token_type_ids, Some(&attn_mask_u32))?;

            // Pool (design §0.4 = Mean; Cls is the model-native mode, gated) then
            // L2-normalize.
            let pooled = match self.pooling {
                Pooling::Mean => mean_pool(&hidden, &attn_mask_u32)?,
                Pooling::Cls => cls_pool(&hidden)?,
            };
            let normed = l2_normalize(&pooled)?; // (1, D)

            let v: Vec<f32> = normed.squeeze(0)?.to_vec1::<f32>()?;
            Ok(v)
        };

        embed_impl().map_err(|e| EmbedderError::Failed { message: format!("forward: {e}") })
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        // Invariant 3 (ADR-0.6.0-embedder-protocol.md): no log/tracing in this fn.
        // One padded (B, L) forward instead of B single-row forwards. The attention
        // mask zeros padded positions, so the mean/cls pooling + L2-norm produce the
        // SAME per-row vectors as `embed()` (parity-locked in tests). On GPU this
        // turns minutes into seconds; on CPU it is a modest win.
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut encodings = Vec::with_capacity(inputs.len());
        for input in inputs {
            let enc = self
                .tokenizer
                .encode(*input, true)
                .map_err(|e| EmbedderError::Failed { message: format!("tokenize: {e}") })?;
            encodings.push(enc);
        }
        let batch = inputs.len();
        // tokenizer truncation pins each row <= MAX_SEQUENCE_TOKENS, so max_len <= 512.
        let max_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0).max(1);

        // Right-pad ids+mask to a common length; pad ids=0, mask=0 (mean/cls pooling
        // ignore masked positions, so padding cannot affect a row's vector).
        let mut ids = vec![0u32; batch * max_len];
        let mut attn = vec![0u32; batch * max_len];
        for (row, enc) in encodings.iter().enumerate() {
            let base = row * max_len;
            for (col, (&id, &mask)) in
                enc.get_ids().iter().zip(enc.get_attention_mask()).enumerate()
            {
                ids[base + col] = id;
                attn[base + col] = mask;
            }
        }

        let embed_impl = || -> candle_core::Result<Vec<Vec<f32>>> {
            let input_ids = Tensor::from_vec(ids, (batch, max_len), &self.device)?;
            let attn_mask_u32 = Tensor::from_vec(attn, (batch, max_len), &self.device)?;
            let token_type_ids = input_ids.zeros_like()?;

            let hidden = self.model.forward(&input_ids, &token_type_ids, Some(&attn_mask_u32))?;
            let pooled = match self.pooling {
                Pooling::Mean => mean_pool(&hidden, &attn_mask_u32)?,
                Pooling::Cls => cls_pool(&hidden)?,
            };
            let normed = l2_normalize(&pooled)?; // (B, D)
            normed.to_vec2::<f32>() // Vec<Vec<f32>>, one row per input
        };

        embed_impl().map_err(|e| EmbedderError::Failed { message: format!("batch forward: {e}") })
    }
}

impl CandleBgeEmbedder {
    /// Measurement-only: ONE forward pass → both the mean-pooled and CLS-pooled
    /// L2-normalized vectors, so an A/B harness can compare pooling strategies
    /// without paying the (dominant) embedding cost twice. Returns
    /// `(mean_pooled, cls_pooled)`. Not part of the `Embedder` trait.
    pub fn embed_dual_for_test(&self, input: &str) -> Result<(Vector, Vector), EmbedderError> {
        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| EmbedderError::Failed { message: format!("tokenize: {e}") })?;
        let ids: Vec<u32> = encoding.get_ids().to_vec();
        let attn: Vec<u32> = encoding.get_attention_mask().to_vec();
        let len = ids.len();

        let dual = || -> candle_core::Result<(Vec<f32>, Vec<f32>)> {
            let input_ids = Tensor::from_vec(ids, (1, len), &self.device)?;
            let attn_mask_u32 = Tensor::from_vec(attn, (1, len), &self.device)?;
            let token_type_ids = input_ids.zeros_like()?;
            let hidden = self.model.forward(&input_ids, &token_type_ids, Some(&attn_mask_u32))?;
            let mean = l2_normalize(&mean_pool(&hidden, &attn_mask_u32)?)?.squeeze(0)?.to_vec1()?;
            let cls = l2_normalize(&cls_pool(&hidden)?)?.squeeze(0)?.to_vec1()?;
            Ok((mean, cls))
        };
        dual().map_err(|e| EmbedderError::Failed { message: format!("forward: {e}") })
    }
}

// The legacy reranker-only parser tests remain in `crate::device`. Strict
// embedder-policy tests live in `device_policy`; the two controls do not share
// a grammar or fallback contract.

#[cfg(all(test, feature = "embed-cuda"))]
mod cuda_probe_error_tests {
    use super::*;
    use candle_core::{
        cuda::{
            cudarc::{
                cublas::{result::CublasError, sys::cublasStatus_t},
                driver::{result::DriverError, sys::CUresult},
            },
            CudaError,
        },
        Error,
    };

    #[test]
    fn driverless_and_known_driver_results_have_stable_classifications() {
        assert_eq!(classify_cuda_driver_presence(false), Err(CudaProbeError::NoVisibleDevice));
        assert_eq!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_NO_DEVICE)),
            CudaProbeError::NoVisibleDevice
        );
        assert_eq!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_STUB_LIBRARY)),
            CudaProbeError::NoVisibleDevice
        );
        assert!(matches!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_SYSTEM_DRIVER_MISMATCH)),
            CudaProbeError::Incompatible { .. }
        ));
        assert!(matches!(
            classify_cuda_driver_error(DriverError(
                CUresult::CUDA_ERROR_COMPAT_NOT_SUPPORTED_ON_DEVICE
            )),
            CudaProbeError::Incompatible { .. }
        ));
        assert!(matches!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_NO_BINARY_FOR_GPU)),
            CudaProbeError::Incompatible { .. }
        ));
        assert!(matches!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_UNSUPPORTED_PTX_VERSION)),
            CudaProbeError::Incompatible { .. }
        ));
        assert!(matches!(
            classify_cuda_driver_error(DriverError(CUresult::CUDA_ERROR_OUT_OF_MEMORY)),
            CudaProbeError::ProbeFailed { .. }
        ));
    }

    #[test]
    fn candle_provider_compatibility_is_not_an_unknown_probe_failure() {
        let error = Error::Cuda(Box::new(CudaError::Cublas(CublasError(
            cublasStatus_t::CUBLAS_STATUS_ARCH_MISMATCH,
        ))));
        assert!(matches!(classify_candle_cuda_error(error), CudaProbeError::Incompatible { .. }));

        let error = Error::Cuda(Box::new(CudaError::Cublas(CublasError(
            cublasStatus_t::CUBLAS_STATUS_NOT_SUPPORTED,
        ))));
        assert!(matches!(classify_candle_cuda_error(error), CudaProbeError::Incompatible { .. }));

        let error = Error::Cuda(Box::new(CudaError::Cublas(CublasError(
            cublasStatus_t::CUBLAS_STATUS_ALLOC_FAILED,
        ))));
        assert!(matches!(classify_candle_cuda_error(error), CudaProbeError::ProbeFailed { .. }));
    }
}
