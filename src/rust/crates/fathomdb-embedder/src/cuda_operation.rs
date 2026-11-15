//! Scope of model/device work and its partial ownership.

use crate::CudaPoolFailure;

#[cfg_attr(not(any(feature = "embed-cuda", feature = "rerank-cuda")), allow(dead_code))]
pub(crate) enum CudaOperationError {
    Loss(CudaPoolFailure),
    Driver(candle_core::Error),
}

impl CudaOperationError {
    #[cfg(feature = "default-embedder")]
    pub(crate) fn into_embedder_error(self) -> fathomdb_embedder_api::EmbedderError {
        match self {
            Self::Loss(failure) => failure.into_embedder_error(),
            Self::Driver(error) => {
                fathomdb_embedder_api::EmbedderError::Failed { message: error.to_string() }
            }
        }
    }
    #[cfg(feature = "default-embedder")]
    pub(crate) fn into_embedder_load_error(self) -> crate::loader::EmbedderLoadError {
        match self {
            Self::Loss(failure) => failure.into_embedder_load_error(),
            Self::Driver(error) => crate::loader::EmbedderLoadError::DeviceInitialization {
                message: error.to_string(),
            },
        }
    }
    #[cfg(feature = "default-reranker")]
    pub(crate) fn into_reranker_load_error(self) -> crate::RerankerLoadError {
        match self {
            Self::Loss(failure) => failure.into_reranker_load_error(),
            Self::Driver(error) => {
                crate::RerankerLoadError::DeviceInitialization { message: error.to_string() }
            }
        }
    }
    #[cfg(any(feature = "default-reranker", feature = "embed-cuda", feature = "rerank-cuda"))]
    pub(crate) fn into_candle_error(self) -> candle_core::Error {
        match self {
            Self::Loss(failure) => candle_core::Error::Cuda(Box::new(failure)),
            Self::Driver(error) => error,
        }
    }
}

pub(crate) struct CudaOperation {
    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    lifetime: Option<std::sync::Arc<candle_core::cuda::cudarc::driver::PrimaryContextLifetime>>,
    ordinal: Option<usize>,
    operation: &'static str,
}

impl CudaOperation {
    pub(crate) fn on(device: &candle_core::Device, operation: &'static str) -> Self {
        #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
        let lifetime = device
            .as_cuda_device()
            .ok()
            .and_then(|device| device.cuda_stream().context().primary_lifetime().cloned());
        let ordinal = match device.location() {
            candle_core::DeviceLocation::Cuda { gpu_id } => Some(gpu_id),
            _ => None,
        };
        Self {
            #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
            lifetime,
            ordinal,
            operation,
        }
    }

    #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
    pub(crate) fn tracked(
        lifetime: std::sync::Arc<candle_core::cuda::cudarc::driver::PrimaryContextLifetime>,
        ordinal: usize,
        operation: &'static str,
    ) -> Self {
        Self { lifetime: Some(lifetime), ordinal: Some(ordinal), operation }
    }

    pub(crate) fn run<T, E>(
        self,
        map_failure: impl Fn(CudaOperationError) -> E,
        body: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        #[cfg(any(feature = "embed-cuda", feature = "rerank-cuda"))]
        {
            let cached = || {
                self.lifetime.as_ref().and_then(|token| token.loss()).map(|loss| {
                    crate::cuda_pool_policy::publish_native_snapshot(
                        self.ordinal.expect("tracked CUDA ordinal"),
                        loss,
                        self.operation,
                    );
                    CudaOperationError::Loss(crate::cuda_pool_policy::native_loss_failure(
                        loss,
                        self.operation,
                    ))
                })
            };
            run_with_admission(
                || match &self.lifetime {
                    Some(token) => token.admit().map(Some).map_err(|error| {
                        map_failure(cached().unwrap_or_else(|| {
                            CudaOperationError::Driver(candle_core::Error::Cuda(Box::new(
                                candle_core::cuda::CudaError::Cuda(error),
                            )))
                        }))
                    }),
                    None if self.ordinal.is_some() => Err(map_failure(CudaOperationError::Driver(
                        candle_core::Error::Msg("untracked CUDA device refused".into()),
                    ))),
                    None => Ok(None),
                },
                || cached().map(&map_failure),
                body,
            )
        }
        #[cfg(not(any(feature = "embed-cuda", feature = "rerank-cuda")))]
        {
            let _ = (self.ordinal, self.operation, map_failure);
            body()
        }
    }
}

#[cfg(any(test, feature = "embed-cuda", feature = "rerank-cuda"))]
fn run_with_admission<T, E, L>(
    admit: impl FnOnce() -> Result<L, E>,
    cached_loss: impl Fn() -> Option<E>,
    body: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let _lease = admit()?;
    let result = body();
    if let Some(error) = cached_loss() {
        drop(result);
        return Err(error);
    }
    result
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
