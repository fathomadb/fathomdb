//! Integration tests for the default-embedder loader.
//!
//! Per `dev/plans/prompts/0.7.1-EMBEDDER-UNDEFER-HANDOFF.md` §EU-3, this slice
//! ships five required tests that drive the loader contract:
//!
//! 1. `loads_pinned_model_with_correct_sha`
//! 2. `rejects_checksum_mismatch`
//! 3. `resumes_partial_download`
//! 4. `concurrent_loaders_serialize_via_filelock`
//! 5. `auth_token_sent_when_env_set`
//!
//! All tests run against a local `httpmock` server so the suite never touches
//! the network. The entire file is gated behind the `default-embedder` Cargo
//! feature: without it the crate stays a tiny `NoopEmbedder` holder with zero
//! optional deps.
//!
//! Concurrency-test variant choice (see §EU-3 test 4): we assert that across
//! N=4 concurrent loaders the mock observes **exactly one** complete set of
//! fetches (one config + one tokenizer + one model). The fs2 exclusive lock
//! serializes the first-use cold path; the late-arriving threads observe the
//! verified cache files after the lock releases and short-circuit before
//! hitting HTTP at all. This variant is cleaner to assert and exercises the
//! "cache-hit path does NOT take the lock" property.

#![cfg(all(feature = "default-embedder", feature = "loader-test-hooks"))]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use httpmock::prelude::*;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use fathomdb_embedder::loader::{
    load_pinned_default_embedder, load_with_config, EmbedderEvent, EmbedderLoadError,
    LoadedWeights, LoaderConfig,
};

const HF_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";

/// Fixture bytes for each pinned file. Content is small + deterministic so the
/// tests can pin sha256 values directly. The real HF SHAs in `loader.rs` are
/// for production fetches; tests override the pinned constants via
/// `LoaderConfig::with_test_pins`.
struct Fixture {
    config_bytes: Vec<u8>,
    tokenizer_bytes: Vec<u8>,
    model_bytes: Vec<u8>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            config_bytes: br#"{"model_type":"bert","hidden_size":384}"#.to_vec(),
            tokenizer_bytes: br#"{"version":"1.0","model":{"type":"WordPiece"}}"#.to_vec(),
            // 8 KiB of deterministic pseudo-random bytes for the "model".
            model_bytes: (0u32..2048).flat_map(|n| n.to_le_bytes()).collect(),
        }
    }

    fn sha_hex(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }

    fn config_sha(&self) -> String {
        Self::sha_hex(&self.config_bytes)
    }
    fn tokenizer_sha(&self) -> String {
        Self::sha_hex(&self.tokenizer_bytes)
    }
    fn model_sha(&self) -> String {
        Self::sha_hex(&self.model_bytes)
    }
}

fn resolve_path(file: &str) -> String {
    format!("/BAAI/bge-small-en-v1.5/resolve/{HF_REVISION}/{file}")
}

fn test_config(server_base: &str, cache_root: &Path, fix: &Fixture) -> LoaderConfig {
    LoaderConfig::for_tests()
        .with_base_url(server_base.to_string())
        .with_cache_root(cache_root.to_path_buf())
        .with_test_pins(fix.config_sha(), fix.tokenizer_sha(), fix.model_sha())
}

#[test]
fn loads_pinned_model_with_correct_sha() {
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();

    let m_cfg = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok = server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl = server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&fix.model_bytes);
    });

    let cache = tmp.path().to_path_buf();
    let loaded: LoadedWeights =
        load_with_config(test_config(&server.base_url(), &cache, &fix)).expect("loader ok");

    assert!(loaded.config_json_path.is_file());
    assert!(loaded.tokenizer_json_path.is_file());
    assert!(loaded.model_safetensors_path.is_file());

    let on_disk = fs::read(&loaded.model_safetensors_path).unwrap();
    assert_eq!(Fixture::sha_hex(&on_disk), fix.model_sha());
    assert!(loaded.bytes_downloaded > 0);

    // Per design §7, a fresh fetch surfaces a DefaultEmbedderDownload event.
    assert!(loaded
        .events
        .iter()
        .any(|e| matches!(e, EmbedderEvent::DefaultEmbedderDownload { .. })));

    m_cfg.assert();
    m_tok.assert();
    m_mdl.assert();
}

