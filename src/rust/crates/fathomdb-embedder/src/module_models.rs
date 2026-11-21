use std::sync::Mutex;

#[derive(Debug, Eq, PartialEq)]
struct Busy;
#[derive(Debug)]
enum AccessError<E> {
    Busy,
    Load(E),
}
struct Gate;
impl Gate {
    const fn new() -> Self {
        Self
    }
    fn release(&self, clear: impl FnOnce()) -> Result<(), Busy> {
        clear();
        Ok(())
    }
}
struct Cell<T: 'static> {
    value: Mutex<Option<&'static T>>,
}
impl<T: 'static> Cell<T> {
    const fn new() -> Self {
        Self { value: Mutex::new(None) }
    }
    fn with<R, E>(
        &self,
        _gate: &Gate,
        load: impl FnOnce() -> Result<T, E>,
        use_model: impl FnOnce(&T) -> R,
    ) -> Result<R, AccessError<E>> {
        let cached = *self.value.lock().unwrap();
        let model = match cached {
            Some(model) => model,
            None => {
                let loaded = Box::leak(Box::new(load().map_err(AccessError::Load)?));
                *self.value.lock().unwrap().get_or_insert(loaded)
            }
        };
        Ok(use_model(model))
    }
    fn clear(&self) {
        self.value.lock().unwrap().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
