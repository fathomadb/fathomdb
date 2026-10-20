//! Adapter-only test seams (feature `test-seams`): hold a store's open or
//! its engine calls on the blocking thread, panic inside a tool handler, or
//! send a burst of progress notifications from one.
//! They live in the adapter's bridge and need no engine hook (MR-003.1,
//! MR-033.1, MR-002.2).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

#[derive(Default)]
struct State {
    open_blocked: BTreeSet<String>,
    calls_blocked: BTreeSet<String>,
    /// Calls held by `block_calls`, per store, oldest first.
    held: BTreeMap<String, VecDeque<u64>>,
    /// Held calls let through one at a time by `release_one_call`.
    released: BTreeSet<u64>,
    next_call: u64,
    panicking_tools: BTreeSet<String>,
    panicking_calls: BTreeSet<String>,
    panicking_waiters: BTreeSet<String>,
    admitted_blocked: BTreeSet<String>,
    arrivals_blocked: BTreeSet<String>,
    admitted_held: BTreeMap<String, usize>,
    progress_bursts: BTreeMap<String, u32>,
}

/// Injectable delays and faults, shared by every clone.
#[derive(Clone, Default)]
pub struct Seams {
    inner: Arc<(Mutex<State>, Condvar)>,
}

impl Seams {
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn wait_while(&self, blocked: impl Fn(&State) -> bool) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap_or_else(|poison| poison.into_inner());
        while blocked(&state) {
            state = condvar.wait(state).unwrap_or_else(|poison| poison.into_inner());
        }
    }

    /// Hold every open of `store` before `Engine::open` until released.
    pub fn block_open(&self, store: &str) {
        self.state().open_blocked.insert(store.to_string());
    }

    /// Let held and future opens of `store` proceed.
    pub fn release_open(&self, store: &str) {
        self.state().open_blocked.remove(store);
        self.inner.1.notify_all();
    }

    /// Hold every engine call on `store` (on its blocking thread) until released.
    pub fn block_calls(&self, store: &str) {
        self.state().calls_blocked.insert(store.to_string());
    }

    /// Let held and future engine calls on `store` proceed.
    pub fn release_calls(&self, store: &str) {
        self.state().calls_blocked.remove(store);
        self.inner.1.notify_all();
    }

    /// Let the oldest call held on `store` proceed; later calls stay held.
    /// Returns false when no call is held.
    pub fn release_one_call(&self, store: &str) -> bool {
        let mut state = self.state();
        let Some(call) = state.held.get_mut(store).and_then(VecDeque::pop_front) else {
            return false;
        };
        state.released.insert(call);
        drop(state);
        self.inner.1.notify_all();
        true
    }

    /// How many calls on `store` are held by `block_calls` right now.
    pub fn held_calls(&self, store: &str) -> usize {
        self.state().held.get(store).map_or(0, VecDeque::len)
    }

    /// Release every held open and call, including calls held after admission.
    pub fn release_all(&self) {
        let mut state = self.state();
        state.open_blocked.clear();
        state.calls_blocked.clear();
        state.admitted_blocked.clear();
        state.arrivals_blocked.clear();
        drop(state);
        self.inner.1.notify_all();
    }

    /// Make every later call to `tool` panic inside its handler.
    pub fn panic_in_tool(&self, tool: &str) {
        self.state().panicking_tools.insert(tool.to_string());
    }

    /// Make every later engine call on `store` panic on its blocking thread.
    pub fn panic_calls(&self, store: &str) {
        self.state().panicking_calls.insert(store.to_string());
    }

    /// Undo [`Seams::panic_calls`].
    pub fn stop_panicking_calls(&self, store: &str) {
        self.state().panicking_calls.remove(store);
    }

    /// Make every later call on `store` that has to wait for an admission
    /// permit panic in its handler task, after joining the queue and before
    /// the wait.
    pub fn panic_queued_calls(&self, store: &str) {
        self.state().panicking_waiters.insert(store.to_string());
    }

    /// Undo [`Seams::panic_queued_calls`].
    pub fn stop_panicking_queued_calls(&self, store: &str) {
        self.state().panicking_waiters.remove(store);
    }

    /// Hold every call on `store` that has to wait for a permit right after
    /// it passed the arrival check (`queue_limit`), until released.
    pub fn hold_arrivals(&self, store: &str) {
        self.state().arrivals_blocked.insert(store.to_string());
    }

    /// Let calls held by [`Seams::hold_arrivals`] proceed.
    pub fn release_arrivals(&self, store: &str) {
        self.state().arrivals_blocked.remove(store);
    }

    /// Hold every call on `store` right after it gets its admission permit,
    /// before its store guard exists, until released.
    pub fn hold_admitted_calls(&self, store: &str) {
        self.state().admitted_blocked.insert(store.to_string());
    }

    /// Let calls held by [`Seams::hold_admitted_calls`] proceed.
    pub fn release_admitted_calls(&self, store: &str) {
        self.state().admitted_blocked.remove(store);
    }

    /// How many calls on `store` are held between permit and guard.
    pub fn admitted_held(&self, store: &str) -> usize {
        self.state().admitted_held.get(store).copied().unwrap_or(0)
    }

    /// Make every later call to `tool` that carries a progress token send `n`
    /// progress notifications (values 1..=n) before the tool body runs.
    pub fn burst_progress(&self, tool: &str, n: u32) {
        self.state().progress_bursts.insert(tool.to_string(), n);
    }

    pub(crate) fn before_open(&self, store: &str) {
        self.wait_while(|state| state.open_blocked.contains(store));
    }

    pub(crate) fn before_call(&self, store: &str) {
        self.hold_call(store);
        if self.state().panicking_calls.contains(store) {
            panic!("test seam: forced panic in an engine call on {store}");
        }
    }

    fn hold_call(&self, store: &str) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap_or_else(|poison| poison.into_inner());
        if !state.calls_blocked.contains(store) {
            return;
        }
        let call = state.next_call;
        state.next_call += 1;
        state.held.entry(store.to_string()).or_default().push_back(call);
        while state.calls_blocked.contains(store) && !state.released.contains(&call) {
            state = condvar.wait(state).unwrap_or_else(|poison| poison.into_inner());
        }
        state.released.remove(&call);
        if let Some(held) = state.held.get_mut(store) {
            held.retain(|other| *other != call);
        }
    }

    pub(crate) fn after_queued(&self, store: &str) {
        if self.state().panicking_waiters.contains(store) {
            panic!("test seam: forced panic in a queued call on {store}");
        }
    }

    // Runs on the async handler task, so it polls instead of parking the
    // thread on the condition variable.
    pub(crate) async fn after_arrival_check(&self, store: &str) {
        while self.state().arrivals_blocked.contains(store) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    // Runs on the async handler task, so it polls instead of parking the
    // thread on the condition variable.
    pub(crate) async fn after_permit(&self, store: &str) {
        if !self.state().admitted_blocked.contains(store) {
            return;
        }
        *self.state().admitted_held.entry(store.to_string()).or_default() += 1;
        while self.state().admitted_blocked.contains(store) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        *self.state().admitted_held.entry(store.to_string()).or_default() -= 1;
    }

    pub(crate) fn tool_panics(&self, tool: &str) -> bool {
        self.state().panicking_tools.contains(tool)
    }

    pub(crate) fn progress_burst(&self, tool: &str) -> u32 {
        self.state().progress_bursts.get(tool).copied().unwrap_or(0)
    }
}

