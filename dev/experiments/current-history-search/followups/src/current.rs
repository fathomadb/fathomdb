use crate::ledger::Result;
use rusqlite::Connection;

/// Snapshot-local certificate tied to actual engine visibility and scope.
#[derive(Clone, Debug, PartialEq)]
pub struct Certificate {
    pub published_at: i64,
    pub next_boundary: i64,
    pub visibility: i64,
    pub scope: String,
}
impl Certificate {
    /// Read a bounded singleton; no corpus, IDs, vocabulary or body prepass.
    pub fn load(connection: &Connection) -> Result<Self> {
        Ok(connection.query_row("SELECT published_at,next_boundary,visibility,scope FROM poc_certificate WHERE singleton=1",[],|r|Ok(Self{published_at:r.get(0)?,next_boundary:r.get(1)?,visibility:r.get(2)?,scope:r.get(3)?}))?)
    }
    /// Whether this exact resolved request may use the snapshot's current index.
    pub fn covers(&self, valid: i64, visibility: i64, scope: &str) -> bool {
        self.published_at <= valid
            && valid < self.next_boundary
            && self.visibility == visibility
            && self.scope == scope
    }
}

use fathomdb_engine::{Engine, PreparedWrite};
use rusqlite::{OpenFlags, StatementStatus};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

/// Native current-family collector result. Scores are native BM25, not hybrid RRF.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub cursor: u64,
    pub body: String,
    pub score: f64,
}
/// Candidate-only typed failure; numeric policies are experiment values.
#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    RefreshRequired,
    BackwardTime,
    Closed,
    Storage(String),
    RetryLimit,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Failure {}

/// Bounded test rendezvous; dropping its sender releases the held snapshot.
pub struct Pause {
    pub ready: mpsc::SyncSender<()>,
    pub release: mpsc::Receiver<()>,
}
struct Request {
    query: String,
    valid: i64,
    limit: usize,
    pause: Option<Pause>,
    reply: mpsc::SyncSender<std::result::Result<ReadResult, Failure>>,
}
enum Work {
    Query(Request),
    Close,
}
/// Attribution proves metadata validation and ranking avoid excluded history.
#[derive(Debug)]
pub struct ReadResult {
    pub rows: Vec<Row>,
    pub vm_steps: i32,
    pub worker: usize,
}

