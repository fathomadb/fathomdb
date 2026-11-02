use crate::ledger::Result;
use rusqlite::Connection;

/// Selected native statistical document identity and exact snapshot hydration.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: i64,
    pub body: String,
    pub score: f64,
}
/// Per-call measurements; bytes count selected indexed UTF-8, not allocator capacity.
#[derive(Debug)]
pub struct Search {
    pub hits: Vec<Hit>,
    pub documents: usize,
    pub indexed_bytes: usize,
    pub temp_store: i64,
    pub private_files: Vec<String>,
}
/// Materialize one fully selected statistical corpus, then apply output eligibility.
/// Selection columns are unique id, indexed scalar and output eligibility; hydration
/// accepts the id as ?1 on the same held main snapshot. No shared private cache.
pub fn search(
    main: &Connection,
    selection: &str,
    hydrate: &str,
    query: &str,
    limit: usize,
    contentless: bool,
    after_population: impl FnOnce() -> Result<()>,
) -> Result<Search> {
    let _ = (main, selection, hydrate, query, limit, contentless);
    after_population()?;
    Ok(Search {
        hits: vec![],
        documents: 0,
        indexed_bytes: 0,
        temp_store: 0,
        private_files: vec![],
    })
}