/// One ranked hit of a synthetic search page for [`shape_search_page`].
#[derive(Clone, Debug, Default)]
pub struct SyntheticHit {
    /// The prefixed reference, for example `l:note:1`.
    pub reference: String,
    pub kind: String,
    /// The whole body; the seam selects its snippet as `search` does, with no match spans.
    pub body: String,
    pub source_id: Option<String>,
    pub score: f64,
}

/// A synthetic search answer: the engine's ranked list and the request
/// facts the shaping step reads. Build with `..Default::default()`.
#[derive(Clone, Debug, Default)]
pub struct SearchPageInput {
    /// The ranked list as the engine returned it (at most 100 hits).
    pub ranked: Vec<SyntheticHit>,
    /// Hits already served by earlier pages.
    pub offset: usize,
    /// The request's `limit`.
    pub limit: usize,
    pub snippet_bytes: usize,
    pub max_tokens: u32,
    /// Hybrid rather than text mode.
    pub hybrid: bool,
    /// `(arm, reason)` degradation entries, in any order.
    pub degradation: Vec<(String, String)>,
    /// Whether a hybrid run could use the vector arm (§ 3.3).
    pub hybrid_available: bool,
    /// The resolved hints setting; `None` skips hint planning altogether,
    /// the no-hints baseline of the allocation test.
    pub hints: Option<bool>,
}

/// A page prepared for shaping: the snippet cut is done, so measuring
/// [`shape_search_page`] measures the budget loop alone.
pub struct PreparedSearchPage {
    page: Vec<(crate::output::Hit, u64, Option<crate::output::PerHitOut>)>,
    page_limit: usize,
    offset: usize,
    max_tokens: u32,
    degradation: Vec<crate::output::Degradation>,
    canonical: crate::shape::CanonicalSearch,
    canonical_json: String,
    hybrid: bool,
    hybrid_available: bool,
    hints: Option<bool>,
}

