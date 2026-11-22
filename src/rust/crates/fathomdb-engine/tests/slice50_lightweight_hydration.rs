//! Complete lexical ranks with winner-only Rust body hydration.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, IdSpace, InitialState, PreparedWrite, ReadView, SearchHit, SearchResult, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;

struct Meter;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);

fn count(size: usize) {
    if ACTIVE.load(Ordering::SeqCst) {
        REQUESTED.fetch_add(size, Ordering::SeqCst);
    }
}

unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Meter = Meter;

struct StopMeter;
impl Drop for StopMeter {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

fn allocated<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    assert!(!ACTIVE.load(Ordering::SeqCst));
    REQUESTED.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    let stop = StopMeter;
    let output = operation();
    drop(stop);
    (output, REQUESTED.load(Ordering::SeqCst))
}

struct RestoreForce(Option<std::ffi::OsString>);
impl RestoreForce {
    fn capture() -> Self {
        Self(std::env::var_os("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST"))
    }
}
impl Drop for RestoreForce {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => unsafe {
                std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", value)
            },
            None => unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") },
        }
    }
}

#[derive(Debug)]
struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice50-allocation", "v1", 8)
    }
    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

const MATCHES: usize = 512;
const BODY_BYTES: usize = 64 * 1024;
const VECTOR_INDEX: usize = 255;
const LIMIT: usize = 50;
const QUERY: &str = "slice50allocation";

fn body(index: usize) -> String {
    let mut body = format!("{QUERY} row{index:06} ");
    while body.len() < BODY_BYTES {
        body.push_str("padding ");
    }
    body.truncate(BODY_BYTES);
    body
}

fn search(engine: &Engine) -> SearchResult {
    engine
        .search_reranked_view_with_limit(
            QUERY,
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
            LIMIT,
        )
        .unwrap()
}

fn assert_manual_ranking(result: &SearchResult, cursors: &[u64]) {
    // All bodies have equal token lengths/frequencies, so text ranks are write
    // order. The only vector is text rank256: 1/31 + 3/286 lies strictly
    // between text-only ranks40 and41. A top200 lexical shortcut loses it.
    let expected: Vec<usize> = (0..40).chain([VECTOR_INDEX]).chain(40..49).collect();
    assert_eq!(result.results.len(), LIMIT);
    for (hit, index) in result.results.iter().zip(expected) {
        assert_eq!(hit.id, IdSpace::logical(format!("row:{index:06}")));
        assert_eq!(hit.write_cursor, cursors[index]);
        assert_eq!(hit.source_id.as_deref(), Some(format!("src:{index:06}").as_str()));
        assert_eq!(hit.body, body(index));
        let expected_score = if index == VECTOR_INDEX {
            1.0_f64 / 31.0 + 3.0_f64 / 286.0
        } else {
            3.0_f64 / (31.0 + index as f64)
        };
        assert_eq!(hit.score.to_bits(), expected_score.to_bits());
    }
}

#[test]
fn full_lexical_ranks_do_not_allocate_every_matching_body() {
    // The meter is process-wide so real reader-worker allocations are visible.
    // Calibrate on another thread outside all product calls and fixture setup.
    let (length, calibration) = allocated(|| {
        std::thread::spawn(|| {
            let bytes = vec![0_u8; BODY_BYTES * 2];
            std::hint::black_box(&bytes);
            bytes.len()
        })
        .join()
        .unwrap()
    });
    assert_eq!(length, BODY_BYTES * 2);
    assert!(calibration >= BODY_BYTES * 2);

    let _restore = RestoreForce::capture();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(format!("lightweight{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let writes: Vec<_> = (0..MATCHES)
        .map(|index| PreparedWrite::Node {
            kind: if index == VECTOR_INDEX { "doc" } else { "note" }.into(),
            body: body(index),
            logical_id: Some(format!("row:{index:06}")),
            source_id: SourceId::new(format!("src:{index:06}")).unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        })
        .collect();
    let receipt = opened.engine.write(&writes).unwrap();
    drop(writes);
    opened.engine.drain(10_000).unwrap();
    let raw =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let vectors: usize =
        raw.query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row.get(0)).unwrap();
    assert_eq!(vectors, 1, "the fixture has exactly one hydrated vector candidate");
    raw.close().unwrap();

    // One test in this process; no search or projection work overlaps flag changes.
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let reference = search(&opened.engine);
    assert_manual_ranking(&reference, &receipt.row_cursors);
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    let (result, bytes) = allocated(|| search(&opened.engine));
    assert_eq!(result, reference);
    assert_manual_ranking(&result, &receipt.row_cursors);

    // Two body copies per vector/winner plus four fixed body-sized allowances;
    // eight rounded public-hit metadata footprints per match cover growing rank,
    // fusion, hash and sort containers without prescribing their private layout.
    // This bounds actual cumulative Rust allocation, not SQLite body reads/I/O.
    let metadata = 8 * std::mem::size_of::<SearchHit>().next_power_of_two();
    let bound = (2 * vectors + 2 * LIMIT + 4) * BODY_BYTES + MATCHES * metadata;
    assert!(bytes <= bound,
        "complete lexical ranking must defer Rust body hydration to winners: allocated {bytes} bytes, bound {bound}, {MATCHES} matches of {BODY_BYTES} bytes");
    opened.engine.close().unwrap();
}
