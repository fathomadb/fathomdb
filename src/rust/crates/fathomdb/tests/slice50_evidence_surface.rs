//! Slice 50 facade contract: evidence types and operations are public.

use fathomdb::{
    EvidenceErrorReasonV1, EvidenceRefV1, EvidenceResolveManyItemV1, EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, EvidenceSearchResultV1, Hint,
    ResolvedEvidenceV1, EVIDENCE_RESOLVE_MANY_MAX,
};

#[test]
fn evidence_contract_is_reexported_by_the_facade() {
    let reference = EvidenceRefV1::new("opaque").unwrap();
    let _resolve_type: Option<EvidenceResolveRequestV1> = None;
    let _resolve_many_request: Option<EvidenceResolveManyRequestV1> = None;
    let _resolve_many_item: Option<EvidenceResolveManyItemV1> = None;
    let _search_request_type: Option<EvidenceSearchRequestV1> = None;
    let _search_result_type: Option<EvidenceSearchResultV1> = None;
    let _resolved_type: Option<ResolvedEvidenceV1> = None;
    let _hint_type: Option<Hint> = None;
    assert_eq!(EVIDENCE_RESOLVE_MANY_MAX, 50);
    assert_eq!(reference.as_str(), "opaque");
    assert_eq!(EvidenceErrorReasonV1::EvidenceUnavailable.as_str(), "evidence_unavailable");
}
