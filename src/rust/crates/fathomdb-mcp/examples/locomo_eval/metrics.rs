//! Retrieval metrics over one ranked list of references and a gold set.
//!
//! `ranked` is in rank order (rank 1 first); duplicates count once, at their
//! first rank. `gold` is the set of relevant references and is never empty for
//! a scored query.

use std::collections::BTreeSet;

/// Relevant references among the first `k` of `ranked`, divided by `k` (not by
/// the number returned: a list shorter than `k` is penalized).
pub fn precision_at_k(ranked: &[String], gold: &BTreeSet<String>, k: usize) -> f64 {
    if k == 0 {
        return 0.0;
    }
    relevant_in_top(ranked, gold, k) as f64 / k as f64
}

/// Relevant references among the first `k` of `ranked`, divided by the gold
/// set size.
pub fn recall_at_k(ranked: &[String], gold: &BTreeSet<String>, k: usize) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    relevant_in_top(ranked, gold, k) as f64 / gold.len() as f64
}

/// Reciprocal rank of the first relevant reference within the first `k`;
/// 0 when none is there.
pub fn reciprocal_rank_at_k(ranked: &[String], gold: &BTreeSet<String>, k: usize) -> f64 {
    ranked
        .iter()
        .take(k)
        .position(|reference| gold.contains(reference))
        .map_or(0.0, |index| 1.0 / (index + 1) as f64)
}

/// 1 when any of the first `k` references is relevant, else 0.
pub fn hit_at_k(ranked: &[String], gold: &BTreeSet<String>, k: usize) -> f64 {
    if relevant_in_top(ranked, gold, k) > 0 {
        1.0
    } else {
        0.0
    }
}

/// Arithmetic mean; 0 for no values.
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// A paired-bootstrap estimate: the observed mean difference and the
/// percentile 95% interval of the resampled means.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BootstrapCi {
    pub mean: f64,
    pub low: f64,
    pub high: f64,
}

/// SplitMix64: a small fixed-seed generator so the interval is reproducible
/// without a `rand` dependency.
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`n > 0`), by rejection so there is no modulo bias.
    pub fn below(&mut self, n: u64) -> u64 {
        let _ = n;
        0
    }
}

/// Nearest-rank percentile of an ascending slice: the value at rank
/// `ceil(q * len)`, clamped to `1..=len`. 0 for an empty slice.
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    let _ = (sorted, q);
    0.0
}

/// Paired bootstrap over per-query differences (`hybrid - text`): draw
/// `resamples` resamples of the queries with replacement, take each mean, and
/// report the 2.5th and 97.5th nearest-rank percentiles.
pub fn paired_bootstrap_ci(diffs: &[f64], resamples: usize, seed: u64) -> BootstrapCi {
    let _ = (diffs, resamples, seed);
    BootstrapCi { mean: 0.0, low: 0.0, high: 0.0 }
}

