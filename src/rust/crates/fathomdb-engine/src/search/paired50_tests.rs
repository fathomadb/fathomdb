use crate::{Engine, InitialState, PreparedWrite, ReadView, SourceId};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const SELECTOR: &str = "FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT";
static SERIAL: Mutex<()> = Mutex::new(());
static COUNTING: AtomicBool = AtomicBool::new(false);
static BYTES: AtomicUsize = AtomicUsize::new(0);
struct Meter;
fn count(size: usize) {
    if COUNTING.load(Ordering::SeqCst) {
        BYTES.fetch_add(size, Ordering::SeqCst);
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
static METER: Meter = Meter;
struct Restore(Option<std::ffi::OsString>);
impl Drop for Restore {
    fn drop(&mut self) {
        COUNTING.store(false, Ordering::SeqCst);
        match &self.0 {
            Some(value) => unsafe { std::env::set_var(SELECTOR, value) },
            None => unsafe { std::env::remove_var(SELECTOR) },
        }
    }
}
struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("paired50-selector", "v1", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}
fn node(index: usize, kind: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body,
        logical_id: Some(format!("record{index}")),
        source_id: SourceId::new(format!("test:paired50:{index}")).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn allocated<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    BYTES.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    let result = operation();
    COUNTING.store(false, Ordering::SeqCst);
    (result, BYTES.load(Ordering::SeqCst))
}

#[test]
fn eager_selector_materializes_losing_bodies_without_changing_record_results() {
    let _serial = SERIAL.lock().unwrap();
    let _restore = Restore(std::env::var_os(SELECTOR));
    unsafe { std::env::remove_var(SELECTOR) };
    let directory = tempfile::tempdir().unwrap();
    let engine =
        Engine::open_with_embedder_for_test(directory.path().join("pair.sqlite"), Arc::new(Fixed))
            .unwrap()
            .engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let body = format!("needle {}", "padding ".repeat(512));
    let writes: Vec<_> = (0..48)
        .map(|index| node(index, "note", body.clone()))
        .chain([node(48, "doc", "vector only".into())])
        .collect();
    let receipt = engine.write(&writes).unwrap();
    engine.drain(10_000).unwrap();
    let search = || {
        engine
            .search_reranked_view_with_limit(
                "needle",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                1,
            )
            .unwrap()
    };
    let (thin, thin_bytes) = allocated(search);
    let direct = engine.search_text_only("needle").unwrap();
    unsafe { std::env::set_var(SELECTOR, "1") };
    let (eager, eager_bytes) = allocated(search);
    assert_eq!(eager, thin);
    assert_eq!(eager.results.len(), 1);
    assert_eq!(eager.results[0].write_cursor, receipt.row_cursors[0]);
    assert_eq!(eager.results[0].source_id.as_deref(), Some("test:paired50:0"));
    assert_eq!(eager.results[0].score.to_bits(), (3.0_f64 / 31.0).to_bits());
    assert_eq!(engine.search_text_only("needle").unwrap(), direct);
    // All48 lexical bodies are materialized by the retained eager branch;
    // the optimized route only owns the selected text winner plus its vector.
    let all_lexical_bytes = 48 * body.len();
    assert!(
        thin_bytes < all_lexical_bytes,
        "thin fixture failed to separate body copies: {thin_bytes}"
    );
    assert!(
        eager_bytes >= all_lexical_bytes,
        "selector failed to materialize losing bodies: {eager_bytes} < {all_lexical_bytes}"
    );
    engine.close().unwrap();
}

#[cfg(feature = "slice30-observer")]
#[test]
fn work_diagnostics_count_actual_bodies_and_utf8_bytes_per_operation() {
    let _serial = SERIAL.lock().unwrap();
    let _restore = Restore(std::env::var_os(SELECTOR));
    let directory = tempfile::tempdir().unwrap();
    let engine =
        Engine::open_with_embedder_for_test(directory.path().join("work.sqlite"), Arc::new(Fixed))
            .unwrap()
            .engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .write(&[
            node(0, "note", "needle café".into()),
            node(1, "note", "needle 梨".into()),
            node(2, "doc", "needle vector".into()),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    let guard = engine.begin_slice30_observation(128).unwrap();
    let mut previous = None;
    for (operation, mode, expected) in [
        (
            71,
            "0",
            serde_json::json!({"route":"winner_hydration", "lightweight_rows":3, "vector_bodies":1, "vector_body_bytes":13, "fts_bodies":1, "fts_body_bytes":12}),
        ),
        (
            72,
            "1",
            serde_json::json!({"route":"eager_text_identity", "lightweight_rows":0, "vector_bodies":1, "vector_body_bytes":13, "fts_bodies":3, "fts_body_bytes":35}),
        ),
    ] {
        unsafe { std::env::set_var(SELECTOR, mode) };
        let result = guard
            .with_operation(operation, || {
                engine.search_reranked_view_with_limit(
                    "needle",
                    None,
                    0,
                    false,
                    0.3,
                    0,
                    false,
                    &ReadView::default(),
                    2,
                )
            })
            .unwrap();
        assert_eq!(result.results.len(), 2);
        if let Some(previous) = previous {
            assert_eq!(result, previous);
        }
        previous = Some(result);
        let batch = guard.take();
        assert_eq!(batch.dropped_records, 0);
        assert!(batch.issues.is_empty());
        let work: Vec<_> =
            batch.records.iter().filter(|record| record.phase == "slice50_work").collect();
        assert_eq!(work.len(), 1, "actual materialization counts are unavailable");
        assert_eq!(work[0].operation_id, operation);
        assert_eq!(work[0].status, "ok");
        assert_eq!(work[0].facts, expected);
    }
    engine.search_text_only("needle").unwrap();
    assert!(
        guard.take().records.is_empty(),
        "unscoped work must not enter this operation's diagnostics"
    );
    drop(guard);
    engine.close().unwrap();
}
