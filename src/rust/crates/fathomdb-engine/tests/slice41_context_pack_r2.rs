//! Slice 41 (0.8.28) R2 code-review fixes: JSON-wire counts that cannot be a
//! `u64` (too large, or a non-finite value a binding forwards as a string) are
//! refused with a message naming the field and the bound.

use fathomdb_engine::pack_context_json;

fn refusal(request: &str) -> String {
    pack_context_json(request).expect_err("must be refused").message().to_string()
}

/// R2 P3-4: an integer above `u64::MAX` names the field and the bound.
#[test]
fn out_of_range_counts_name_the_field_and_the_bound() {
    let policy = refusal(r#"{"hits": [], "policy": {"full_items": 18446744073709551616}}"#);
    assert!(policy.contains("policy.full_items"), "{policy}");
    assert!(policy.contains("at most 18446744073709551615"), "{policy}");
    let position = refusal(r#"{"hits": [{"id": "a", "parts": [], "position": 1e30}]}"#);
    assert!(position.contains("hits[0].position"), "{position}");
    assert!(position.contains("at most 18446744073709551615"), "{position}");
}

/// R2 P3-4: a non-finite value forwarded as a string names the field.
#[test]
fn non_finite_counts_forwarded_as_strings_name_the_field() {
    let message = refusal(r#"{"hits": [], "policy": {"budget_chars": "nan"}}"#);
    assert!(message.contains("policy.budget_chars"), "{message}");
    assert!(message.contains("non-negative integer"), "{message}");
}