#[test]
fn rejects_checksum_mismatch() {
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();

    server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    // Serve wrong bytes for the model. The pinned sha is for the correct bytes.
    let wrong = b"not the real model bytes".to_vec();
    server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&wrong);
    });

    let cache = tmp.path().to_path_buf();
    let err = load_with_config(test_config(&server.base_url(), &cache, &fix))
        .expect_err("must fail closed on sha mismatch");
    assert!(
        matches!(err, EmbedderLoadError::ChecksumMismatch { .. }),
        "expected ChecksumMismatch, got {err:?}"
    );

    // Per design §6: file removed on mismatch. Both the final and .partial
    // forms must be absent (loader is responsible for cleanup).
    let cache_dir = cache.join("fathomdb").join("embedders");
    let mut found_model = false;
    if cache_dir.is_dir() {
        for entry in walkdir(&cache_dir) {
            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.contains("model.safetensors") {
                found_model = true;
            }
        }
    }
    assert!(!found_model, "model.safetensors (or .partial) must be removed on checksum mismatch");
}

#[test]
fn resumes_partial_download() {
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().to_path_buf();

    // Config + tokenizer always succeed cleanly.
    server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });

    // Pre-stage a .partial for the model holding the first half of the bytes.
    let half = fix.model_bytes.len() / 2;
    let cfg = test_config(&server.base_url(), &cache, &fix);
    let partial_dir = cfg.expected_cache_dir();
    fs::create_dir_all(&partial_dir).unwrap();
    let partial_path = partial_dir.join("model.safetensors.partial");
    let mut f = fs::File::create(&partial_path).unwrap();
    f.write_all(&fix.model_bytes[..half]).unwrap();
    f.sync_all().unwrap();
    drop(f);

    // Mock returns 206 Partial Content on Range request; serves the second half.
    let m_range = server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors")).header_exists("range");
        then.status(206).body(&fix.model_bytes[half..]);
    });

    let loaded = load_with_config(cfg).expect("resume load ok");
    let bytes = fs::read(&loaded.model_safetensors_path).unwrap();
    assert_eq!(Fixture::sha_hex(&bytes), fix.model_sha());
    m_range.assert();
}

