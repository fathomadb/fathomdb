/// Canonical nearest-rank quantiles, only for a complete successful experiment cell.
pub fn qualified_percentiles(samples: &[f64], errors: usize) -> Option<(f64, f64)> {
    if samples.len() != 1000 || errors != 0 {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some((*sorted.get(499)?, *sorted.get(989)?))
}
