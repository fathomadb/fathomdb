use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Canonical owner and real writer, with an additive disposable assertion ledger.
pub struct Ledger {
    pub engine: Engine,
    pub read: Connection,
}

impl Ledger {
    /// Create a real engine database and install only experiment-owned tables.
    pub fn open(path: &Path) -> Result<Self> {
        let engine = Engine::open(path)?.engine;
        engine.execute_for_test("CREATE TABLE IF NOT EXISTS poc_fragments(fragment INTEGER PRIMARY KEY,claim TEXT NOT NULL,revision INTEGER NOT NULL,source TEXT NOT NULL,v0 INTEGER NOT NULL,v1 INTEGER NOT NULL,k0 INTEGER NOT NULL,k1 INTEGER,withdrawn INTEGER NOT NULL);")?;
        let read = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        read.execute_batch("PRAGMA query_only=ON")?;
        Ok(Self { engine, read })
    }

    /// Submit the actual governed node write; the candidate associates interval intent.
    pub fn change(
        &self,
        claim: &str,
        body: &str,
        source: &str,
        _valid: (i64, i64),
        _withdrawn: bool,
    ) -> Result<u64> {
        Ok(self
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: body.into(),
                logical_id: Some(claim.into()),
                source_id: SourceId::new(source)?,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])?
            .row_cursors[0])
    }

    /// Exact canonical revision selected by a half-open valid/knowledge scope.
    pub fn selected(&self, claim: &str, valid: i64, known: i64) -> Result<Vec<(u64, String)>> {
        Ok(self.read.prepare("SELECT f.revision,n.body FROM poc_fragments f JOIN canonical_nodes n ON n.write_cursor=f.revision WHERE f.claim=?1 AND f.v0<=?2 AND ?2<f.v1 AND f.k0<=?3 AND (f.k1 IS NULL OR ?3<f.k1) AND f.withdrawn=0 ORDER BY f.fragment")?
            .query_map(rusqlite::params![claim, valid, known], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
