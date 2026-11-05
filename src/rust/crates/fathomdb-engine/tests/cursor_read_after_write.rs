//! Concurrent reader/writer test that pins the read-after-write cursor
//! invariant from `dev/design/engine.md` § Cursor contract and AC-059b /
//! REQ-013 / REQ-055.
//!
//! Invariant under test: the number of canonical rows visible to a
//! `search` MUST be ≤ the search's reported `projection_cursor`. Because
//! every matching canonical row requires a successful write in this
//! fixture, the row count the search returned is a lower bound on "how
//! many writes the reader's snapshot saw"; that value can never exceed
//! the cursor the reader also returned without violating the cursor
//! contract. Repeated identical bodies may reuse a canonical row.
//!
//! Failure mode under test: the engine loads `next_cursor` from the
//! writer-side atomic before the reader connection runs its query; a
//! writer commit between those two events appears in the reader's WAL
//! snapshot but not in the reported cursor.
//!
//! Bounded operationally: 1,000 search calls, a 60 s anti-hang fuse (not a
//! product SLA), and a throttled writer.
//!
//! Runtime-budget category: long-run only (~1000-iteration race fixture).
//! `agent-verify.sh` skips this test entirely for runtime budget; the
//! ~1000-iteration race fixture is exercised only by `scripts/check.sh`
//! with `AGENT_LONG=1`. There is no smoke variant — the AC-059b evidence
//! comes exclusively from the long-run gate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_engine::{Engine, EngineError, PreparedWrite};
use tempfile::TempDir;

fn long_run_enabled() -> bool {
    std::env::var_os("AGENT_LONG").is_some()
}

#[derive(Debug, Default)]
struct WriterProgress {
    successful: usize,
    during_read: usize,
}

struct WriterCleanup {
    stop: Arc<AtomicBool>,
    read_phase: Option<mpsc::Sender<()>>,
    writer: Option<thread::JoinHandle<Result<WriterProgress, EngineError>>>,
}

impl WriterCleanup {
    fn finish(&mut self) -> Result<WriterProgress, EngineError> {
        self.stop.store(true, Ordering::Relaxed);
        self.read_phase.take();
        self.writer.take().expect("writer handle").join().expect("writer thread")
    }
}

impl Drop for WriterCleanup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Disconnect the rendezvous if a reader failure precedes its first search.
        self.read_phase.take();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

#[test]
fn projection_cursor_bounds_observed_row_count() {
    if !long_run_enabled() {
        return;
    }

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cursor_race.sqlite");
    let opened = Engine::open(&path).expect("open");
    let engine = Arc::new(opened.engine);

    let stop = Arc::new(AtomicBool::new(false));
    let (first_commit, first_committed) = mpsc::channel();
    let (read_phase, reading) = mpsc::channel();
    let (overlap_commit, overlap_committed) = mpsc::channel();
    let writer = {
        let engine = Arc::clone(&engine);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut progress = WriterProgress::default();
            while !stop.load(Ordering::Relaxed) {
                engine.write(&[PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "needle".to_string(),
                    source_id: fathomdb_engine::SourceId::new("test:fixture")
                        .expect("test source id"),
                    logical_id: None,
                    state: fathomdb_engine::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                }])?;
                progress.successful += 1;
                if progress.successful == 1 {
                    if first_commit.send(()).is_err()
                        || reading.recv_timeout(Duration::from_secs(60)).is_err()
                    {
                        return Ok(progress);
                    }
                } else {
                    progress.during_read += 1;
                    if progress.during_read == 1 && overlap_commit.send(()).is_err() {
                        return Ok(progress);
                    }
                }
                thread::sleep(Duration::from_micros(50));
            }
            Ok(progress)
        })
    };
    let mut cleanup = WriterCleanup { stop, read_phase: Some(read_phase), writer: Some(writer) };

    let iterations = 1000usize;
    let mut searches = 0usize;
    let mut nonempty = 0usize;
    let mut observed_matching_rows = 0u64;
    let mut violations = 0usize;
    let started = Instant::now();
    let read_result = (|| -> Result<(), String> {
        first_committed
            .recv_timeout(Duration::from_secs(60))
            .map_err(|error| format!("first successful write: {error}"))?;
        for i in 0..iterations {
            if started.elapsed() > Duration::from_secs(60) {
                return Err(format!(
                    "cursor invariant test exceeded 60 s wall clock at iteration {i}"
                ));
            }
            let result = engine.search("needle").map_err(|error| format!("search: {error:?}"))?;
            searches += 1;
            let row_count =
                result.results.iter().filter(|h| h.body.as_str() == "needle").count() as u64;
            observed_matching_rows += row_count;
            nonempty += usize::from(row_count != 0);
            if row_count > result.projection_cursor {
                violations += 1;
                if violations <= 5 {
                    eprintln!(
                        "iter {i}: rows_returned={row_count} > projection_cursor={}",
                        result.projection_cursor
                    );
                }
            }
            if i == 0 {
                cleanup
                    .read_phase
                    .take()
                    .expect("read phase sender")
                    .send(())
                    .map_err(|error| format!("start read phase: {error}"))?;
                overlap_committed
                    .recv_timeout(Duration::from_secs(60))
                    .map_err(|error| format!("successful write during read phase: {error}"))?;
            }
        }
        Ok(())
    })();
    // Normal completion propagates write failures; Drop still joins on every unwind path.
    let writer_result = cleanup.finish();
    match &writer_result {
        Ok(progress) => eprintln!("cursor_progress searches={searches} successful_writes={} during_read={} nonempty_searches={nonempty} observed_matching_rows={observed_matching_rows} violations={violations}", progress.successful, progress.during_read),
        Err(error) => eprintln!("cursor_progress searches={searches} writer_error={error:?} nonempty_searches={nonempty} observed_matching_rows={observed_matching_rows} violations={violations}"),
    }
    let progress = writer_result.expect("writer failed");
    read_result.expect("reader failed");
    assert_eq!(searches, iterations);
    assert!(progress.successful > 0, "writer must commit real rows");
    assert!(progress.during_read > 0, "writer must commit during the read phase");
    assert!(nonempty > 0 && observed_matching_rows > 0, "searches must observe matching rows");
    assert_eq!(
        violations, 0,
        "cursor invariant violated {violations}/{iterations} times: search returned more \
         matching rows than the reported projection_cursor permits — projection_cursor was \
         derived from a writer-side atomic that races ahead of the reader's WAL snapshot",
    );
}
