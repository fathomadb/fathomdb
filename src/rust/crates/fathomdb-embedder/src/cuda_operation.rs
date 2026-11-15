//! Scope of model/device work and its partial ownership.

use crate::CudaPoolFailure;

pub(crate) struct CudaOperation {
    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    lifetime: Option<std::sync::Arc<candle_core::cuda::cudarc::driver::PrimaryContextLifetime>>,
    operation: &'static str,
}

impl CudaOperation {
    pub(crate) fn on(device: &candle_core::Device, operation: &'static str) -> Self {
        #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
        let lifetime = device
            .as_cuda_device()
            .ok()
            .and_then(|device| device.cuda_stream().context().primary_lifetime().cloned());
        #[cfg(not(any(feature = "embed-cuda", feature = "rerank-cuda")))]
        let _ = device;
        Self {
            #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
            lifetime,
            operation,
        }
    }

    pub(crate) fn run<T, E>(
        self,
        _map_failure: impl Fn(CudaPoolFailure) -> E,
        body: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let _ = self.operation;
        #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
        let _ = self.lifetime;
        run_with_admission(|| Ok::<_, E>(()), || None, body)
    }
}

fn run_with_admission<T, E, L>(
    _admit: impl FnOnce() -> Result<L, E>,
    _cached_loss: impl Fn() -> Option<E>,
    body: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    body()
}

#[cfg(test)]
mod tests {
    use super::run_with_admission;
    use std::cell::{Cell, RefCell};

    #[test]
    fn lost_operation_refuses_before_native_work() {
        let calls = Cell::new(0);
        let actual = run_with_admission(
            || Err::<(), _>("lost"),
            || Some("lost"),
            || {
                calls.set(calls.get() + 1);
                Ok(7)
            },
        );
        assert_eq!(actual, Err("lost"));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn terminal_loss_defeats_prepared_success_and_drops_under_lease() {
        struct Owner<'a>(&'a RefCell<Vec<&'static str>>, &'static str);
        impl Drop for Owner<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().push(self.1);
            }
        }
        let drops = RefCell::new(Vec::new());
        let actual = run_with_admission(
            || Ok(Owner(&drops, "lease")),
            || Some("loss discovered during cleanup"),
            || {
                let _partial = Owner(&drops, "partial");
                Ok(Owner(&drops, "candidate"))
            },
        );
        assert!(matches!(actual, Err("loss discovered during cleanup")));
        assert_eq!(*drops.borrow(), vec!["partial", "candidate", "lease"]);
    }

    #[test]
    fn partial_load_unwind_drops_owned_parameter_before_admission() {
        struct Owner<'a>(&'a RefCell<Vec<&'static str>>, &'static str);
        impl Drop for Owner<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().push(self.1);
            }
        }
        let drops = RefCell::new(Vec::new());
        let device = Owner(&drops, "owned device");
        let drops_ref = &drops;
        let actual = run_with_admission(
            || Ok(Owner(&drops, "lease")),
            || None,
            move || {
                let _device = device;
                let _partial = Owner(drops_ref, "partial model");
                Err::<(), _>("load failure")
            },
        );
        assert_eq!(actual, Err("load failure"));
        assert_eq!(*drops.borrow(), vec!["partial model", "owned device", "lease"]);
    }
}
