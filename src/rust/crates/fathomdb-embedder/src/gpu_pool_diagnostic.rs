use crate::gpu_witness::GpuMemorySample;

const CONTROL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
struct Sample {
    used: u64,
    reserved: u64,
    global: GpuMemorySample,
}

trait PoolProbe {
    fn complete(&mut self) -> Result<(), String>;
    fn read(&mut self) -> Result<Sample, String>;
    fn allocate(&mut self, bytes: u64) -> Result<(), String>;
    fn touch(&mut self, pattern: u8) -> Result<(), String>;
    fn release(&mut self) -> Result<(), String>;
}

#[derive(Default)]
struct Diagnostic {
    samples: Vec<(&'static str, Sample)>,
    errors: Vec<String>,
    control_valid: bool,
    forward_dimension: Option<usize>,
}

impl Diagnostic {
    fn forward_completed(&mut self, dimension: usize) {
        self.forward_dimension = Some(dimension);
    }
}

pub(super) fn run_optional<D, V>(
    mut diagnostic: Option<D>,
    work: impl FnOnce(Option<&mut D>) -> V,
    emit: impl FnOnce(&D, &V),
) -> V {
    let verdict = work(diagnostic.as_mut());
    if let Some(diagnostic) = diagnostic.as_ref() {
        emit(diagnostic, &verdict);
    }
    verdict
}

fn after_verdict<T>(diagnostic: &mut Diagnostic, probe: &mut dyn PoolProbe, verdict: T) -> T {
    if !diagnostic.errors.is_empty() {
        return verdict;
    }
    let baseline = match probe.complete().and_then(|()| probe.read()) {
        Ok(sample) => sample,
        Err(error) => {
            diagnostic.errors.push(format!("control_baseline: {error}"));
            return verdict;
        }
    };
    diagnostic.samples.push(("control_baseline", baseline));
    let live = (|| {
        probe.allocate(CONTROL_BYTES)?;
        probe.touch(0xa5)?;
        probe.complete()?;
        probe.read()
    })();
    match &live {
        Ok(sample) => diagnostic.samples.push(("control_live", *sample)),
        Err(error) => diagnostic.errors.push(format!("control_live: {error}")),
    }
    // Cleanup is required even when allocation, touch, completion or reading failed.
    let released = probe.release().and_then(|()| probe.complete()).and_then(|()| probe.read());
    match &released {
        Ok(sample) => diagnostic.samples.push(("control_released", *sample)),
        Err(error) => diagnostic.errors.push(format!("control_released: {error}")),
    }
    if let (Ok(live), Ok(released)) = (live, released) {
        if live.used.checked_sub(baseline.used).is_none_or(|rise| rise < CONTROL_BYTES) {
            diagnostic.errors.push("control USED rise is smaller than request".into());
        }
        if released.used != baseline.used {
            diagnostic.errors.push("control release did not restore exact USED baseline".into());
        }
    }
    diagnostic.control_valid = diagnostic.errors.is_empty();
    verdict
}

#[cfg(feature = "embed-cuda")]
mod hardware {
    use super::*;
    use crate::gpu_witness::{GpuAllocationWitness, GpuMemorySampler, GpuWitnessError};
    use crate::gpu_witness_private::{HeldPrivatePool, PrivateWitnessProbe};
    use serde_json::{json, Value};
    use std::io::Write;

    impl PoolProbe for HeldPrivatePool {
        fn complete(&mut self) -> Result<(), String> {
            PrivateWitnessProbe::complete(self).map_err(|error| error.to_string())
        }
        fn read(&mut self) -> Result<Sample, String> {
            let used = self.read_used().map_err(|error| error.to_string())?;
            let reserved = self.read_reserved().map_err(|error| error.to_string())?;
            let global = self.read_global().map_err(|error| error.to_string())?;
            Ok(Sample { used, reserved, global })
        }
        fn allocate(&mut self, bytes: u64) -> Result<(), String> {
            PrivateWitnessProbe::allocate(self, bytes).map_err(|error| error.to_string())
        }
        fn touch(&mut self, pattern: u8) -> Result<(), String> {
            PrivateWitnessProbe::touch(self, pattern).map_err(|error| error.to_string())
        }
        fn release(&mut self) -> Result<(), String> {
            PrivateWitnessProbe::release(self).map_err(|error| error.to_string())
        }
    }

