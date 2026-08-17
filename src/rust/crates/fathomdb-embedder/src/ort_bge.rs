//! `OrtBgeEmbedder` — cross-vendor ONNX-Runtime BGE-small embedder.
//!
//! A caller-supplied `impl fathomdb_embedder_api::Embedder`, sibling of
//! `candle_bge` / `nomic`, that produces `BAAI/bge-small-en-v1.5` vectors
//! (dim 384) through the `ort` ONNX-Runtime binding. Injected purely via
//! `EmbedderChoice::Caller(Arc::new(OrtBgeEmbedder::…))` — the engine never
//! names it, so there is ZERO engine change (ADR-0.8.16-onnx-embedder-backend
//! §2). The `Default` variant stays candle-only, preserving the footprint
//! invariant.
//!
//! ONNX Runtime is an opt-in backend behind the NON-default `onnx-embedder`
//! Cargo feature, so the thin default build gains zero dependencies
//! (EMB-3 wheel-size gate). Its CPU/CUDA selection follows the same strict
//! `FATHOMDB_EMBED_DEVICE` policy as the default embedder.
//!
//! Numeric equivalence to the candle reference is MEASURED (not enforced) at
//! Slice 15 (ADR §3 / design §5); the interim guard is same-backend
//! build-and-read, enforced here structurally by giving ONNX a DISTINCT
//! embedder identity name (`…-onnx`) so the engine's identity check never
//! silently reads candle-written vectors with the ONNX backend.

use std::path::Path;
use std::sync::Mutex;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use ort::execution_providers::{
    CPUExecutionProvider, CUDAExecutionProvider, ExecutionProvider, ExecutionProviderDispatch,
};
use ort::session::Session;
use ort::value::Tensor;
use tokenizers::{Tokenizer, TruncationParams};

use crate::{
    resolve_embed_device_policy_from_env, CudaDeviceInfo, CudaProbeError, CudaProvider,
    DeviceResolution, EffectiveEmbedDevice, EmbedDevicePolicy, EmbedDevicePolicyError,
};

#[cfg(test)]
use crate::{resolve_embed_device_policy, DeviceResolutionError};

/// Engine-facing identity name. Deliberately DISTINCT from the candle default
/// (`fathomdb-bge-small-en-v1.5`) so the engine's identity check enforces the
/// R-ONNX-3 same-backend build-and-read discipline: candle-written vectors and
/// ONNX-read queries never silently mix until 0.8.18 #5 enforces a candle↔ONNX
/// tolerance (ADR §3).
pub const ORT_BGE_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5-onnx";

/// Output dimension for `bge-small-en-v1.5` (matches the candle reference).
pub const ORT_BGE_EMBEDDER_DIM: u32 = 384;

/// Pinned HF revision of `BAAI/bge-small-en-v1.5` — same commit the candle
/// loader pins (`loader::HF_REVISION`), recorded so an ONNX build is traceable
/// to the same upstream weights the equivalence measurement compares against.
/// This is the BASE of the composed identity revision; the actually-loaded
/// asset digest is appended (`+onnx-<hex>`) so the identity self-describes the
/// bytes that were opened (see [`derive_asset_revision`]).
const HF_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";

/// Tokenizer truncation ceiling — BGE-small's 512-slot learned position
/// embeddings, identical to the candle path (`candle_bge::MAX_SEQUENCE_TOKENS`).
const MAX_SEQUENCE_TOKENS: usize = 512;

/// Sentence-vector pooling. Mirrors `candle_bge::Pooling`. Default is
/// [`OrtPooling::Cls`] — the model-native, CLS-corrected mode the candle
/// reference is compared against at Slice 15 (design §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrtPooling {
    /// Mean over the attention mask (candle's historical default).
    Mean,
    /// `[CLS]` token (position 0) — the mode BGE-small was trained for.
    Cls,
}

/// An ORT execution provider selected by the shared strict device policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OrtProvider {
    Cpu,
    Cuda(i32),
}

