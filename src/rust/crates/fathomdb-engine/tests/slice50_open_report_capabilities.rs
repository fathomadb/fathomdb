//! Slice 50 (0.8.31) — `OpenReport` runtime capability booleans
//! (`reranker_compiled`, `cuda_compiled`, `rerank_available`).

use fathomdb_engine::Engine;
use tempfile::TempDir;

#[test]
fn open_report_capability_booleans_match_compile_features() {
    let dir = TempDir::new().unwrap();
    let prev_device = std::env::var_os("FATHOMDB_RERANK_DEVICE");
    let prev_cache = std::env::var_os("FATHOMDB_RERANKER_CACHE");
    let empty_cache = dir.path().join("empty_reranker_cache");
    unsafe {
        std::env::set_var("FATHOMDB_RERANK_DEVICE", "cpu");
        std::env::set_var("FATHOMDB_RERANKER_CACHE", &empty_cache);
    }
    let _ = fathomdb_embedder::module_models::release_models();

    let opened = Engine::open(dir.path().join("caps.sqlite")).expect("open succeeds");
    assert_eq!(opened.report.reranker_compiled, cfg!(feature = "default-reranker"));
    assert_eq!(
        opened.report.cuda_compiled,
        cfg!(any(feature = "embed-cuda", feature = "rerank-cuda"))
    );
    assert!(
        !opened.report.rerank_available,
        "empty cache (or feature-off) must report rerank_available == false"
    );
    assert!(
        !empty_cache.exists(),
        "open must not create cache directories or initiate network downloads"
    );

    opened.engine.close().unwrap();

    match prev_device {
        Some(v) => unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", v) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANK_DEVICE") },
    }
    match prev_cache {
        Some(v) => unsafe { std::env::set_var("FATHOMDB_RERANKER_CACHE", v) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANKER_CACHE") },
    }
}

#[cfg(feature = "default-reranker")]
#[test]
fn open_report_rerank_available_true_when_local_cache_sha256_verified() {
    // Check if the host's default cache has the pinned reranker weights.
    let Some(default_cache) = dirs::cache_dir() else {
        return;
    };
    let prev_device = std::env::var_os("FATHOMDB_RERANK_DEVICE");
    let prev_cache = std::env::var_os("FATHOMDB_RERANKER_CACHE");
    unsafe {
        std::env::set_var("FATHOMDB_RERANK_DEVICE", "cpu");
        std::env::set_var("FATHOMDB_RERANKER_CACHE", &default_cache);
    }
    let _ = fathomdb_embedder::module_models::release_models();

    if fathomdb_embedder::reranker_locally_ready() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("warm_caps.sqlite")).expect("open succeeds");
        assert!(opened.report.reranker_compiled);
        assert!(
            opened.report.rerank_available,
            "warm SHA-256-verified cache must report rerank_available == true"
        );
        opened.engine.close().unwrap();
    }

    match prev_device {
        Some(v) => unsafe { std::env::set_var("FATHOMDB_RERANK_DEVICE", v) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANK_DEVICE") },
    }
    match prev_cache {
        Some(v) => unsafe { std::env::set_var("FATHOMDB_RERANKER_CACHE", v) },
        None => unsafe { std::env::remove_var("FATHOMDB_RERANKER_CACHE") },
    }
}
