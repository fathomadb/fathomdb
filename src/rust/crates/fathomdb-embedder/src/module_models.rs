#[cfg(any(test, feature = "default-reranker"))]
use crate::RerankerDevicePolicyError;
use std::sync::Mutex;
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
use std::sync::{Arc, Condvar};

#[derive(Debug, Eq, PartialEq)]
struct Busy;
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
#[derive(Debug)]
enum AccessError<E> {
    Busy,
    Load(E),
}
struct Gate {
    state: Mutex<(usize, bool)>,
}
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
struct Lease<'a>(&'a Gate);
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).0 -= 1;
    }
}
struct Release<'a>(&'a Gate);
impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.0.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).1 = false;
    }
}
impl Gate {
    const fn new() -> Self {
        Self { state: Mutex::new((0, false)) }
    }
    #[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
    fn enter(&self) -> Result<Lease<'_>, Busy> {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.1 {
            return Err(Busy);
        }
        state.0 += 1;
        Ok(Lease(self))
    }
    fn release(&self, clear: impl FnOnce()) -> Result<(), Busy> {
        {
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.0 != 0 || state.1 {
                return Err(Busy);
            }
            state.1 = true;
        }
        let _release = Release(self);
        clear();
        Ok(())
    }
}
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
enum State<T> {
    Empty,
    Loading,
    Ready(Arc<T>),
}
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
struct Cell<T> {
    value: Mutex<State<T>>,
    changed: Condvar,
}
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
struct Loading<'a, T>(&'a Cell<T>, bool);
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
impl<T> Drop for Loading<'_, T> {
    fn drop(&mut self) {
        if self.1 {
            *self.0.value.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = State::Empty;
            self.0.changed.notify_all();
        }
    }
}
#[cfg(any(test, feature = "default-embedder", feature = "default-reranker"))]
impl<T> Cell<T> {
    const fn new() -> Self {
        Self { value: Mutex::new(State::Empty), changed: Condvar::new() }
    }
    fn with<R, E>(
        &self,
        gate: &Gate,
        load: impl FnOnce() -> Result<T, E>,
        use_model: impl FnOnce(&T) -> R,
    ) -> Result<R, AccessError<E>> {
        let _lease = gate.enter().map_err(|_| AccessError::Busy)?;
        let model = {
            let mut state = self.value.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                match &*state {
                    State::Ready(model) => break model.clone(),
                    State::Loading => {
                        state = self
                            .changed
                            .wait(state)
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                    }
                    State::Empty => {
                        *state = State::Loading;
                        drop(state);
                        let mut loading = Loading(self, true);
                        let model = Arc::new(load().map_err(AccessError::Load)?);
                        *self.value.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                            State::Ready(model.clone());
                        loading.1 = false;
                        self.changed.notify_all();
                        break model;
                    }
                }
            }
        };
        Ok(use_model(&model))
    }
    fn take(&self) -> State<T> {
        std::mem::replace(
            &mut *self.value.lock().unwrap_or_else(std::sync::PoisonError::into_inner),
            State::Empty,
        )
    }
    #[cfg(test)]
    fn clear(&self) {
        drop(self.take());
    }
}

#[cfg(any(test, all(feature = "default-embedder", feature = "default-reranker")))]
fn clear_both<A, B>(first: &Cell<A>, second: &Cell<B>) {
    let first = first.take();
    let second = second.take();
    drop((first, second));
}

static GATE: Gate = Gate::new();

/// Release the cached module-level CLS embedder and reranker, synchronously.
///
/// Returns `ModelBusy` without clearing either cache if module model work is
/// admitted or another release is dropping models. Engine-owned providers and
/// the terminal CUDA generation-loss record are unaffected. Empty/repeated
/// release succeeds without loading a model or initializing CUDA.
pub fn release_models() -> Result<(), crate::RerankerDevicePolicyError> {
    GATE.release(|| {
        #[cfg(all(feature = "default-embedder", feature = "default-reranker"))]
        clear_both(&CLS, &RERANKER);
        #[cfg(all(feature = "default-embedder", not(feature = "default-reranker")))]
        drop(CLS.take());
        #[cfg(all(feature = "default-reranker", not(feature = "default-embedder")))]
        drop(RERANKER.take());
    })
    .map_err(|_| crate::RerankerDevicePolicyError::ModelBusy)
}

/// An owned admission lease covering standalone reranker native preflight.
/// Dropping it permits explicit release after all nested cache leases end.
#[cfg(feature = "default-reranker")]
pub struct ModuleModelLease {
    _lease: Lease<'static>,
}