    pub(crate) struct Session {
        record: Diagnostic,
        identity: Option<Value>,
        probe: Option<HeldPrivatePool>,
        witness_global_samples: Vec<Value>,
        witness_verdict_reached: bool,
    }

    impl Session {
        pub(crate) fn enabled() -> Option<Self> {
            (std::env::var("FATHOMDB_GPU_POOL_DIAGNOSTIC").as_deref() == Ok("1")).then(|| Self {
                record: Diagnostic::default(),
                identity: None,
                probe: None,
                witness_global_samples: Vec::new(),
                witness_verdict_reached: false,
            })
        }

        pub(crate) fn attach(
            &mut self,
            device: &candle_core::Device,
            requested: usize,
            retained_uuid: &str,
        ) {
            if let Err(error) = self.attach_checked(device, requested, retained_uuid) {
                self.record.errors.push(error);
            }
        }

        fn attach_checked(
            &mut self,
            device: &candle_core::Device,
            requested: usize,
            retained_uuid: &str,
        ) -> Result<(), String> {
            let stream = device.as_cuda_device().map_err(|error| error.to_string())?.cuda_stream();
            let context = stream.context().clone();
            let pool = context.mem_pool().cloned();
            self.identity = Some(json!({
                "pid": std::process::id(), "requested_ordinal": requested,
                "retained_ordinal": context.ordinal(), "attested_uuid": retained_uuid,
                "context_handle": format!("{:p}", context.cu_ctx()),
                "stream_handle": format!("{:p}", stream.cu_stream()),
                "pool_handle": pool.as_ref().map(|pool| format!("{:p}", pool.raw())),
                "allocation_mode": format!("{:?}", context.alloc_mode()),
            }));
            let probe = HeldPrivatePool::new(device, requested, retained_uuid)
                .map_err(|error| error.to_string())?;
            if let Some(identity) = self.identity.as_mut() {
                identity["context_id"] = json!(probe.context_id().to_string());
                identity["context_uuid"] = json!(probe.context_uuid());
            }
            self.probe = Some(probe);
            Ok(())
        }

        fn observe(
            &mut self,
            stage: &'static str,
            sample: &Result<GpuMemorySample, GpuWitnessError>,
        ) {
            match sample {
                Ok(sample) => self.witness_global_samples.push(json!({"stage": stage, "free_bytes": sample.free_bytes, "total_bytes": sample.total_bytes})),
                Err(error) => self.record.errors.push(format!("witness {stage}: {error}")),
            }
            if !self.record.errors.is_empty() {
                return;
            }
            if let Some(probe) = self.probe.as_mut() {
                match PoolProbe::read(probe) {
                    Ok(sample) => self.record.samples.push((stage, sample)),
                    Err(error) => self.record.errors.push(format!("{stage}: {error}")),
                }
            }
        }

        pub(crate) fn retain_private_global_samples(
            &mut self,
            before: GpuMemorySample,
            after: GpuMemorySample,
        ) {
            for (stage, sample) in [("before_load", before), ("after_checked_load", after)] {
                self.witness_global_samples.push(json!({"stage":stage,
                    "free_bytes":sample.free_bytes,"total_bytes":sample.total_bytes}));
            }
        }

        pub(crate) fn forward_completed(&mut self, dimension: usize) {
            self.record.forward_completed(dimension);
        }

        pub(crate) fn finish<V>(&mut self, verdict: V) -> V {
            self.witness_verdict_reached = true;
            match self.probe.as_mut() {
                Some(probe) => after_verdict(&mut self.record, probe, verdict),
                None => verdict,
            }
        }