impl OrtProvider {
    /// Stable string label for the provider (index dropped — the calibration
    /// leg records the requested device string separately). Used by the
    /// additive [`OrtBgeEmbedder::effective_provider`] accessor.
    fn label(self) -> &'static str {
        match self {
            OrtProvider::Cpu => "cpu",
            OrtProvider::Cuda(_) => "cuda",
        }
    }
}

/// ONNX Runtime's CUDA availability probe expressed through the shared
/// default-embedder policy boundary.
struct OrtCudaProvider;

impl CudaProvider for OrtCudaProvider {
    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        if CUDAExecutionProvider::default().is_available().unwrap_or(false) {
            Ok(CudaDeviceInfo {
                ordinal,
                name: None,
                driver_version: None,
                compute_capability: None,
                cuda_toolkit_version: None,
            })
        } else {
            Err(CudaProbeError::NoVisibleDevice)
        }
    }
}

#[cfg(test)]
fn resolve_ort_device_policy_with(
    policy: EmbedDevicePolicy,
    provider: &mut dyn CudaProvider,
) -> Result<DeviceResolution, DeviceResolutionError> {
    resolve_embed_device_policy(policy, true, provider)
}

fn ort_provider_from_resolution(resolution: &DeviceResolution) -> OrtProvider {
    match &resolution.effective_device {
        EffectiveEmbedDevice::Cpu => OrtProvider::Cpu,
        EffectiveEmbedDevice::Cuda(info) => OrtProvider::Cuda(info.ordinal as i32),
    }
}

fn resolve_ort_device_policy_from_env(
) -> Result<(EmbedDevicePolicy, DeviceResolution), EmbedDevicePolicyError> {
    let mut provider = OrtCudaProvider;
    let resolution = resolve_embed_device_policy_from_env(true, &mut provider)?;
    Ok((resolution.requested_policy, resolution))
}

/// Build the concrete `ort` execution-provider dispatch for a resolved
/// [`OrtProvider`].
///
/// ORT's default CUDA dispatch can fall back silently. Mark it
/// `error_on_failure` so the product policy, not ORT, decides whether `auto`
/// may use CPU. Forced CUDA therefore cannot become a CPU session.
fn provider_dispatch(provider: OrtProvider) -> ExecutionProviderDispatch {
    match provider {
        OrtProvider::Cpu => CPUExecutionProvider::default().build(),
        OrtProvider::Cuda(idx) => {
            CUDAExecutionProvider::default().with_device_id(idx).build().error_on_failure()
        }
    }
}

/// Build an ORT session under the same policy as the default Candle embedder.
/// `auto` may retry CPU after a CUDA build failure; forced CUDA returns that
/// failure untouched.
fn build_session_with_policy<S, E, F>(
    policy: EmbedDevicePolicy,
    effective: OrtProvider,
    build: F,
) -> Result<(S, Option<String>), E>
where
    F: Fn(OrtProvider) -> Result<S, E>,
    E: std::fmt::Display,
{
    match build(effective) {
        Ok(session) => Ok((session, None)),
        // A CPU build failure is a genuine error — there is nothing to fall
        // back to, so do not retry.
        Err(e) if matches!(effective, OrtProvider::Cpu) => Err(e),
        Err(e) if matches!(policy, EmbedDevicePolicy::Auto) => {
            let session = build(OrtProvider::Cpu)?;
            Ok((session, Some(format!("auto CUDA ONNX session build failed ({e}); selected CPU"))))
        }
        Err(e) => Err(e),
    }
}

