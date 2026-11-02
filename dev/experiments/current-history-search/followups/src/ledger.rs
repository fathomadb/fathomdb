use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};
use std::{path::Path, sync::Mutex};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Canonical owner and real writer, with an additive disposable assertion ledger.
pub struct Ledger {
    pub engine: Engine,
    pub read: Connection,
    admission: Mutex<()>,
}

impl Ledger {
    /// Create a real engine database and install only experiment-owned tables.
    pub fn open(path: &Path) -> Result<Self> {
        let engine = Engine::open(path)?.engine;
        engine.execute_for_test(include_str!("ledger.sql"))?;
        let read = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        read.execute_batch("PRAGMA query_only=ON")?;
        Ok(Self { engine, read, admission: Mutex::new(()) })
    }

    /// One temporal write per committed operation; multiwrite temporal batches are unsupported.
    pub fn change(
        &self,
        claim: &str,
        body: &str,
        source: &str,
        valid: (i64, i64),
        withdrawn: bool,
    ) -> Result<u64> {
        let source_id = SourceId::new(source)?;
        if valid.0 >= valid.1 {
            return Err("empty or inverted valid interval".into());
        }
        let _admission = self.admission.lock().map_err(|_| "admission poisoned")?;
        let q = |text: &str| text.replace('\'', "''");
        self.engine.execute_for_test(&format!("BEGIN; INSERT INTO poc_operation VALUES('{}','{}','{}'); INSERT INTO poc_intent VALUES('{}',{},{},{}); COMMIT;",q(claim),q(body),q(source),q(claim),valid.0,valid.1,i64::from(withdrawn)))?;
        let result = self.engine.write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: body.into(),
            logical_id: Some(claim.into()),
            source_id,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }]);
        self.engine.execute_for_test("DELETE FROM poc_intent; DELETE FROM poc_operation;")?;
        let receipt = result?;
        assert_eq!(receipt.cursor, receipt.row_cursors[0]);
        Ok(receipt.cursor)
    }

    /// Exact canonical revision selected at a recorded committed temporal cut.
    pub fn selected(&self, claim: &str, valid: i64, known: i64) -> Result<Vec<(u64, String)>> {
        let committed: bool = self.read.query_row(
            "SELECT EXISTS(SELECT 1 FROM poc_cuts WHERE cut=?1)",
            [known],
            |r| r.get(0),
        )?;
        if !committed {
            return Err(
                "not a committed temporal cut; arbitrary batch-interior cursors are unsupported"
                    .into(),
            );
        }
        Ok(self.read.prepare("SELECT f.revision,n.body FROM poc_fragments f JOIN canonical_nodes n ON n.write_cursor=f.revision WHERE n.state='active' AND f.claim=?1 AND f.v0<=?2 AND ?2<f.v1 AND f.k0<=?3 AND (f.k1 IS NULL OR ?3<f.k1) AND f.withdrawn=0 ORDER BY f.fragment")?
            .query_map(rusqlite::params![claim, valid, known], |r| Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