        pub(crate) fn emit(&self, verdict: &Result<GpuAllocationWitness, GpuWitnessError>) {
            let witness = match verdict {
                Ok(witness) => {
                    json!({"outcome":"pass", "record_json": witness.to_canonical_json()})
                }
                Err(error) => {
                    json!({"outcome":"error", "kind":error.as_str(), "message":error.to_string()})
                }
            };
            let samples: Vec<Value> = self.record.samples.iter().map(|(stage, sample)| json!({
                "stage":stage,"used_bytes":sample.used,"reserved_bytes":sample.reserved,
                "global_free_bytes":sample.global.free_bytes,"global_total_bytes":sample.global.total_bytes,
            })).collect();
            let record = json!({
                "schema":"fathomdb.tegra-gpu-pool-diagnostic/v2", "pid":std::process::id(),
                "identity":self.identity,"sampling":"sequential non-atomic; original global then USED/RESERVED/global",
                "witness":witness,"witness_global_samples":self.witness_global_samples,"samples":samples,"errors":self.record.errors,
                "witness_verdict_reached":self.witness_verdict_reached,
                "embedded_vector_dim":self.record.forward_dimension,
                "control_request_bytes":CONTROL_BYTES,"control_valid":self.record.control_valid,
                "diagnostic_valid":self.witness_verdict_reached && self.identity.is_some() && self.record.control_valid && self.record.errors.is_empty(),
                "exclusivity":"external single-open driver attestation required; not enforced by runtime",
                "acceptance":false,
            });
            // A diagnostic sink failure must not replace the original witness result.
            let _ = writeln!(std::io::stderr().lock(), "fathomdb-tegra-pool-diagnostic {record}");
        }
    }

    pub(crate) struct ObservedSampler<'a> {
        sampler: &'a mut dyn GpuMemorySampler,
        diagnostic: Option<&'a mut Session>,
        load_phase: bool,
        sample_index: usize,
    }

    impl<'a> ObservedSampler<'a> {
        pub(crate) fn new(
            sampler: &'a mut dyn GpuMemorySampler,
            diagnostic: Option<&'a mut Session>,
            load_phase: bool,
        ) -> Self {
            Self { sampler, diagnostic, load_phase, sample_index: 0 }
        }
    }

    impl GpuMemorySampler for ObservedSampler<'_> {
        fn sample(&mut self) -> Result<GpuMemorySample, GpuWitnessError> {
            let sample = self.sampler.sample();
            if let Some(diagnostic) = self.diagnostic.as_deref_mut() {
                let stage = if self.load_phase {
                    if self.sample_index == 0 {
                        "before_load"
                    } else {
                        "after_checked_load"
                    }
                } else {
                    "original_raw_control"
                };
                diagnostic.observe(stage, &sample);
            }
            self.sample_index += 1;
            sample
        }
    }
}

#[cfg(feature = "embed-cuda")]
pub(super) use hardware::{ObservedSampler, Session};

#[cfg(test)]
mod tests {
    use super::*;

    struct QueuedPool {
        used: u64,
        pending: u64,
        fail: &'static str,
        release_bytes: u64,
        events: Vec<&'static str>,
    }

    impl QueuedPool {
        fn new(fail: &'static str) -> Self {
            Self { used: 1000, pending: 1000, fail, release_bytes: 1000, events: Vec::new() }
        }
    }

    impl PoolProbe for QueuedPool {
        fn complete(&mut self) -> Result<(), String> {
            self.events.push("complete");
            if self.fail == "complete"
                || (self.fail == "complete-live"
                    && self.events.iter().filter(|event| **event == "complete").count() == 2)
            {
                return Err("completion failed".into());
            }
            self.used = self.pending;
            Ok(())
        }
        fn read(&mut self) -> Result<Sample, String> {
            self.events.push("read");
            if self.fail == "query"
                || (self.fail == "query-live"
                    && self.events.iter().filter(|event| **event == "read").count() == 2)
            {
                return Err("USED query failed".into());
            }
            Ok(Sample {
                used: self.used,
                reserved: self.used + 4096,
                global: GpuMemorySample { free_bytes: 100, total_bytes: 1000 },
            })
        }
        fn allocate(&mut self, bytes: u64) -> Result<(), String> {
            self.events.push("allocate");
            assert_eq!(bytes, CONTROL_BYTES);
            if self.fail == "allocate" {
                return Err("allocation failed".into());
            }
            self.pending += if self.fail == "rise" { bytes - 1 } else { bytes };
            Ok(())
        }
        fn touch(&mut self, pattern: u8) -> Result<(), String> {
            self.events.push("touch");
            assert_ne!(pattern, 0);
            if self.fail == "touch" {
                return Err("touch failed".into());
            }
            Ok(())
        }
        fn release(&mut self) -> Result<(), String> {
            self.events.push("release");
            self.pending =
                if self.fail == "release" { self.release_bytes + 1 } else { self.release_bytes };
            Ok(())
        }
    }

