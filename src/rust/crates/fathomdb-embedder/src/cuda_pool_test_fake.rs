//! A pool driver for the Slice 31 pure tests (the close release and the
//! probe read). The frozen Slice 30 test file keeps its own `FakeDriver`.

use std::panic::panic_any;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, Once, OnceLock};

use super::*;

/// `cuDeviceTotalMem` on the AGX Orin 64 GB (61.36 GiB).
pub(super) const ORIN_64_TOTAL: u64 = 65_879_896_064;
/// The context id the fake's first private context reports.
pub(super) const FIRST_CONTEXT_ID: u64 = 7;

pub(super) type FakeDecision = Decision<FakePool, FakeContext>;

#[derive(Debug)]
pub(super) struct FakePool;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FakeContext(pub(super) u64);

pub(super) struct Fake {
    pub(super) settings: RawPoolSettings,
    pub(super) init: ModuleLoadInit,
    pub(super) driver_initialized: bool,
    pub(super) primary_active: AtomicBool,
    pub(super) current_id: Mutex<Result<u64, u32>>,
    pub(super) fail_synchronize: AtomicBool,
    pub(super) fail_trim: AtomicBool,
    pub(super) panic_in_trim: AtomicBool,
    pub(super) synchronizes: AtomicUsize,
    pub(super) trims: AtomicUsize,
    pub(super) snapshot: Once,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            settings: RawPoolSettings::default(),
            init: ModuleLoadInit::Ran,
            driver_initialized: false,
            primary_active: AtomicBool::new(true),
            current_id: Mutex::new(Ok(FIRST_CONTEXT_ID)),
            fail_synchronize: AtomicBool::new(false),
            fail_trim: AtomicBool::new(false),
            panic_in_trim: AtomicBool::new(false),
            synchronizes: AtomicUsize::new(0),
            trims: AtomicUsize::new(0),
            snapshot: Once::new(),
        }
    }
}

impl Fake {
    /// A process whose decision on ordinal 0 is private.
    pub(super) fn private_decision(&self) -> FakeDecision {
        let decision = decide_and_build(self, 0);
        assert!(matches!(decision, Decision::Private(_)), "expected a private decision");
        decision
    }

    pub(super) fn calls(&self) -> (usize, usize) {
        (self.synchronizes.load(Ordering::SeqCst), self.trims.load(Ordering::SeqCst))
    }
}

pub(super) fn orin_64() -> DeviceFacts {
    DeviceFacts {
        integrated: true,
        pools_supported: true,
        total_mem: ORIN_64_TOTAL,
        compute_capability: (8, 7),
        tegra: true,
    }
}

fn driver_error(code: u32) -> DriverFailure {
    DriverFailure { code: Some(code), message: format!("driver error {code}") }
}

impl PoolDriver for Fake {
    type Pool = FakePool;
    type Context = FakeContext;
    type Device = FakeContext;
    type DefaultError = String;

    fn raw_settings(&self) -> RawPoolSettings {
        self.settings.clone()
    }

    fn module_load_init(&self) -> ModuleLoadInit {
        self.init
    }

    fn driver_initialized(&self) -> bool {
        self.driver_initialized
    }

    fn with_primary_retained<R>(
        &self,
        _ordinal: usize,
        body: impl FnOnce() -> R,
    ) -> Result<R, DriverFailure> {
        Ok(body())
    }

    fn device_facts(&self, _ordinal: usize) -> Result<DeviceFacts, DriverFailure> {
        Ok(orin_64())
    }

    fn create_pool(
        &self,
        _ordinal: usize,
        _max_size_bytes: u64,
        _threshold: ReleaseThreshold,
    ) -> Result<Self::Pool, DriverFailure> {
        Ok(FakePool)
    }

    fn probe_pool(&self, _pool: &Self::Pool) -> Result<(), DriverFailure> {
        Ok(())
    }

    fn new_private_context(
        &self,
        _ordinal: usize,
        _pool: &Self::Pool,
    ) -> Result<Self::Context, DriverFailure> {
        Ok(FakeContext(FIRST_CONTEXT_ID))
    }

    fn context_id(&self, context: &Self::Context) -> Result<u64, DriverFailure> {
        Ok(context.0)
    }

    fn wrap_context(&self, context: Self::Context) -> Result<Self::Device, DriverFailure> {
        Ok(context)
    }

