//! Private Core transport; callbacks keep feature-specific references in the host.

use super::CoreObserver;
use fathomdb_engine::Engine;
use serde_json::{json, Value};
use std::time::Instant;

pub(super) struct Observation<'a> {
    engine: &'a Engine,
    callbacks: CoreObserver,
    enabled: bool,
    dropped: Option<u64>,
    pub(super) setup_ns: u64,
}

impl<'a> Observation<'a> {
    pub(super) fn new(engine: &'a Engine, callbacks: CoreObserver, enabled: bool) -> Self {
        let started = Instant::now();
        callbacks.0(engine, enabled).unwrap();
        let stats = callbacks.2(engine).unwrap();
        let dropped = Some(stats["dropped"].as_u64().unwrap());
        Self {
            engine,
            callbacks,
            enabled,
            dropped,
            setup_ns: u64::try_from(started.elapsed().as_nanos()).unwrap(),
        }
    }

    pub(super) fn complete(&mut self, query: bool) -> Value {
        let started = Instant::now();
        let collected = self.callbacks.1(self.engine);
        let stats = self.callbacks.2(self.engine);
        let transport_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        let mut issues = Vec::new();
        let records_available = collected.is_ok();
        let records = collected.unwrap_or_else(|error| {
            issues.push(format!("record transport: {error:?}"));
            Vec::new()
        });
        let (capacity, dropped_after) = match stats {
            Ok(stats) => (stats["capacity"].as_u64(), stats["dropped"].as_u64()),
            Err(error) => {
                issues.push(format!("counter transport: {error:?}"));
                (None, None)
            }
        };
        let dropped_before = self.dropped;
        let dropped_delta =
            dropped_before.zip(dropped_after).and_then(|(before, after)| after.checked_sub(before));
        if capacity.is_none() || dropped_delta.is_none() {
            issues.push("counter observation unavailable".into());
        }
        self.dropped = dropped_after;
        let expected = self.enabled && query;
        let unpaired_records = records_available.then(|| {
            if expected {
                records.len().saturating_sub(1)
            } else {
                records.len()
            }
        });
        let status = if !issues.is_empty() {
            "unavailable"
        } else if records.is_empty() && !query {
            "not_applicable"
        } else if records.is_empty() && !self.enabled {
            "disabled"
        } else if expected && records.len() == 1 {
            "observed"
        } else if records.is_empty() {
            "missing"
        } else {
            "unpaired"
        };
        json!({"status":status,"records":records,"records_available":records_available,
            "unpaired_records":unpaired_records,"capacity":capacity,"dropped_before":dropped_before,
            "dropped_after":dropped_after,"dropped_delta":dropped_delta,"issues":issues,
            "transport_ns":transport_ns})
    }
}
