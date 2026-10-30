//! Per-statement page-cache attribution for reader connections (experiment
//! tooling, `dev/experiments/ac013-vector-reads/`).
//!
//! When `FATHOMDB_PERF_PAGE_TRACE` is set at open, every reader connection
//! gets an `sqlite3_trace_v2` (`STMT` | `PROFILE`) callback. At each
//! statement start it records the connection's cumulative
//! `SQLITE_DBSTATUS_CACHE_HIT` / `_CACHE_MISS`; at each statement end it
//! records the delta. Statements that start while another is open on the
//! same connection (the statements `vec0` prepares inside its `xFilter`, or a
//! Rust caller interleaving two statements) are children: their inclusive
//! cost is subtracted from the parent's, so `exclusive` sums to the
//! connection total over the traced window. Page reads made outside any
//! statement are not attributed here; compare with the per-worker totals.
//! The `PROFILE` event's elapsed nanoseconds are attributed the same way
//! (`inclusive_ns` / `exclusive_ns`).
//!
//! Page requests made through `sqlite3_blob_read` inside a `vec0` callback
//! belong to the innermost statement open at that moment: the KNN scan's
//! chunk blob reads land on the chunk iterator statement, and a point
//! lookup's vector blob read (made after its rowid statement has been reset)
//! lands on the statement that drives the lookup.
//!
//! Only compiled with `test-hooks`. The aggregate is process-global.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::os::raw::{c_int, c_uint, c_void};
use std::sync::Mutex;

use rusqlite::Connection;

/// Aggregated cost of one SQL text over the traced window.
#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct PageTraceRow {
    /// Whitespace-collapsed SQL text (at most 240 characters).
    pub sql: String,
    /// Runs that started while another statement was open on the connection.
    pub nested_runs: u64,
    /// Completed runs.
    pub runs: u64,
    pub inclusive_hit: i64,
    pub inclusive_miss: i64,
    pub exclusive_hit: i64,
    pub exclusive_miss: i64,
    /// Elapsed nanoseconds from `SQLITE_TRACE_PROFILE`, summed over runs. A
    /// run's time spans its first step to its reset, so it includes the
    /// caller's work between steps (row decoding in a `query_map` closure).
    pub inclusive_ns: u64,
    /// `inclusive_ns` minus the time of statements nested inside it.
    pub exclusive_ns: u64,
}

struct Frame {
    stmt: usize,
    hit0: i64,
    miss0: i64,
    child_hit: i64,
    child_miss: i64,
    child_ns: u64,
    nested: bool,
}