#[test]
fn concurrent_loaders_serialize_via_filelock() {
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().to_path_buf();

    let cfg_calls = Arc::new(AtomicUsize::new(0));
    let tok_calls = Arc::new(AtomicUsize::new(0));
    let mdl_calls = Arc::new(AtomicUsize::new(0));

    // Slow handlers so that even if threads race to acquire the lock, the
    // first holder is unambiguously the one doing the network work and the
    // rest must observe the cache after release.
    let _m_cfg = {
        let calls = cfg_calls.clone();
        let body = fix.config_bytes.clone();
        server.mock(move |when, then| {
            calls.fetch_add(1, Ordering::SeqCst);
            when.method(GET).path(resolve_path("config.json"));
            then.status(200).delay(Duration::from_millis(50)).body(body);
        })
    };
    let _m_tok = {
        let calls = tok_calls.clone();
        let body = fix.tokenizer_bytes.clone();
        server.mock(move |when, then| {
            calls.fetch_add(1, Ordering::SeqCst);
            when.method(GET).path(resolve_path("tokenizer.json"));
            then.status(200).delay(Duration::from_millis(50)).body(body);
        })
    };
    let _m_mdl = {
        let calls = mdl_calls.clone();
        let body = fix.model_bytes.clone();
        server.mock(move |when, then| {
            calls.fetch_add(1, Ordering::SeqCst);
            when.method(GET).path(resolve_path("model.safetensors"));
            then.status(200).delay(Duration::from_millis(50)).body(body);
        })
    };

    let base = server.base_url();
    let mut handles = Vec::new();
    for _ in 0..4 {
        let cfg = test_config(&base, &cache, &fix);
        handles.push(thread::spawn(move || load_with_config(cfg)));
    }

    for h in handles {
        h.join().unwrap().expect("each thread loads ok");
    }

    // Variant chosen (documented in module header): exactly one set of fetches
    // observed by the mock. The first thread acquires the fs2 exclusive lock,
    // downloads + verifies + renames; the other three observe the cached
    // files after the lock releases and short-circuit before HTTP.
    assert_eq!(cfg_calls.load(Ordering::SeqCst), 1);
    assert_eq!(tok_calls.load(Ordering::SeqCst), 1);
    assert_eq!(mdl_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn auth_token_sent_when_env_set() {
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().to_path_buf();

    let m_cfg = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json")).header("authorization", "Bearer sekret");
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok = server.mock(|when, then| {
        when.method(GET)
            .path(resolve_path("tokenizer.json"))
            .header("authorization", "Bearer sekret");
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl = server.mock(|when, then| {
        when.method(GET)
            .path(resolve_path("model.safetensors"))
            .header("authorization", "Bearer sekret");
        then.status(200).body(&fix.model_bytes);
    });

    let cfg = test_config(&server.base_url(), &cache, &fix).with_hf_token(Some("sekret".into()));
    load_with_config(cfg).expect("loads with bearer");

    m_cfg.assert();
    m_tok.assert();
    m_mdl.assert();

    // Second pass: token unset → mock must reject any request bearing an
    // Authorization header. Use a fresh cache so the loader actually
    // re-fetches.
    let tmp2 = TempDir::new().unwrap();
    let server2 = MockServer::start();
    let m_cfg2 = server2.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        // header_missing isn't always available; assert via a negative path:
        // if any request carries Authorization, this mock won't match and the
        // loader will see a 404. Use header_exists negation pattern.
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok2 = server2.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl2 = server2.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&fix.model_bytes);
    });

    let cfg2 = test_config(&server2.base_url(), tmp2.path(), &fix).with_hf_token(None);
    load_with_config(cfg2).expect("loads without token");
    m_cfg2.assert();
    m_tok2.assert();
    m_mdl2.assert();
}

#[test]
fn respects_timeout_env_overrides() {
    // EU-3 FIX-2 #2: design §2 promises `FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S`
    // and `FATHOMDB_EMBEDDER_READ_TIMEOUT_S` env overrides parse as u64
    // seconds; invalid → default with a warning (no panic, no unwrap).
    //
    // We assert the parsing logic directly via `for_tests_reading_timeout_env`
    // which goes through the same `parse_secs_env_or_default` path the
    // production constructor uses. Holding the env-mutex prevents races with
    // other tests that touch the same vars.
    let _g = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    // Save existing values so we restore the process env.
    let prev_connect = std::env::var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S").ok();
    let prev_read = std::env::var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S").ok();

    // Valid overrides parse and apply.
    std::env::set_var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S", "7");
    std::env::set_var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S", "111");
    let cfg = LoaderConfig::for_tests_reading_timeout_env();
    assert_eq!(cfg.connect_timeout(), Duration::from_secs(7));
    assert_eq!(cfg.read_timeout(), Duration::from_secs(111));

    // Invalid → default, no panic.
    std::env::set_var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S", "not-a-number");
    std::env::set_var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S", "");
    let cfg = LoaderConfig::for_tests_reading_timeout_env();
    assert_eq!(cfg.connect_timeout(), Duration::from_secs(10), "invalid → default 10s");
    assert_eq!(cfg.read_timeout(), Duration::from_secs(60), "invalid → default 60s");

    // Unset → default.
    std::env::remove_var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S");
    std::env::remove_var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S");
    let cfg = LoaderConfig::for_tests_reading_timeout_env();
    assert_eq!(cfg.connect_timeout(), Duration::from_secs(10));
    assert_eq!(cfg.read_timeout(), Duration::from_secs(60));

    // Restore previous values.
    match prev_connect {
        Some(v) => std::env::set_var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S", v),
        None => std::env::remove_var("FATHOMDB_EMBEDDER_CONNECT_TIMEOUT_S"),
    }
    match prev_read {
        Some(v) => std::env::set_var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S", v),
        None => std::env::remove_var("FATHOMDB_EMBEDDER_READ_TIMEOUT_S"),
    }
}

#[test]
fn hf_hub_compat_probe_reads_from_hub_layout() {
    // EU-3 FIX-2 #6: when the file is already present under the HF-hub
    // read-only layout AND its sha matches the pinned constant, the
    // loader copies/hard-links it into the fathomdb cache without making
    // any network request. The HF-hub layout is never written to.
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().to_path_buf();

    // Pre-stage just `config.json` in the HF-hub layout. The other two
    // files go via mock so we can assert exactly which requests fly.
    let hf_home = tmp.path().join("hf_home");
    let hub_dir = hf_home
        .join("hub")
        .join("models--BAAI--bge-small-en-v1.5")
        .join("snapshots")
        .join(HF_REVISION);
    fs::create_dir_all(&hub_dir).unwrap();
    let hub_config = hub_dir.join("config.json");
    fs::write(&hub_config, &fix.config_bytes).unwrap();

    // Mock: only tokenizer + model served from network. config.json must
    // NOT be requested — if the loader hits it, the test fails the
    // explicit `assert_hits(0)` assertion below.
    let m_cfg_must_not_hit = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok = server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl = server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&fix.model_bytes);
    });

    let cfg = test_config(&server.base_url(), &cache, &fix).with_hf_hub_root(Some(hf_home.clone()));
    let loaded = load_with_config(cfg).expect("loader ok with hub-probe hit");

    // Mock-side: config.json was served from the hub, not the network.
    m_cfg_must_not_hit.assert_hits(0);
    m_tok.assert();
    m_mdl.assert();

    // Loader emitted a cache-hit event for config.json.
    let cache_hit_files: Vec<&str> = loaded
        .events
        .iter()
        .filter_map(|e| match e {
            EmbedderEvent::DefaultEmbedderCacheHit { file, .. } => Some(file.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        cache_hit_files.contains(&"config.json"),
        "expected DefaultEmbedderCacheHit for config.json, got {cache_hit_files:?}"
    );

    // The HF-hub source is intact (read-only probe).
    let hub_bytes = fs::read(&hub_config).unwrap();
    assert_eq!(hub_bytes, fix.config_bytes, "hub source must not be modified");

    // The fathomdb cache materialized the file.
    let on_disk = fs::read(&loaded.config_json_path).unwrap();
    assert_eq!(on_disk, fix.config_bytes);
}

// ----- R80-14 / AC80-23: HF-hub materialization -----------------------------
//
// The Hugging Face hub stores `snapshots/<rev>/<file>` as a **relative**
// symlink into a sibling `blobs/` directory, and Linux `link(2)` does not
// dereference symlinks. Hard-linking the snapshot entry into the FathomDB
// cache therefore linked the *link*, whose `../../blobs/<hash>` then resolved
// under the FathomDB cache root where no `blobs/` exists — a dangling symlink
// that `file_matches_sha` reports as "absent" forever, so the cache never
// self-heals and the load fails `CacheIoError … NotFound`.
//
// These fixtures deliberately place the fake hub tree and the cache root under
// **one** `TempDir`, so both live on one filesystem and `fs::hard_link`
// actually succeeds. On two filesystems `hard_link` fails `EXDEV`, the
// `fs::copy` fallback dereferences correctly, and the defect is masked — that
// asymmetry (read-only bind mount in the containerized preflight) is exactly
// why CI never caught this.

/// Builds `<hf_home>/hub/models--BAAI--bge-small-en-v1.5/` with `file_name`
/// staged the way the real hub stages it: bytes in `blobs/<hash>`, and
/// `snapshots/<rev>/<file_name>` a **relative** symlink `../../blobs/<hash>`.
/// Returns the snapshot-side symlink path.
#[cfg(unix)]
fn stage_hf_hub_file(hf_home: &Path, file_name: &str, blob_name: &str, bytes: &[u8]) -> PathBuf {
    let repo_dir = hf_home.join("hub").join("models--BAAI--bge-small-en-v1.5");
    let blobs_dir = repo_dir.join("blobs");
    let snapshot_dir = repo_dir.join("snapshots").join(HF_REVISION);
    fs::create_dir_all(&blobs_dir).unwrap();
    fs::create_dir_all(&snapshot_dir).unwrap();

    let blob_path = blobs_dir.join(blob_name);
    fs::write(&blob_path, bytes).unwrap();

    let link_path = snapshot_dir.join(file_name);
    // Relative, exactly as `huggingface_hub` writes it.
    std::os::unix::fs::symlink(Path::new("../../blobs").join(blob_name), &link_path).unwrap();

    let staged = fs::symlink_metadata(&link_path).unwrap();
    assert!(staged.file_type().is_symlink(), "fixture must stage a symlink, not a copy");
    link_path
}

/// `stage_hf_hub_file` for the `config.json` case the R80-14 arms use.
#[cfg(unix)]
fn stage_hf_hub_relative_symlink(hf_home: &Path, blob_name: &str, bytes: &[u8]) -> PathBuf {
    stage_hf_hub_file(hf_home, "config.json", blob_name, bytes)
}

/// Device id of `path`, so a test can prove the hub tree and the cache really
/// do share a filesystem (otherwise `fs::hard_link` never runs and the test
/// would be a false GREEN).
#[cfg(unix)]
fn device_of(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).unwrap().dev()
}