/// Stream a file through SHA-256, returning the 32-byte digest. Read in fixed
/// chunks rather than buffering the whole file so a ~130 MB `.onnx` export is
/// not held in memory (this runs once, at construction).
fn sha256_file(path: &Path) -> Result<[u8; 32], EmbedderError> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| err("asset digest open", e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let n =
            std::io::Read::read(&mut file, &mut buf).map_err(|e| err("asset digest read", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// PURE composition of the SELF-DESCRIBING identity revision from the two asset
/// content digests (codex §9 fix-5, remedy (b)). The revision is the pinned
/// base [`HF_REVISION`] plus a short hex of `SHA-256(model_digest ||
/// tokenizer_digest)`:
///
/// `"<HF_REVISION>+onnx-<12 hex>"`
///
/// Net effect: two different model/tokenizer assets → two DIFFERENT revisions
/// (hence different [`EmbedderIdentity`], so the engine rejects mixing vectors
/// written under a different asset), while the SAME asset yields a STABLE
/// revision across opens. Composing the two per-file digests into one final
/// hash is deterministic and unambiguous (both inputs are fixed 32-byte
/// digests, so there is no boundary ambiguity). Digest inputs make this a pure
/// function — no ORT session, unit-testable with arbitrary bytes.
fn compose_asset_revision(model_digest: &[u8; 32], tokenizer_digest: &[u8; 32]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(model_digest);
    hasher.update(tokenizer_digest);
    let combined = hasher.finalize();
    let mut hex = String::with_capacity(12);
    for byte in combined.iter().take(6) {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("{HF_REVISION}+onnx-{hex}")
}

/// Derive the self-describing identity revision from the ACTUALLY-loaded asset
/// files on disk: hash the model bytes and the tokenizer bytes, then compose
/// via [`compose_asset_revision`]. Takes paths (not an ORT session) so the
/// revision derivation is exercisable with arbitrary temp files in tests.
fn derive_asset_revision(
    model_path: &Path,
    tokenizer_path: &Path,
) -> Result<String, EmbedderError> {
    let model_digest = sha256_file(model_path)?;
    let tokenizer_digest = sha256_file(tokenizer_path)?;
    Ok(compose_asset_revision(&model_digest, &tokenizer_digest))
}

/// Cross-vendor ONNX-Runtime BGE-small embedder.
///
/// `Session::run` needs `&mut self` but the `Embedder` trait is `&self` +
/// `Send + Sync`, so the session lives behind a `Mutex`. Embedding is a short
/// forward pass, so lock contention is not a concern for the offline/eval use
/// this backend targets.
pub struct OrtBgeEmbedder {
    identity: EmbedderIdentity,
    tokenizer: Tokenizer,
    session: Mutex<Session>,
    pooling: OrtPooling,
    /// The effective ORT execution provider this session was built with.
    /// Captured at construction and exposed via
    /// [`OrtBgeEmbedder::effective_provider`].
    effective_provider: OrtProvider,
}

fn err(context: &str, e: impl std::fmt::Display) -> EmbedderError {
    EmbedderError::Failed { message: format!("ort_bge {context}: {e}") }
}

impl OrtBgeEmbedder {
    /// Construct from an on-disk `.onnx` model + `tokenizer.json`, selecting the
    /// ORT execution provider at runtime from `FATHOMDB_EMBED_DEVICE`.
    /// Paths are caller-supplied (no hardcoded absolute path) so the model is an
    /// offline-build/eval asset the caller provisions.
    pub fn from_files(model_path: &Path, tokenizer_path: &Path) -> Result<Self, EmbedderError> {
        let (policy, resolution) =
            resolve_ort_device_policy_from_env().map_err(|error| err("device policy", error))?;
        Self::from_files_with_policy(
            model_path,
            tokenizer_path,
            policy,
            ort_provider_from_resolution(&resolution),
        )
    }

    /// Construct from `FATHOMDB_ONNX_MODEL_PATH` + `FATHOMDB_ONNX_TOKENIZER_PATH`
    /// (device from `FATHOMDB_EMBED_DEVICE`). The env-driven entry point an eval
    /// harness / caller uses to engage the ONNX backend without recompiling.
    pub fn from_env() -> Result<Self, EmbedderError> {
        let model = std::env::var("FATHOMDB_ONNX_MODEL_PATH")
            .map_err(|_| err("from_env", "FATHOMDB_ONNX_MODEL_PATH is unset"))?;
        let tok = std::env::var("FATHOMDB_ONNX_TOKENIZER_PATH")
            .map_err(|_| err("from_env", "FATHOMDB_ONNX_TOKENIZER_PATH is unset"))?;
        Self::from_files(Path::new(&model), Path::new(&tok))
    }

    #[cfg(test)]
    fn from_files_with_provider(
        model_path: &Path,
        tokenizer_path: &Path,
        provider: OrtProvider,
    ) -> Result<Self, EmbedderError> {
        Self::from_files_with_policy(model_path, tokenizer_path, EmbedDevicePolicy::Cpu, provider)
    }

    fn from_files_with_policy(
        model_path: &Path,
        tokenizer_path: &Path,
        policy: EmbedDevicePolicy,
        provider: OrtProvider,
    ) -> Result<Self, EmbedderError> {
        let mut tokenizer =
            Tokenizer::from_file(tokenizer_path).map_err(|e| err("tokenizer load", e))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQUENCE_TOKENS,
                ..Default::default()
            }))
            .map_err(|e| err("tokenizer truncation", e))?;

        let (session, build_warn) = build_session_with_policy(policy, provider, |p| {
            Session::builder()?
                .with_execution_providers([provider_dispatch(p)])?
                .commit_from_file(model_path)
        })
        .map_err(|e| err("session build", e))?;
        let effective_provider = if build_warn.is_some() { OrtProvider::Cpu } else { provider };

        // Self-describing identity (codex §9 fix-5): derive the revision from a
        // content digest of the ACTUALLY-loaded model + tokenizer assets rather
        // than advertising the pinned base revision unconditionally. A stale or
        // alternate FATHOMDB_ONNX_MODEL_PATH now yields a DIFFERENT identity, so
        // the engine's EmbedderIdentity check rejects reading vectors written
        // under a different embedding space (R-ONNX-3 same-asset discipline).
        let revision = derive_asset_revision(model_path, tokenizer_path)?;
        let identity = EmbedderIdentity::new(ORT_BGE_EMBEDDER_NAME, revision, ORT_BGE_EMBEDDER_DIM);

        Ok(Self {
            identity,
            tokenizer,
            session: Mutex::new(session),
            pooling: OrtPooling::Cls,
            effective_provider,
        })
    }

    /// Select the pooling strategy (default [`OrtPooling::Cls`]). Does NOT change
    /// the identity — use only on a fresh workspace / in the equivalence harness.
    #[must_use]
    pub fn with_pooling(mut self, pooling: OrtPooling) -> Self {
        self.pooling = pooling;
        self
    }

    /// The effective ORT execution provider label (`"cpu"` / `"cuda"`) this
    /// session was built with. A forced CUDA request never becomes CPU; `auto`
    /// may select CPU when CUDA cannot be used. Does not change identity.
    #[must_use]
    pub fn effective_provider(&self) -> &'static str {
        self.effective_provider.label()
    }
}

