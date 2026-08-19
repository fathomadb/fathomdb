//! Default-embedder weight loader.
//!
//! Implements the contract specified in
//! `dev/plans/prompts/0.7.1-EMBEDDER-UNDEFER-HANDOFF.md` §EU-3 (which
//! itself materialises `dev/design/embedder.md` §§1–10):
//!
//! 1. HF resolve URL pattern, three pinned files, sha256 pinned in Rust.
//! 2. `ureq` blocking transport, 10s connect / 60s read, 3-attempt backoff,
//!    Range resume on partial.
//! 3. `HF_TOKEN` env var → `Authorization: Bearer`. No keychain. No file.
//! 4. `<dirs::cache_dir>/fathomdb/embedders/<model-sha-prefix>/<file>`;
//!    best-effort read-only probe of `~/.cache/huggingface` for users who
//!    already have the file.
//! 5. `.partial` → fsync → rename.
//! 6. sha256-verify; on mismatch remove `.partial` and fail with
//!    `EmbedderLoadError::ChecksumMismatch`. No trust-on-first-use.
//! 7. Returns `Vec<EmbedderEvent>` so EU-5 can splice it into
//!    `OpenReport.embedder_events` without re-running the work.
//! 9. `EmbedderLoadError` taxonomy mirrors the design.
//! 10. `fs2::FileExt::lock_exclusive` on a `.lock` sibling for the cache
//!     directory; held only during fetch+verify+rename; cache-hit path does
//!     NOT take the lock.
//!
//! Candle / `BertModel` construction is **EU-4** and is intentionally NOT
//! in this slice. The loader's deliverable is byte-buffer-backed file
//! paths plus an events log.
//!
//! ## Scope guardrails (ADR-0.7.1-default-embedder-weight-fetch)
//!
//! Pinned constants are `pub(crate) const`, not `pub const`. No public
//! function takes a `&str` model name, URL, or repo. The only way to
//! reach the loader from outside this module is the zero-arg entry
//! point `load_pinned_default_embedder()`. Tests reach the override
//! surface via `LoaderConfig::for_tests()` which is `cfg(any(test, ...))`
//! gated — see the impl below.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use fs2::FileExt;
use sha2::{Digest, Sha256};
use thiserror::Error;

// ----- Pinned constants (design §1) -----------------------------------------

/// Hugging Face model repository hosting the default embedder weights.
pub(crate) const HF_REPO: &str = "BAAI/bge-small-en-v1.5";

/// Pinned revision (commit SHA) on the HF repo. Bumping this is a deliberate
/// release-engineering action, not a runtime input.
///
/// Exposed as `pub` only under `cfg(any(test, feature = "loader-test-hooks"))`
/// so the integration test crate can reference it directly (avoiding the
/// drift hazard of a duplicated constant). Production callers see
/// `pub(crate)` and cannot reach this symbol from another crate.
#[cfg(any(test, feature = "loader-test-hooks"))]
pub const HF_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";
#[cfg(not(any(test, feature = "loader-test-hooks")))]
pub(crate) const HF_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";

/// Production HF base URL. Tests override via `LoaderConfig::with_base_url`.
pub(crate) const HF_BASE_URL: &str = "https://huggingface.co";

/// sha256 of `config.json` at `HF_REVISION`. Computed once via
/// `curl -sL <resolve-url> | sha256sum` and pinned here.
pub(crate) const CONFIG_JSON_SHA256: &str =
    "094f8e891b932f2000c92cfc663bac4c62069f5d8af5b5278c4306aef3084750";

/// sha256 of `tokenizer.json` at `HF_REVISION`.
pub(crate) const TOKENIZER_JSON_SHA256: &str =
    "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66";

/// sha256 of `model.safetensors` at `HF_REVISION`.
pub(crate) const MODEL_SAFETENSORS_SHA256: &str =
    "3c9f31665447c8911517620762200d2245a2518d6e7208acc78cd9db317e21ad";

/// Default connect timeout (design §2).
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default read timeout (design §2).
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(60);
/// Total attempts including the first (design §2).
///
/// Design §2 calls out "three attempts per file, with exponential backoff
/// of 1s, 2s, 4s between attempts." We interpret that literally as **3
/// attempt slots** (initial + 2 retries), with sleeps of **1s and 2s**
/// between them — the trailing "4s" in the design phrasing is the
/// hypothetical sleep that would precede a fourth slot we never make.
/// See Issue 4 in the EU-3 FIX-1 review notes.
const MAX_ATTEMPTS: u32 = 3;
/// Default lock acquisition timeout (design §10).
const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(120);

/// Env var consulted for the lock timeout override (design §10).
pub(crate) const ENV_LOCK_TIMEOUT: &str = "FATHOMDB_EMBEDDER_LOCK_TIMEOUT_S";

/// Env var consulted for the HTTP connect timeout override (design §2).
pub(crate) const ENV_CONNECT_TIMEOUT: &str = "FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S";

/// Env var consulted for the HTTP read timeout override (design §2).
pub(crate) const ENV_READ_TIMEOUT: &str = "FATHOMDB_EMBEDDER_READ_TIMEOUT_S";

/// Parse a `u64`-seconds env var, returning the default on missing/invalid.
/// On invalid input emits a `stderr` warning (no panic, no `unwrap`).
fn parse_secs_env_or_default(var: &str, default: Duration) -> Duration {
    match std::env::var(var) {
        Ok(s) => match s.parse::<u64>() {
            Ok(n) => Duration::from_secs(n),
            Err(_) => {
                eprintln!(
                    "fathomdb-embedder: invalid value for {var} ({s:?}); falling back to default \
                     {default:?}"
                );
                default
            }
        },
        Err(_) => default,
    }
}

/// Returns the 12-hex-char model-sha-prefix used in the cache layout
/// (design §4): `sha256("<repo>@<revision>")[..12]`. Computed once and
/// memoized — the inputs are compile-time constants but `sha2::Sha256`
/// has no const-eval path, so we lazy-cache.
fn model_sha_prefix() -> &'static str {
    static PREFIX: OnceLock<String> = OnceLock::new();
    PREFIX.get_or_init(|| {
        let mut h = Sha256::new();
        h.update(format!("{HF_REPO}@{HF_REVISION}").as_bytes());
        // digest 0.11's `Array` output drops the `LowerHex` impl `GenericArray`
        // had; format explicitly to the same lowercase, zero-padded hex.
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        hex[..12].to_string()
    })
}

// ----- Public types ---------------------------------------------------------

/// Handles into the verified weight cache.
///
/// EU-4 will accept this and construct `BertModel` + `Tokenizer` from these
/// paths. The loader's contract is "the paths exist on disk and their bytes
/// hash to the pinned constants" — nothing more.
#[derive(Debug, Clone)]
pub struct LoadedWeights {
    pub config_json_path: PathBuf,
    pub tokenizer_json_path: PathBuf,
    pub model_safetensors_path: PathBuf,
    /// Bytes pulled over the network during this call. `0` on a full cache
    /// hit (cold-start of a process whose cache is already populated).
    pub bytes_downloaded: u64,
    /// Structured events surfaced into `OpenReport.embedder_events` by
    /// EU-5. Ordering matches the order the loader observed them.
    pub events: Vec<EmbedderEvent>,
}

// `EmbedderEvent` lives at the crate root (`super::EmbedderEvent`) so the
// engine can reference it without enabling the `default-embedder` feature.
// Re-exported here for ergonomic use in loader.rs.
pub use super::EmbedderEvent;

