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
}

fn after_verdict<T>(_diagnostic: &mut Diagnostic, _probe: &mut dyn PoolProbe, verdict: T) -> T {
    verdict
}

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