impl Embedder for OrtBgeEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        let encoding = self.tokenizer.encode(input, true).map_err(|e| err("tokenize", e))?;
        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| i64::from(x)).collect();
        let mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| i64::from(x)).collect();
        let len = ids.len();
        let token_type: Vec<i64> = vec![0; len];
        let shape = vec![1_i64, len as i64];

        let ids_t = Tensor::from_array((shape.clone(), ids)).map_err(|e| err("input_ids", e))?;
        let mask_t =
            Tensor::from_array((shape.clone(), mask)).map_err(|e| err("attention_mask", e))?;
        let tt_t = Tensor::from_array((shape, token_type)).map_err(|e| err("token_type_ids", e))?;

        let mut session = self.session.lock().map_err(|_| err("session lock", "poisoned"))?;
        let outputs = session
            .run(ort::inputs![
                "input_ids" => ids_t,
                "attention_mask" => mask_t,
                "token_type_ids" => tt_t,
            ])
            .map_err(|e| err("forward", e))?;

        // last_hidden_state — first output, shape (1, L, H).
        let (out_shape, data) =
            outputs[0].try_extract_tensor::<f32>().map_err(|e| err("extract", e))?;
        let dims: Vec<usize> = out_shape.iter().map(|&d| d as usize).collect();
        if dims.len() != 3 {
            return Err(err("output shape", format!("expected rank-3 (1,L,H), got {dims:?}")));
        }
        let (seq_len, hidden) = (dims[1], dims[2]);
        if hidden != ORT_BGE_EMBEDDER_DIM as usize {
            return Err(err(
                "output dim",
                format!("expected hidden {ORT_BGE_EMBEDDER_DIM}, got {hidden}"),
            ));
        }

        // Pool. `embed()` is single-input (no padding), so the attention mask is
        // all-ones and mean-pool reduces to a plain mean over `seq_len`.
        let mut pooled = vec![0.0_f32; hidden];
        match self.pooling {
            OrtPooling::Cls => {
                pooled.copy_from_slice(&data[0..hidden]);
            }
            OrtPooling::Mean => {
                for pos in 0..seq_len {
                    let base = pos * hidden;
                    for (j, slot) in pooled.iter_mut().enumerate() {
                        *slot += data[base + j];
                    }
                }
                let denom = seq_len.max(1) as f32;
                for slot in &mut pooled {
                    *slot /= denom;
                }
            }
        }

        // L2-normalize (matches candle's `l2_normalize`).
        let norm = pooled.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
        for slot in &mut pooled {
            *slot /= norm;
        }
        Ok(pooled)
    }
}