/// Failure taxonomy (design §9). Engine-level mapping is owned by EU-5.
#[derive(Debug, Error)]
pub enum EmbedderLoadError {
    /// Network unavailable after retry exhaustion.
    ///
    /// `source` is widened from `ureq::Error` to `Box<dyn Error + Send + Sync>`
    /// so it can carry both real `ureq::Error` values from connect / status
    /// failures AND wrapped `io::Error`s from mid-stream response-body reads
    /// (ureq's `Transport` has no public constructor, so we cannot forge a
    /// `ureq::Error::Transport` from an `io::Error`). Design §9 reflects
    /// this shape.
    #[error("network unavailable after {attempts} attempts: {source}")]
    NetworkUnavailable {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        attempts: u32,
    },

    #[error("checksum mismatch for {file:?}: expected {expected}, actual {actual}")]
    ChecksumMismatch { file: PathBuf, expected: String, actual: String },

    /// Pinned model config violates the protocol expected by the embedder
    /// (e.g. `hidden_size` does not match `DEFAULT_EMBEDDER_DIM`). This is
    /// distinct from `ModelDeserialize` (a parse failure) — the bytes
    /// parsed cleanly but their content disagreed with a hard-coded
    /// invariant. Always points at a deliberate model/version drift.
    #[error("model dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: u32, actual: u32 },

    /// An explicitly requested benchmark device could not be constructed.
    /// The cache-only factory never converts this failure into a CPU fallback.
    #[error("requested device {device} is unavailable: {reason}")]
    DeviceUnavailable { device: String, reason: String },

    #[error("cache I/O error at {path:?}: {source}")]
    CacheIoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Model byte → `BertModel` parse failure (EU-4). Wraps the candle
    /// error verbatim so the engine layer (EU-5) can pattern-match on
    /// concrete variants when classifying.
    #[error("model deserialize: {source}")]
    ModelDeserialize {
        #[source]
        source: candle_core::Error,
    },

    /// A previously selected CUDA device could not be constructed for the
    /// model. This is never converted into a CPU fallback.
    #[error("selected embedder device could not be initialized: {message}")]
    DeviceInitialization { message: String },

    /// `tokenizer.json` parse failure (EU-4). Wraps the tokenizers error
    /// verbatim. Boxed because `tokenizers::Error` is a
    /// `Box<dyn Error + Send + Sync>` alias, and we need a sized type.
    #[error("tokenizer load: {source}")]
    TokenizerLoad {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("timed out acquiring embedder cache lock at {lock_path:?} after {waited_s}s")]
    LockTimeout { lock_path: PathBuf, waited_s: u64 },
}

/// Per-attempt download error. This is the internal error type used by
/// `download_once`/`download_with_retries`: it preserves the concrete
/// `ureq::Error` for HTTP-layer failures and a raw `io::Error` for
/// mid-stream body reads. After retry exhaustion both are boxed into
/// `EmbedderLoadError::NetworkUnavailable.source` (design §9 widened
/// to `Box<dyn Error + Send + Sync>` for exactly this reason — see
/// `NetworkStreamIo` doc). Fatal cache I/O failures abort retries
/// immediately.
enum DownloadAttemptError {
    /// Network-class failure (connect, read, HTTP status). Classified into
    /// `RetryDecision` via `retry_decision_ureq`. Boxed because
    /// `ureq::Error` carries a `Response` (with its own buffer state),
    /// making the bare variant much larger than its peers — clippy
    /// `large_enum_variant`. Boxing keeps the enum compact.
    Network(Box<ureq::Error>),
    /// Mid-stream read failure on the response body. ureq does not expose
    /// a public `Transport` constructor, so we cannot package this as a
    /// `ureq::Error::Transport` directly; instead we keep the raw
    /// `io::Error` and the retry loop maps it to the same retry-class as
    /// a `Transport`-level error per design §2 (read timeouts retry).
    NetworkStreamIo(std::io::Error),
    /// Cache I/O failure during the attempt (writing the `.partial`,
    /// opening, fsync, etc.). Always fail-fast.
    CacheIo { path: PathBuf, source: std::io::Error },
}

