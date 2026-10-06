// FATHOMDB PATCH BEGIN: explicit memory pool primitive (see FATHOMDB-PATCH.md).
//! An explicit, opt-in device memory pool ([CudaMemPool]).

use crate::driver::{
    result::{self, DriverError},
    sys,
};

use std::vec::Vec;

/// Properties of an explicit device memory pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemPoolProps {
    /// The pool's `maxSize` in bytes; 0 asks for the driver's default. Applied
    /// only when the bindings have the field (`cuda-12020` and later);
    /// earlier bindings ignore it.
    pub max_size: usize,
    /// `CU_MEMPOOL_ATTR_RELEASE_THRESHOLD`: bytes of freed memory the pool may
    /// keep at a synchronization point (`u64::MAX` keeps everything).
    pub release_threshold: u64,
}

/// The driver's pool properties for `props`: pinned device memory on device
/// `ordinal`, no export handle types.
pub(crate) fn pool_props(ordinal: i32, props: &MemPoolProps) -> sys::CUmemPoolProps {
    // SAFETY: CUmemPoolProps is a plain C struct; all-zero is its documented
    // "unset" value (reserved bytes must be zero).
    let mut raw: sys::CUmemPoolProps = unsafe { std::mem::zeroed() };
    raw.allocType = sys::CUmemAllocationType::CU_MEM_ALLOCATION_TYPE_PINNED;
    raw.handleTypes = sys::CUmemAllocationHandleType::CU_MEM_HANDLE_TYPE_NONE;
    raw.location = sys::CUmemLocation {
        type_: sys::CUmemLocationType::CU_MEM_LOCATION_TYPE_DEVICE,
        id: ordinal,
    };
    set_max_size(&mut raw, props.max_size);
    raw
}

#[cfg(any(
    feature = "cuda-12020",
    feature = "cuda-12030",
    feature = "cuda-12040",
    feature = "cuda-12050",
    feature = "cuda-12060",
    feature = "cuda-12080",
    feature = "cuda-12090",
    feature = "cuda-13000",
    feature = "cuda-13010",
    feature = "cuda-13020"
))]
fn set_max_size(raw: &mut sys::CUmemPoolProps, max_size: usize) {
    raw.maxSize = max_size;
}

#[cfg(not(any(
    feature = "cuda-12020",
    feature = "cuda-12030",
    feature = "cuda-12040",
    feature = "cuda-12050",
    feature = "cuda-12060",
    feature = "cuda-12080",
    feature = "cuda-12090",
    feature = "cuda-13000",
    feature = "cuda-13010",
    feature = "cuda-13020"
)))]
fn set_max_size(_raw: &mut sys::CUmemPoolProps, _max_size: usize) {}

/// Which explicit pool, if any, was installed on each device through
/// [CudaMemPool::install] in this process. The allocator decision reads it, so
/// a process that never installs a pool makes no extra driver call.
pub(crate) struct InstalledPools(std::sync::Mutex<Vec<(sys::CUdevice, usize)>>);

impl InstalledPools {
    pub(crate) const fn new() -> Self {
        Self(std::sync::Mutex::new(Vec::new()))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(sys::CUdevice, usize)>> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub(crate) fn record(&self, cu_device: sys::CUdevice, pool: sys::CUmemoryPool) {
        let mut pools = self.lock();
        pools.retain(|(device, _)| *device != cu_device);
        pools.push((cu_device, pool as usize));
    }

    pub(crate) fn get(&self, cu_device: sys::CUdevice) -> Option<sys::CUmemoryPool> {
        self.lock()
            .iter()
            .find(|(device, _)| *device == cu_device)
            .map(|&(_, pool)| pool as sys::CUmemoryPool)
    }

    /// Forgets `pool` if it is the one recorded for `cu_device`.
    pub(crate) fn forget(&self, cu_device: sys::CUdevice, pool: sys::CUmemoryPool) {
        self.lock()
            .retain(|&(device, recorded)| !(device == cu_device && recorded == pool as usize));
    }
}

pub(crate) static INSTALLED_POOLS: InstalledPools = InstalledPools::new();

/// An explicit memory pool for one device (`cuMemPoolCreate`).
///
/// Creating a pool changes nothing until [CudaMemPool::install] makes it the
/// device's current pool (`cuDeviceSetMemPool`). From then on every
/// stream-ordered allocation on that device (`cuMemAllocAsync`, which
/// [crate::driver::CudaStream::alloc] uses when
/// [crate::driver::CudaContext::has_async_alloc] is true) draws from it, and a
/// context created afterwards selects it as its allocator when the pool can
/// serve a small probe allocation ([crate::driver::CudaContext::alloc_mode]).
///
/// Install the pool before creating the first [crate::driver::CudaContext] for
/// the device: the allocator decision is made once per device per process.
///
/// # Drop
/// Dropping the pool destroys it (`cuMemPoolDestroy`). Destroying a pool that
/// is the device's current pool reverts the device to its default pool, which
/// may be unavailable; a process that installed a pool and keeps allocating
/// on that device must keep the pool alive (for example in a `static`).
#[derive(Debug)]
pub struct CudaMemPool {
    cu_device: sys::CUdevice,
    pool: sys::CUmemoryPool,
}

unsafe impl Send for CudaMemPool {}
unsafe impl Sync for CudaMemPool {}

impl CudaMemPool {
    /// Creates a pool of pinned device memory on device `ordinal` with
    /// `props`. Initializes the driver if needed; needs no current context.
    ///
    /// # Errors
    /// The driver error of `cuInit`, `cuDeviceGet`, `cuMemPoolCreate` (for
    /// example `CUDA_ERROR_OUT_OF_MEMORY` when the process has no room for the
    /// pool's address range) or of setting the release threshold; on the
    /// last, the pool is destroyed before returning.
    pub fn create(ordinal: usize, props: &MemPoolProps) -> Result<Self, DriverError> {
        result::init()?;
        let cu_device = result::device::get(ordinal as i32)?;
        let raw = pool_props(ordinal as i32, props);
        let pool = unsafe { result::mem_pool::create(&raw) }?;
        let created = Self { cu_device, pool };
        let mut threshold = props.release_threshold;
        unsafe {
            result::mem_pool::set_attribute(
                pool,
                sys::CUmemPool_attribute::CU_MEMPOOL_ATTR_RELEASE_THRESHOLD,
                (&mut threshold as *mut u64).cast(),
            )
        }?;
        Ok(created)
    }

