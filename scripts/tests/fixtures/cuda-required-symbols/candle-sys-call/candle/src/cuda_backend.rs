//! Fixture: Candle uses cudarc types, never a sys function.
use cudarc::cublas::sys;
pub const OP: sys::cublasOperation_t = sys::cublasOperation_t::CUBLAS_OP_N;
pub unsafe fn direct() {
    sys::cublasSgemm_v2(core::ptr::null_mut());
}