/// Whether a given network error should be retried within
/// `download_with_retries` (design §2). Connect failures, 5xx, read
/// timeouts, 408, and 429 are retryable; 4xx other than 408/429 fail fast.
fn retry_decision_ureq(err: &ureq::Error) -> RetryDecision {
    match err {
        ureq::Error::Status(code, _) => {
            if (500..=599).contains(code) || *code == 408 || *code == 429 {
                RetryDecision::Retry
            } else {
                RetryDecision::FailFast
            }
        }
        // Transport-level errors (DNS, connect, read timeout) always retry.
        ureq::Error::Transport(_) => RetryDecision::Retry,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryDecision {
    Retry,
    FailFast,
}

// ----- Loader configuration -------------------------------------------------

/// Configuration for the loader. Production code constructs this via
/// `LoaderConfig::production()` (called from `load_pinned_default_embedder`).
/// Tests construct via `LoaderConfig::for_tests()` and then override only the
/// surfaces explicitly designed for testing (base URL, cache root, pinned
/// shas, HF token).
///
/// **Scope guardrail (ADR-0.7.1 #1)**: outside the `loader-test-hooks`
/// feature, this type's constructors, setters, and the
/// `load_with_config` entry point are unreachable from external crates.
/// In production, `load_pinned_default_embedder()` is the only public
/// surface — base URL, repo, and pinned shas cannot be substituted.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    base_url: String,
    cache_root: PathBuf,
    hf_token: Option<String>,
    config_sha: String,
    tokenizer_sha: String,
    model_sha: String,
    connect_timeout: Duration,
    read_timeout: Duration,
    lock_timeout: Duration,
    /// HF-hub root used by the design-§4 compat probe.
    ///
    /// `None` → production behavior: probe `$HF_HOME` (or
    /// `~/.cache/huggingface` if unset). `Some(p)` → probe only `p`.
    /// Tests use `Some(<tempdir>)` (or `Some(<nonexistent>)`) to keep
    /// the probe deterministic and prevent the user's actual HF cache
    /// from leaking into the test harness.
    hf_hub_root: Option<PathBuf>,
}

impl LoaderConfig {
    /// Production constructor: real HF base URL, OS cache dir, real pinned
    /// shas, `HF_TOKEN` env var (if set).
    ///
    /// `pub(crate)` so only `load_pinned_default_embedder()` can build a
    /// production-config — downstream callers cannot reach a `LoaderConfig`
    /// without enabling `loader-test-hooks` (ADR-0.7.1 scope guardrail #1).
    pub(crate) fn production() -> Result<Self, EmbedderLoadError> {
        let cache_root = dirs::cache_dir().ok_or_else(|| EmbedderLoadError::CacheIoError {
            path: PathBuf::from("<dirs::cache_dir>"),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "platform cache dir unavailable",
            ),
        })?;
        let lock_timeout = parse_secs_env_or_default(ENV_LOCK_TIMEOUT, DEFAULT_LOCK_TIMEOUT);
        let connect_timeout =
            parse_secs_env_or_default(ENV_CONNECT_TIMEOUT, DEFAULT_CONNECT_TIMEOUT);
        let read_timeout = parse_secs_env_or_default(ENV_READ_TIMEOUT, DEFAULT_READ_TIMEOUT);
        Ok(Self {
            base_url: HF_BASE_URL.to_string(),
            cache_root,
            hf_token: std::env::var("HF_TOKEN").ok(),
            config_sha: CONFIG_JSON_SHA256.to_string(),
            tokenizer_sha: TOKENIZER_JSON_SHA256.to_string(),
            model_sha: MODEL_SAFETENSORS_SHA256.to_string(),
            connect_timeout,
            read_timeout,
            lock_timeout,
            hf_hub_root: None,
        })
    }

    /// Test constructor: dummy base URL / cache, placeholder shas. Callers
    /// (tests only — see module docs) override what they care about via the
    /// builder setters below.
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn for_tests() -> Self {
        Self {
            base_url: "http://127.0.0.1:0".to_string(),
            cache_root: PathBuf::from("/tmp/fathomdb-embedder-tests"),
            hf_token: None,
            config_sha: String::new(),
            tokenizer_sha: String::new(),
            model_sha: String::new(),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            read_timeout: DEFAULT_READ_TIMEOUT,
            lock_timeout: DEFAULT_LOCK_TIMEOUT,
            // Default test posture: point hub probe at a guaranteed-empty
            // path so a developer's real `~/.cache/huggingface` cannot
            // shadow the mock server in the test harness.
            hf_hub_root: Some(PathBuf::from("/nonexistent-fathomdb-embedder-test-hub")),
        }
    }

    /// Override the HF-hub root for the design-§4 compat probe. Tests
    /// pass `Some(<tempdir>)` to verify the probe; production callers
    /// have no way to reach this setter (it is `loader-test-hooks`-gated).
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn with_hf_hub_root(mut self, root: Option<PathBuf>) -> Self {
        self.hf_hub_root = root;
        self
    }

    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn with_cache_root(mut self, cache_root: PathBuf) -> Self {
        self.cache_root = cache_root;
        self
    }

    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn with_hf_token(mut self, token: Option<String>) -> Self {
        self.hf_token = token;
        self
    }

    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn with_test_pins(
        mut self,
        config_sha: String,
        tokenizer_sha: String,
        model_sha: String,
    ) -> Self {
        self.config_sha = config_sha;
        self.tokenizer_sha = tokenizer_sha;
        self.model_sha = model_sha;
        self
    }

    /// Directory where the three files will live. Exposed for tests so they
    /// can pre-stage `.partial` fixtures.
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn expected_cache_dir(&self) -> PathBuf {
        self.cache_dir_internal()
    }

    /// Construct a `LoaderConfig` reading `connect_timeout`/`read_timeout`
    /// from the design-§2 env vars (with the standard defaults on
    /// missing/invalid). Exposed for tests that assert the env-override
    /// parsing path produces the expected `Duration` values.
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn for_tests_reading_timeout_env() -> Self {
        let connect_timeout =
            parse_secs_env_or_default(ENV_CONNECT_TIMEOUT, DEFAULT_CONNECT_TIMEOUT);
        let read_timeout = parse_secs_env_or_default(ENV_READ_TIMEOUT, DEFAULT_READ_TIMEOUT);
        let mut cfg = Self::for_tests();
        cfg.connect_timeout = connect_timeout;
        cfg.read_timeout = read_timeout;
        cfg
    }

    /// Returns the loader's effective HTTP connect timeout. Exposed for
    /// tests asserting env-override behavior.
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// Returns the loader's effective HTTP read timeout. Exposed for
    /// tests asserting env-override behavior.
    #[cfg(any(test, feature = "loader-test-hooks"))]
    pub fn read_timeout(&self) -> Duration {
        self.read_timeout
    }

    /// Internal cache-dir resolver — uses `sha256("<repo>@<revision>")[..12]`
    /// per design §4 (Issue 2 fix). Note: this depends only on the repo +
    /// revision identity, NOT on the file's own sha — so an empty
    /// `model_sha` (as in `for_tests()`) no longer collapses the prefix to
    /// `""`, and a future revision bump that touches only `config.json`
    /// still lands in a distinct cache dir.
    fn cache_dir_internal(&self) -> PathBuf {
        self.cache_root.join("fathomdb").join("embedders").join(model_sha_prefix())
    }
}

// ----- Public entry points --------------------------------------------------

/// Zero-arg production entry point. The only function a caller outside the
/// crate ever needs.
pub fn load_pinned_default_embedder() -> Result<LoadedWeights, EmbedderLoadError> {
    load_with_config_internal(LoaderConfig::production()?)
}

/// Verifies a complete local TC-5 asset directory and returns its path-free
/// content identity without invoking the downloader or reading ambient config.
#[cfg(feature = "tc5-benchmark")]
pub fn tc5_local_asset_directory_identity(asset_dir: &Path) -> Result<String, EmbedderLoadError> {
    let weights = load_pinned_default_embedder_from_local_asset(asset_dir)?;
    let fields = [
        ("config.json", weights.config_json_path),
        ("tokenizer.json", weights.tokenizer_json_path),
        ("model.safetensors", weights.model_safetensors_path),
    ];
    let mut hasher = Sha256::new();
    for (name, path) in fields {
        let checksum = sha256_file(&path)
            .map_err(|source| EmbedderLoadError::CacheIoError { path, source })?;
        for value in [name, checksum.as_str()] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Opens the three pinned default-embedder files from an already-populated
/// local asset directory.
///
/// This is the cache-only half of the TC-5 benchmark boundary. It deliberately
/// does not create a cache directory, probe the Hugging Face cache, acquire a
/// loader lock, inspect environment variables, or invoke the downloader. Every
/// required file is present and checksum-verified before a caller can create a
/// device-backed model.
pub(crate) fn load_pinned_default_embedder_from_local_asset(
    asset_dir: &Path,
) -> Result<LoadedWeights, EmbedderLoadError> {
    let files = [
        ("config.json", CONFIG_JSON_SHA256),
        ("tokenizer.json", TOKENIZER_JSON_SHA256),
        ("model.safetensors", MODEL_SAFETENSORS_SHA256),
    ];
    let mut paths = Vec::with_capacity(files.len());

    for (file_name, expected_sha) in files {
        let path = asset_dir.join(file_name);
        if !path.is_file() {
            return Err(EmbedderLoadError::CacheIoError {
                path,
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "required local TC-5 model asset is not a file",
                ),
            });
        }
        let actual = sha256_file(&path)
            .map_err(|source| EmbedderLoadError::CacheIoError { path: path.clone(), source })?;
        if actual != expected_sha {
            return Err(EmbedderLoadError::ChecksumMismatch {
                file: path,
                expected: expected_sha.to_string(),
                actual,
            });
        }
        paths.push(path);
    }

    Ok(LoadedWeights {
        config_json_path: paths[0].clone(),
        tokenizer_json_path: paths[1].clone(),
        model_safetensors_path: paths[2].clone(),
        bytes_downloaded: 0,
        events: Vec::new(),
    })
}

/// Test/integration entry point. Same body as the production path but takes
/// an explicit `LoaderConfig`. Gated behind `loader-test-hooks` so
/// downstream crates cannot substitute base URL / pinned shas in
/// production builds (ADR-0.7.1 scope guardrail #1).
#[cfg(any(test, feature = "loader-test-hooks"))]
pub fn load_with_config(cfg: LoaderConfig) -> Result<LoadedWeights, EmbedderLoadError> {
    load_with_config_internal(cfg)
}

fn load_with_config_internal(cfg: LoaderConfig) -> Result<LoadedWeights, EmbedderLoadError> {
    let cache_dir = cfg.cache_dir_internal();
    fs::create_dir_all(&cache_dir)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: cache_dir.clone(), source })?;

    let mut events = Vec::new();
    let mut bytes_downloaded: u64 = 0;

    let files = [
        ("config.json", cfg.config_sha.clone()),
        ("tokenizer.json", cfg.tokenizer_sha.clone()),
        ("model.safetensors", cfg.model_sha.clone()),
    ];

    let mut paths = Vec::with_capacity(3);
    for (file_name, expected_sha) in &files {
        let final_path = cache_dir.join(file_name);

        // Fast path: cache already valid; no lock needed (design §10).
        //
        // `file_matches_sha` uses `Path::is_file()`, which FOLLOWS symlinks,
        // so a symlink here whose target hashes to the pin is accepted. That
        // is deliberately more permissive than `verify_materialized`, which
        // rejects a symlink outright: the loader is strict about what it
        // PUBLISHES and permissive about what it READS. Users legitimately
        // symlink a large model cache onto another disk, and rejecting that
        // would be a real regression; R80-14 is still satisfied because the
        // bytes hash to the pin, i.e. the entry is readable and correct.
        if file_matches_sha(&final_path, expected_sha)? {
            events.push(EmbedderEvent::DefaultEmbedderCacheHit {
                file: (*file_name).to_string(),
                sha256: expected_sha.clone(),
                cache_path: final_path.clone(),
            });
            paths.push(final_path);
            continue;
        }

        // HF-hub read-only compat probe (design §4): if the user already
        // has the file under `$HF_HOME/hub/...` and its sha matches the
        // pinned constant, copy/hard-link it into our cache and short-circuit
        // network. The HF-hub layout is NEVER written to.
        if let Some(hub_path) = hf_hub_candidate_path(&cfg, file_name) {
            if file_matches_sha(&hub_path, expected_sha)? {
                // Records the cache hit only once the destination is proven
                // readable and pin-matching (R80-14): nothing downstream
                // re-hashes these paths.
                materialize_from_hf_hub(&hub_path, &final_path, expected_sha)?;
                events.push(EmbedderEvent::DefaultEmbedderCacheHit {
                    file: (*file_name).to_string(),
                    sha256: expected_sha.clone(),
                    cache_path: final_path.clone(),
                });
                paths.push(final_path);
                continue;
            }
        }

        // Cold or stale: lock, re-check, fetch.
        let (n, fetched_event) = fetch_under_lock(&cfg, &cache_dir, file_name, expected_sha)?;
        bytes_downloaded = bytes_downloaded.saturating_add(n);
        match fetched_event {
            FetchOutcome::Downloaded(ev) => events.push(ev),
            FetchOutcome::CacheHitAfterLock(ev) => events.push(ev),
        }
        paths.push(final_path);
    }

    Ok(LoadedWeights {
        config_json_path: paths[0].clone(),
        tokenizer_json_path: paths[1].clone(),
        model_safetensors_path: paths[2].clone(),
        bytes_downloaded,
        events,
    })
}