    /// Makes this pool the device's current pool (`cuDeviceSetMemPool`).
    ///
    /// # Errors
    /// The driver error of `cuDeviceSetMemPool`; the device's current pool is
    /// then unchanged.
    pub fn install(&self) -> Result<(), DriverError> {
        unsafe { result::device::set_mem_pool(self.cu_device, self.pool) }?;
        INSTALLED_POOLS.record(self.cu_device, self.pool);
        Ok(())
    }

    /// Reads a 64-bit pool attribute (thresholds and the reserved/used
    /// counters).
    ///
    /// # Errors
    /// The driver error of `cuMemPoolGetAttribute`.
    pub fn attribute(&self, attr: sys::CUmemPool_attribute) -> Result<u64, DriverError> {
        let mut value: u64 = 0;
        unsafe { result::mem_pool::get_attribute(self.pool, attr, (&mut value as *mut u64).cast()) }?;
        Ok(value)
    }

    /// Releases unused memory until the pool holds at most `keep` bytes
    /// (`cuMemPoolTrimTo`).
    ///
    /// # Errors
    /// The driver error of `cuMemPoolTrimTo`.
    pub fn trim_to(&self, keep: usize) -> Result<(), DriverError> {
        unsafe { result::mem_pool::trim_to(self.pool, keep) }
    }

    /// The device this pool belongs to.
    pub(crate) fn cu_device(&self) -> sys::CUdevice {
        self.cu_device
    }

    /// The raw pool handle. It stays owned by this value.
    pub fn raw(&self) -> sys::CUmemoryPool {
        self.pool
    }
}

impl Drop for CudaMemPool {
    fn drop(&mut self) {
        INSTALLED_POOLS.forget(self.cu_device, self.pool);
        let _ = unsafe { result::mem_pool::destroy(self.pool) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(n: usize) -> sys::CUmemoryPool {
        n as sys::CUmemoryPool
    }

    #[test]
    fn pool_props_are_pinned_device_memory_on_the_ordinal() {
        let raw = pool_props(
            3,
            &MemPoolProps {
                max_size: 3 << 30,
                release_threshold: 0,
            },
        );
        assert_eq!(
            raw.allocType,
            sys::CUmemAllocationType::CU_MEM_ALLOCATION_TYPE_PINNED
        );
        assert_eq!(
            raw.handleTypes,
            sys::CUmemAllocationHandleType::CU_MEM_HANDLE_TYPE_NONE
        );
        assert_eq!(
            raw.location.type_,
            sys::CUmemLocationType::CU_MEM_LOCATION_TYPE_DEVICE
        );
        assert_eq!(raw.location.id, 3);
        assert!(raw.win32SecurityAttributes.is_null());
        #[cfg(feature = "cuda-12060")]
        assert_eq!(raw.maxSize, 3 << 30);
    }

    #[test]
    fn installed_pools_are_recorded_per_device_and_replaced_on_reinstall() {
        let pools = InstalledPools::new();
        assert_eq!(pools.get(0), None);
        pools.record(0, fake(0x10));
        pools.record(1, fake(0x20));
        assert_eq!(pools.get(0), Some(fake(0x10)));
        assert_eq!(pools.get(1), Some(fake(0x20)));
        pools.record(0, fake(0x30));
        assert_eq!(pools.get(0), Some(fake(0x30)));
        assert_eq!(pools.get(1), Some(fake(0x20)));
    }

    #[test]
    fn forgetting_a_pool_that_is_no_longer_recorded_keeps_the_current_one() {
        let pools = InstalledPools::new();
        pools.record(0, fake(0x10));
        pools.record(0, fake(0x30));
        pools.forget(0, fake(0x10));
        assert_eq!(pools.get(0), Some(fake(0x30)));
        pools.forget(1, fake(0x30));
        assert_eq!(pools.get(0), Some(fake(0x30)));
        pools.forget(0, fake(0x30));
        assert_eq!(pools.get(0), None);
    }
}
// FATHOMDB PATCH END