/// Eight actual read-only connections; source writes remain on the actual Engine writer.
pub struct CurrentStore {
    engine: Arc<Engine>,
    admission: Mutex<()>,
    senders: Vec<mpsc::SyncSender<Work>>,
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
    next: AtomicUsize,
    closed: AtomicBool,
    pub refreshes: AtomicU64,
    pub writer_contacts: AtomicU64,
}
impl CurrentStore {
    /// Open an experiment database, preserving the actual schema and canonical writer.
    pub fn open(path: &Path) -> Result<Self> {
        let engine = Arc::new(Engine::open(path)?.engine);
        engine.execute_for_test(include_str!("current.sql"))?;
        let mut senders = Vec::new();
        let mut threads = Vec::new();
        for worker in 0..8 {
            let (send, receive) = mpsc::sync_channel(32);
            let path = path.to_path_buf();
            threads.push(thread::spawn(move || {
                let mut connection =
                    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
                connection.execute_batch("PRAGMA query_only=ON").unwrap();
                while let Ok(work) = receive.recv() {
                    match work {
                        Work::Close => break,
                        Work::Query(request) => {
                            let result = read_current(&mut connection, &request, worker);
                            let _ = request.reply.send(result);
                        }
                    }
                }
            }));
            senders.push(send);
        }
        Ok(Self {
            engine,
            admission: Mutex::new(()),
            senders,
            threads: Mutex::new(threads),
            next: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
            refreshes: AtomicU64::new(0),
            writer_contacts: AtomicU64::new(0),
        })
    }
    /// Serialize actual canonical writes with candidate publication.
    pub fn write(&self, writes: &[PreparedWrite]) -> Result<()> {
        let _guard = self.admission.lock().unwrap();
        self.engine.write(writes)?;
        self.engine.drain(600_000)?;
        Ok(())
    }
    /// Execute a controlled actual writer mutation inside admission, for race probes.
    pub fn with_writer<T>(&self, operation: impl FnOnce(&Engine) -> T) -> T {
        let _guard = self.admission.lock().unwrap();
        operation(&self.engine)
    }
    /// Bootstrap the current family from dirty canonical rows and initial bounds.
    pub fn bootstrap(&self, valid: i64) -> Result<()> {
        self.publish(valid, true)
    }
    fn publish(&self, valid: i64, bootstrap: bool) -> Result<()> {
        let _guard = self.admission.lock().unwrap();
        self.writer_contacts.fetch_add(1, Ordering::Relaxed);
        if self.closed.load(Ordering::Acquire) {
            return Err(Failure::Closed.into());
        }
        let current =
            self.engine.query_i64_col_for_test("SELECT published_at FROM poc_certificate")?[0];
        if valid < current {
            return Err(Failure::BackwardTime.into());
        }
        let next = if bootstrap {
            self.engine.query_i64_col_for_test(&format!("SELECT COALESCE(MIN(boundary),9223372036854775807) FROM (SELECT valid_from boundary FROM canonical_nodes UNION ALL SELECT valid_until FROM canonical_nodes) WHERE boundary>{valid}"))?[0]
        } else {
            i64::MAX
        };
        let sql=format!("BEGIN IMMEDIATE;
            DELETE FROM poc_current WHERE rowid IN (SELECT cursor FROM poc_dirty);
            INSERT INTO poc_current(rowid,body) SELECT n.write_cursor,n.body FROM poc_dirty d JOIN canonical_nodes n ON n.write_cursor=d.cursor WHERE n.state='active' AND n.superseded_at IS NULL AND (n.valid_from IS NULL OR n.valid_from<={valid}) AND (n.valid_until IS NULL OR n.valid_until>{valid});
            DELETE FROM poc_dirty;
            UPDATE poc_certificate SET published_at={valid},next_boundary={next},visibility=(SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1),scope='strict-v1' WHERE singleton=1;
            COMMIT;");
        if let Err(error) = self.engine.execute_for_test(&sql) {
            let _ = self.engine.execute_for_test("ROLLBACK");
            return Err(error.into());
        }
        self.refreshes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    /// End-to-end candidate read, including certificate, refresh admission and reacquisition.
    pub fn search(
        &self,
        query: &str,
        valid: i64,
        limit: usize,
    ) -> std::result::Result<ReadResult, Failure> {
        self.search_paused(query, valid, limit, None)
    }
    /// Test-only snapshot rendezvous on the same candidate collector.
    pub fn search_paused(
        &self,
        query: &str,
        valid: i64,
        limit: usize,
        mut pause: Option<Pause>,
    ) -> std::result::Result<ReadResult, Failure> {
        for _ in 0..3 {
            if self.closed.load(Ordering::Acquire) {
                return Err(Failure::Closed);
            }
            let (reply, receive) = mpsc::sync_channel(1);
            let worker = self.next.fetch_add(1, Ordering::Relaxed) % self.senders.len();
            self.senders[worker]
                .send(Work::Query(Request {
                    query: query.into(),
                    valid,
                    limit,
                    pause: pause.take(),
                    reply,
                }))
                .map_err(|_| Failure::Closed)?;
            match receive.recv().map_err(|_| Failure::Closed)? {
                Err(Failure::RefreshRequired) => self.publish(valid, false).map_err(|e| {
                    if e.downcast_ref::<Failure>() == Some(&Failure::BackwardTime) {
                        Failure::BackwardTime
                    } else {
                        Failure::Storage(e.to_string())
                    }
                })?,
                result => return result,
            }
        }
        Err(Failure::RetryLimit)
    }
    /// Stop admission and wait for active/queued work; receiver drop is not cancellation.
    pub fn close(&self) -> Result<()> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        for sender in &self.senders {
            let _ = sender.send(Work::Close);
        }
        for thread in self.threads.lock().unwrap().drain(..) {
            thread.join().map_err(|_| "reader panic")?;
        }
        self.engine.close()?;
        Ok(())
    }
}
impl Drop for CurrentStore {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn read_current(
    connection: &mut Connection,
    request: &Request,
    worker: usize,
) -> std::result::Result<ReadResult, Failure> {
    let read = || -> Result<ReadResult> {
        let tx = connection.unchecked_transaction()?;
        let certificate = Certificate::load(&tx)?;
        let visibility: i64 = tx.query_row(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        if !certificate.covers(request.valid, visibility, "strict-v1") {
            return Err(Failure::RefreshRequired.into());
        }
        if let Some(pause) = &request.pause {
            let _ = pause.ready.send(());
            let _ = pause.release.recv_timeout(Duration::from_secs(30));
        }
        let compiled = fathomdb_query::compile_text_query(&request.query);
        let mut statement=tx.prepare("SELECT rowid,body,bm25(poc_current) FROM poc_current WHERE poc_current MATCH ?1 ORDER BY bm25(poc_current),rowid")?;
        let mut rows = Vec::new();
        let mut bodies = std::collections::HashSet::new();
        if !compiled.match_expression.is_empty() {
            let mut cursor = statement.query([compiled.match_expression])?;
            while let Some(row) = cursor.next()? {
                let body: String = row.get(1)?;
                if bodies.insert(body.clone()) {
                    rows.push(Row { cursor: row.get(0)?, body, score: row.get(2)? });
                    if rows.len() >= request.limit {
                        break;
                    }
                }
            }
        }
        let vm_steps = statement.get_status(StatementStatus::VmStep);
        Ok(ReadResult { rows, vm_steps, worker })
    };
    read().map_err(|e| {
        e.downcast_ref::<Failure>().cloned().unwrap_or_else(|| Failure::Storage(e.to_string()))
    })
}
