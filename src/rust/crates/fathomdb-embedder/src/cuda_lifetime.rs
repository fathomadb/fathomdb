use candle_core::cuda::cudarc::driver::{DriverError, PrimaryContextLifetime, PrimaryContextLoss};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[derive(Default)]
struct SlotState {
    initializing: bool,
    lifetime: Option<Arc<PrimaryContextLifetime>>,
    libraries: Vec<String>,
    uuid: Option<String>,
    first_operation: Option<String>,
}

#[derive(Default)]
struct Slot {
    state: Mutex<SlotState>,
    ready: Condvar,
}

struct Initialization<'a>(&'a Slot);
impl Drop for Initialization<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(|p| p.into_inner()).initializing = false;
        self.0.ready.notify_all();
    }
}

static LIFETIMES: OnceLock<Mutex<BTreeMap<usize, Arc<Slot>>>> = OnceLock::new();

fn slot(ordinal: usize, create: bool) -> Option<Arc<Slot>> {
    let map = if create { LIFETIMES.get_or_init(Mutex::default) } else { LIFETIMES.get()? };
    let mut map = map.lock().unwrap_or_else(|p| p.into_inner());
    if create {
        Some(map.entry(ordinal).or_default().clone())
    } else {
        map.get(&ordinal).cloned()
    }
}

pub(crate) fn existing(ordinal: usize) -> Option<Arc<PrimaryContextLifetime>> {
    let slot = slot(ordinal, false)?;
    let result = slot.state.lock().unwrap_or_else(|p| p.into_inner()).lifetime.clone();
    result
}

pub(crate) fn get(ordinal: usize) -> Result<Arc<PrimaryContextLifetime>, DriverError> {
    let slot = slot(ordinal, true).expect("creating an ordinal slot always returns it");
    let mut state = slot.state.lock().unwrap_or_else(|p| p.into_inner());
    loop {
        if let Some(lifetime) = &state.lifetime {
            return Ok(lifetime.clone());
        }
        if !state.initializing {
            state.initializing = true;
            break;
        }
        state = slot.ready.wait(state).unwrap_or_else(|p| p.into_inner());
    }
    drop(state);
    // No registry/slot lock covers native retain, identity lookup or unwind.
    let _initializing = Initialization(&slot);
    let lifetime = PrimaryContextLifetime::new(ordinal)?;
    slot.state.lock().unwrap_or_else(|p| p.into_inner()).lifetime = Some(lifetime.clone());
    let libraries = std::fs::read_to_string("/proc/self/maps")
        .map(|maps| crate::cuda_pool_policy::cuda_libraries_in_maps(&maps))
        .unwrap_or_default();
    let mut state = slot.state.lock().unwrap_or_else(|p| p.into_inner());
    state.libraries = libraries;
    drop(state);
    Ok(lifetime)
}

pub(crate) fn loss(ordinal: usize) -> Option<PrimaryContextLoss> {
    existing(ordinal)?.loss()
}

pub(crate) fn failure(ordinal: usize, operation: &str) -> Option<crate::CudaPoolFailure> {
    let loss = loss(ordinal)?;
    crate::cuda_pool_policy::publish_native_snapshot(ordinal, loss, operation);
    Some(crate::cuda_pool_policy::native_loss_failure(loss, operation))
}

pub(crate) fn libraries(ordinal: usize) -> Vec<String> {
    let Some(slot) = slot(ordinal, false) else {
        return Vec::new();
    };
    let result = slot.state.lock().unwrap_or_else(|p| p.into_inner()).libraries.clone();
    result
}
#[cfg(any(test, feature = "cuda-pool-observer"))]
pub(crate) fn uuid(ordinal: usize) -> Option<String> {
    let slot = slot(ordinal, false)?;
    let result = slot.state.lock().unwrap_or_else(|p| p.into_inner()).uuid.clone();
    result
}

pub(crate) fn record_operation(ordinal: usize, operation: &str) {
    if let Some(slot) = slot(ordinal, false) {
        slot.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .first_operation
            .get_or_insert_with(|| operation.to_owned());
    }
}
#[cfg(any(test, feature = "cuda-pool-observer"))]
pub(crate) fn first_operation(ordinal: usize) -> Option<String> {
    let slot = slot(ordinal, false)?;
    let result = slot.state.lock().unwrap_or_else(|p| p.into_inner()).first_operation.clone();
    result
}

pub(crate) fn record_uuid(ordinal: usize, uuid: String) {
    if let Some(slot) = slot(ordinal, false) {
        slot.state.lock().unwrap_or_else(|p| p.into_inner()).uuid.get_or_insert(uuid);
    }
}
