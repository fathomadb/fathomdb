//! Ordinary OOM preservation on the allocator actually selected by the product.

use super::*;
use candle_core::cuda::cudarc::driver::{result, sys, AllocMode, CudaContext};
use candle_core::{DType, Tensor};
use std::sync::Arc;

const MIB: usize = 1 << 20;
const GIB: usize = 1 << 30;
const CHUNK: usize = 32 * MIB;

fn pool_allocation(context: &Arc<CudaContext>) -> (u64, u64) {
    let _admission = context.primary_lifetime().unwrap().admit().expect("healthy pool observation");
    if let Some(pool) = context.mem_pool() {
        return (pool.used_current().unwrap(), pool.reserved_current().unwrap());
    }
    let device = result::device::get(context.ordinal() as i32).unwrap();
    // SAFETY: this initialized device owns its default pool for the process
    // lifetime; healthy generation admission excludes stale native observation.
    let pool = unsafe { result::device::get_default_mem_pool(device) }.unwrap();
    let read = |attribute: sys::CUmemPool_attribute| {
        let mut bytes = 0_u64;
        // SAFETY: the driver-owned pool is live under the admission above;
        // both requested attributes write exactly one u64 to this valid output.
        unsafe {
            result::mem_pool::get_attribute(pool, attribute, (&mut bytes as *mut u64).cast())
        }
        .unwrap();
        bytes
    };
    (
        read(sys::CUmemPool_attribute::CU_MEMPOOL_ATTR_USED_MEM_CURRENT),
        read(sys::CUmemPool_attribute::CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT),
    )
}

#[test]
#[ignore = "one reviewed exclusive CUDA child; outer 120s deadline and resource floors required"]
fn gpu_ordinary_oom_preserves_generation_allocator_cleanup_and_retry() {
    assert_eq!(std::env::var("FATHOMDB_REQUIRE_LIVE").as_deref(), Ok("1"));
    assert_eq!(std::env::var("FATHOMDB_EMBED_DEVICE").as_deref(), Ok("cuda:0"));
    for setting in [ENV_POOL_MODE, ENV_POOL_MAXSIZE, ENV_POOL_RELEASE_THRESHOLD] {
        assert!(std::env::var_os(setting).is_none(), "use the actual default policy: {setting}");
    }
    #[cfg(target_arch = "aarch64")]
    assert_eq!(
        crate::cuda_driver_init::run_module_load_early_init(true, true),
        ModuleLoadInit::Ran
    );

    let device = new_cuda_device(0).expect("actual product device");
    let context = device.as_cuda_device().unwrap().cuda_stream().context().clone();
    let lifetime = context.primary_lifetime().expect("tracked product generation").clone();
    let generation = lifetime.recorded_id();
    let allocator = context.alloc_mode();
    let report = allocator_report(&device);
    let name = context.name().expect("device name");
    #[cfg(target_arch = "aarch64")]
    {
        assert!(name.contains("Orin"), "not the selected Orin venue: {name}");
        assert_eq!(allocator, AllocMode::Private);
        assert_eq!(report.as_ref().unwrap().pool_max_size_bytes, Some((3 * GIB) as u64));
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        assert!(name.contains("RTX 3090"), "not the selected 3090 venue: {name}");
        assert_eq!(allocator, AllocMode::Default);
        assert!(context.total_mem().unwrap() <= 26 * GIB);
    }
    let before = pool_allocation(&context);
    assert!(lifetime.loss().is_none());
    let model_guard = PrivatePoolModelGuard::new(&device);
    let live = Tensor::ones(MIB, DType::F32, &device).expect("4MiB healthy live buffer");
    let temporary = (&live + 2.0).expect("4MiB healthy temporary");
    device.synchronize().expect("healthy preparation complete");
    let open = pool_allocation(&context);
    assert!(
        open.0 >= before.0 + (8 * MIB) as u64,
        "live and temporary buffers must allocate: {before:?} -> {open:?}"
    );
    assert_eq!(temporary.narrow(0, 0, 4).unwrap().to_vec1::<f32>().unwrap(), [3.0; 4]);

    let mut held = Vec::new();
    let error = if allocator == AllocMode::Private {
        let cap = usize::try_from(report.as_ref().unwrap().pool_max_size_bytes.unwrap()).unwrap();
        assert_eq!(cap, 3 * GIB);
        let mut failure = None;
        for _ in 0..=cap / CHUNK {
            match Tensor::zeros(CHUNK, DType::U8, &device) {
                Ok(tensor) => {
                    held.push(tensor);
                    assert!(held.len() * CHUNK + 8 * MIB <= cap, "private cap was not enforced");
                }
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        failure.expect("finite bounded private-pool capacity must be enforced")
    } else {
        // Device-only allocation: no host array or managed/unified overcommit.
        let bytes = context.total_mem().unwrap().checked_add(CHUNK).unwrap();
        Tensor::zeros(bytes, DType::U8, &device).expect_err("request exceeds physical GPU capacity")
    };
    assert_eq!(candle_driver_code(&error), Some(CUDA_ERROR_OUT_OF_MEMORY));
    let typed = forward_failure(&error, &device, "ordinary OOM preservation");
    if allocator == AllocMode::Private {
        assert!(
            matches!(typed, Some(CudaPoolFailure::Exhausted { ordinal: 0, max_size_bytes, .. }) if max_size_bytes == (3 * GIB) as u64),
            "{typed:?}"
        );
    } else {
        assert!(typed.is_none(), "default OOM must not fabricate private exhaustion: {typed:?}");
    }
    assert!(lifetime.loss().is_none(), "ordinary OOM became terminal loss");
    println!("ordinary-oom observed allocator={allocator:?} generation={generation} held_bytes={} native_status=2 typed={typed:?}", held.len() * CHUNK);
    drop(held);
    drop(temporary);
    drop(live);
    drop(model_guard);
    device.synchronize().expect("healthy cleanup after OOM");
    let cleaned = pool_allocation(&context);
    assert_eq!(cleaned.0, before.0, "all live/temporary allocations must be freed");
    if allocator == AllocMode::Private {
        assert_eq!(cleaned.1, 0, "last-model healthy sync/trim remains effective");
    }
    println!("ordinary-oom cleanup before={before:?} open={open:?} cleaned={cleaned:?}");
    drop(lifetime.admit().expect("same generation remains admitted"));
    let retry_guard = PrivatePoolModelGuard::new(&device);
    let retry = Tensor::ones(MIB, DType::F32, &device).expect("retry after pressure removal");
    device.synchronize().expect("retry completed");
    assert_eq!(retry.narrow(0, 0, 4).unwrap().to_vec1::<f32>().unwrap(), [1.0; 4]);
    assert_eq!(context.alloc_mode(), allocator);
    assert_eq!(allocator_report(&device), report);
    assert_eq!(lifetime.recorded_id(), generation);
    assert!(Arc::ptr_eq(&lifetime, &crate::cuda_lifetime::existing(0).unwrap()));
    assert!(lifetime.loss().is_none());
    drop(retry);
    drop(retry_guard);
    device.synchronize().expect("retry buffers freed");
    assert_eq!(pool_allocation(&context).0, before.0);
    drop(context);
    drop(device);
    println!(
        "ordinary-oom retry-and-drop-complete allocator={allocator:?} generation={generation}"
    );
}