#[cfg(test)]
mod tests {
    use super::OrtProvider;
    use crate::EmbedDevicePolicy;

    #[test]
    fn strict_onnx_forced_cuda_never_resolves_to_cpu() {
        use crate::{
            CudaProbeError, CudaProvider, DeviceResolutionError, DeviceResolutionReason,
            EmbedDevicePolicy,
        };

        struct UnavailableCudaProvider;

        impl CudaProvider for UnavailableCudaProvider {
            fn probe_cuda(
                &mut self,
                _ordinal: usize,
            ) -> Result<crate::CudaDeviceInfo, CudaProbeError> {
                Err(CudaProbeError::NoVisibleDevice)
            }
        }

        let error = super::resolve_ort_device_policy_with(
            EmbedDevicePolicy::Cuda(2),
            &mut UnavailableCudaProvider,
        )
        .expect_err("forced CUDA must fail rather than become an ONNX CPU session");

        assert_eq!(
            error,
            DeviceResolutionError::ForcedCudaUnavailable {
                ordinal: 2,
                reason: DeviceResolutionReason::NoVisibleCudaDevice,
            },
        );
    }

    #[test]
    fn forced_cuda_session_build_never_retries_cpu() {
        use std::cell::RefCell;

        let attempts = RefCell::new(Vec::new());
        let result = super::build_session_with_policy(
            EmbedDevicePolicy::Cuda(2),
            OrtProvider::Cuda(2),
            |provider| -> Result<&'static str, &'static str> {
                attempts.borrow_mut().push(provider);
                Err("CUDA session unavailable")
            },
        );

        assert_eq!(result, Err("CUDA session unavailable"));
        assert_eq!(*attempts.borrow(), vec![OrtProvider::Cuda(2)]);
    }

    #[test]
    fn auto_cuda_session_build_may_retry_cpu() {
        use std::cell::RefCell;

        let attempts = RefCell::new(Vec::new());
        let (session, warning) = super::build_session_with_policy(
            EmbedDevicePolicy::Auto,
            OrtProvider::Cuda(0),
            |provider| -> Result<&'static str, &'static str> {
                attempts.borrow_mut().push(provider);
                match provider {
                    OrtProvider::Cuda(_) => Err("CUDA session unavailable"),
                    OrtProvider::Cpu => Ok("cpu session"),
                }
            },
        )
        .expect("auto may retry CPU after a CUDA session failure");

        assert_eq!(session, "cpu session");
        assert!(warning.is_some());
        assert_eq!(*attempts.borrow(), vec![OrtProvider::Cuda(0), OrtProvider::Cpu]);
    }

