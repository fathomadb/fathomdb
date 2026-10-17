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
}
