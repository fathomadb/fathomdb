use current_history_followups::current::Certificate;
#[test]
fn certificate_requires_exact_scope_generation_and_half_open_time() {
    let cert = Certificate {
        published_at: 10,
        next_boundary: 20,
        visibility: 3,
        scope: "strict-v1".into(),
    };
    assert!(cert.covers(10, 3, "strict-v1"));
    assert!(cert.covers(19, 3, "strict-v1"));
    assert!(!cert.covers(20, 3, "strict-v1"), "clock boundary must invalidate even with no write");
    assert!(!cert.covers(9, 3, "strict-v1"));
    assert!(!cert.covers(10, 4, "strict-v1"));
    assert!(!cert.covers(10, 3, "archive"));
}