/// Admit standalone reranker preflight before it can create native resources.
/// Returns `ModelBusy` while explicit teardown is active.
#[cfg(feature = "default-reranker")]
pub fn admit_reranker() -> Result<ModuleModelLease, crate::RerankerDevicePolicyError> {
    Ok(ModuleModelLease {
        _lease: GATE.enter().map_err(|_| crate::RerankerDevicePolicyError::ModelBusy)?,
    })
}

/// A cached CLS operation failed before or during inference.
#[cfg(feature = "default-embedder")]
#[derive(Debug)]
pub enum ModuleEmbeddingError {
    /// A serialized explicit release is dropping the previous models.
    Busy,
    /// Loading failed; this failure is not memoized.
    Load(crate::loader::EmbedderLoadError),
    /// The loaded model rejected the inference operation.
    Forward(fathomdb_embedder_api::EmbedderError),
}
#[cfg(feature = "default-embedder")]
static CLS: Cell<crate::CandleBgeEmbedder> = Cell::new();

/// Run CLS-pooled embedding under a module model lease. No model reference escapes.
#[cfg(feature = "default-embedder")]
pub fn embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>, ModuleEmbeddingError> {
    use fathomdb_embedder_api::Embedder;
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    CLS.with(
        &GATE,
        || crate::CandleBgeEmbedder::new().map(|model| model.with_pooling(crate::Pooling::Cls)),
        |model| model.embed_batch(texts),
    )
    .map_err(|error| match error {
        AccessError::Busy => ModuleEmbeddingError::Busy,
        AccessError::Load(error) => ModuleEmbeddingError::Load(error),
    })?
    .map_err(ModuleEmbeddingError::Forward)
}

/// What the process-wide reranker singleton remembers.
#[cfg(any(feature = "default-reranker", test))]
enum SingletonState<T: 'static> {
    Loaded(T),
    /// No weights and no network: memoized so a query does not retry the load.
    Unavailable,
    DevicePolicy(RerankerDevicePolicyError),
}

/// A failed load, as the singleton treats it.
#[cfg(any(feature = "default-reranker", test))]
#[derive(Debug, PartialEq)]
enum LoadFailure {
    /// Memoized and returned on every call.
    DevicePolicy(RerankerDevicePolicyError),
    /// A CUDA pool kind: returned, not memoized; the next call loads again.
    Retry(RerankerDevicePolicyError),
    /// Memoized as "no reranker"; callers fall back to RRF order.
    Unavailable,
}

#[cfg(any(feature = "default-reranker", test))]
fn get_or_load<T, R>(
    cell: &Cell<SingletonState<T>>,
    gate: &Gate,
    load: impl FnOnce() -> Result<T, LoadFailure>,
    use_model: impl FnOnce(&T) -> Result<R, crate::RerankerDevicePolicyError>,
) -> Result<Option<R>, crate::RerankerDevicePolicyError> {
    cell.with(
        gate,
        || {
            Ok(match load() {
                Ok(model) => SingletonState::Loaded(model),
                Err(LoadFailure::DevicePolicy(error)) => SingletonState::DevicePolicy(error),
                Err(LoadFailure::Unavailable) => SingletonState::Unavailable,
                Err(LoadFailure::Retry(error)) => return Err(error),
            })
        },
        |state| match state {
            SingletonState::Loaded(model) => use_model(model).map(Some),
            SingletonState::Unavailable => Ok(None),
            SingletonState::DevicePolicy(error) => Err(error.clone()),
        },
    )
    .map_err(|error| match error {
        AccessError::Busy => crate::RerankerDevicePolicyError::ModelBusy,
        AccessError::Load(error) => error,
    })?
}

#[cfg(feature = "default-reranker")]
fn classify_reranker_load_error(error: crate::RerankerLoadError) -> LoadFailure {
    use crate::RerankerLoadError;
    match error {
        // A private-pool failure in the resolution probe: returned, and the
        // next call resolves again.
        RerankerLoadError::DevicePolicy(
            error @ (RerankerDevicePolicyError::CudaPoolExhausted { .. }
            | RerankerDevicePolicyError::CudaContextLost { .. }
            | RerankerDevicePolicyError::CudaPrivateBuildRefused { .. }),
        ) => LoadFailure::Retry(error),
        RerankerLoadError::DevicePolicy(error) => LoadFailure::DevicePolicy(error),
        RerankerLoadError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
            LoadFailure::Retry(RerankerDevicePolicyError::CudaPoolExhausted {
                ordinal,
                max_size_bytes,
                message,
            })
        }
        RerankerLoadError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        } => LoadFailure::Retry(RerankerDevicePolicyError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        }),
        RerankerLoadError::CudaPrivateBuildRefused { ordinal, message } => {
            LoadFailure::Retry(RerankerDevicePolicyError::CudaPrivateBuildRefused {
                ordinal,
                message,
            })
        }
        _ => LoadFailure::Unavailable,
    }
}

