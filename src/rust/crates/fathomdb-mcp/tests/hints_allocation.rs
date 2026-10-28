//! Hints allocation deltas (hints-sidecar § 4.2, § 4.3.1; review fix 2): a
//! counting global allocator, counting on the test thread only, around the
//! shaping of one fixed 100-hit synthetic page.
//!
//! BLOCK: hints off allocates exactly what the no-hints baseline does, and
//! planning hints on allocates at most 1 + refs. The budget loop's on-versus-
//! off delta is printed (notify-only).

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use fathomdb_mcp::seams::{
    plan_search_hints, prepare_search_page, shape_planned_search_page, shape_search_page,
    PreparedSearchPage, SearchPageInput, SyntheticHit,
};

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

fn count_one() {
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
}

// SAFETY: every call forwards to the system allocator unchanged; the
// thread-local counter is const-initialized, so counting never allocates.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_one();
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_one();
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_one();
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocations<T>(f: impl FnOnce() -> T) -> (u64, T) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (ALLOCATIONS.with(Cell::get) - before, value)
}

/// 100 hits, bodies of 200 bytes and 4 KB alternating, so half are over
/// `snippet_bytes` (300) and 50 refs are cut.
fn page(hints: Option<bool>) -> PreparedSearchPage {
    let ranked = (0..100)
        .map(|index| SyntheticHit {
            reference: format!("l:note:{index:03}"),
            kind: "note".to_string(),
            body: "river ".repeat(if index % 2 == 0 { 34 } else { 683 }),
            source_id: Some("fixture".to_string()),
            score: 1.0 / f64::from(index + 1),
        })
        .collect();
    prepare_search_page(&SearchPageInput {
        ranked,
        limit: 100,
        snippet_bytes: 300,
        max_tokens: 1_000_000,
        hints,
        ..Default::default()
    })
}

#[test]
fn hints_allocation_deltas() {
    let baseline_page = page(None);
    let off_page = page(Some(false));
    let on_page = page(Some(true));
    // Warm every path once so one-time initialization is not counted.
    for prepared in [&baseline_page, &off_page, &on_page] {
        let _ = shape_search_page(prepared);
    }

    let (baseline, shaped) = allocations(|| shape_search_page(&baseline_page));
    assert_eq!(shaped.iterations, 1);
    let (off, shaped_off) = allocations(|| shape_search_page(&off_page));
    assert_eq!(off, baseline, "BLOCK: hints off allocates exactly the no-hints baseline");
    assert!(shaped_off.structured.get("hints").is_none());

    let refs = 50_u64;
    let (planning_off, _) = allocations(|| plan_search_hints(&off_page));
    assert_eq!(planning_off, 0, "BLOCK: planning with hints off allocates nothing");
    let (planning_on, plan_on) = allocations(|| plan_search_hints(&on_page));
    assert!(
        planning_on <= 1 + refs,
        "BLOCK: planning allocated {planning_on} > 1 + refs ({})",
        1 + refs
    );

    let plan_off = plan_search_hints(&off_page);
    let (render_off, shaped_off) = allocations(|| shape_planned_search_page(&off_page, &plan_off));
    let (render_on, shaped_on) = allocations(|| shape_planned_search_page(&on_page, &plan_on));
    assert_eq!(shaped_on.iterations, 1);
    assert_eq!(shaped_off.iterations, 1);
    let listed = shaped_on.structured["hints"][0]["refs"].as_array().map_or(0, Vec::len);
    assert_eq!(listed as u64, refs, "the fixed page cuts 50 refs");
    let delta = render_on.saturating_sub(render_off);
    let bound = (1 + refs) * shaped_on.iterations as u64;
    let multiplier = delta as f64 / bound as f64;
    println!(
        "hints allocation deltas: baseline={baseline} off={off} planning_on={planning_on} \
         render_off={render_off} render_on={render_on} render_delta={delta} \
         (1+refs)*iterations={bound} multiplier={multiplier:.2}"
    );
    if delta > bound {
        println!(
            "NOTIFY: render-step allocation delta {delta} exceeds (1+refs)*iterations {bound}"
        );
    }
}
