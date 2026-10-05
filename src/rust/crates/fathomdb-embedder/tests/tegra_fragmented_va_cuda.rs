//! Forced CUDA on Jetson must survive a fragmented process address space.
//!
//! On Tegra (measured on a Jetson AGX Orin 64 GB, L4T R36, CUDA 12.6) the
//! driver's default memory pool needs one contiguous range of roughly a third
//! of device memory inside [8 GiB, 128 GiB) of the process address space.
//! Three 4 KiB `PROT_NONE` pages at 38, 68 and 98 GiB, mapped before `cuInit`,
//! leave no such range: `cuDeviceGetDefaultMemPool` and `cuMemAllocAsync`
//! then return `CUDA_ERROR_OUT_OF_MEMORY` with tens of GB free, while
//! synchronous `cuMemAlloc` still works. Node/V8 heaps produce the same
//! layout by accident. A CUDA context whose default pool is unavailable must
//! therefore allocate synchronously, so the forced `cuda:0` probe succeeds.
//!
//! The layout must exist before the first CUDA call in the process, so this
//! file holds exactly one test and nothing else in this binary touches CUDA.
//!
//! The test asserts that the layout really makes the default pool unavailable
//! and that the context fell back to synchronous allocation, so it can never
//! pass without exercising the fallback. That outcome is only established for
//! integrated (Tegra) GPUs; on aarch64 Linux hosts without a CUDA driver, without
//! a device, or whose `cuda:0` is a discrete GPU, it prints a SKIP notice naming
//! the reason and returns (or panics under `FATHOMDB_REQUIRE_LIVE=1`, which
//! promises a provisioned Tegra GPU).
//!
//! Run it on a Jetson with:
//!
//! ```text
//! export PATH=/usr/local/cuda-12.6/bin:$PATH CUDA_COMPUTE_CAP=87
//! cargo test -p fathomdb-embedder --features embed-cuda \
//!   --test tegra_fragmented_va_cuda -- --nocapture
//! ```
#![cfg(all(target_os = "linux", target_arch = "aarch64", feature = "embed-cuda"))]

use std::ffi::{c_int, c_long, c_void};

use candle_core::cuda::cudarc::driver::{result, sys, DriverError};
use candle_core::{DType, Device, Tensor};
use fathomdb_embedder::{resolve_default_embedder_device_from_env, EffectiveEmbedDevice};

#[path = "support/live.rs"]
mod live;

const GIB: usize = 1 << 30;
const PAGE: usize = 4096;
const BLOCKER_ADDRESSES: [usize; 3] = [38 * GIB, 68 * GIB, 98 * GIB];

const PROT_NONE: c_int = 0;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
// Unlike MAP_FIXED this refuses to replace an existing mapping, so the test
// can never clobber memory the runtime already owns.
const MAP_FIXED_NOREPLACE: c_int = 0x10_0000;

extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: c_long,
    ) -> *mut c_void;
}

fn map_address_space_blockers() {
    for address in BLOCKER_ADDRESSES {
        // SAFETY: an anonymous PROT_NONE mapping at an address the kernel
        // verifies is unmapped; nothing reads or writes it, and it is
        // deliberately leaked for the lifetime of the test process.
        let mapped = unsafe {
            mmap(
                address as *mut c_void,
                PAGE,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE,
                -1,
                0,
            )
        };
        assert_eq!(
            mapped as usize,
            address,
            "could not map the 4 KiB blocker at {address:#x}: {}",
            std::io::Error::last_os_error()
        );
    }
}