// ----- Internals ------------------------------------------------------------

enum FetchOutcome {
    Downloaded(EmbedderEvent),
    CacheHitAfterLock(EmbedderEvent),
}

fn fetch_under_lock(
    cfg: &LoaderConfig,
    cache_dir: &Path,
    file_name: &str,
    expected_sha: &str,
) -> Result<(u64, FetchOutcome), EmbedderLoadError> {
    let lock_path = cache_dir.join(".lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: lock_path.clone(), source })?;

    acquire_exclusive_with_timeout(&lock_file, &lock_path, cfg.lock_timeout)?;

    // RAII lock release on drop (fs2 unlocks on close).
    let _guard = LockGuard(&lock_file);

    let final_path = cache_dir.join(file_name);

    // Double-checked locking (design §10): another thread may have completed
    // the fetch while we were queued behind the lock.
    if file_matches_sha(&final_path, expected_sha)? {
        return Ok((
            0,
            FetchOutcome::CacheHitAfterLock(EmbedderEvent::DefaultEmbedderCacheHit {
                file: file_name.to_string(),
                sha256: expected_sha.to_string(),
                cache_path: final_path,
            }),
        ));
    }

    let partial_path = cache_dir.join(format!("{file_name}.partial"));
    let url = format!("{}/{}/resolve/{}/{}", cfg.base_url, HF_REPO, HF_REVISION, file_name);

    let start = Instant::now();
    let bytes = download_with_retries(cfg, &url, &partial_path, file_name)?;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Verify before rename (design §5/§6).
    let observed_sha = sha256_file(&partial_path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: partial_path.clone(), source })?;
    if observed_sha != expected_sha {
        // Fail-closed: remove the partial (design §6).
        let _ = fs::remove_file(&partial_path);
        return Err(EmbedderLoadError::ChecksumMismatch {
            file: partial_path.clone(),
            expected: expected_sha.to_string(),
            actual: observed_sha,
        });
    }

    // Atomic rename (design §5).
    fs::rename(&partial_path, &final_path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: final_path.clone(), source })?;

    // fsync the parent directory so the rename survives a power loss
    // between the rename and the next implicit fsync. POSIX only;
    // Windows journaling already covers this (design §5 step 6).
    #[cfg(unix)]
    fsync_parent_dir(&final_path)?;

    Ok((
        bytes,
        FetchOutcome::Downloaded(EmbedderEvent::DefaultEmbedderDownload {
            file: file_name.to_string(),
            url,
            bytes,
            sha256: observed_sha,
            cache_path: final_path,
            duration_ms,
        }),
    ))
}

