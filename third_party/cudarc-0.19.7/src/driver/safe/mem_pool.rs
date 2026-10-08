// FATHOMDB PATCH BEGIN: private memory pool primitive (see FATHOMDB-PATCH.md).
//! An explicit device memory pool ([CudaMemPool]) owned by the caller.

use crate::driver::{
    result::{self, DriverError},
    sys,
};

/// Properties of a [CudaMemPool].
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
    // SAFETY: CUmemPoolProps is a plain C struct whose all-zero value means
    // "unset", and its reserved bytes must be zero.
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

/// A memory pool for one device (`cuMemPoolCreate`), created and owned by the
/// caller.
///
/// The pool is never made the device's current pool, so creating it changes
/// nothing for other users of the device. Allocations draw from it only
/// through a context built with
/// [crate::driver::CudaContext::new_with_mem_pool], which keeps the pool alive
/// for as long as the context and its streams and slices exist.
///
/// # Drop
/// Dropping the last reference destroys the pool (`cuMemPoolDestroy`).
#[derive(Debug)]
pub struct CudaMemPool {
    pub(crate) cu_device: sys::CUdevice,
    pub(crate) pool: sys::CUmemoryPool,
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
        // Owned from here, so an error below destroys the pool.
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

    /// Reads a 64-bit pool attribute: the release threshold or one of the
    /// reserved / used memory counters.
    ///
    /// # Errors
    /// The driver error of `cuMemPoolGetAttribute`, including
    /// `CUDA_ERROR_INVALID_VALUE` for an attribute that is not 64 bits wide.
    pub fn attribute(&self, attr: sys::CUmemPool_attribute) -> Result<u64, DriverError> {
        use sys::CUmemPool_attribute::*;
        if !matches!(
            attr,
            CU_MEMPOOL_ATTR_RELEASE_THRESHOLD
                | CU_MEMPOOL_ATTR_RESERVED_MEM_CURRENT
                | CU_MEMPOOL_ATTR_RESERVED_MEM_HIGH
                | CU_MEMPOOL_ATTR_USED_MEM_CURRENT
                | CU_MEMPOOL_ATTR_USED_MEM_HIGH
        ) {
            // The other attributes are 32-bit; the driver would write only
            // half of `value`.
            return Err(DriverError(sys::CUresult::CUDA_ERROR_INVALID_VALUE));
        }
        let mut value: u64 = 0;
        unsafe {
            result::mem_pool::get_attribute(self.pool, attr, (&mut value as *mut u64).cast())
        }?;
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

    /// The raw pool handle. It stays owned by this value.
    pub fn raw(&self) -> sys::CUmemoryPool {
        self.pool
    }
}

impl Drop for CudaMemPool {
    fn drop(&mut self) {
        let _ = unsafe { result::mem_pool::destroy(self.pool) };
    }
}
// FATHOMDB PATCH END