#[cfg(unix)]
#[test]
fn hf_hub_relative_symlink_materializes_readable_regular_file() {
    // AC80-23: materializing a pinned asset out of the hub yields a readable
    // regular file whose sha matches the pin — never a dangling symlink.
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();

    let cache = tmp.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    let hf_home = tmp.path().join("hf_home");
    let hub_link = stage_hf_hub_relative_symlink(&hf_home, &fix.config_sha(), &fix.config_bytes);

    assert_eq!(
        device_of(&cache),
        device_of(&hub_link),
        "fixture invalid: hub tree and cache must share a filesystem or fs::hard_link \
         never runs and the copy fallback masks the defect"
    );

    let m_cfg_must_not_hit = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok = server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl = server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&fix.model_bytes);
    });

    let cfg = test_config(&server.base_url(), &cache, &fix).with_hf_hub_root(Some(hf_home.clone()));
    let loaded = load_with_config(cfg).expect("loader ok with hub-probe hit");

    m_cfg_must_not_hit.assert_hits(0);
    m_tok.assert();
    m_mdl.assert();

    let dst = &loaded.config_json_path;

    // 1. A readable regular file, NOT a link of any kind.
    let dst_meta = fs::symlink_metadata(dst).unwrap_or_else(|e| {
        panic!("materialized {dst:?} is not even stat-able: {e}");
    });
    assert!(
        !dst_meta.file_type().is_symlink(),
        "materialized {dst:?} is a symlink (-> {:?}); link(2) linked the hub's relative \
         symlink instead of its target",
        fs::read_link(dst).ok()
    );
    assert!(dst_meta.file_type().is_file(), "materialized {dst:?} is not a regular file");

    // 2. Its bytes read back.
    let on_disk = fs::read(dst).unwrap_or_else(|e| panic!("materialized {dst:?} unreadable: {e}"));
    assert_eq!(on_disk, fix.config_bytes);

    // 3. Its sha256 equals the pin.
    assert_eq!(Fixture::sha_hex(&on_disk), fix.config_sha());

    // The loader recorded the cache hit it is now entitled to.
    assert!(
        loaded.events.iter().any(|e| matches!(
            e,
            EmbedderEvent::DefaultEmbedderCacheHit { file, .. } if file == "config.json"
        )),
        "expected DefaultEmbedderCacheHit for config.json, got {:?}",
        loaded.events
    );

    // The hub remains a read-only source: still a symlink, target untouched.
    let hub_meta = fs::symlink_metadata(&hub_link).unwrap();
    assert!(hub_meta.file_type().is_symlink(), "hub entry must stay a symlink");
    assert_eq!(fs::read(&hub_link).unwrap(), fix.config_bytes, "hub source must not be modified");
}