/// `fsync` the parent directory of `path`. POSIX only — used after a
/// rename to make the directory-entry change durable across a power loss
/// before the next file-level fsync (design §5 step 6).
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) -> Result<(), EmbedderLoadError> {
    if let Some(parent) = path.parent() {
        let dir = File::open(parent).map_err(|source| EmbedderLoadError::CacheIoError {
            path: parent.to_path_buf(),
            source,
        })?;
        dir.sync_all().map_err(|source| EmbedderLoadError::CacheIoError {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

/// Compute the HF-hub layout path for `file_name` at the pinned repo +
/// revision: `$HF_HOME/hub/models--<owner>--<repo>/snapshots/<rev>/<file>`.
/// `$HF_HOME` defaults to `~/.cache/huggingface`. Returns `None` if no
/// home directory can be resolved (and `HF_HOME` is unset).
fn hf_hub_candidate_path(cfg: &LoaderConfig, file_name: &str) -> Option<PathBuf> {
    let hf_home = if let Some(root) = &cfg.hf_hub_root {
        root.clone()
    } else {
        match std::env::var_os("HF_HOME") {
            Some(p) => PathBuf::from(p),
            None => dirs::home_dir()?.join(".cache").join("huggingface"),
        }
    };
    let repo_encoded = format!("models--{}", HF_REPO.replace('/', "--"));
    Some(hf_home.join("hub").join(repo_encoded).join("snapshots").join(HF_REVISION).join(file_name))
}

/// Publish the HF-hub asset at `src` into the cache entry `dst`, preferring a
/// POSIX hard-link when possible (same filesystem; saves disk + lets the
/// kernel share inodes) and falling back to a byte copy when `hard_link`
/// cannot be used (different filesystem, permission, Windows, ...). The HF-hub
/// source is never modified. Surfaces failures as `CacheIoError`.
///
/// Four defects are closed here relative to a bare `fs::hard_link(src, dst)`:
///
/// 1. **The link must be taken on the target, not the link** (R80-14). Linux
///    `link(2)` does not dereference symlinks, and the Hugging Face hub
///    stores `snapshots/<rev>/<file>` as a *relative* symlink
///    `../../blobs/<hash>` that resolves only inside the hub tree.
///    Hard-linking it into `<cache>/fathomdb/embedders/<id>/` produced an
///    entry whose `../../blobs/` now resolved under the FathomDB cache
///    root, where no `blobs/` exists — a dangling symlink. So we resolve
///    `src` first and link its target. (`fs::copy` already dereferences,
///    which is why the defect fired only when the two caches shared a
///    filesystem and the hard-link path was actually taken.)
/// 2. **Success is declared on read-verified content, not on link
///    creation** (R80-14). The caller records a `DefaultEmbedderCacheHit` on
///    `Ok` and nothing downstream re-hashes the result, so this is the one
///    place the "never record a cache hit for an asset we cannot
///    subsequently read" invariant can be enforced.
/// 3. **The published name is written atomically, and cleanup is
///    owner-scoped** (R80-15). This function runs at step 2 of the loader
///    loop, *outside* the `<cache_dir>/.lock` that `fetch_under_lock` takes
///    at step 3, so concurrent cold starts publish to the same path at the
///    same time. Writing `dst` in place let one racer's `fs::copy` (reached
///    via `EEXIST`) truncate the entry another racer was hashing for (2),
///    which then failed verification on data that was never wrong and
///    deleted an entry it did not own. We therefore use the module's own
///    established pattern from `fetch_under_lock`: materialize into a
///    privately-named temporary beside `dst`, verify *that*, then
///    `fs::rename` it into place and fsync the parent (design §5/§6). A
///    concurrent reader sees the complete old entry or the complete new one.
///    No lock is needed: the content is pin-addressed, so a last-writer-wins
///    rename replaces a byte-identical file.
/// 4. **The temporary is created exclusively, not merely named unlikely**
///    (R80-15). See the claim loop below: a PID-plus-counter name collides
///    between containers sharing a bind-mounted cache, so the claim rests on
///    `link(2)`/`O_CREAT|O_EXCL` instead, and the byte-copy branch is fsynced
///    because `std` promises no durability for a copy.
fn materialize_from_hf_hub(
    src: &Path,
    dst: &Path,
    expected_sha: &str,
) -> Result<(), EmbedderLoadError> {
    materialize_from_hf_hub_named(src, dst, expected_sha, &mut || hub_temp_path(dst))
}

/// `materialize_from_hf_hub` with the temporary-name source injected, so tests
/// can drive a name collision deterministically instead of hoping for one.
fn materialize_from_hf_hub_named(
    src: &Path,
    dst: &Path,
    expected_sha: &str,
    next_temp_name: &mut dyn FnMut() -> PathBuf,
) -> Result<(), EmbedderLoadError> {
    // Resolve the hub's relative symlink to its target once, before any
    // temporary exists (R80-14 defect 1).
    #[cfg(unix)]
    let resolved = fs::canonicalize(src).ok();

    // Claim a temporary by CREATING it exclusively, never by assuming a name
    // is ours. A name built from PID + counter is not unique across processes
    // that share a cache directory — containers on a bind-mounted
    // `XDG_CACHE_HOME` (which `scripts/release/cuda-preflight.sh` sets up) are
    // all PID 1, an NFS-mounted home is shared across machines, and PIDs are
    // reused after a crash. A collision there would reproduce the R80-15 bug
    // one level down: our write would truncate the temporary another process
    // was still filling, and our cleanup would delete it. Name quality now
    // only affects how many attempts we make; correctness rests on the
    // atomic create-exclusive primitives below.
    let mut claimed: Option<(PathBuf, TempReservation)> = None;
    let mut last_candidate = PathBuf::new();
    for _ in 0..MAX_TEMP_NAME_ATTEMPTS {
        let candidate = next_temp_name();
        last_candidate = candidate.clone();

        // `link(2)` is itself create-exclusive — it fails `EEXIST` if the new
        // name exists — so its SUCCESS is proof that we created that name and
        // nobody else owns it. Linking (rather than copying) into the
        // temporary is also what keeps the 133 MB model free of extra disk.
        #[cfg(unix)]
        if let Some(resolved) = resolved.as_deref() {
            match fs::hard_link(resolved, &candidate) {
                Ok(()) => {
                    claimed = Some((candidate, TempReservation::Linked));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                // EXDEV (hub on another filesystem), EPERM, Windows, ... —
                // not a collision, so try to reserve this same name for the
                // byte-copy fallback instead.
                Err(_) => {}
            }
        }

        // Copy fallback: reserve with `O_CREAT|O_EXCL` and keep the handle.
        match File::create_new(&candidate) {
            Ok(file) => {
                claimed = Some((candidate, TempReservation::Created(file)));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(EmbedderLoadError::CacheIoError { path: candidate, source }),
        }
    }

    let (tmp, reservation) = claimed.ok_or_else(|| EmbedderLoadError::CacheIoError {
        path: last_candidate,
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "no unused temporary name available for the cache entry",
        ),
    })?;
    let _guard = TempEntryGuard(tmp.clone());

    if let TempReservation::Created(mut file) = reservation {
        // We hold the only handle to the name we reserved, so we fill it
        // directly rather than re-opening by path. `std::io::copy`
        // specializes to the same in-kernel `copy_file_range`/`sendfile` path
        // `fs::copy` uses for file-to-file on Linux, so nothing is lost.
        // Opening `src` follows the hub's symlink, which is what makes this
        // fallback dereference correctly.
        let mut source = File::open(src).map_err(|source| EmbedderLoadError::CacheIoError {
            path: src.to_path_buf(),
            source,
        })?;
        std::io::copy(&mut source, &mut file)
            .map_err(|source| EmbedderLoadError::CacheIoError { path: tmp.clone(), source })?;
        // Durability: `std` makes no durability claim for a byte copy, and
        // `verify_materialized` reading the bytes back proves CONTENT, not
        // writeback — it reads through the page cache. Without this fsync a
        // crash after the rename below could leave a durable directory entry
        // pointing at unallocated data blocks (ext4 delayed allocation;
        // `auto_da_alloc` mitigates but is an ext4-specific default, not a
        // guarantee, and does not cover xfs/btrfs/APFS). This mirrors
        // `download_once`, which already `sync_all`s its `.partial`.
        //
        // The `Linked` arm needs no file fsync: it writes no file data, only
        // a directory entry, which `fsync_parent_dir` below covers.
        file.sync_all()
            .map_err(|source| EmbedderLoadError::CacheIoError { path: tmp.clone(), source })?;
    }

    verify_materialized(&tmp, expected_sha)?;

    // Atomic publish. `rename(2)` replaces whatever is at `dst` — including a
    // dangling symlink left by a pre-fix run, which is how a poisoned cache
    // self-heals (AC80-23) without this process ever unlinking an entry it
    // does not own. When `dst` already shares our inode (a racer published the
    // same hub blob first) this is a documented no-op; `_guard` cleans up the
    // temporary either way.
    //
    // Renaming over `dst` also closes a sharper hazard than the dangling link
    // itself: the pre-fix `fs::copy(src, dst)` opened the destination
    // `O_WRONLY|O_CREAT|O_TRUNC`, which FOLLOWS a symlink. Against the shipped
    // bug's `../../blobs/<hash>` entry it would therefore create or truncate a
    // file at the link's resolution target — writing OUTSIDE the cache entry
    // it believed it owned. `rename` replaces the link itself and never
    // follows it.
    //
    // On NFS, `rename(2)` NOTES warns that a retransmitted RPC after a server
    // crash can report failure for a rename that actually succeeded. That
    // fails safe here: the guard removes an already-gone temporary
    // (`ENOENT`, ignored), `dst` holds correct content, and step 1's
    // `file_matches_sha` finds it on the next call.
    fs::rename(&tmp, dst)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: dst.to_path_buf(), source })?;

    // Make the directory entry durable, exactly as the download path does
    // after its own rename (design §5 step 6).
    #[cfg(unix)]
    fsync_parent_dir(dst)?;

    Ok(())
}

/// Upper bound on candidate temporary names tried before failing closed. A
/// collision is already improbable; needing 16 in a row means something is
/// systematically wrong with the cache directory, and forcing our way onto an
/// occupied name would be worse than a typed error.
const MAX_TEMP_NAME_ATTEMPTS: u32 = 16;

/// How a temporary was claimed. Both variants are produced by an **atomic
/// create-exclusive** primitive, which is what makes the claim a fact rather
/// than an assumption (R80-15).
enum TempReservation {
    /// `link(2)` created the name and it already holds the hub blob's inode:
    /// no bytes need copying, and no file fsync is required because no file
    /// data was written.
    Linked,
    /// `O_CREAT|O_EXCL` created the name. The handle is the only one open on
    /// it and the bytes still have to be copied in.
    Created(File),
}

/// A candidate temporary name for a hub materialization, in the **same
/// directory** as `dst` so the publishing `fs::rename` stays inside one
/// filesystem (R80-15).
///
/// The download path can use a single fixed `<file>.partial` because it only
/// ever runs under `<cache_dir>/.lock`. The hub probe runs outside that lock,
/// so concurrent materializations must not pick the same temporary. This name
/// is only a *candidate*: exclusivity is established by the atomic creation in
/// `materialize_from_hf_hub_named`, not by the name, because no name scheme is
/// unique across containers sharing a bind-mounted cache (all PID 1), an
/// NFS-shared home, or PID reuse. The PID, a process-global counter and the
/// wall-clock nanoseconds are mixed in purely to keep the retry count at one —
/// `huggingface_hub` uses a truncated uuid4 for the same reason.
fn hub_temp_path(dst: &Path) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, AtomicOrdering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let stem = dst.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let pid = std::process::id();
    let dir = dst.parent().unwrap_or_else(|| Path::new("."));
    dir.join(format!(".{stem}.hub-{pid}-{seq}-{nanos:08x}.tmp"))
}

/// Removes the temporary this process created, on every exit path including
/// panics. Owner-scoped by construction: it only ever holds a path this
/// process picked via `hub_temp_path`, never the published cache entry
/// (R80-15).
///
/// It stays armed even after a successful publish, because a successful
/// `rename` does not always consume the source: POSIX specifies that if
/// `oldpath` and `newpath` are hard links to the **same inode**, `rename`
/// does nothing and returns success. That is the common case here — every
/// racer hard-links the same hub blob, so once one has published, the others'
/// renames are no-ops that would otherwise leave their temporaries behind.
/// Removing unconditionally is safe: the name is never reused, so the removal
/// is either a no-op `ENOENT` (the rename did move it) or drops the redundant
/// extra link (the rename was a no-op, and `dst` already shares the inode we
/// verified). Either way it can never delete the published entry.
///
/// The same-inode case cannot arise on Windows at all: the hard-link claim is
/// `#[cfg(unix)]`-gated, so a Windows temporary is always a freshly created
/// file with its own identity and the rename always genuinely moves it. That
/// makes Windows' (undocumented) same-inode rename behavior moot here rather
/// than an open question.
struct TempEntryGuard(PathBuf);

impl Drop for TempEntryGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Prove a materialized entry is a readable regular file whose bytes hash to
/// the pin, before it is published and the caller records a cache hit for it
/// (R80-14).
///
/// Pure: it never removes anything. It is called on a temporary whose lifetime
/// `TempEntryGuard` owns, which is what keeps failure cleanup owner-scoped
/// (R80-15) — an earlier revision deleted the path it verified, which on the
/// unlocked hub path meant deleting an entry another process had published.
fn verify_materialized(path: &Path, expected_sha: &str) -> Result<(), EmbedderLoadError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: path.to_path_buf(), source })?;
    // Deliberately `symlink_metadata`: a link here — even one that happens to
    // resolve — is exactly the failure mode R80-14 forbids, because it can
    // resolve outside the destination cache.
    if !meta.file_type().is_file() {
        return Err(EmbedderLoadError::CacheIoError {
            path: path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "materialized cache entry is not a regular file",
            ),
        });
    }

    let observed = sha256_file(path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: path.to_path_buf(), source })?;
    if observed != expected_sha {
        return Err(EmbedderLoadError::ChecksumMismatch {
            file: path.to_path_buf(),
            expected: expected_sha.to_string(),
            actual: observed,
        });
    }
    Ok(())
}

