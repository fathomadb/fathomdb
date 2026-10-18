//! Hints overhead, devloop tier (hints-sidecar § 4.2, § 4.3.2; review fix 3):
//! the time hints add to one search shaping, measured through the shaping
//! seam on synthetic 10- and 100-hit pages with bodies of 0.2-4 KB, half over
//! `snippet_bytes`. 31 trials interleave off and on; each trial takes the
//! median of its repetitions, and the result is the median of the trial
//! medians. `max_tokens` is set so the budget loop renders once.
//!
//! BLOCK: at `limit` 100, at most 200 µs added per budget-loop iteration.
//! WARN (notify-only): at `limit` 10, more than 10 µs p50 or 25 µs p99.

use std::time::{Duration, Instant};

use fathomdb_mcp::seams::{
    prepare_search_page, shape_search_page, PreparedSearchPage, SearchPageInput, SyntheticHit,
};

const TRIALS: usize = 31;
const REPETITIONS: usize = 15;

fn page(hits: u32, hints: bool) -> PreparedSearchPage {
    let ranked = (0..hits)
        .map(|index| SyntheticHit {
            reference: format!("l:note:{index:03}"),
            kind: "note".to_string(),
            // 204 bytes or 4 098 bytes, alternating: half over snippet_bytes.
            body: "river ".repeat(if index % 2 == 0 { 34 } else { 683 }),
            source_id: Some("fixture".to_string()),
            score: 1.0 / f64::from(index + 1),
        })
        .collect();
    prepare_search_page(&SearchPageInput {
        ranked,
        limit: hits as usize,
        snippet_bytes: 300,
        max_tokens: 1_000_000,
        hints: Some(hints),
        ..Default::default()
    })
}

fn median(values: &mut [Duration]) -> Duration {
    values.sort();
    values[values.len() / 2]
}

/// Per-trial `on - off` medians, in microseconds, and the iteration count.
fn deltas(hits: u32) -> (Vec<f64>, usize) {
    let off = page(hits, false);
    let on = page(hits, true);
    let iterations = shape_search_page(&on).iterations;
    assert_eq!(iterations, 1, "max_tokens keeps the budget loop to one render");
    assert_eq!(shape_search_page(&off).iterations, 1);
    let mut trial_deltas = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        let mut off_times = Vec::with_capacity(REPETITIONS);
        let mut on_times = Vec::with_capacity(REPETITIONS);
        for _ in 0..REPETITIONS {
            for (prepared, times) in [(&off, &mut off_times), (&on, &mut on_times)] {
                let started = Instant::now();
                let shaped = shape_search_page(prepared);
                times.push(started.elapsed());
                std::hint::black_box(shaped);
            }
        }
        let delta = median(&mut on_times).as_secs_f64() - median(&mut off_times).as_secs_f64();
        trial_deltas.push(delta * 1e6);
    }
    trial_deltas.sort_by(f64::total_cmp);
    (trial_deltas, iterations)
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index]
}

#[test]
fn hints_overhead_devloop() {
    let (at_10, _) = deltas(10);
    let p50_10 = percentile(&at_10, 0.5);
    let p99_10 = percentile(&at_10, 0.99);
    let (at_100, iterations) = deltas(100);
    let p50_100 = percentile(&at_100, 0.5);
    let per_iteration = p50_100 / iterations as f64;
    println!(
        "hints overhead: limit 10 p50={p50_10:.1}us p99={p99_10:.1}us; \
         limit 100 p50={p50_100:.1}us ({iterations} iteration, {per_iteration:.1}us per iteration)"
    );
    if p50_10 > 10.0 {
        println!("WARN: hints add {p50_10:.1}us p50 at limit 10 (notify bound 10us)");
    }
    if p99_10 > 25.0 {
        println!("WARN: hints add {p99_10:.1}us p99 at limit 10 (notify bound 25us)");
    }
    assert!(
        per_iteration <= 200.0,
        "BLOCK: hints add {per_iteration:.1}us per budget-loop iteration at limit 100 (> 200us)"
    );
}
