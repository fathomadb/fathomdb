//! Slice 70 `doctor gpu` fixture matrix.
//!
//! This is deliberately independent of CUDA hardware.  It exercises the
//! inventory/provider boundary that the CLI consumes; the production Candle
//! provider is separately responsible for obtaining this evidence from the
//! CUDA driver.

use std::str::FromStr;

use fathomdb_embedder::{
    diagnose_gpu, CudaDeviceInfo, CudaProbeError, CudaProvider, CudaVisibleDevice,
    DoctorGpuDiagnosticResult, DoctorGpuStatus, EmbedDevicePolicy,
};

#[derive(Debug)]
struct FixtureCudaProvider {
    inventory: Result<Vec<CudaVisibleDevice>, CudaProbeError>,
    probes: Vec<Result<CudaDeviceInfo, CudaProbeError>>,
    enumerate_calls: usize,
    probe_calls: Vec<usize>,
}

impl FixtureCudaProvider {
    fn new(
        inventory: Result<Vec<CudaVisibleDevice>, CudaProbeError>,
        probes: Vec<Result<CudaDeviceInfo, CudaProbeError>>,
    ) -> Self {
        Self { inventory, probes, enumerate_calls: 0, probe_calls: Vec::new() }
    }
}

impl CudaProvider for FixtureCudaProvider {
    fn enumerate_visible_cuda_devices(&mut self) -> Result<Vec<CudaVisibleDevice>, CudaProbeError> {
        self.enumerate_calls += 1;
        self.inventory.clone()
    }

    fn probe_cuda(&mut self, ordinal: usize) -> Result<CudaDeviceInfo, CudaProbeError> {
        self.probe_calls.push(ordinal);
        self.probes.get(ordinal).cloned().unwrap_or_else(|| Err(CudaProbeError::NoVisibleDevice))
    }
}

fn visible(ordinal: usize, uuid: &str) -> CudaVisibleDevice {
    CudaVisibleDevice {
        visible_ordinal: ordinal,
        uuid: uuid.to_owned(),
        name: format!("GPU {ordinal}"),
        compute_capability: Some("8.6".to_owned()),
    }
}

fn compatible(ordinal: usize, uuid: &str) -> Result<CudaDeviceInfo, CudaProbeError> {
    Ok(CudaDeviceInfo {
        ordinal,
        uuid: Some(uuid.to_owned()),
        name: Some(format!("GPU {ordinal}")),
        driver_version: None,
        compute_capability: Some("8.6".to_owned()),
        cuda_toolkit_version: None,
    })
}

#[test]
fn doctor_gpu_has_the_exact_thirteen_row_matrix_and_never_uses_open_resolution() {
    let cases = [
        ("cpu", false, Ok(vec![]), vec![], DoctorGpuStatus::SelectedCpuNoCuda, Some("cpu"), 0),
        ("auto", false, Ok(vec![]), vec![], DoctorGpuStatus::CudaNotCompiled, Some("cpu"), 0),
        ("auto", true, Ok(vec![]), vec![], DoctorGpuStatus::CudaUnavailable, Some("cpu"), 0),
        (
            "auto",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![Err(CudaProbeError::Incompatible { message: "old driver".into() })],
            DoctorGpuStatus::CudaIncompatible,
            Some("cpu"),
            0,
        ),
        (
            "auto",
            true,
            Err(CudaProbeError::ProbeFailed { message: "driver".into() }),
            vec![],
            DoctorGpuStatus::ProbeFailed,
            Some("cpu"),
            70,
        ),
        (
            "auto",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![Err(CudaProbeError::ProbeFailed { message: "allocation".into() })],
            DoctorGpuStatus::ProbeFailed,
            Some("cpu"),
            70,
        ),
        (
            "auto",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![compatible(0, "GPU-a")],
            DoctorGpuStatus::SelectedCuda,
            Some("cuda:0"),
            0,
        ),
        ("cuda:0", false, Ok(vec![]), vec![], DoctorGpuStatus::CudaNotCompiled, None, 65),
        (
            "cuda:1",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![compatible(0, "GPU-a")],
            DoctorGpuStatus::CudaUnavailable,
            None,
            65,
        ),
        (
            "cuda:0",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![Err(CudaProbeError::Incompatible { message: "old driver".into() })],
            DoctorGpuStatus::CudaIncompatible,
            None,
            65,
        ),
        (
            "cuda:0",
            true,
            Err(CudaProbeError::ProbeFailed { message: "driver".into() }),
            vec![],
            DoctorGpuStatus::ProbeFailed,
            None,
            70,
        ),
        (
            "cuda:0",
            true,
            Ok(vec![visible(0, "GPU-a")]),
            vec![Err(CudaProbeError::ProbeFailed { message: "allocation".into() })],
            DoctorGpuStatus::ProbeFailed,
            None,
            70,
        ),
    ];

    for (raw, cuda_compiled, inventory, probes, status, effective_device, exit_code) in cases {
        let policy = EmbedDevicePolicy::from_str(raw).unwrap();
        let mut provider = FixtureCudaProvider::new(inventory, probes);

        let diagnostic = diagnose_gpu(policy, cuda_compiled, &mut provider);

        assert_eq!(diagnostic.status(), status, "{raw}");
        assert_eq!(diagnostic.effective_device(), effective_device, "{raw}");
        assert_eq!(diagnostic.exit_code(), exit_code, "{raw}");
        if status == DoctorGpuStatus::SelectedCuda {
            assert_eq!(diagnostic.selected_uuid(), Some("GPU-a"), "{raw}");
        } else {
            assert_eq!(diagnostic.selected_uuid(), None, "{raw}");
        }
        if raw == "cpu" || (!cuda_compiled && raw == "auto") {
            assert_eq!(provider.enumerate_calls, 0, "{raw}");
            assert!(provider.probe_calls.is_empty(), "{raw}");
        }
    }

    let mut provider = FixtureCudaProvider::new(Ok(vec![]), vec![]);
    let invalid = DoctorGpuDiagnosticResult::from_invalid_policy(&mut provider);
    assert_eq!(invalid.status(), DoctorGpuStatus::InvalidPolicy);
    assert_eq!(invalid.effective_device(), None);
    assert_eq!(invalid.exit_code(), 70);
    assert_eq!(provider.enumerate_calls, 0);
    assert!(provider.probe_calls.is_empty());
}

#[test]
fn doctor_gpu_preserves_the_visible_inventory_and_binds_selected_uuid_to_ordinal() {
    let devices = vec![visible(0, "GPU-first"), visible(1, "GPU-second")];
    let mut provider = FixtureCudaProvider::new(
        Ok(devices.clone()),
        vec![compatible(0, "GPU-first"), compatible(1, "GPU-second")],
    );

    let diagnostic = diagnose_gpu(EmbedDevicePolicy::Cuda(1), true, &mut provider);

    assert_eq!(diagnostic.status(), DoctorGpuStatus::SelectedCuda);
    assert_eq!(diagnostic.devices(), devices.as_slice());
    assert_eq!(diagnostic.selected_uuid(), Some("GPU-second"));
    assert_eq!(diagnostic.effective_device(), Some("cuda:1"));
    assert_eq!(provider.probe_calls, vec![1]);
}