fn acquire_exclusive_with_timeout(
    f: &File,
    lock_path: &Path,
    timeout: Duration,
) -> Result<(), EmbedderLoadError> {
    let deadline = Instant::now() + timeout;
    loop {
        match f.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(e) => {
                // Only `WouldBlock` means "another holder; retry". Real
                // I/O errors (permission denied, EIO, ...) are fatal —
                // surface immediately rather than polling until timeout.
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(EmbedderLoadError::CacheIoError {
                        path: lock_path.to_path_buf(),
                        source: e,
                    });
                }
                if Instant::now() >= deadline {
                    return Err(EmbedderLoadError::LockTimeout {
                        lock_path: lock_path.to_path_buf(),
                        waited_s: timeout.as_secs(),
                    });
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

struct LockGuard<'a>(&'a File);
impl Drop for LockGuard<'_> {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(self.0);
    }
}

fn download_with_retries(
    cfg: &LoaderConfig,
    url: &str,
    partial_path: &Path,
    _file_name: &str,
) -> Result<u64, EmbedderLoadError> {
    let mut last_net_err: Option<Box<dyn std::error::Error + Send + Sync>> = None;
    let mut completed_attempts: u32 = 0;
    for attempt in 0..MAX_ATTEMPTS {
        completed_attempts = attempt + 1;
        match download_once(cfg, url, partial_path) {
            Ok(n) => return Ok(n),
            Err(DownloadAttemptError::CacheIo { path, source }) => {
                // Fail-fast: cache I/O is not a transient network condition.
                return Err(EmbedderLoadError::CacheIoError { path, source });
            }
            Err(DownloadAttemptError::Network(e)) => {
                if retry_decision_ureq(&e) == RetryDecision::FailFast {
                    return Err(EmbedderLoadError::NetworkUnavailable {
                        source: e,
                        attempts: completed_attempts,
                    });
                }
                last_net_err = Some(e);
                if attempt + 1 < MAX_ATTEMPTS {
                    // Design §2: 1s, 2s, (4s) — for MAX_ATTEMPTS=3 that's
                    // 1s then 2s before the second and third tries.
                    let secs = 1u64 << attempt;
                    std::thread::sleep(Duration::from_secs(secs));
                }
            }
            Err(DownloadAttemptError::NetworkStreamIo(io)) => {
                // Read-timeout / connection-reset mid-body: design §2
                // classes as a retryable read error. With the §9
                // `NetworkUnavailable.source` widened to a boxed dyn
                // Error, we now box the raw io::Error directly — no need
                // to drop it onto the CacheIoError path.
                last_net_err = Some(Box::new(io));
                if attempt + 1 < MAX_ATTEMPTS {
                    let secs = 1u64 << attempt;
                    std::thread::sleep(Duration::from_secs(secs));
                }
            }
        }
    }
    // All attempts exhausted. The boxed `source` carries whichever of
    // `ureq::Error` or mid-stream `io::Error` was observed last.
    Err(EmbedderLoadError::NetworkUnavailable {
        source: last_net_err.expect("at least one retryable attempt produced an error"),
        attempts: completed_attempts,
    })
}

