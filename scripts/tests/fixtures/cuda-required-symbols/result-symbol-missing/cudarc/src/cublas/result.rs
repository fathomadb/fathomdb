//! Fixture: the sys calls of cudarc's cuBLAS result layer.
use super::sys::{self};

pub type Handle = sys::cublasHandle_t;

pub unsafe fn call_0() {
    sys::cublasCreate_v2(core::ptr::null_mut()).result()
}

pub unsafe fn call_1() {
    sys::cublasDestroy_v2(core::ptr::null_mut()).result()
}

pub unsafe fn call_2() {
    sys::cublasSgemm_v2(core::ptr::null_mut()).result()
}

pub unsafe fn call_3() {
    sys::cublasSnrm2_v2(core::ptr::null_mut()).result()
}

