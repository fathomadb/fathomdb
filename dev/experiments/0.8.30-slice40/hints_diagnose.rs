//! Bounded companion to the unchanged MCP hints devloop gate.

use fathomdb_mcp::seams::{
    plan_search_hints, prepare_search_page, shape_planned_search_page, shape_search_page,
    PreparedSearchPage, SearchPageInput, SyntheticHit,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

const TRIALS: usize = 31;
const REPETITIONS: usize = 15;

fn page(hits: u32, hints: bool) -> PreparedSearchPage {
    let ranked = (0..hits)
        .map(|index| SyntheticHit {
            reference: format!("l:note:{index:03}"),
            kind: "note".into(),
            body: "river ".repeat(if index % 2 == 0 { 34 } else { 683 }),
            source_id: Some("fixture".into()),
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

fn median(samples: &mut [Duration]) -> f64 {
    samples.sort_unstable();
    samples[samples.len() / 2].as_secs_f64() * 1e6
}

fn stage<T>(limit: u32, name: &str, run: impl Fn(usize) -> T) {
    let mut paired_deltas = Vec::with_capacity(TRIALS);
    let mut off_medians = Vec::with_capacity(TRIALS);
    let mut on_medians = Vec::with_capacity(TRIALS);
    for trial in 0..TRIALS {
        let mut elapsed = [[Duration::ZERO; REPETITIONS]; 2];
        for repeat in 0..REPETITIONS {
            let first = (trial + repeat) % 2;
            for arm in [first, 1 - first] {
                let start = Instant::now();
                let result = run(arm);
                elapsed[arm][repeat] = start.elapsed();
                black_box(result);
            }
        }
        let off_us = median(&mut elapsed[0]);
        let on_us = median(&mut elapsed[1]);
        off_medians.push(off_us);
        on_medians.push(on_us);
        paired_deltas.push(on_us - off_us);
        println!("trial,{limit},{name},{trial},{off_us:.3},{on_us:.3},{:.3}", on_us - off_us);
    }
    off_medians.sort_by(f64::total_cmp);
    on_medians.sort_by(f64::total_cmp);
    paired_deltas.sort_by(f64::total_cmp);
    println!(
        "summary,{limit},{name},off_p50_us={:.3},on_p50_us={:.3},delta_p50_us={:.3},delta_p99_us={:.3}",
        off_medians[TRIALS / 2],
        on_medians[TRIALS / 2],
        paired_deltas[TRIALS / 2],
        paired_deltas[TRIALS - 1],
    );
}

fn main() {
    for limit in [10, 100] {
        let pages = [page(limit, false), page(limit, true)];
        let planned = pages.each_ref().map(plan_search_hints);
        for (arm, page) in pages.iter().enumerate() {
            let full = shape_search_page(page);
            let split = shape_planned_search_page(page, &planned[arm]);
            assert_eq!(full.kept, limit as usize);
            assert_eq!(full.iterations, 1);
            assert_eq!(full.structured, split.structured);
            println!(
                "output,{limit},{arm},kept={},iterations={},rendered_structured_bytes={}",
                full.kept,
                full.iterations,
                full.text.len(),
            );
        }
        stage(limit, "plan", |arm| plan_search_hints(&pages[arm]));
        stage(limit, "preplanned-shape", |arm| {
            shape_planned_search_page(&pages[arm], &planned[arm])
        });
        stage(limit, "combined", |arm| shape_search_page(&pages[arm]));
    }
}
