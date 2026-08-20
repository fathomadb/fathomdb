//! The TC-5 factory accepts an explicit device and local asset directory only.

use std::path::Path;

use fathomdb_embedder::{CandleBgeEmbedder, ExplicitCandleDevice};
use tempfile::tempdir;

#[test]
fn cache_only_factory_refuses_missing_local_asset_before_any_loader_or_device_fallback() {
    let result = CandleBgeEmbedder::new_from_local_asset_on_device(
        Path::new("/definitely-missing-tc5-local-asset"),
        ExplicitCandleDevice::Cpu,
    );
    assert!(result.is_err());
}

#[test]
fn cache_only_factory_checksum_verifies_local_assets_before_cuda_initialization() {
    let asset_dir = tempdir().unwrap();
    std::fs::write(asset_dir.path().join("config.json"), b"not the pinned config").unwrap();

    let result = CandleBgeEmbedder::new_from_local_asset_on_device(
        asset_dir.path(),
        ExplicitCandleDevice::Cuda(usize::MAX),
    );
    let error = match result {
        Ok(_) => panic!("a checksum-mismatched local asset must not construct an embedder"),
        Err(error) => error,
    };

    assert!(error.to_string().starts_with("checksum mismatch"));
}
