use candle_core::cuda::cudarc::driver::{DriverError, PrimaryContextLifetime, PrimaryContextLoss};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

struct SlotState<T = PrimaryContextLifetime> {
    initializing: bool,
    lifetime: Option<Arc<T>>,
    libraries: Vec<String>,
    uuid: Option<String>,
    first_operation: Option<String>,
}

struct Slot<T = PrimaryContextLifetime> {
    state: Mutex<SlotState<T>>,
    ready: Condvar,
}

struct Initialization<'a, T>(&'a Slot<T>);
impl<T> Drop for Initialization<'_, T> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(|p| p.into_inner()).initializing = false;
        self.0.ready.notify_all();
    }
}

impl<T> Default for SlotState<T> {
    fn default() -> Self {
        Self {
            initializing: false,
            lifetime: None,
            libraries: Vec::new(),
            uuid: None,
            first_operation: None,
        }
    }
}
impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self { state: Mutex::new(SlotState::default()), ready: Condvar::new() }
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

fn initialize<T, E>(
    slot: &Slot<T>,
    create: impl FnOnce() -> Result<Arc<T>, E>,
) -> Result<Arc<T>, E> {
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
    let _initializing = Initialization(slot);
    let lifetime = create()?;
    slot.state.lock().unwrap_or_else(|p| p.into_inner()).lifetime = Some(lifetime.clone());
    Ok(lifetime)
}

pub(crate) fn get(ordinal: usize) -> Result<Arc<PrimaryContextLifetime>, DriverError> {
    let slot = slot(ordinal, true).expect("creating an ordinal slot always returns it");
    initialize(&slot, || {
        let lifetime = PrimaryContextLifetime::new(ordinal)?;
        let libraries = std::fs::read_to_string("/proc/self/maps")
            .map(|maps| crate::cuda_pool_policy::cuda_libraries_in_maps(&maps))
            .unwrap_or_default();
        slot.state.lock().unwrap_or_else(|p| p.into_inner()).libraries = libraries;
        Ok(lifetime)
    })
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

#[cfg(test)]
mod tests {
    use super::{initialize, Slot};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    };

    #[test]
    fn concurrent_constructors_publish_one_anchor() {
        let slot = Arc::new(Slot::<usize>::default());
        let ready = Arc::new(Barrier::new(8));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let (slot, ready, calls) = (slot.clone(), ready.clone(), calls.clone());
            workers.push(std::thread::spawn(move || {
                ready.wait();
                initialize(&slot, || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(Arc::new(7))
                })
                .unwrap()
            }));
        }
        let owners = workers.into_iter().map(|w| w.join().unwrap()).collect::<Vec<_>>();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(owners.iter().all(|owner| Arc::ptr_eq(owner, &owners[0])));
    }

    #[test]
    fn failed_and_panicking_anchor_creation_leave_retry_admission() {
        let slot = Slot::<usize>::default();
        assert_eq!(initialize(&slot, || Err::<Arc<usize>, _>("ordinary OOM")), Err("ordinary OOM"));
        assert!(std::panic::catch_unwind(|| initialize(&slot, || -> Result<Arc<usize>, ()> {
            panic!("constructor unwind")
        }))
        .is_err());
        let anchor = initialize(&slot, || Ok::<_, ()>(Arc::new(7))).unwrap();
        assert_eq!(*anchor, 7);
    }

    #[test]
    fn published_anchor_survives_failed_load_and_last_consumer_drop() {
        let slot = Slot::<usize>::default();
        let anchor = initialize(&slot, || Ok::<_, ()>(Arc::new(7))).unwrap();
        let original = Arc::downgrade(&anchor);
        let load_result: Result<(), &str> = Err("model deserialize");
        assert!(load_result.is_err());
        drop(anchor);
        let later = initialize(&slot, || -> Result<Arc<usize>, ()> {
            panic!("published anchor was replaced")
        })
        .unwrap();
        assert!(Arc::ptr_eq(&later, &original.upgrade().expect("registry retains its anchor")));
    }
}
