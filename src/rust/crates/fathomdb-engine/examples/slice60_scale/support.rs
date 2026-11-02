pub fn percentile(values: &[f64], fraction: f64) -> f64 {
    assert!(!values.is_empty() && fraction > 0.0 && fraction <= 1.0);
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[(fraction * sorted.len() as f64).ceil() as usize - 1]
}

// Same FNV-seeded xorshift and L2 normalization as exp_ac013_vector_reads.
pub fn vector(text: &str, dimension: usize) -> Vec<f32> {
    let mut state = 0xcbf29ce484222325u64;
    for byte in text.bytes() {
        state = (state ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    if state == 0 {
        state = 0xdeadbeef_cafebabe;
    }
    let mut result = Vec::with_capacity(dimension);
    for _ in 0..dimension {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        result.push(((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0));
    }
    let norm = result.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for value in &mut result {
        *value /= norm;
    }
    result
}

// Only cutoff-distance membership can vary across vec0 physical layouts.
pub fn equivalent_candidates(a: &[(i64, i64)], b: &[(i64, i64)]) -> bool {
    if a.len() != b.len() || a.is_empty() {
        return a == b;
    }
    let cutoff = a.iter().map(|(_, distance)| *distance).max().unwrap();
    if b.iter().map(|(_, distance)| *distance).max() != Some(cutoff) {
        return false;
    }
    let strict = |rows: &[(i64, i64)]| {
        rows.iter().copied().filter(|(_, d)| *d < cutoff).collect::<std::collections::BTreeSet<_>>()
    };
    strict(a) == strict(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_rank_percentiles_sort_without_dropping_samples() {
        assert_eq!(percentile(&[50., 10., 40., 20., 30.], 0.5), 30.);
        assert_eq!(percentile(&[50., 10., 40., 20., 30.], 0.95), 50.);
        assert_eq!(percentile(&[7.], 0.5), 7.);
    }
    #[test]
    fn generated_vectors_are_deterministic_unit_length_and_distinct() {
        let a = vector("corpus:42", 384);
        assert_eq!(a.len(), 384);
        assert_eq!(a, vector("corpus:42", 384));
        assert_ne!(a, vector("query:42", 384));
        assert!((a.iter().map(|x| x * x).sum::<f32>() - 1.).abs() < 1e-5);
    }
    #[test]
    fn only_cutoff_ties_can_change_membership() {
        let a = [(1, 2), (2, 3), (3, 3)];
        assert!(equivalent_candidates(&a, &[(1, 2), (4, 3), (2, 3)]));
        assert!(!equivalent_candidates(&a, &[(8, 2), (4, 3), (2, 3)]));
        assert!(!equivalent_candidates(&a, &[(1, 2), (2, 3), (3, 4)]));
        assert!(!equivalent_candidates(&a, &[(1, 2), (2, 3)]));
    }
}
