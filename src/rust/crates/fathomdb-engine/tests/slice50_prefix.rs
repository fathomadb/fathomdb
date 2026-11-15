//! Record-start allocation bound and independent Unicode selector equivalence.

use fathomdb_engine::{
    snippet_for, snippet_for_full_reference_for_test, SnippetAnchor, SnippetOptions,
};
use proptest::prelude::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct Meter;
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static REQUESTED: Cell<usize> = const { Cell::new(0) };
}
fn count(bytes: usize) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            let _ = REQUESTED.try_with(|total| total.set(total.get() + bytes));
        }
    });
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
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
        count(bytes);
        unsafe { System.realloc(pointer, layout, bytes) }
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
        ACTIVE.with(|active| active.set(false));
    }
}
fn allocated<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    ACTIVE.with(|active| assert!(!active.get()));
    REQUESTED.with(|total| total.set(0));
    ACTIVE.with(|active| active.set(true));
    let stop = StopMeter;
    let value = operation();
    drop(stop);
    (value, REQUESTED.with(Cell::get))
}
fn options(bytes: u32) -> SnippetOptions {
    let mut options = SnippetOptions::default();
    options.max_bytes = bytes;
    options
}
const ASCII: &str = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda ";
const EXPECTED: &str = "alpha beta gamma delta epsilon zeta eta theta iota kappa …";

#[test]
fn record_start_allocation_is_bounded_independently_of_ascii_suffix_length() {
    let (calibration, bytes) = allocated(|| {
        let bytes = vec![0_u8; 65536];
        std::hint::black_box(&bytes);
        bytes
    });
    assert_eq!(calibration.len(), 65536);
    assert!(bytes >= 65536);
    drop(calibration);
    let options = options(62);
    for length in [8192_usize, 131072, 2097152] {
        let body = ASCII.repeat(length.div_ceil(ASCII.len()));
        let reference = snippet_for_full_reference_for_test(&body, &[], &options);
        assert_eq!(reference.text, EXPECTED);
        assert_eq!(reference.anchor, SnippetAnchor::Start);
        assert_eq!(reference.windows, [(0, 56)]);
        assert!(reference.spans.is_empty() && reference.highlights.is_empty() && reference.cut);
        let (actual, bytes) = allocated(|| snippet_for(&body, &[], &options));
        assert_eq!(actual, reference);
        // Four boundary arrays over the inspected budget+lookahead, plus four
        // output budgets for text/window bookkeeping; independent of the suffix.
        let bound = 4 * (options.max_bytes as usize + 1) * std::mem::size_of::<usize>()
            + 4 * options.max_bytes as usize;
        assert!(bytes<=bound,"record-start selector must allocate by bounded prefix: requested {bytes}, bound {bound}, body bytes {}",body.len());
    }
}

#[test]
fn prefix_examples_preserve_unicode_graphemes_and_body_byte_coordinates() {
    let options = options(60);
    let nfd = "e\u{301}".repeat(40);
    let nfd_snippet = snippet_for(&nfd, &[], &options);
    assert_eq!(nfd_snippet.text, format!("{} …", "e\u{301}".repeat(18)));
    assert_eq!(nfd_snippet.windows, [(0, 54)]);
    assert_eq!(nfd_snippet.anchor, SnippetAnchor::Start);
    for body in [
        nfd,
        "👩\u{200d}🔬".repeat(40),
        "🇺🇸🇨🇦".repeat(30),
        "a\u{2060}b ".repeat(100),
        "a\r\nb ".repeat(100),
        " ".repeat(1000),
        "a".repeat(1000),
    ] {
        let actual = snippet_for(&body, &[], &options);
        assert_eq!(actual, snippet_for_full_reference_for_test(&body, &[], &options));
        assert!(actual.text.len() <= 60);
        for &(start, end) in &actual.windows {
            assert!(body.is_char_boundary(start as usize) && body.is_char_boundary(end as usize));
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn arbitrary_unicode_and_ascii_prefix_suffixes_equal_full_selector(
        chars in proptest::collection::vec(any::<char>(),0..512),
        raw in proptest::collection::vec((0_u32..1500,0_u32..1500),0..8),
        budget in prop::sample::select(vec![0,39,40,41,60,62,300,3999,4000,4001,u32::MAX]),
        windows in 0_u8..5, head in any::<u32>(), highlights in any::<bool>(),
    ) {
        let suffix: String=chars.into_iter().collect();
        let mut options=options(budget); options.max_windows=windows; options.head_bytes=head; options.highlights=highlights;
        let clamped=budget.clamp(40,4000) as usize;
        let ascii=ASCII.repeat((clamped+1).div_ceil(ASCII.len()));
        for body in [&suffix, &format!("{ascii}{suffix}")] {
            let valid=body.char_indices().nth(2).map(|(start,ch)|vec![(start as u32,(start+ch.len_utf8()) as u32)]).unwrap_or_default();
            for spans in [&[][..],raw.as_slice(),valid.as_slice()] {
                prop_assert_eq!(snippet_for(body,spans,&options),snippet_for_full_reference_for_test(body,spans,&options));
            }
        }
    }
}
