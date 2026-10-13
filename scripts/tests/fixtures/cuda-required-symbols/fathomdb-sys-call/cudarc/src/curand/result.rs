//! Fixture: the sys calls of cudarc's cuRAND result layer.
use super::sys::{self};

pub type Handle = sys::curandHandle_t;

pub unsafe fn call_0() {
    sys::curandCreateGenerator(core::ptr::null_mut()).result()
}

pub unsafe fn call_1() {
    sys::curandDestroyGenerator(core::ptr::null_mut()).result()
}

