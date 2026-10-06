#![cfg(all(target_arch = "aarch64", target_os = "linux"))]

use candle_core::{DType, Device, Error, Tensor};

#[test]
fn cpu_f16_matmul_returns_an_error_without_full_fp16() -> candle_core::Result<()> {
    let lhs = Tensor::from_vec(vec![half::f16::from_f32(1.0)], (1, 1), &Device::Cpu)?;
    let rhs = Tensor::from_vec(vec![half::f16::from_f32(1.0)], (1, 1), &Device::Cpu)?;

    let error = lhs.matmul(&rhs).expect_err("Linux AArch64 CPU F16 matmul must be unsupported");
    assert!(matches!(error, Error::UnsupportedDTypeForOp(DType::F16, "matmul")));
    Ok(())
}
