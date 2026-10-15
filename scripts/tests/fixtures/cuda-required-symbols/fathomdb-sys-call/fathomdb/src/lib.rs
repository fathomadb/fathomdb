//! Fixture: FathomDB reaches cuBLAS only through Candle.
pub fn noop() {}
pub unsafe fn direct() {
    candle_core::cuda::cudarc::curand::sys::curandCreateGenerator(core::ptr::null_mut(), 0);
}
