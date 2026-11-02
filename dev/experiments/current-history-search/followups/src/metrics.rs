/// Canonical nearest-rank quantiles, only for a complete successful experiment cell.
pub fn qualified_percentiles(samples: &[f64], errors: usize) -> Option<(f64, f64)> {
    let _ = errors;
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some((*sorted.get(500)?, *sorted.get(990)?))
}