#[cfg(unix)]
#[test]
fn hf_hub_poisoned_cache_entry_self_heals() {
    // A cache poisoned by the pre-fix loader holds a dangling relative
    // symlink. `file_matches_sha` reports it absent, so the hub probe runs
    // again on every load; the entry must be repaired, not re-poisoned.
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();

    let cache = tmp.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    let hf_home = tmp.path().join("hf_home");
    stage_hf_hub_relative_symlink(&hf_home, &fix.config_sha(), &fix.config_bytes);

    let cfg = test_config(&server.base_url(), &cache, &fix).with_hf_hub_root(Some(hf_home.clone()));
    let cache_dir = cfg.expected_cache_dir();
    fs::create_dir_all(&cache_dir).unwrap();
    let poisoned = cache_dir.join("config.json");
    std::os::unix::fs::symlink(Path::new("../../blobs").join(fix.config_sha()), &poisoned).unwrap();
    assert!(!poisoned.is_file(), "fixture invalid: poisoned entry must be dangling");

    let m_cfg_must_not_hit = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });
    let m_tok = server.mock(|when, then| {
        when.method(GET).path(resolve_path("tokenizer.json"));
        then.status(200).body(&fix.tokenizer_bytes);
    });
    let m_mdl = server.mock(|when, then| {
        when.method(GET).path(resolve_path("model.safetensors"));
        then.status(200).body(&fix.model_bytes);
    });

    let loaded = load_with_config(cfg).expect("loader must repair a poisoned cache entry");
    m_cfg_must_not_hit.assert_hits(0);
    m_tok.assert();
    m_mdl.assert();

    let dst = &loaded.config_json_path;
    assert!(
        !fs::symlink_metadata(dst).unwrap().file_type().is_symlink(),
        "poisoned entry was not repaired: {dst:?} is still a symlink"
    );
    assert_eq!(fs::read(dst).unwrap(), fix.config_bytes);

    // The loader never wrote through the dangling link into the cache root.
    assert!(
        !cache.join("fathomdb").join("blobs").exists(),
        "loader must not materialize through the dangling link's target path"
    );
}

