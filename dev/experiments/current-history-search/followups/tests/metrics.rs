#[test]
fn canonical_nearest_rank_requires_every_scheduled_request_successful() {
    let samples = (0..1000).map(|i| i as f64).collect::<Vec<_>>();
    assert_eq!(
        current_history_followups::metrics::qualified_percentiles(&samples, 0),
        Some((499., 989.))
    );
    assert_eq!(current_history_followups::metrics::qualified_percentiles(&samples[..999], 1), None);
    assert_eq!(current_history_followups::metrics::qualified_percentiles(&samples, 1), None);
}