fn relevant_in_top(ranked: &[String], gold: &BTreeSet<String>, k: usize) -> usize {
    let mut seen = BTreeSet::new();
    ranked
        .iter()
        .take(k)
        .filter(|reference| seen.insert(reference.as_str()) && gold.contains(*reference))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    fn gold(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_string()).collect()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn precision_counts_relevant_in_the_top_k_over_k() {
        let ranked = refs(&["a", "x", "b", "c"]);
        let relevant = gold(&["a", "b", "z"]);
        assert!(close(precision_at_k(&ranked, &relevant, 3), 2.0 / 3.0));
        assert!(close(precision_at_k(&ranked, &relevant, 1), 1.0));
        assert!(close(precision_at_k(&ranked, &relevant, 2), 0.5));
    }

    #[test]
    fn precision_divides_by_k_when_fewer_results_return() {
        let ranked = refs(&["a"]);
        assert!(close(precision_at_k(&ranked, &gold(&["a"]), 3), 1.0 / 3.0));
        assert!(close(precision_at_k(&[], &gold(&["a"]), 3), 0.0));
    }

    #[test]
    fn precision_counts_a_duplicate_reference_once() {
        let ranked = refs(&["a", "a", "b"]);
        assert!(close(precision_at_k(&ranked, &gold(&["a"]), 3), 1.0 / 3.0));
    }

    #[test]
    fn recall_counts_relevant_in_the_top_k_over_the_gold_size() {
        let ranked = refs(&["x", "a", "y", "b", "z"]);
        let relevant = gold(&["a", "b", "c", "d"]);
        assert!(close(recall_at_k(&ranked, &relevant, 5), 0.5));
        assert!(close(recall_at_k(&ranked, &relevant, 2), 0.25));
        assert!(close(recall_at_k(&ranked, &relevant, 1), 0.0));
        assert!(close(recall_at_k(&ranked, &gold(&["b"]), 10), 1.0));
    }

    #[test]
    fn recall_counts_a_duplicate_reference_once() {
        let ranked = refs(&["a", "a"]);
        assert!(close(recall_at_k(&ranked, &gold(&["a", "b"]), 5), 0.5));
    }

    #[test]
    fn reciprocal_rank_is_one_over_the_first_relevant_rank() {
        let relevant = gold(&["b", "c"]);
        assert!(close(
            reciprocal_rank_at_k(&refs(&["x", "y", "c", "b"]), &relevant, 10),
            1.0 / 3.0
        ));
        assert!(close(reciprocal_rank_at_k(&refs(&["b"]), &relevant, 10), 1.0));
        assert!(close(reciprocal_rank_at_k(&refs(&["x", "y", "c"]), &relevant, 2), 0.0));
        assert!(close(reciprocal_rank_at_k(&[], &relevant, 10), 0.0));
    }

    #[test]
    fn mean_reciprocal_rank_over_queries_is_the_mean() {
        let relevant = gold(&["a"]);
        let values = [
            reciprocal_rank_at_k(&refs(&["a"]), &relevant, 10),
            reciprocal_rank_at_k(&refs(&["x", "a"]), &relevant, 10),
            reciprocal_rank_at_k(&refs(&["x"]), &relevant, 10),
        ];
        assert!(close(mean(&values), (1.0 + 0.5 + 0.0) / 3.0));
        assert!(close(mean(&[]), 0.0));
    }

    #[test]
    fn hit_is_one_when_any_top_k_reference_is_relevant() {
        let relevant = gold(&["b"]);
        assert!(close(hit_at_k(&refs(&["b", "x"]), &relevant, 1), 1.0));
        assert!(close(hit_at_k(&refs(&["x", "b"]), &relevant, 1), 0.0));
        assert!(close(hit_at_k(&refs(&["x", "b"]), &relevant, 2), 1.0));
        assert!(close(hit_at_k(&[], &relevant, 1), 0.0));
    }

    #[test]
    fn percentile_is_nearest_rank() {
        let sorted: Vec<f64> = (1..=100).map(f64::from).collect();
        assert!(close(percentile(&sorted, 0.025), 3.0));
        assert!(close(percentile(&sorted, 0.975), 98.0));
        assert!(close(percentile(&sorted, 0.5), 50.0));
        assert!(close(percentile(&sorted, 0.0), 1.0));
        assert!(close(percentile(&sorted, 1.0), 100.0));
        assert!(close(percentile(&[], 0.5), 0.0));
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = SplitMix64::new(7);
        let mut seen = [false; 3];
        for _ in 0..300 {
            let value = rng.below(3);
            assert!(value < 3);
            seen[value as usize] = true;
        }
        assert_eq!(seen, [true, true, true]);
    }

    #[test]
    fn bootstrap_of_a_constant_difference_is_that_constant() {
        let ci = paired_bootstrap_ci(&[0.25; 5], 1_000, 42);
        assert_eq!(ci, BootstrapCi { mean: 0.25, low: 0.25, high: 0.25 });
    }

    #[test]
    fn bootstrap_of_zero_and_one_spans_zero_to_one() {
        // Resample means of two queries are 0, 0.5 or 1 with probability
        // 1/4, 1/2, 1/4, so the 2.5th and 97.5th percentiles are 0 and 1.
        let ci = paired_bootstrap_ci(&[0.0, 1.0], 2_000, 42);
        assert!(close(ci.mean, 0.5));
        assert!(close(ci.low, 0.0));
        assert!(close(ci.high, 1.0));
    }

    #[test]
    fn bootstrap_is_reproducible_for_a_seed_and_brackets_the_mean() {
        let diffs = [1.0, 2.0, 3.0, -1.0, 0.0, 0.5];
        let first = paired_bootstrap_ci(&diffs, 1_000, 9);
        assert_eq!(first, paired_bootstrap_ci(&diffs, 1_000, 9));
        assert!(close(first.mean, 5.5 / 6.0));
        assert!(first.low < first.mean && first.mean < first.high);
        assert!(first.low >= -1.0 && first.high <= 3.0);
    }

    #[test]
    fn bootstrap_of_no_queries_is_zero() {
        assert_eq!(
            paired_bootstrap_ci(&[], 1_000, 1),
            BootstrapCi { mean: 0.0, low: 0.0, high: 0.0 }
        );
    }
}