    fn default_device(&self, ordinal: usize) -> Result<Self::Device, Self::DefaultError> {
        Ok(FakeContext(ordinal as u64))
    }

    fn primary_state(&self, _ordinal: usize) -> Result<CudaPrimaryContextState, DriverFailure> {
        Ok(CudaPrimaryContextState { active: self.primary_active.load(Ordering::SeqCst), flags: 0 })
    }

    fn retained_primary_context_id(&self, _ordinal: usize) -> Result<u64, DriverFailure> {
        (*self.current_id.lock().unwrap()).map_err(driver_error)
    }

    fn pool_counters(&self, _pool: &Self::Pool) -> Result<PoolCounters, DriverFailure> {
        Ok(PoolCounters::default())
    }

    fn live_private_contexts(&self, _pool: &Self::Pool) -> usize {
        1
    }

    fn cuda_libraries(&self) -> Vec<String> {
        Vec::new()
    }

    fn snapshot_once(&self) -> &Once {
        &self.snapshot
    }

    fn emit_snapshot(&self, _line: &str) {}
}

impl PoolRelease for Fake {
    fn synchronize(&self, _context: &Self::Context) -> Result<(), DriverFailure> {
        self.synchronizes.fetch_add(1, Ordering::SeqCst);
        if self.fail_synchronize.load(Ordering::SeqCst) {
            return Err(driver_error(CUDA_ERROR_INVALID_CONTEXT));
        }
        Ok(())
    }

    fn trim(&self, _pool: &Self::Pool) -> Result<(), DriverFailure> {
        self.trims.fetch_add(1, Ordering::SeqCst);
        if self.panic_in_trim.load(Ordering::SeqCst) {
            panic_any("injected panic in trim");
        }
        if self.fail_trim.load(Ordering::SeqCst) {
            return Err(driver_error(CUDA_ERROR_OUT_OF_MEMORY));
        }
        Ok(())
    }
}

/// A driver that overrides nothing optional, so a test can read
/// `PoolDriver`'s default bodies. No method is ever called.
pub(super) struct DefaultsOnly;

impl PoolDriver for DefaultsOnly {
    type Pool = ();
    type Context = ();
    type Device = ();
    type DefaultError = ();

    fn raw_settings(&self) -> RawPoolSettings {
        unreachable!()
    }
    fn module_load_init(&self) -> ModuleLoadInit {
        unreachable!()
    }
    fn with_primary_retained<R>(
        &self,
        _ordinal: usize,
        _body: impl FnOnce() -> R,
    ) -> Result<R, DriverFailure> {
        unreachable!()
    }
    fn device_facts(&self, _ordinal: usize) -> Result<DeviceFacts, DriverFailure> {
        unreachable!()
    }
    fn create_pool(
        &self,
        _ordinal: usize,
        _max_size_bytes: u64,
        _threshold: ReleaseThreshold,
    ) -> Result<(), DriverFailure> {
        unreachable!()
    }
    fn probe_pool(&self, _pool: &()) -> Result<(), DriverFailure> {
        unreachable!()
    }
    fn new_private_context(&self, _ordinal: usize, _pool: &()) -> Result<(), DriverFailure> {
        unreachable!()
    }
    fn context_id(&self, _context: &()) -> Result<u64, DriverFailure> {
        unreachable!()
    }
    fn wrap_context(&self, _context: ()) -> Result<(), DriverFailure> {
        unreachable!()
    }
    fn default_device(&self, _ordinal: usize) -> Result<(), ()> {
        unreachable!()
    }
    fn primary_state(&self, _ordinal: usize) -> Result<CudaPrimaryContextState, DriverFailure> {
        unreachable!()
    }
    fn retained_primary_context_id(&self, _ordinal: usize) -> Result<u64, DriverFailure> {
        unreachable!()
    }
    fn pool_counters(&self, _pool: &()) -> Result<PoolCounters, DriverFailure> {
        unreachable!()
    }
    fn live_private_contexts(&self, _pool: &()) -> usize {
        unreachable!()
    }
    fn cuda_libraries(&self) -> Vec<String> {
        unreachable!()
    }
    fn snapshot_once(&self) -> &Once {
        static ONCE: OnceLock<Once> = OnceLock::new();
        ONCE.get_or_init(Once::new)
    }
    fn emit_snapshot(&self, _line: &str) {}
}