    #[test]
    fn auto_cuda_session_build_fallback_is_recorded_in_device_resolution() {
        let resolution = crate::DeviceResolution {
            requested_policy: EmbedDevicePolicy::Auto,
            cuda_compiled: true,
            effective_device: crate::EffectiveEmbedDevice::Cuda(crate::CudaDeviceInfo {
                ordinal: 0,
                name: None,
                driver_version: None,
                compute_capability: None,
                cuda_toolkit_version: None,
            }),
            reason: None,
        };

        let (session, resolution) = super::build_session_with_device_resolution(
            resolution,
            |provider| -> Result<&'static str, &'static str> {
                match provider {
                    OrtProvider::Cuda(_) => Err("CUDA session unavailable"),
                    OrtProvider::Cpu => Ok("cpu session"),
                }
            },
        )
        .expect("auto may retry CPU after a CUDA session failure");

        assert_eq!(session, "cpu session");
        assert_eq!(resolution.effective_device, crate::EffectiveEmbedDevice::Cpu);
        assert_eq!(resolution.reason, Some(crate::DeviceResolutionReason::CudaProbeFailed),);
    }

    /// codex §9 fix-5 — the identity revision self-describes the loaded assets.
    /// Written a set of arbitrary temp "model"/"tokenizer" byte contents and
    /// assert the derived revision distinguishes distinct assets, is stable for
    /// identical bytes, is sensitive to EITHER file changing, keeps the pinned
    /// base + `+onnx-<12 hex>` shape, and does not depend on file NAMES (only
    /// bytes). Pure — arbitrary temp files, no ORT session / native lib.
    mod asset_revision {
        use std::io::Write as _;
        use std::path::Path;

        use super::super::{compose_asset_revision, derive_asset_revision, HF_REVISION};

        fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
            // Honors TMPDIR — the sandboxed scratch dir set for this build.
            let dir = std::env::temp_dir();
            let path = dir.join(format!("fathomdb-ort-rev-{}-{name}", std::process::id()));
            let mut f = std::fs::File::create(&path).expect("create temp asset");
            f.write_all(bytes).expect("write temp asset");
            f.flush().expect("flush temp asset");
            path
        }

        fn revision_of(model_bytes: &[u8], tok_bytes: &[u8], tag: &str) -> String {
            let m = write_temp(&format!("model-{tag}.onnx"), model_bytes);
            let t = write_temp(&format!("tok-{tag}.json"), tok_bytes);
            let rev = derive_asset_revision(Path::new(&m), Path::new(&t)).expect("derive revision");
            let _ = std::fs::remove_file(&m);
            let _ = std::fs::remove_file(&t);
            rev
        }

        #[test]
        fn distinct_assets_yield_distinct_revisions() {
            let a = revision_of(b"model-bytes-AAAA", b"tokenizer-bytes-AAAA", "a");
            let b = revision_of(b"model-bytes-BBBB", b"tokenizer-bytes-AAAA", "b");
            assert_ne!(a, b, "a different MODEL must change the identity revision");

            let c = revision_of(b"model-bytes-AAAA", b"tokenizer-bytes-CCCC", "c");
            assert_ne!(a, c, "a different TOKENIZER must change the identity revision");
            assert_ne!(b, c, "distinct model+tokenizer pairs must differ");
        }

        #[test]
        fn identical_bytes_yield_stable_revision() {
            // Same bytes, DIFFERENT file names/paths + separate opens → identical
            // revision (stable/deterministic; depends only on content).
            let first = revision_of(b"same-model-bytes", b"same-tokenizer-bytes", "stable1");
            let second = revision_of(b"same-model-bytes", b"same-tokenizer-bytes", "stable2");
            assert_eq!(first, second, "identical asset bytes must yield a stable revision");
        }

        #[test]
        fn revision_has_pinned_base_and_onnx_digest_shape() {
            let rev = revision_of(b"m", b"t", "shape");
            let prefix = format!("{HF_REVISION}+onnx-");
            let hex = rev.strip_prefix(&prefix).expect("revision must start with pinned base");
            assert_eq!(hex.len(), 12, "digest suffix must be 12 hex chars, got {hex:?}");
            assert!(
                hex.chars().all(|c| c.is_ascii_hexdigit()),
                "digest suffix must be lower-hex, got {hex:?}"
            );
        }