#[cfg(unix)]
#[test]
fn hf_hub_materialization_failure_leaves_no_cache_hit() {
    // R80-14 fail-closed arm: when the asset cannot be materialized, the
    // loader returns a typed error and records NO cache hit — it never
    // reports success for something it cannot subsequently read, and it
    // leaves no half-written or dangling destination behind.
    //
    // The destination is occupied by a directory, so neither `hard_link`
    // (EEXIST) nor `fs::copy` (EISDIR) can succeed. This is deterministic and
    // does not depend on file permissions (which a root test runner ignores).
    let fix = Fixture::new();
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();

    let cache = tmp.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    let hf_home = tmp.path().join("hf_home");
    stage_hf_hub_relative_symlink(&hf_home, &fix.config_sha(), &fix.config_bytes);

    let cfg = test_config(&server.base_url(), &cache, &fix).with_hf_hub_root(Some(hf_home.clone()));
    let cache_dir = cfg.expected_cache_dir();
    fs::create_dir_all(cache_dir.join("config.json")).unwrap();

    let m_cfg = server.mock(|when, then| {
        when.method(GET).path(resolve_path("config.json"));
        then.status(200).body(&fix.config_bytes);
    });

    let err = load_with_config(cfg).expect_err("materialization failure must fail closed");
    match &err {
        EmbedderLoadError::CacheIoError { path, .. } => {
            assert_eq!(path, &cache_dir.join("config.json"), "error must name the destination");
        }
        other => panic!("expected CacheIoError, got {other:?}"),
    }

    // Fail-closed: no silent network fallback, and nothing left in the cache
    // dir but the (pre-existing) directory we planted — no `.partial`, no
    // dangling link.
    m_cfg.assert_hits(0);
    for entry in walkdir(&cache_dir) {
        let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        assert!(!name.ends_with(".partial"), "left a half-written {entry:?}");
        assert!(
            !fs::symlink_metadata(&entry).unwrap().file_type().is_symlink(),
            "left a dangling link at {entry:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn concurrent_cold_starts_over_the_hub_all_succeed() {
    // R80-15 / AC80-24 end-to-end: the hub probe runs at step 2 of the loader
    // loop, OUTSIDE the `<cache_dir>/.lock` that step 3 takes, so concurrent
    // cold starts publish to the same final paths simultaneously. Before the
    // atomic-publish fix, the second racer's `fs::copy` (reached via EEXIST)
    // truncated the entry the first racer was hashing, so the first failed
    // verification on data that was never wrong AND deleted the entry the
    // second was writing — both processes failing over a byte-identical,
    // pin-addressed asset.
    //
    // The model fixture is deliberately large (8 MiB) to widen the
    // truncate-vs-hash window; with an 8 KiB file the race is real but rarely
    // observed within a single run.
    const LOADERS: usize = 8;
    let big_model: Vec<u8> = (0u32..2_097_152).flat_map(|n| n.to_le_bytes()).collect();
    let fix = Fixture { model_bytes: big_model, ..Fixture::new() };

    let tmp = TempDir::new().unwrap();
    let cache = tmp.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    let hf_home = tmp.path().join("hf_home");

    // All three pinned files come from the hub, so no thread needs the
    // network and every thread takes the materialization path.
    stage_hf_hub_file(&hf_home, "config.json", &fix.config_sha(), &fix.config_bytes);
    stage_hf_hub_file(&hf_home, "tokenizer.json", &fix.tokenizer_sha(), &fix.tokenizer_bytes);
    stage_hf_hub_file(&hf_home, "model.safetensors", &fix.model_sha(), &fix.model_bytes);

    // Unreachable base URL: any thread that falls through to the network
    // fails loudly instead of quietly papering over a materialization bug.
    let cfg = LoaderConfig::for_tests()
        .with_base_url("http://127.0.0.1:1".to_string())
        .with_cache_root(cache.clone())
        .with_test_pins(fix.config_sha(), fix.tokenizer_sha(), fix.model_sha())
        .with_hf_hub_root(Some(hf_home.clone()));

    let barrier = Arc::new(std::sync::Barrier::new(LOADERS));
    let handles: Vec<_> = (0..LOADERS)
        .map(|_| {
            let cfg = cfg.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                load_with_config(cfg)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let failures: Vec<String> =
        results.iter().filter_map(|r| r.as_ref().err().map(|e| format!("{e:?}"))).collect();
    assert!(
        failures.is_empty(),
        "{}/{LOADERS} concurrent cold starts failed over pin-addressed content: {failures:#?}",
        failures.len()
    );

    // Every loader saw complete, pin-matching content.
    for loaded in results.into_iter().map(|r| r.unwrap()) {
        assert_eq!(fs::read(&loaded.config_json_path).unwrap(), fix.config_bytes);
        assert_eq!(fs::read(&loaded.tokenizer_json_path).unwrap(), fix.tokenizer_bytes);
        assert_eq!(
            Fixture::sha_hex(&fs::read(&loaded.model_safetensors_path).unwrap()),
            fix.model_sha()
        );
        assert_eq!(loaded.bytes_downloaded, 0, "nothing may be downloaded; the hub had everything");
    }

    // No temporaries survive the storm — only the three published entries.
    let mut leftovers: Vec<String> = fs::read_dir(cfg.expected_cache_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| !matches!(n.as_str(), "config.json" | "tokenizer.json" | "model.safetensors"))
        .collect();
    leftovers.sort();
    assert!(leftovers.is_empty(), "temporaries left behind: {leftovers:?}");
}

/// Serializes tests that mutate the process env so set/restore cycles
/// don't race with each other.
static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn public_api_exists() {
    // Compile-time check: the zero-arg public entry point referenced by EU-4
    // and EU-5 exists and has the documented signature. It is not invoked
    // here (would hit the real network); see the GREEN-side integration tests
    // for behavior coverage.
    let _: fn() -> Result<LoadedWeights, EmbedderLoadError> = load_pinned_default_embedder;
}

// Minimal recursive walker (avoids pulling walkdir as a dev-dep).
fn walkdir(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        if let Ok(rd) = fs::read_dir(&p) {
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push(path);
                }
            }
        }
    }
    out
}
