use crate::gpu_witness::{GpuMemorySample, GpuWitnessError};

pub(crate) const CONTROL_BYTES: u64 = 67_108_864;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WitnessMethod {
    GlobalFree,
    PrivatePoolUsed,
}

pub(crate) fn select_method(
    integrated: bool,
    private: bool,
    pool_present: bool,
) -> Result<WitnessMethod, GpuWitnessError> {
    match (integrated, private, pool_present) {
        (false, false, false) => Ok(WitnessMethod::GlobalFree),
        (true, true, true) => Ok(WitnessMethod::PrivatePoolUsed),
        _ => Err(GpuWitnessError::UnsupportedAllocationMethod {
            device_integrated: integrated,
            allocator_private: private,
            pool_present,
        }),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PrivateSample {
    pub(crate) used: u64,
    pub(crate) global: GpuMemorySample,
}

pub(crate) trait PrivateWitnessProbe {
    fn complete(&mut self) -> Result<(), GpuWitnessError>;
    fn read(&mut self) -> Result<PrivateSample, GpuWitnessError>;
    fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError>;
    fn touch(&mut self, pattern: u8) -> Result<(), GpuWitnessError>;
    fn release(&mut self) -> Result<(), GpuWitnessError>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PrivateObservations {
    pub(crate) before: PrivateSample,
    pub(crate) after: PrivateSample,
    pub(crate) dimension: usize,
    pub(crate) control_before_bytes: u64,
    pub(crate) control_live_bytes: u64,
    pub(crate) control_released_bytes: u64,
}

pub(crate) fn observe_private_model<M>(
    probe: &mut dyn PrivateWitnessProbe,
    load: impl FnOnce() -> Result<M, GpuWitnessError>,
    forward: impl FnOnce(&M) -> Result<usize, GpuWitnessError>,
) -> Result<(M, PrivateObservations), GpuWitnessError> {
    probe.complete()?;
    let before = probe.read()?;
    let model = load()?;
    probe.complete()?;
    let after = probe.read()?;
    let dimension = forward(&model)?;
    if dimension != crate::gpu_witness::WITNESS_VECTOR_DIM {
        return Err(GpuWitnessError::ProbeFailed {
            message: format!("private witness forward produced {dimension} dimensions"),
        });
    }
    probe.complete()?;
    let baseline = probe.read()?;
    let live = (|| {
        probe.allocate(CONTROL_BYTES)?;
        probe.touch(0xa5)?;
        probe.complete()?;
        probe.read()
    })();
    // Always attempt checked cleanup completion, even if release reports an
    // error. Finish cleanup while the model lives and preserve the first error.
    let release = probe.release();
    let completion = probe.complete();
    let released = match (release, completion) {
        (Ok(()), Ok(())) => probe.read(),
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    let live = live?;
    let released = released?;
    Ok((
        model,
        PrivateObservations {
            before,
            after,
            dimension,
            control_before_bytes: baseline.used,
            control_live_bytes: live.used,
            control_released_bytes: released.used,
        },
    ))
}

#[cfg(feature = "embed-cuda")]
mod hardware {
    use super::*;
    use crate::gpu_witness::normalize_cuda_uuid;
    use candle_core::cuda::cudarc::driver::{
        result, sys, AllocMode, CudaContext, CudaMemPool, CudaSlice, CudaStream, DevicePtrMut,
    };
    use std::sync::Arc;

    fn driver(error: result::DriverError) -> GpuWitnessError {
        GpuWitnessError::from_driver_status(error.0 as u32, &error.to_string())
    }

    pub(crate) fn method_for_device(
        device: &candle_core::Device,
    ) -> Result<(WitnessMethod, bool), GpuWitnessError> {
        let cuda = device
            .as_cuda_device()
            .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;
        let stream = cuda.cuda_stream();
        let context = stream.context();
        let integrated = match context
            .attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_INTEGRATED)
            .map_err(driver)?
        {
            0 => false,
            1 => true,
            value => {
                return Err(GpuWitnessError::ProbeFailed {
                    message: format!("invalid actual INTEGRATED attribute {value}"),
                })
            }
        };
        let method = select_method(
            integrated,
            context.alloc_mode() == AllocMode::Private,
            context.mem_pool().is_some(),
        )?;
        Ok((method, integrated))
    }

    pub(crate) struct HeldPrivatePool {
        context: Arc<CudaContext>,
        stream: Arc<CudaStream>,
        pool: Arc<CudaMemPool>,
        control: Option<CudaSlice<u8>>,
        context_id: u64,
        context_uuid: String,
    }

    impl HeldPrivatePool {
        pub(crate) fn new(
            device: &candle_core::Device,
            requested: usize,
            retained_uuid: &str,
        ) -> Result<Self, GpuWitnessError> {
            let (method, integrated) = method_for_device(device)?;
            let stream = device
                .as_cuda_device()
                .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?
                .cuda_stream();
            let context = stream.context().clone();
            if method != WitnessMethod::PrivatePoolUsed {
                return Err(GpuWitnessError::UnsupportedAllocationMethod {
                    device_integrated: integrated,
                    allocator_private: context.alloc_mode() == AllocMode::Private,
                    pool_present: context.mem_pool().is_some(),
                });
            }
            let pool = context.mem_pool().cloned().ok_or_else(|| GpuWitnessError::ProbeFailed {
                message: "actual private pool disappeared".into(),
            })?;
            if context.ordinal() != requested {
                return Err(GpuWitnessError::OrdinalMismatch {
                    requested,
                    retained: context.ordinal(),
                });
            }
            context.bind_to_thread().map_err(driver)?;
            let mut context_id = 0;
            // Read identity from the retained context; never construct or swap one.
            unsafe { sys::cuCtxGetId(context.cu_ctx(), &mut context_id) }
                .result()
                .map_err(driver)?;
            let uuid = context.uuid().map_err(driver)?.bytes.map(|byte| byte.to_ne_bytes()[0]);
            let hex: String = uuid.iter().map(|byte| format!("{byte:02x}")).collect();
            let context_uuid = format!(
                "GPU-{}-{}-{}-{}-{}",
                &hex[..8],
                &hex[8..12],
                &hex[12..16],
                &hex[16..20],
                &hex[20..]
            );
            if normalize_cuda_uuid(&context_uuid) != normalize_cuda_uuid(retained_uuid) {
                return Err(GpuWitnessError::UuidMismatch {
                    probed: context_uuid,
                    retained: retained_uuid.to_owned(),
                });
            }
            Ok(Self { context, stream, pool, control: None, context_id, context_uuid })
        }

        pub(crate) fn context_id(&self) -> u64 {
            self.context_id
        }
        pub(crate) fn context_uuid(&self) -> &str {
            &self.context_uuid
        }
        pub(crate) fn pool_identity(&self) -> String {
            format!("{:p}", self.pool.raw())
        }

        pub(crate) fn read_used(&self) -> Result<u64, GpuWitnessError> {
            self.context.bind_to_thread().map_err(driver)?;
            self.pool.used_current().map_err(driver)
        }

        pub(crate) fn read_reserved(&self) -> Result<u64, GpuWitnessError> {
            self.context.bind_to_thread().map_err(driver)?;
            self.pool.reserved_current().map_err(driver)
        }

        pub(crate) fn read_global(&self) -> Result<GpuMemorySample, GpuWitnessError> {
            let (free, total) = self.context.mem_get_info().map_err(driver)?;
            Ok(GpuMemorySample { free_bytes: free as u64, total_bytes: total as u64 })
        }
    }

    impl PrivateWitnessProbe for HeldPrivatePool {
        fn complete(&mut self) -> Result<(), GpuWitnessError> {
            self.context.synchronize().map_err(driver)
        }
        fn read(&mut self) -> Result<PrivateSample, GpuWitnessError> {
            Ok(PrivateSample { used: self.read_used()?, global: self.read_global()? })
        }
        fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError> {
            if bytes != CONTROL_BYTES || self.control.is_some() {
                return Err(GpuWitnessError::ProbeFailed {
                    message: "private control must be one exact 64 MiB allocation".into(),
                });
            }
            let bytes = usize::try_from(bytes)
                .map_err(|error| GpuWitnessError::ProbeFailed { message: error.to_string() })?;
            // The held stream allocates through its context's actual private pool.
            self.control = Some(unsafe { self.stream.alloc::<u8>(bytes) }.map_err(driver)?);
            Ok(())
        }
        fn touch(&mut self, pattern: u8) -> Result<(), GpuWitnessError> {
            let control = self.control.as_mut().ok_or_else(|| GpuWitnessError::ProbeFailed {
                message: "private control absent".into(),
            })?;
            let (pointer, _record) = control.device_ptr_mut(&self.stream);
            // The buffer is retained through checked completion and the live read.
            unsafe {
                result::memset_d8_async(
                    pointer,
                    pattern,
                    CONTROL_BYTES as usize,
                    self.stream.cu_stream(),
                )
            }
            .map_err(driver)
        }
        fn release(&mut self) -> Result<(), GpuWitnessError> {
            drop(self.control.take());
            self.context.check_err().map_err(driver)
        }
    }
}

#[cfg(feature = "embed-cuda")]
pub(crate) use hardware::{method_for_device, HeldPrivatePool};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Failure {
        None,
        Complete(usize),
        Read(usize),
        Load,
        Forward,
        Allocate,
        Touch,
        Release,
        TouchAndRelease,
    }

    struct State {
        failure: Failure,
        events: Vec<&'static str>,
        used: u64,
        queued: u64,
        completions: usize,
        reads: usize,
        model_live: bool,
        control_live: bool,
    }

    impl State {
        fn shared(failure: Failure) -> Rc<RefCell<Self>> {
            Rc::new(RefCell::new(Self {
                failure,
                events: Vec::new(),
                used: 0,
                queued: 8192,
                completions: 0,
                reads: 0,
                model_live: false,
                control_live: false,
            }))
        }
    }

    struct QueuedPool(Rc<RefCell<State>>);
    struct Model(Rc<RefCell<State>>);

    impl Drop for Model {
        fn drop(&mut self) {
            let mut state = self.0.borrow_mut();
            assert!(!state.control_live, "control must be released before model teardown");
            state.model_live = false;
            state.events.push("drop-model");
        }
    }

    fn error(message: &str) -> GpuWitnessError {
        GpuWitnessError::ProbeFailed { message: message.into() }
    }

    impl PrivateWitnessProbe for QueuedPool {
        fn complete(&mut self) -> Result<(), GpuWitnessError> {
            let mut state = self.0.borrow_mut();
            state.events.push("complete");
            state.completions += 1;
            if state.failure == Failure::Complete(state.completions) {
                return Err(GpuWitnessError::NoCudaContext {
                    message: "checked completion failed".into(),
                });
            }
            state.used = state.queued;
            Ok(())
        }
        fn read(&mut self) -> Result<PrivateSample, GpuWitnessError> {
            let mut state = self.0.borrow_mut();
            state.events.push("read");
            state.reads += 1;
            if state.reads > 1 {
                assert!(state.model_live, "all post-load/control observations require live model");
            }
            if state.failure == Failure::Read(state.reads) {
                return Err(error("read failed"));
            }
            Ok(PrivateSample {
                used: state.used,
                global: GpuMemorySample {
                    free_bytes: 50_000_000_000 + if state.model_live { 374_050_816 } else { 0 },
                    total_bytes: 65_879_896_064,
                },
            })
        }
        fn allocate(&mut self, bytes: u64) -> Result<(), GpuWitnessError> {
            let mut state = self.0.borrow_mut();
            state.events.push("allocate");
            assert!(state.model_live);
            assert_eq!(bytes, 67_108_864, "one exact 64 MiB control");
            assert!(!state.control_live, "never allocate a second control");
            if state.failure == Failure::Allocate {
                return Err(error("allocate failed"));
            }
            state.queued += bytes;
            state.control_live = true;
            Ok(())
        }
        fn touch(&mut self, pattern: u8) -> Result<(), GpuWitnessError> {
            let mut state = self.0.borrow_mut();
            state.events.push("touch");
            assert!(state.model_live && state.control_live);
            assert_eq!(pattern, 0xa5);
            if matches!(state.failure, Failure::Touch | Failure::TouchAndRelease) {
                return Err(error("touch failed"));
            }
            Ok(())
        }
        fn release(&mut self) -> Result<(), GpuWitnessError> {
            let mut state = self.0.borrow_mut();
            state.events.push("release");
            assert!(state.model_live, "release only control, retain model");
            if state.control_live {
                state.queued -= 67_108_864;
                state.control_live = false;
            }
            if matches!(state.failure, Failure::Release | Failure::TouchAndRelease) {
                return Err(error("release failed"));
            }
            Ok(())
        }
    }

    fn observe(
        state: &Rc<RefCell<State>>,
    ) -> Result<(Model, PrivateObservations), GpuWitnessError> {
        observe_private_model(
            &mut QueuedPool(state.clone()),
            || {
                let mut inner = state.borrow_mut();
                inner.events.push("load");
                if inner.failure == Failure::Load {
                    return Err(error("load failed"));
                }
                inner.queued += 132_848_640;
                inner.model_live = true;
                Ok(Model(state.clone()))
            },
            |_| {
                let mut inner = state.borrow_mut();
                inner.events.push("forward");
                assert!(inner.model_live);
                if inner.failure == Failure::Forward {
                    return Err(error("forward failed"));
                }
                inner.queued += 256;
                Ok(384)
            },
        )
    }

    #[test]
    fn selector_uses_actual_hardware_allocator_and_pool_before_observations() {
        assert_eq!(select_method(false, false, false).unwrap(), WitnessMethod::GlobalFree);
        assert_eq!(select_method(true, true, true).unwrap(), WitnessMethod::PrivatePoolUsed);
    }

    #[test]
    fn unsupported_actual_allocator_combinations_are_named_refusals() {
        for facts in [
            (false, false, true),
            (true, false, true),
            (true, false, false),
            (true, true, false),
            (false, true, true),
            (false, true, false),
        ] {
            let result = select_method(facts.0, facts.1, facts.2);
            assert!(result.is_err(), "unsupported actual facts {facts:?}");
            assert_eq!(result.unwrap_err().as_str(), "unsupported_allocation_method");
        }
    }

    #[test]
    fn private_production_bracket_completes_baseline_and_retains_model_through_control() {
        let state = State::shared(Failure::None);
        let (model, observations) = observe(&state).expect("completed same-pool witness bracket");
        assert_eq!(
            observations.before.used, 8192,
            "initial queued work must complete before baseline"
        );
        assert_eq!(observations.after.used, 8192 + 132_848_640);
        assert_eq!(observations.dimension, 384);
        assert_eq!(observations.control_before_bytes, observations.after.used + 256);
        assert_eq!(observations.control_live_bytes - observations.control_before_bytes, 67_108_864);
        assert_eq!(observations.control_released_bytes, observations.control_before_bytes);
        assert!(observations.after.global.free_bytes > observations.before.global.free_bytes);
        assert!(state.borrow().model_live);
        assert!(!state.borrow().control_live);
        assert_eq!(
            state.borrow().events,
            [
                "complete", "read", "load", "complete", "read", "forward", "complete", "read",
                "allocate", "touch", "complete", "read", "release", "complete", "read",
            ]
        );
        drop(model);
        assert!(!state.borrow().model_live);
    }

    #[test]
    fn live_control_error_survives_cleanup_error_and_completion_precedes_model_drop() {
        let state = State::shared(Failure::TouchAndRelease);
        let result = observe(&state).err().expect("compound control failure");
        assert_eq!(result, error("touch failed"));
        let inner = state.borrow();
        assert!(!inner.model_live && !inner.control_live);
        let release = inner.events.iter().position(|event| *event == "release").unwrap();
        let completion = inner.events.iter().rposition(|event| *event == "complete").unwrap();
        let model_drop = inner.events.iter().position(|event| *event == "drop-model").unwrap();
        assert!(
            release < completion && completion < model_drop,
            "attempt checked cleanup completion even when release fails: {:?}",
            inner.events
        );
    }

    #[test]
    fn private_production_failures_stop_or_clean_control_without_replacing_original_error() {
        for failure in [
            Failure::Complete(1),
            Failure::Read(1),
            Failure::Load,
            Failure::Complete(2),
            Failure::Read(2),
            Failure::Forward,
            Failure::Complete(3),
            Failure::Read(3),
            Failure::Allocate,
            Failure::Touch,
            Failure::Complete(4),
            Failure::Read(4),
            Failure::Release,
            Failure::Complete(5),
            Failure::Read(5),
        ] {
            let state = State::shared(failure);
            let result = observe(&state);
            assert!(result.is_err(), "accepted {failure:?}");
            let result = result.err().unwrap();
            if matches!(failure, Failure::Complete(_)) {
                assert_eq!(result.as_str(), "no_cuda_context", "{failure:?}");
            } else {
                let expected = match failure {
                    Failure::Read(_) => "read failed",
                    Failure::Load => "load failed",
                    Failure::Forward => "forward failed",
                    Failure::Allocate => "allocate failed",
                    Failure::Touch => "touch failed",
                    Failure::Release => "release failed",
                    _ => unreachable!(),
                };
                assert_eq!(result, error(expected), "{failure:?}");
            }
            let inner = state.borrow();
            assert!(!inner.model_live && !inner.control_live, "live resources after {failure:?}");
            if failure == Failure::Complete(1) {
                assert_eq!(
                    inner.events,
                    ["complete"],
                    "initial failure must prevent baseline/load"
                );
            }
            if matches!(
                failure,
                Failure::Complete(1..=3) | Failure::Read(1..=3) | Failure::Load | Failure::Forward
            ) {
                assert!(
                    !inner.events.contains(&"allocate"),
                    "early failure must prevent control: {failure:?}"
                );
            } else {
                assert!(inner.events.contains(&"release"), "control cleanup after {failure:?}");
            }
        }
    }
}
