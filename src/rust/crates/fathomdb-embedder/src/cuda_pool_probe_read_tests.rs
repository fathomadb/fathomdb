//! 0.8.28 Slice 31 D-19 (design § 14.2, owner ruling Q36): a plain Rust
//! process reaches the private pool through the `cuInit` its own probe
//! already made. The decision reads that record; FathomDB adds no call.

use super::test_fake::{DefaultsOnly, Fake};
use super::*;

const EVERY_INIT: [ModuleLoadInit; 6] = [
    ModuleLoadInit::Ran,
    ModuleLoadInit::OptedOut,
    ModuleLoadInit::SkippedCpuOnly,
    ModuleLoadInit::DriverAbsent,
    ModuleLoadInit::Failed(2),
    ModuleLoadInit::NotAtLoad,
];

fn gate(init: ModuleLoadInit, probe_ok: bool) -> Result<(), CudaAllocatorReason> {
    pre_device_gates_with_probe(&RawPoolSettings::default(), init, probe_ok).map(|_| ())
}

#[test]
fn not_at_load_is_admitted_only_when_the_driver_was_initialized() {
    assert_eq!(gate(ModuleLoadInit::NotAtLoad, true), Ok(()));
    assert_eq!(gate(ModuleLoadInit::NotAtLoad, false), Err(CudaAllocatorReason::CuinitNotAtLoad));
}

#[test]
fn every_other_load_outcome_keeps_its_result_whether_or_not_the_driver_was_initialized() {
    for init in EVERY_INIT.into_iter().filter(|init| *init != ModuleLoadInit::NotAtLoad) {
        let expected = init_gate(init);
        assert_eq!(gate(init, true), expected, "{init:?} with the driver initialized");
        assert_eq!(gate(init, false), expected, "{init:?} without it");
    }
}

#[test]
fn the_existing_pre_device_gates_reads_no_probe() {
    for init in EVERY_INIT {
        assert_eq!(
            pre_device_gates(&RawPoolSettings::default(), init).map(|_| ()),
            gate(init, false),
            "{init:?}"
        );
    }
}

#[test]
fn mode_off_still_wins_over_an_admitted_probe() {
    let raw = RawPoolSettings { mode: Some("off".to_owned()), ..RawPoolSettings::default() };
    assert_eq!(
        pre_device_gates_with_probe(&raw, ModuleLoadInit::NotAtLoad, true).map(|_| ()),
        Err(CudaAllocatorReason::ModeOff)
    );
}

#[test]
fn a_pool_driver_reports_no_initialized_driver_by_default() {
    assert!(!DefaultsOnly.driver_initialized());
}

#[test]
fn a_process_with_no_load_hook_and_an_initialized_driver_decides_private() {
    let driver =
        Fake { init: ModuleLoadInit::NotAtLoad, driver_initialized: true, ..Fake::default() };
    let decision = decide_and_build(&driver, 0);
    assert!(matches!(decision, Decision::Private(_)));
    let report = decision.report(0, Some(CudaAllocatorPath::Private));
    assert_eq!(report.reason, CudaAllocatorReason::PrivatePool);
    assert_eq!(report.module_load_init, ModuleLoadInit::NotAtLoad);
}

#[test]
fn a_process_with_no_load_hook_and_no_initialized_driver_keeps_cuinit_not_at_load() {
    let driver =
        Fake { init: ModuleLoadInit::NotAtLoad, driver_initialized: false, ..Fake::default() };
    let decision = decide_and_build(&driver, 0);
    let report = decision.report(0, None);
    assert_eq!(report.reason, CudaAllocatorReason::CuinitNotAtLoad);
    assert_eq!(report.module_load_init, ModuleLoadInit::NotAtLoad);
}

#[test]
fn a_recorded_load_outcome_is_not_overridden_by_the_probe() {
    let driver =
        Fake { init: ModuleLoadInit::OptedOut, driver_initialized: true, ..Fake::default() };
    let decision = decide_and_build(&driver, 0);
    assert_eq!(decision.report(0, None).reason, CudaAllocatorReason::CuinitOptedOut);
}