/// Why this host is outside the measured target, or `None` when `cuda:0` is an
/// integrated (Tegra) GPU.
fn non_target_reason() -> Option<String> {
    // SAFETY: only probes for the driver library; loads nothing on failure.
    if !unsafe { sys::is_culib_present() } {
        return Some("no CUDA driver library".to_owned());
    }
    match result::init() {
        Ok(()) => {}
        Err(DriverError(sys::CUresult::CUDA_ERROR_NO_DEVICE)) => {
            return Some("no CUDA device".to_owned());
        }
        Err(error) => panic!("cuInit failed with the blockers in place: {error:?}"),
    }
    let count = result::device::get_count().expect("cuDeviceGetCount");
    if count < 1 {
        return Some("no CUDA device".to_owned());
    }
    let device = result::device::get(0).expect("cuDeviceGet(0)");
    // SAFETY: `device` was just returned by the initialized driver.
    let integrated = unsafe {
        result::device::get_attribute(
            device,
            sys::CUdevice_attribute_enum::CU_DEVICE_ATTRIBUTE_INTEGRATED,
        )
    }
    .expect("CU_DEVICE_ATTRIBUTE_INTEGRATED");
    (integrated == 0).then(|| {
        "cuda:0 is a discrete GPU; the fragmented-layout pool failure is only \
         established on integrated (Tegra) GPUs"
            .to_owned()
    })
}

#[test]
fn forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable() {
    // Must precede every CUDA call in this process (the driver lays out its
    // GPU virtual-address reservations during `cuInit`).
    map_address_space_blockers();

    if let Some(reason) = non_target_reason() {
        live::require_live_or_skip(&format!("SKIP tegra_fragmented_va_cuda: {reason}"));
        return;
    }

    std::env::set_var("FATHOMDB_EMBED_DEVICE", "cuda:0");
    let resolution = match resolve_default_embedder_device_from_env() {
        Ok(resolution) => resolution,
        Err(error) => {
            // The typed refusal deliberately drops the driver error, so repeat
            // the probe's two Candle calls to show the cause.
            let direct = Device::new_cuda(0)
                .and_then(|device| Tensor::zeros(1, DType::F32, &device).map(|_| ()))
                .map_err(|direct_error| direct_error.to_string());
            panic!(
                "forced cuda:0 was refused [{}]: {error}; direct Candle probe: {direct:?}",
                error.kind()
            );
        }
    };
    match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(info) => assert_eq!(info.ordinal, 0),
        EffectiveEmbedDevice::Cpu => panic!("forced cuda:0 resolved to CPU: {resolution:?}"),
    }

    let device = Device::new_cuda(0).expect("Candle CUDA device after a successful probe");
    let Device::Cuda(cuda) = &device else { panic!("expected a CUDA device, got {device:?}") };
    // SAFETY: the ordinal was just initialized by Candle in this process.
    let pool =
        unsafe { result::device::get(0).and_then(|dev| result::device::get_default_mem_pool(dev)) };
    let async_alloc = cuda.cuda_stream().context().has_async_alloc();
    eprintln!("default memory pool: {pool:?}; stream-ordered allocation: {async_alloc}");
    assert_eq!(
        pool,
        Err(DriverError(sys::CUresult::CUDA_ERROR_OUT_OF_MEMORY)),
        "the blockers must leave the default memory pool unavailable on this Tegra device"
    );
    assert!(
        !async_alloc,
        "a context whose default pool is unavailable must allocate synchronously"
    );

    let lhs = Tensor::from_vec(vec![1f32, 2., 3., 4.], (2, 2), &device).expect("upload lhs");
    let rhs = Tensor::from_vec(vec![5f32, 6., 7., 8.], (2, 2), &device).expect("upload rhs");
    let product = lhs.matmul(&rhs).expect("CUDA matmul").to_vec2::<f32>().expect("download");
    assert_eq!(product, vec![vec![19., 22.], vec![43., 50.]]);
    let summed = (&lhs + 1.0).expect("CUDA affine").to_vec2::<f32>().expect("download");
    assert_eq!(summed, vec![vec![2., 3.], vec![4., 5.]]);

    let empty = Tensor::zeros(0, DType::F32, &device).expect("zero-element zeros");
    assert!(empty.to_vec1::<f32>().expect("download zero-element zeros").is_empty());
    let empty_rows = Tensor::zeros((0, 384), DType::F32, &device).expect("zero-row zeros");
    assert_eq!(empty_rows.dims(), &[0, 384]);
    let uploaded = Tensor::from_vec(Vec::<f32>::new(), 0, &device).expect("zero-element upload");
    assert!((&uploaded + 1.0)
        .expect("zero-element op")
        .to_vec1::<f32>()
        .expect("download")
        .is_empty());
    drop((empty, empty_rows, uploaded, lhs, rhs));
    device.synchronize().expect("synchronize after zero-length frees");
}