fn download_once(
    cfg: &LoaderConfig,
    url: &str,
    partial_path: &Path,
) -> Result<u64, DownloadAttemptError> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(cfg.connect_timeout)
        .timeout_read(cfg.read_timeout)
        // Design §2: explicit redirect budget (≥3). ureq's default is 5,
        // which satisfies the design floor incidentally; we set it
        // deliberately so the value is part of the contract.
        .redirects(3)
        .build();

    // Resume support (design §2): if a `.partial` exists, request the suffix.
    let existing = fs::metadata(partial_path).map(|m| m.len()).unwrap_or(0);

    let mut req = agent.get(url);
    if let Some(token) = &cfg.hf_token {
        req = req.set("Authorization", &format!("Bearer {token}"));
    }
    if existing > 0 {
        req = req.set("Range", &format!("bytes={existing}-"));
    }

    let resp = req.call().map_err(|e| DownloadAttemptError::Network(Box::new(e)))?;

    let status = resp.status();
    if !(status == 200 || status == 206) {
        // Convert into a synthetic ureq::Status error so it goes through
        // the same retry-decision path as a directly-surfaced HTTP error.
        // `resp.into()` builds the Status variant with the response payload.
        return Err(DownloadAttemptError::Network(Box::new(ureq::Error::Status(status, resp))));
    }

    let mk_io = |source: std::io::Error| DownloadAttemptError::CacheIo {
        path: partial_path.to_path_buf(),
        source,
    };

    // Non-resume path uses `create_new` per design §5 step 2: a stale
    // `.partial` from a crashed prior run that didn't pass sha verification
    // must NOT be silently appended-to. Issue 3 (FIX-1): if a stale partial
    // is present here, we are by definition not in the resume path
    // (`existing == 0` OR server returned 200, discarding the old bytes), so
    // the partial is stale and we delete-then-recreate. The alternative
    // would be to fail; we pick delete-and-retry because it self-heals.
    let mut f = if status == 206 && existing > 0 {
        let mut f = OpenOptions::new().write(true).open(partial_path).map_err(mk_io)?;
        f.seek(SeekFrom::End(0)).map_err(mk_io)?;
        f
    } else {
        match OpenOptions::new().write(true).create_new(true).open(partial_path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Stale partial from a crashed prior run. Design §5 step 2
                // forbids silent overwrite — clean up explicitly, then
                // retry create_new.
                fs::remove_file(partial_path).map_err(mk_io)?;
                OpenOptions::new().write(true).create_new(true).open(partial_path).map_err(mk_io)?
            }
            Err(source) => {
                return Err(mk_io(source));
            }
        }
    };

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 64 * 1024];
    let mut written: u64 = 0;
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                f.write_all(&buf[..n]).map_err(mk_io)?;
                written += n as u64;
            }
            Err(source) => {
                // Mid-stream read failure on the response body. ureq's
                // `Transport` has no public constructor so we cannot
                // forge a `ureq::Error::Transport`; instead we carry the
                // raw `io::Error` through a dedicated variant so the
                // retry loop treats it as a read-timeout-class retryable
                // failure (design §2).
                return Err(DownloadAttemptError::NetworkStreamIo(source));
            }
        }
    }

    f.sync_all().map_err(mk_io)?;

    Ok(written)
}

