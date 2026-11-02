use std::collections::BTreeMap;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family { Node, Edge, Property }

#[derive(Clone, Copy, Debug)]
enum Intent { Initial, Revision }

#[derive(Clone, Debug)]
struct Change {
    claim: String,
    body: String,
    family: Family,
    properties: BTreeMap<String, String>,
    known: i64,
    valid_from: i64,
    valid_to: Option<i64>,
    intent: Intent,
    source: String,
    endpoints: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
struct Hit { claim: String, version: i64, body: String, score: f64 }

struct Store;

impl Store {
    fn open(_path: &Path, _now: i64) -> Result<Self> { Ok(Self) }
    fn put(&mut self, _change: Change) -> Result<()> { Ok(()) }
    fn search(&self, _query: &str, _family: Family, _name: Option<&str>, _limit: usize) -> Result<Vec<Hit>> { Ok(Vec::new()) }
}

/// Run the reproducible, synthetic POC cost sweep; errors stop the run.
pub fn run_benchmark() -> Result<()> { Ok(()) }

#[cfg(test)]
#[path = "../tests/hypotheses.rs"]
mod hypotheses;
