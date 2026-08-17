//! The TC-5 factory accepts an explicit device and local asset directory only.

use std::path::Path;

use fathomdb_embedder::{CandleBgeEmbedder, ExplicitCandleDevice};

#[test]
fn cache_only_factory_refuses_missing_local_asset_before_any_loader_or_device_fallback() {
    let result = CandleBgeEmbedder::new_from_local_asset_on_device(
        Path::new("/definitely-missing-tc5-local-asset"),
        ExplicitCandleDevice::Cpu,
    );
    assert!(result.is_err());
}