fn file_matches_sha(path: &Path, expected: &str) -> Result<bool, EmbedderLoadError> {
    if !path.is_file() {
        return Ok(false);
    }
    let observed = sha256_file(path)
        .map_err(|source| EmbedderLoadError::CacheIoError { path: path.to_path_buf(), source })?;
    Ok(observed == expected)
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    // digest 0.11 `Array` output: format to identical lowercase, zero-padded hex.
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

// ----- R80-15 / AC80-24: atomic publish + owner-scoped cleanup --------------
//
// These are unit tests rather than integration tests on purpose. Both arms
// require the loader to materialize a file whose **final cache path already
// holds an entry**, and `load_with_config_internal`'s step-1 fast path
// short-circuits on any final entry that matches the pin — so a "pre-existing
// valid entry" is unreachable through `load_with_config`. The property under
// test belongs to `materialize_from_hf_hub` itself, so it is asserted here,
// where the pre-condition can actually be established. The concurrency arm
// that DOES go through the public entry point lives in `tests/loader.rs`.
#[cfg(all(test, unix))]
mod hub_publish_tests {
    use super::*;

    use std::os::unix::fs::MetadataExt;

    use tempfile::TempDir;

    const PINNED: &[u8] = br#"{"model_type":"bert","hidden_size":384}"#;
    const STALE: &[u8] = b"stale bytes from an earlier revision";

    fn sha_of(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Stages the real hub shape under `root`: bytes in `blobs/<sha>`, and
    /// `snapshots/<rev>/config.json` a **relative** symlink to it. Returns
    /// `(snapshot symlink, blob path)`.
    fn stage_hub(root: &Path) -> (PathBuf, PathBuf) {
        let repo = root.join("hub").join("models--BAAI--bge-small-en-v1.5");
        let blobs = repo.join("blobs");
        let snapshot = repo.join("snapshots").join(HF_REVISION);
        fs::create_dir_all(&blobs).unwrap();
        fs::create_dir_all(&snapshot).unwrap();
        let blob = blobs.join(sha_of(PINNED));
        fs::write(&blob, PINNED).unwrap();
        let link = snapshot.join("config.json");
        std::os::unix::fs::symlink(Path::new("../../blobs").join(sha_of(PINNED)), &link).unwrap();
        (link, blob)
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn publishes_by_rename_and_never_writes_the_published_name_in_place() {
        // AC80-24 arm 1. One TempDir so hub and cache share a filesystem:
        // `hard_link` must really succeed, and `rename` must stay intra-fs.
        let tmp = TempDir::new().unwrap();
        let (hub_link, blob) = stage_hub(tmp.path());
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        // A stale entry is already published at the final name — exactly the
        // state that lets the loader reach the hub probe at all.
        let final_path = cache_dir.join("config.json");
        fs::write(&final_path, STALE).unwrap();
        let before_ino = fs::metadata(&final_path).unwrap().ino();

        // A concurrent reader that opened the published entry before we start.
        let mut reader = File::open(&final_path).unwrap();

        materialize_from_hf_hub(&hub_link, &final_path, &sha_of(PINNED)).expect("materialize ok");

        // The new entry is published, complete.
        assert_eq!(fs::read(&final_path).unwrap(), PINNED);

        // ... at a NEW inode. An in-place `fs::copy` would have reused the
        // published inode, i.e. written the final name in situ.
        let after = fs::metadata(&final_path).unwrap();
        assert_ne!(
            after.ino(),
            before_ino,
            "final name was written in place (same inode); R80-15 requires publish-by-rename"
        );

        // The inode is the hub blob's, so hard-linking (and its zero extra
        // disk cost for the 133 MB model) survived the rename.
        assert_eq!(
            after.ino(),
            fs::metadata(&blob).unwrap().ino(),
            "publish must still hard-link the hub blob, not copy it"
        );

        // The concurrent reader still sees the COMPLETE previous entry, never
        // a truncated one. `fs::copy` opens `O_TRUNC` on the shared inode, so
        // an in-place publish makes this read return the new bytes.
        let mut seen = Vec::new();
        reader.read_to_end(&mut seen).unwrap();
        assert_eq!(
            seen, STALE,
            "a reader holding the previous entry observed the publish in progress"
        );

        // No temporary left behind.
        assert_eq!(file_names(&cache_dir), vec!["config.json".to_string()]);
    }

    #[test]
    fn failed_verification_removes_only_our_temporary_never_the_published_entry() {
        // AC80-24 arm 2 — the destructive regression this criterion exists to
        // prevent. A valid entry is already published (as if by a racing
        // process that just won); our materialization then fails verification.
        // It must clean up after itself and leave the published entry alone.
        let tmp = TempDir::new().unwrap();
        let (hub_link, _blob) = stage_hub(tmp.path());
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();

        let final_path = cache_dir.join("config.json");
        fs::write(&final_path, PINNED).unwrap();
        let published_ino = fs::metadata(&final_path).unwrap().ino();

        let wrong_pin = sha_of(b"a different asset entirely");
        let err = materialize_from_hf_hub(&hub_link, &final_path, &wrong_pin)
            .expect_err("verification must fail against a pin the bytes do not match");
        assert!(
            matches!(err, EmbedderLoadError::ChecksumMismatch { .. }),
            "expected ChecksumMismatch, got {err:?}"
        );

        // The published entry we do NOT own is untouched: still there, same
        // inode, same bytes.
        assert!(final_path.is_file(), "failed materialization deleted the published entry");
        assert_eq!(fs::read(&final_path).unwrap(), PINNED);
        assert_eq!(
            fs::metadata(&final_path).unwrap().ino(),
            published_ino,
            "published entry was replaced by a failed materialization"
        );

        // And our own temporary is gone.
        assert_eq!(file_names(&cache_dir), vec!["config.json".to_string()]);
    }

    /// A temporary "another process" is in the middle of writing. If our
    /// materialization ever touches one of these, we have reproduced the very
    /// bug R80-15 closed, one level down: their next read is torn.
    fn plant_foreign_temp(path: &Path, marker: &[u8]) {
        fs::write(path, marker).unwrap();
    }

    fn assert_foreign_temp_intact(path: &Path, marker: &[u8], ino: u64) {
        assert!(path.is_file(), "another process's temporary {path:?} was deleted");
        assert_eq!(fs::read(path).unwrap(), marker, "another process's temporary was overwritten");
        assert_eq!(
            fs::metadata(path).unwrap().ino(),
            ino,
            "another process's temporary was replaced ({path:?})"
        );
    }

    #[test]
    fn a_colliding_temp_name_is_never_clobbered_and_materialization_retries() {
        // A shared cache directory (containers on a bind-mounted
        // XDG_CACHE_HOME are both pid 1; NFS-shared homes; PID reuse) makes
        // two processes pick the same temporary name. Creation must be
        // EXCLUSIVE, so a collision costs another name, never another
        // process's bytes.
        let tmp = TempDir::new().unwrap();
        let (hub_link, _blob) = stage_hub(tmp.path());
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let dst = cache_dir.join("config.json");

        // The first two names we will be handed are already taken.
        let taken_a = cache_dir.join(".config.json.hub-1-0.tmp");
        let taken_b = cache_dir.join(".config.json.hub-1-1.tmp");
        let free = cache_dir.join(".config.json.hub-1-2.tmp");
        plant_foreign_temp(&taken_a, b"process A's half-written temporary");
        plant_foreign_temp(&taken_b, b"process B's half-written temporary");
        let ino_a = fs::metadata(&taken_a).unwrap().ino();
        let ino_b = fs::metadata(&taken_b).unwrap().ino();

        let mut names = vec![free.clone(), taken_b.clone(), taken_a.clone()];
        materialize_from_hf_hub_named(&hub_link, &dst, &sha_of(PINNED), &mut || {
            names.pop().expect("more temp names requested than scripted")
        })
        .expect("materialization must skip the taken names and publish");

        // We published, using neither foreign temporary.
        assert_eq!(fs::read(&dst).unwrap(), PINNED);
        assert_foreign_temp_intact(&taken_a, b"process A's half-written temporary", ino_a);
        assert_foreign_temp_intact(&taken_b, b"process B's half-written temporary", ino_b);
        assert!(!free.exists(), "our own temporary must be cleaned up");
    }

    #[test]
    fn exhausting_temp_names_fails_closed_without_touching_the_collided_name() {
        // If every candidate collides, fail closed with a typed error rather
        // than forcing our way onto a name another process owns.
        let tmp = TempDir::new().unwrap();
        let (hub_link, _blob) = stage_hub(tmp.path());
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let dst = cache_dir.join("config.json");

        let taken = cache_dir.join(".config.json.hub-1-0.tmp");
        plant_foreign_temp(&taken, b"someone else's temporary, forever");
        let ino = fs::metadata(&taken).unwrap().ino();

        let err =
            materialize_from_hf_hub_named(&hub_link, &dst, &sha_of(PINNED), &mut || taken.clone())
                .expect_err("must fail closed when no temporary name can be obtained");
        assert!(
            matches!(err, EmbedderLoadError::CacheIoError { .. }),
            "expected a typed CacheIoError, got {err:?}"
        );

        assert_foreign_temp_intact(&taken, b"someone else's temporary, forever", ino);
        assert!(!dst.exists(), "nothing may be published when materialization failed");
    }

    #[test]
    fn copy_fallback_also_reserves_its_temporary_exclusively() {
        // Covers the non-hard-link branch's reservation. A hub source that
        // resolves to a directory makes `link(2)` fail EPERM (Linux forbids
        // directory hard links outright), which is the same fall-through the
        // real EXDEV/Windows cases take — so the copy branch runs, must skip
        // the taken name, and must still fail closed on the unusable source.
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let a_directory = tmp.path().join("not-a-blob");
        fs::create_dir_all(&a_directory).unwrap();
        let hub_link = tmp.path().join("snapshot-entry");
        std::os::unix::fs::symlink(&a_directory, &hub_link).unwrap();

        let dst = cache_dir.join("config.json");
        let taken = cache_dir.join(".config.json.hub-1-0.tmp");
        let next = cache_dir.join(".config.json.hub-1-1.tmp");
        plant_foreign_temp(&taken, b"not ours");
        let ino = fs::metadata(&taken).unwrap().ino();

        let mut names = vec![next.clone(), taken.clone()];
        let err = materialize_from_hf_hub_named(&hub_link, &dst, &sha_of(PINNED), &mut || {
            names.pop().expect("more temp names requested than scripted")
        })
        .expect_err("an unusable hub source must fail closed");
        assert!(
            matches!(err, EmbedderLoadError::CacheIoError { .. }),
            "expected a typed CacheIoError, got {err:?}"
        );

        assert_foreign_temp_intact(&taken, b"not ours", ino);
        assert!(!next.exists(), "our own reserved temporary must be cleaned up");
        assert!(!dst.exists(), "nothing may be published when materialization failed");
    }

    #[test]
    fn temporaries_are_unique_per_call_so_racers_cannot_clobber_each_other() {
        // A single fixed temp name (`config.json.partial`) would let two
        // concurrent materializations write the same temporary.
        let tmp = TempDir::new().unwrap();
        let dst = tmp.path().join("config.json");
        let a = hub_temp_path(&dst);
        let b = hub_temp_path(&dst);
        assert_ne!(a, b, "temporary names must differ between calls");
        assert_eq!(
            a.parent(),
            dst.parent(),
            "temporary must sit beside dst so rename stays intra-fs"
        );
        assert_ne!(a, dst);
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.contains(&std::process::id().to_string()),
            "temporary name must be process-unique, got {name:?}"
        );
    }
}