#[cfg(feature = "default-reranker")]
static RERANKER: Cell<SingletonState<crate::CandleTinyBertReranker>> = Cell::new();

/// Use the cached reranker under a model lease; no model reference escapes.
/// Unavailable weights and policy refusals are memoized until explicit release.
/// CUDA failures are returned without memoization; generation loss remains
/// terminal in the separate CUDA lifetime registry.
#[cfg(feature = "default-reranker")]
pub fn with_reranker<R>(
    use_model: impl FnOnce(
        &crate::CandleTinyBertReranker,
    ) -> Result<R, crate::RerankerDevicePolicyError>,
) -> Result<Option<R>, crate::RerankerDevicePolicyError> {
    get_or_load(
        &RERANKER,
        &GATE,
        || crate::CandleTinyBertReranker::try_load().map_err(classify_reranker_load_error),
        use_model,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RerankerDeviceResolutionError, RerankerDeviceResolutionReason};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    };
    use std::time::Duration;
    struct Probe(Arc<AtomicUsize>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn exhausted() -> RerankerDevicePolicyError {
        RerankerDevicePolicyError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "rerank forward: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
        }
    }

    #[cfg(feature = "default-reranker")]
    fn context_lost() -> RerankerDevicePolicyError {
        RerankerDevicePolicyError::CudaContextLost {
            recorded_context_id: 1,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "rerank forward".to_owned(),
        }
    }

    fn forced() -> RerankerDevicePolicyError {
        RerankerDevicePolicyError::Resolution(
            RerankerDeviceResolutionError::ForcedCudaUnavailable {
                ordinal: 0,
                reason: RerankerDeviceResolutionReason::CudaProbeFailed,
            },
        )
    }

    #[test]
    fn explicit_release_drops_both_cached_models_before_return() {
        let gate = Gate::new();
        let cls = Cell::new();
        let reranker = Cell::new();
        let drops = Arc::new(AtomicUsize::new(0));
        for cell in [&cls, &reranker] {
            assert_eq!(cell.with(&gate, || Ok::<_, ()>(Probe(drops.clone())), |_| 7).unwrap(), 7);
        }
        gate.release(|| {
            cls.clear();
            reranker.clear();
        })
        .unwrap();
        assert_eq!(
            drops.load(Ordering::SeqCst),
            2,
            "explicit release must destroy both cached models while the process is alive"
        );
        gate.release(|| {
            cls.clear();
            reranker.clear();
        })
        .unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        cls.with(&gate, || Ok::<_, ()>(Probe(drops.clone())), |_| 9).unwrap();
        gate.release(|| cls.clear()).unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 3, "healthy reload is separately owned");
    }
    #[test]
    fn active_inference_refuses_release_without_clearing_either_model() {
        for active_index in 0..2 {
            let gate = Arc::new(Gate::new());
            let cells = [Arc::new(Cell::new()), Arc::new(Cell::new())];
            let drops = Arc::new(AtomicUsize::new(0));
            for cell in &cells {
                cell.with(&gate, || Ok::<_, ()>(Probe(drops.clone())), |_| ()).unwrap();
            }
            let (entered, reached) = mpsc::channel();
            let (finish, released) = mpsc::channel();
            let active = cells[active_index].clone();
            let worker_gate = gate.clone();
            let worker = std::thread::spawn(move || {
                active.with(
                    &worker_gate,
                    || -> Result<Probe, ()> { panic!("cached model must be reused") },
                    |_| {
                        entered.send(()).unwrap();
                        released.recv_timeout(Duration::from_secs(3)).unwrap();
                        17
                    },
                )
            });
            reached.recv_timeout(Duration::from_secs(3)).unwrap();
            let result = gate.release(|| {
                cells[0].clear();
                cells[1].clear();
            });
            finish.send(()).unwrap();
            assert_eq!(worker.join().unwrap().unwrap(), 17, "admitted work completes unchanged");
            assert_eq!(result, Err(Busy), "the overlapping release caller is refused");
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            for cell in &cells {
                cell.with(
                    &gate,
                    || -> Result<Probe, ()> { panic!("refused release cannot clear either cache") },
                    |_| (),
                )
                .unwrap();
            }
            gate.release(|| {
                cells[0].clear();
                cells[1].clear();
            })
            .unwrap();
            assert_eq!(drops.load(Ordering::SeqCst), 2);
        }
    }
    #[test]
    fn a_pool_kind_load_failure_is_returned_and_retried_next_call() {
        let cell = Cell::new();
        let gate = Gate::new();
        let loads = AtomicUsize::new(0);
        let first = get_or_load(
            &cell,
            &gate,
            || {
                loads.fetch_add(1, Ordering::SeqCst);
                Err(LoadFailure::Retry(exhausted()))
            },
            |value| Ok(*value),
        );
        assert_eq!(first, Err(exhausted()));
        let second = get_or_load(
            &cell,
            &gate,
            || {
                loads.fetch_add(1, Ordering::SeqCst);
                Ok(7)
            },
            |value| Ok(*value),
        );
        assert_eq!(second, Ok(Some(7)));
        let third = get_or_load(
            &cell,
            &gate,
            || -> Result<u32, LoadFailure> { panic!("a loaded model is never reloaded") },
            |value| Ok(*value),
        );
        assert_eq!(third, Ok(Some(7)));
        assert_eq!(loads.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_genuinely_unavailable_model_is_memoized() {
        let cell = Cell::new();
        let gate = Gate::new();
        assert_eq!(
            get_or_load(&cell, &gate, || Err(LoadFailure::Unavailable), |value: &u32| Ok(*value)),
            Ok(None)
        );
        let again = get_or_load(
            &cell,
            &gate,
            || -> Result<u32, LoadFailure> { panic!("an unavailable model is not reloaded") },
            |value| Ok(*value),
        );
        assert_eq!(again, Ok(None));
    }

    #[test]
    fn a_device_policy_refusal_is_memoized() {
        let cell = Cell::new();
        let gate = Gate::new();
        assert_eq!(
            get_or_load(
                &cell,
                &gate,
                || Err(LoadFailure::DevicePolicy(forced())),
                |value: &u32| Ok(*value)
            ),
            Err(forced())
        );
        let again = get_or_load(
            &cell,
            &gate,
            || -> Result<u32, LoadFailure> { panic!("a device-policy refusal is not reloaded") },
            |value| Ok(*value),
        );
        assert_eq!(again, Err(forced()));
    }

    #[cfg(feature = "default-reranker")]
    #[test]
    fn load_errors_classify_into_the_three_singleton_outcomes() {
        use crate::RerankerLoadError;

        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::CudaPoolExhausted {
                ordinal: 0,
                max_size_bytes: 1,
                message: "m".to_owned(),
            }),
            LoadFailure::Retry(RerankerDevicePolicyError::CudaPoolExhausted {
                ordinal: 0,
                max_size_bytes: 1,
                message: "m".to_owned(),
            })
        );
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::CudaContextLost {
                recorded_context_id: 1,
                current_context_id: Some(2),
                driver_error: "d".to_owned(),
                operation: "o".to_owned(),
            }),
            LoadFailure::Retry(RerankerDevicePolicyError::CudaContextLost {
                recorded_context_id: 1,
                current_context_id: Some(2),
                driver_error: "d".to_owned(),
                operation: "o".to_owned(),
            })
        );
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::CudaPrivateBuildRefused {
                ordinal: 0,
                message: "m".to_owned(),
            }),
            LoadFailure::Retry(RerankerDevicePolicyError::CudaPrivateBuildRefused {
                ordinal: 0,
                message: "m".to_owned(),
            })
        );
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::DevicePolicy(forced())),
            LoadFailure::DevicePolicy(forced())
        );
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::TokenizerLoad("t".to_owned())),
            LoadFailure::Unavailable
        );
    }

    /// A pool failure in the open-time reranker probe reaches the singleton as a
    /// device-policy error; it is returned and retried, never memoized.
    #[cfg(feature = "default-reranker")]
    #[test]
    fn a_pool_kind_device_policy_failure_is_retried_not_memoized() {
        use crate::RerankerLoadError;

        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::DevicePolicy(exhausted())),
            LoadFailure::Retry(exhausted())
        );
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::DevicePolicy(context_lost())),
            LoadFailure::Retry(context_lost())
        );
        let refused = RerankerDevicePolicyError::CudaPrivateBuildRefused {
            ordinal: 0,
            message: "m".to_owned(),
        };
        assert_eq!(
            classify_reranker_load_error(RerankerLoadError::DevicePolicy(refused.clone())),
            LoadFailure::Retry(refused)
        );
    }
    #[test]
    fn cls_cell_caches_only_on_success() {
        let cell = Cell::new();
        let gate = Gate::new();
        assert!(matches!(
            cell.with(&gate, || Err::<u32, _>("transient"), |value| *value),
            Err(AccessError::Load("transient"))
        ));
        assert_eq!(cell.with(&gate, || Ok::<_, &str>(7), |value| *value).unwrap(), 7);
        assert_eq!(
            cell.with(
                &gate,
                || -> Result<u32, &str> { panic!("must not re-init") },
                |value| *value
            )
            .unwrap(),
            7
        );
    }
    #[test]
    fn loading_is_admitted_and_release_does_not_interrupt_it() {
        let gate = Arc::new(Gate::new());
        let cell = Arc::new(Cell::new());
        let (entered, reached) = mpsc::channel();
        let (finish, released) = mpsc::channel();
        let worker_gate = gate.clone();
        let worker_cell = cell.clone();
        let worker = std::thread::spawn(move || {
            worker_cell.with(
                &worker_gate,
                || {
                    entered.send(()).unwrap();
                    released.recv_timeout(Duration::from_secs(3)).unwrap();
                    Ok::<_, ()>(23)
                },
                |value| *value,
            )
        });
        reached.recv_timeout(Duration::from_secs(3)).unwrap();
        let result = gate.release(|| cell.clear());
        finish.send(()).unwrap();
        assert_eq!(worker.join().unwrap().unwrap(), 23);
        assert_eq!(result, Err(Busy));
        gate.release(|| cell.clear()).unwrap();
    }

    #[test]
    fn teardown_blocks_reload_and_release_until_native_drop_finishes() {
        struct BlockingDrop {
            entered: mpsc::Sender<()>,
            finish: Mutex<mpsc::Receiver<()>>,
        }
        impl Drop for BlockingDrop {
            fn drop(&mut self) {
                self.entered.send(()).unwrap();
                self.finish.lock().unwrap().recv_timeout(Duration::from_secs(3)).unwrap();
            }
        }
        let gate = Arc::new(Gate::new());
        let cell = Arc::new(Cell::new());
        let (entered, reached) = mpsc::channel();
        let (finish, released) = mpsc::channel();
        cell.with(
            &gate,
            || Ok::<_, ()>(BlockingDrop { entered, finish: Mutex::new(released) }),
            |_| (),
        )
        .unwrap();
        let worker_gate = gate.clone();
        let worker_cell = cell.clone();
        let worker = std::thread::spawn(move || worker_gate.release(|| worker_cell.clear()));
        reached.recv_timeout(Duration::from_secs(3)).unwrap();
        let attempted_loads = AtomicUsize::new(0);
        let inference = cell.with(
            &gate,
            || {
                attempted_loads.fetch_add(1, Ordering::SeqCst);
                Err::<BlockingDrop, ()>(())
            },
            |_| (),
        );
        let release = gate.release(|| cell.clear());
        finish.send(()).unwrap();
        worker.join().unwrap().unwrap();
        assert!(matches!(inference, Err(AccessError::Busy)));
        assert_eq!(release, Err(Busy));
        assert_eq!(attempted_loads.load(Ordering::SeqCst), 0);
        gate.release(|| cell.clear()).unwrap();
    }

    #[test]
    fn panicking_load_and_teardown_restore_admission() {
        let gate = Gate::new();
        let cell = Cell::<u32>::new();
        assert!(std::panic::catch_unwind(|| cell.with(
            &gate,
            || -> Result<u32, ()> { panic!("loader") },
            |value| *value
        ))
        .is_err());
        assert_eq!(cell.with(&gate, || Ok::<_, ()>(31), |value| *value).unwrap(), 31);
        assert!(std::panic::catch_unwind(|| gate.release(|| {
            cell.clear();
            panic!("destructor");
        }))
        .is_err());
        assert_eq!(cell.with(&gate, || Ok::<_, ()>(37), |value| *value).unwrap(), 37);
        gate.release(|| cell.clear()).unwrap();
    }
    #[test]
    fn panicking_first_destructor_still_removes_both_cache_owners() {
        struct PanicDrop;
        impl Drop for PanicDrop {
            fn drop(&mut self) {
                panic!("native destructor");
            }
        }
        let gate = Gate::new();
        let first = Cell::new();
        let second = Cell::new();
        let drops = Arc::new(AtomicUsize::new(0));
        first.with(&gate, || Ok::<_, ()>(PanicDrop), |_| ()).unwrap();
        second.with(&gate, || Ok::<_, ()>(Probe(drops.clone())), |_| ()).unwrap();
        assert!(std::panic::catch_unwind(|| gate.release(|| clear_both(&first, &second))).is_err());
        assert_eq!(
            drops.load(Ordering::SeqCst),
            1,
            "both owners must be detached before either destructor runs"
        );
        gate.release(|| clear_both(&first, &second)).unwrap();
    }
}