thread_local! {
    static STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

static AGGREGATE: Mutex<BTreeMap<String, PageTraceRow>> = Mutex::new(BTreeMap::new());

fn cache_counters(db: *mut rusqlite::ffi::sqlite3) -> (i64, i64) {
    let mut hit: c_int = 0;
    let mut miss: c_int = 0;
    let mut hiwtr: c_int = 0;
    // SAFETY: `db` comes from `sqlite3_db_handle` of a live statement inside
    // a trace callback on the thread that owns the connection.
    unsafe {
        rusqlite::ffi::sqlite3_db_status(
            db,
            rusqlite::ffi::SQLITE_DBSTATUS_CACHE_HIT,
            &mut hit,
            &mut hiwtr,
            0,
        );
        rusqlite::ffi::sqlite3_db_status(
            db,
            rusqlite::ffi::SQLITE_DBSTATUS_CACHE_MISS,
            &mut miss,
            &mut hiwtr,
            0,
        );
    }
    (i64::from(hit), i64::from(miss))
}

fn sql_key(stmt: *mut rusqlite::ffi::sqlite3_stmt) -> String {
    // SAFETY: `stmt` is the live statement the trace callback was given.
    let raw = unsafe { rusqlite::ffi::sqlite3_sql(stmt) };
    if raw.is_null() {
        return "<no sql>".to_string();
    }
    // SAFETY: `sqlite3_sql` returns a NUL-terminated UTF-8 string owned by
    // the statement.
    let text = unsafe { std::ffi::CStr::from_ptr(raw) }.to_string_lossy();
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(240).collect()
}

unsafe extern "C" fn trace_callback(
    event: c_uint,
    _ctx: *mut c_void,
    p: *mut c_void,
    x: *mut c_void,
) -> c_int {
    let stmt = p.cast::<rusqlite::ffi::sqlite3_stmt>();
    if stmt.is_null() {
        return 0;
    }
    // SAFETY: for STMT and PROFILE events `p` is the running statement.
    let db = unsafe { rusqlite::ffi::sqlite3_db_handle(stmt) };
    let (hit, miss) = cache_counters(db);
    if event == rusqlite::ffi::SQLITE_TRACE_STMT as c_uint {
        STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let nested = !stack.is_empty();
            stack.push(Frame {
                stmt: stmt as usize,
                hit0: hit,
                miss0: miss,
                child_hit: 0,
                child_miss: 0,
                child_ns: 0,
                nested,
            });
        });
    } else if event == rusqlite::ffi::SQLITE_TRACE_PROFILE as c_uint {
        // SAFETY: for PROFILE events `x` points to the statement's elapsed
        // time as an `sqlite3_uint64` of nanoseconds.
        let ns = if x.is_null() { 0 } else { unsafe { *x.cast::<u64>() } };
        let finished = STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let position = stack.iter().rposition(|frame| frame.stmt == stmt as usize)?;
            let frame = stack.remove(position);
            let inclusive_hit = hit - frame.hit0;
            let inclusive_miss = miss - frame.miss0;
            if position > 0 {
                let parent = &mut stack[position - 1];
                parent.child_hit += inclusive_hit;
                parent.child_miss += inclusive_miss;
                parent.child_ns += ns;
            }
            Some((frame, inclusive_hit, inclusive_miss))
        });
        if let Some((frame, inclusive_hit, inclusive_miss)) = finished {
            let key = sql_key(stmt);
            if let Ok(mut aggregate) = AGGREGATE.lock() {
                let row = aggregate
                    .entry(key.clone())
                    .or_insert_with(|| PageTraceRow { sql: key, ..PageTraceRow::default() });
                row.runs += 1;
                row.nested_runs += u64::from(frame.nested);
                row.inclusive_hit += inclusive_hit;
                row.inclusive_miss += inclusive_miss;
                row.exclusive_hit += inclusive_hit - frame.child_hit;
                row.exclusive_miss += inclusive_miss - frame.child_miss;
                row.inclusive_ns += ns;
                row.exclusive_ns += ns.saturating_sub(frame.child_ns);
            }
        }
    }
    0
}

/// Install the tracer on a reader connection when `FATHOMDB_PERF_PAGE_TRACE`
/// is set. Called at reader open, before the engine's profile callback; the
/// two coexist because `sqlite3_profile` keeps the `trace_v2` mask bits.
pub(crate) fn maybe_install_page_trace(connection: &Connection) {
    if std::env::var_os("FATHOMDB_PERF_PAGE_TRACE").is_none() {
        return;
    }
    let mask = (rusqlite::ffi::SQLITE_TRACE_STMT | rusqlite::ffi::SQLITE_TRACE_PROFILE) as c_uint;
    // SAFETY: the callback uses no user data and only thread-local and
    // process-global state, so it stays valid for the connection's lifetime.
    unsafe {
        rusqlite::ffi::sqlite3_trace_v2(
            connection.handle(),
            mask,
            Some(trace_callback),
            std::ptr::null_mut(),
        );
    }
}

/// Take and clear the aggregate, sorted by exclusive hits plus misses,
/// largest first.
#[doc(hidden)]
pub fn take_page_trace_for_test() -> Vec<PageTraceRow> {
    let mut rows: Vec<PageTraceRow> = match AGGREGATE.lock() {
        Ok(mut aggregate) => std::mem::take(&mut *aggregate).into_values().collect(),
        Err(_) => Vec::new(),
    };
    rows.sort_by_key(|row| std::cmp::Reverse(row.exclusive_hit + row.exclusive_miss));
    rows
}
