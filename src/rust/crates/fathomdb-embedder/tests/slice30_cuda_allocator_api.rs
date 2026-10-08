//! The public surface added for the Tegra private CUDA memory pool: the
//! allocator report on `CudaDeviceInfo` and the three typed pool errors.

use fathomdb_embedder::{
    CudaAllocatorPath, CudaAllocatorReason, CudaAllocatorReport, CudaDeviceInfo, ModuleLoadInit,
    ReleaseThreshold, RerankerDevicePolicyError,
};

#[test]
fn a_constructed_device_info_has_no_allocator_report_until_one_is_attached() {
    let info = CudaDeviceInfo::new(
        0,
        Some("GPU-0".to_owned()),
        Some("Orin".to_owned()),
        None,
        Some("8.7".to_owned()),
        None,
    );
    assert_eq!(info.ordinal, 0);
    assert_eq!(info.uuid.as_deref(), Some("GPU-0"));
    assert_eq!(info.name.as_deref(), Some("Orin"));
    assert_eq!(info.driver_version, None);
    assert_eq!(info.compute_capability.as_deref(), Some("8.7"));
    assert_eq!(info.cuda_toolkit_version, None);
    assert_eq!(info.cuda_allocator, None);

    let report = CudaAllocatorReport::new(
        Some(CudaAllocatorPath::Private),
        CudaAllocatorReason::PrivatePool,
        Some(3 << 30),
        Some(ReleaseThreshold::Zero),
        ModuleLoadInit::Ran,
    );
    let info = info.with_cuda_allocator(Some(report.clone()));
    assert_eq!(info.cuda_allocator, Some(report));
}

#[test]
fn the_report_names_are_stable() {
    assert_eq!(CudaAllocatorReason::NotBuilt.as_str(), "not_built");
    assert_eq!(CudaAllocatorPath::Synchronous.as_str(), "synchronous");
    assert_eq!(ReleaseThreshold::Max.as_str(), "max");
    assert_eq!(ModuleLoadInit::NotAtLoad.as_str(), "not_at_load");
}

#[test]
fn the_reranker_policy_error_has_a_kind_for_each_pool_error() {
    let exhausted = RerankerDevicePolicyError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 3 << 30,
        message: "forward: out of memory".to_owned(),
    };
    assert_eq!(exhausted.kind(), "cuda_pool_exhausted");
    assert_eq!(exhausted.ordinal(), Some(0));
    assert!(exhausted.to_string().contains("forward: out of memory"), "{exhausted}");

    let lost = RerankerDevicePolicyError::CudaContextLost {
        recorded_context_id: 3,
        current_context_id: None,
        driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
        operation: "forward".to_owned(),
    };
    assert_eq!(lost.kind(), "cuda_context_lost");
    assert_eq!(lost.ordinal(), None);
    assert!(lost.to_string().contains("CUDA_ERROR_CONTEXT_IS_DESTROYED"), "{lost}");

    let refused = RerankerDevicePolicyError::CudaPrivateBuildRefused {
        ordinal: 1,
        message: "build failed".to_owned(),
    };
    assert_eq!(refused.kind(), "cuda_private_build_refused");
    assert_eq!(refused.ordinal(), Some(1));
    assert!(refused.to_string().contains("build failed"), "{refused}");
}

#[cfg(feature = "default-embedder")]
#[test]
fn the_embedder_load_error_carries_each_pool_error() {
    use fathomdb_embedder::loader::EmbedderLoadError;
    let errors = [
        EmbedderLoadError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 1,
            message: "exhausted".to_owned(),
        },
        EmbedderLoadError::CudaContextLost {
            recorded_context_id: 1,
            current_context_id: Some(2),
            driver_error: "lost".to_owned(),
            operation: "load".to_owned(),
        },
        EmbedderLoadError::CudaPrivateBuildRefused { ordinal: 0, message: "refused".to_owned() },
    ];
    for (error, needle) in errors.iter().zip(["exhausted", "lost", "refused"]) {
        assert!(error.to_string().contains(needle), "{error}");
    }
}

#[cfg(feature = "default-reranker")]
#[test]
fn the_reranker_load_error_carries_each_pool_error() {
    use fathomdb_embedder::RerankerLoadError;
    let errors = [
        RerankerLoadError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 1,
            message: "exhausted".to_owned(),
        },
        RerankerLoadError::CudaContextLost {
            recorded_context_id: 1,
            current_context_id: None,
            driver_error: "lost".to_owned(),
            operation: "load".to_owned(),
        },
        RerankerLoadError::CudaPrivateBuildRefused { ordinal: 0, message: "refused".to_owned() },
    ];
    for (error, needle) in errors.iter().zip(["exhausted", "lost", "refused"]) {
        assert!(error.to_string().contains(needle), "{error}");
    }
}