        #[test]
        fn compose_is_pure_and_order_sensitive() {
            let m = [1_u8; 32];
            let t = [2_u8; 32];
            // Pure: same inputs → same output.
            assert_eq!(compose_asset_revision(&m, &t), compose_asset_revision(&m, &t));
            // Model/tokenizer roles are distinct positions in the composed hash,
            // so swapping the two digests generally changes the revision.
            assert_ne!(
                compose_asset_revision(&m, &t),
                compose_asset_revision(&t, &m),
                "the two asset roles must not be interchangeable"
            );
        }
    }

    /// R-ONNX-1 real-vector test — runs FOR REAL against the offline ONNX asset
    /// on the ONNX-Runtime CPU EP (the same-backend fidelity baseline per policy
    /// `649a8d45`; GPU is a Slice-15 re-embed speed concern, not needed here).
    ///
    /// ENV-GATED (not `#[ignore]`) so CI without the provisioned asset skips it
    /// cleanly while a provisioned host runs it: set `ORT_DYLIB_PATH` (the
    /// on-host `libonnxruntime.so`, load-dynamic), `FATHOMDB_ONNX_MODEL_PATH`
    /// (the offline export from `dev/tools/onnx/export_bge_small_onnx.py`), and
    /// `FATHOMDB_ONNX_TOKENIZER_PATH` (the pinned `tokenizer.json`). When those
    /// are unset the test returns early (recorded skip); when set it asserts a
    /// fixture text → a 384-dim, finite, L2-normalized, deterministic vector via
    /// the `Embedder` trait, and that the identity revision self-describes the
    /// loaded asset digest (fix-5). See `dev/tools/onnx/README.md` for the exact
    /// invocation and the chosen ORT lib.
    #[test]
    fn ort_bge_embeds_384_dim_finite_deterministic_vector() {
        use std::path::Path;

        use fathomdb_embedder_api::Embedder;

        use super::{OrtBgeEmbedder, OrtProvider};

        let (Ok(_dylib), Ok(model), Ok(tok)) = (
            std::env::var("ORT_DYLIB_PATH"),
            std::env::var("FATHOMDB_ONNX_MODEL_PATH"),
            std::env::var("FATHOMDB_ONNX_TOKENIZER_PATH"),
        ) else {
            eprintln!(
                "SKIP ort_bge_embeds_384_dim_finite_deterministic_vector: set ORT_DYLIB_PATH + \
                 FATHOMDB_ONNX_MODEL_PATH + FATHOMDB_ONNX_TOKENIZER_PATH to run the real-vector \
                 R-ONNX-1 test (see dev/tools/onnx/README.md)"
            );
            return;
        };

        // CPU same-backend baseline (policy 649a8d45): force the ONNX CPU EP
        // explicitly (not via the ambient FATHOMDB_EMBED_DEVICE) so this
        // fidelity assertion is deterministic and does not depend on env order.
        let embedder = OrtBgeEmbedder::from_files_with_provider(
            Path::new(&model),
            Path::new(&tok),
            OrtProvider::Cpu,
        )
        .expect("open OrtBgeEmbedder on the CPU EP from the provisioned asset");

        // Identity self-describes the loaded asset (fix-5): distinct ONNX name,
        // dim 384, revision = pinned base + "+onnx-<12 hex>" asset digest.
        let id = embedder.identity();
        assert_eq!(id.name, super::ORT_BGE_EMBEDDER_NAME, "distinct ONNX identity name");
        assert_eq!(id.dimension, super::ORT_BGE_EMBEDDER_DIM, "384-dim identity");
        assert!(
            id.revision.contains("+onnx-"),
            "revision must carry the asset digest, got {:?}",
            id.revision
        );

        let v1 = embedder.embed("the quick brown fox").expect("embed");
        let v2 = embedder.embed("the quick brown fox").expect("embed");
        assert_eq!(v1.len(), super::ORT_BGE_EMBEDDER_DIM as usize);
        assert!(v1.iter().all(|x| x.is_finite()), "all components finite");
        assert_eq!(v1, v2, "deterministic for identical input");
        let norm = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "L2-normalized (got {norm})");
    }
}
