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
    main: &rusqlite::Transaction<'_>,
    selection: &str,
    hydrate: &str,
    query: &str,
    limit: usize,
    contentless: bool,
    after_population: impl FnOnce() -> Result<()>,
) -> Result<Search> {
    let mut private = Connection::open_in_memory()?;
    private.execute_batch("PRAGMA temp_store=MEMORY;")?;
    private.execute_batch(if contentless { "CREATE VIRTUAL TABLE corpus USING fts5(body,content='',detail=full,columnsize=1,tokenize='porter unicode61 remove_diacritics 2');" } else { "CREATE VIRTUAL TABLE corpus USING fts5(body,detail=full,columnsize=1,tokenize='porter unicode61 remove_diacritics 2');" })?;
    private.execute_batch(
        "CREATE TABLE eligible(id INTEGER PRIMARY KEY,admitted INTEGER NOT NULL);",
    )?;
    let mut documents = 0;
    let mut indexed_bytes = 0;
    {
        let tx = private.transaction()?;
        {
            let mut insert = tx.prepare("INSERT INTO corpus(rowid,body) VALUES(?1,?2)")?;
            let mut admit = tx.prepare("INSERT INTO eligible VALUES(?1,?2)")?;
            let mut source = main.prepare(selection)?;
            let mut rows = source.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let body: String = row.get(1)?;
                let eligible: bool = row.get(2)?;
                insert.execute(rusqlite::params![id, body])?;
                admit.execute(rusqlite::params![id, eligible])?;
                documents += 1;
                indexed_bytes += body.len();
            }
        }
        tx.commit()?;
    }
    after_population()?;
    let mut hits = Vec::new();
    let compiled = fathomdb_query::compile_text_query(query);
    if limit > 0 && !compiled.match_expression.is_empty() {
        let mut rank=private.prepare("SELECT corpus.rowid,bm25(corpus) FROM corpus JOIN eligible ON eligible.id=corpus.rowid WHERE corpus MATCH ?1 AND eligible.admitted=1 ORDER BY bm25(corpus),corpus.rowid")?;
        let mut rows = rank.query([compiled.match_expression])?;
        let mut hydration = main.prepare(hydrate)?;
        let mut bodies = std::collections::HashSet::new();
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let body: String = hydration.query_row([id], |r| r.get(0))?;
            if bodies.insert(body.clone()) {
                hits.push(Hit { id, body, score: row.get(1)? });
                if hits.len() >= limit {
                    break;
                }
            }
        }
    }
    let temp_store = private.query_row("PRAGMA temp_store", [], |r| r.get(0))?;
    let private_files = private
        .prepare("PRAGMA database_list")?
        .query_map([], |r| r.get::<_, String>(2))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Search { hits, documents, indexed_bytes, temp_store, private_files })
}