    #[test]
    fn successful_forward_dimension_survives_original_v1_failure() {
        let mut diagnostic = Diagnostic::default();
        assert_eq!(diagnostic.forward_dimension, None);
        diagnostic.forward_completed(384);
        let mut pool = QueuedPool::new("");
        let verdict: Result<(), &str> =
            after_verdict(&mut diagnostic, &mut pool, Err("insufficient_delta"));
        assert_eq!(verdict, Err("insufficient_delta"));
        assert_eq!(diagnostic.forward_dimension, Some(384));
    }

    #[test]
    fn optional_production_wrapper_does_no_diagnostic_work_when_off_and_emits_v1_error_when_on() {
        use std::cell::RefCell;
        for enabled in [false, true] {
            let events = RefCell::new(Vec::new());
            let result = run_optional(
                enabled.then(Diagnostic::default),
                |diagnostic| {
                    events.borrow_mut().push("original-v1");
                    if diagnostic.is_some() {
                        events.borrow_mut().push("diagnostic-probe");
                    }
                    Err::<(), _>("original-v1-error")
                },
                |_, result| {
                    assert_eq!(*result, Err("original-v1-error"));
                    events.borrow_mut().push("emit-error");
                },
            );
            assert_eq!(result, Err("original-v1-error"));
            if enabled {
                assert_eq!(*events.borrow(), ["original-v1", "diagnostic-probe", "emit-error"]);
            } else {
                assert_eq!(*events.borrow(), ["original-v1"]);
            }
        }
    }

    #[test]
    fn completed_same_pool_control_is_measured_only_after_original_verdict() {
        let mut pool = QueuedPool::new("");
        let mut diagnostic = Diagnostic::default();
        pool.events.push("v1-forward");
        pool.events.push("v1-verdict");
        let result: Result<(), &str> =
            after_verdict(&mut diagnostic, &mut pool, Err("insufficient_delta"));
        assert_eq!(result, Err("insufficient_delta"));
        assert!(diagnostic.control_valid, "a completed live/released control is required");
        assert_eq!(
            pool.events,
            [
                "v1-forward",
                "v1-verdict",
                "complete",
                "read",
                "allocate",
                "touch",
                "complete",
                "read",
                "release",
                "complete",
                "read"
            ]
        );
        assert_eq!(diagnostic.samples[0].1.reserved, 5096);
        assert_eq!(
            diagnostic.samples[0].1.global,
            GpuMemorySample { free_bytes: 100, total_bytes: 1000 }
        );
        assert_eq!(
            diagnostic
                .samples
                .iter()
                .map(|(stage, sample)| (*stage, sample.used))
                .collect::<Vec<_>>(),
            [
                ("control_baseline", 1000),
                ("control_live", 1000 + CONTROL_BYTES),
                ("control_released", 1000)
            ]
        );
    }

    #[test]
    fn checked_query_or_completion_failure_never_starts_a_control_or_changes_v1() {
        for failure in ["complete", "query"] {
            let mut pool = QueuedPool::new(failure);
            let mut diagnostic = Diagnostic::default();
            assert_eq!(after_verdict(&mut diagnostic, &mut pool, Ok::<_, &str>(7)), Ok(7));
            assert!(!diagnostic.errors.is_empty(), "{failure} must remain diagnostic evidence");
            assert!(!pool.events.contains(&"allocate"));
            assert!(!diagnostic.control_valid);
        }
    }

    #[test]
    fn failed_control_and_inexact_live_or_release_accounting_preserve_v1_failure() {
        for failure in ["allocate", "touch", "complete-live", "query-live", "rise", "release"] {
            let mut pool = QueuedPool::new(failure);
            let mut diagnostic = Diagnostic::default();
            assert_eq!(
                after_verdict(&mut diagnostic, &mut pool, Err::<(), _>("original-v1-error")),
                Err("original-v1-error")
            );
            assert!(!diagnostic.errors.is_empty(), "{failure} must invalidate the diagnostic");
            assert!(!diagnostic.control_valid);
        }
    }
}