/// What the search tool would answer for a prepared page.
#[derive(Clone, Debug)]
pub struct ShapedSearchPage {
    pub structured: serde_json::Value,
    pub text: String,
    /// Hits kept after the budget loop.
    pub kept: usize,
    /// Times the budget loop rendered the page.
    pub iterations: usize,
    /// The offset a continuation of this page resumes at, when it has one.
    pub next_offset: Option<usize>,
}

/// Cut the page of `input` to snippets, as `search` does before its budget
/// loop.
#[must_use]
pub fn prepare_search_page(input: &SearchPageInput) -> PreparedSearchPage {
    let (run_limit, page_limit) =
        crate::shape::page_window(input.offset, input.limit, crate::tools::MAX_RANKED);
    let page = input
        .ranked
        .iter()
        .take(run_limit)
        .skip(input.offset)
        .enumerate()
        .map(|(index, hit)| {
            let bytes = u32::try_from(input.snippet_bytes).unwrap_or(u32::MAX);
            // Synthetic hits carry no match spans, as a hit the engine did
            // not anchor; the options are the ones `search` requests.
            let snippet = fathomdb_sdk::snippet_for(
                &hit.body,
                &[],
                &fathomdb_sdk::SnippetOptions {
                    max_bytes: bytes,
                    max_windows: 1,
                    head_bytes: crate::config::snippet_head_bytes(bytes),
                    ..fathomdb_sdk::SnippetOptions::default()
                },
            );
            (
                crate::output::Hit {
                    reference: hit.reference.clone(),
                    kind: hit.kind.clone(),
                    snippet: snippet.text,
                    snippet_cut: snippet.cut,
                    snippet_anchor: snippet.anchor.as_str().to_string(),
                    highlights: None,
                    source_id: hit.source_id.clone(),
                    score: hit.score,
                    arm: "text".to_string(),
                    ce_score: None,
                },
                index as u64,
                None,
            )
        })
        .collect();
    let canonical = crate::shape::CanonicalSearch {
        query: "synthetic".to_string(),
        mode: if input.hybrid { "hybrid" } else { "text" },
        kind: None,
        source_type: None,
        status: None,
        created_after: None,
        attributes: Vec::new(),
        as_of: Some(0),
        limit: input.limit,
        explain: false,
    };
    let canonical_json = serde_json::to_string(&canonical).expect("serializes");
    PreparedSearchPage {
        page,
        page_limit,
        offset: input.offset,
        max_tokens: input.max_tokens,
        degradation: crate::shape::canonical_degradation(
            input
                .degradation
                .iter()
                .map(|(arm, reason)| crate::output::Degradation {
                    arm: arm.clone(),
                    reason: reason.clone(),
                })
                .collect(),
        ),
        canonical,
        canonical_json,
        hybrid: input.hybrid,
        hybrid_available: input.hybrid_available,
        hints: input.hints,
    }
}

/// Plan the hints of a prepared page, as `search` does before its budget
/// loop. `None` returns the empty plan without planning.
#[must_use]
pub fn plan_search_hints(page: &PreparedSearchPage) -> PlannedHints {
    PlannedHints(match page.hints {
        None => crate::hints::SearchPlan::default(),
        Some(enabled) => crate::hints::SearchPlan::new(
            enabled,
            &crate::hints::SearchPageFacts {
                page: &page.page,
                page_limit: page.page_limit,
                hybrid: page.hybrid,
                degradation: &page.degradation,
                hybrid_available: page.hybrid_available,
            },
        ),
    })
}

/// A planned set of hints for one page.
pub struct PlannedHints(crate::hints::SearchPlan);

/// Run the search budget loop on a prepared page with its planned hints.
#[must_use]
pub fn shape_planned_search_page(
    page: &PreparedSearchPage,
    plan: &PlannedHints,
) -> ShapedSearchPage {
    shape_with(page, &plan.0)
}

/// Plan the page's hints and run the search budget loop on it (epoch 1,
/// store `s`), as `search` does.
#[must_use]
pub fn shape_search_page(page: &PreparedSearchPage) -> ShapedSearchPage {
    shape_with(page, &plan_search_hints(page).0)
}

fn shape_with(page: &PreparedSearchPage, plan: &crate::hints::SearchPlan) -> ShapedSearchPage {
    let shaped = crate::tools::shape_search_page(&crate::tools::SearchShape {
        store: "s",
        epoch: 1,
        validity_instant: 0,
        degradation: &page.degradation,
        max_tokens: page.max_tokens,
        canonical: &page.canonical,
        canonical_json: &page.canonical_json,
        offset: page.offset,
        page: &page.page,
        page_limit: page.page_limit,
        trace: None,
        hosted: false,
        hints: plan,
    });
    ShapedSearchPage {
        next_offset: shaped.continuation.as_ref().map(|_| page.offset + shaped.kept),
        structured: shaped.rendered.structured,
        text: shaped.rendered.text,
        kept: shaped.kept,
        iterations: shaped.iterations,
    }
}
