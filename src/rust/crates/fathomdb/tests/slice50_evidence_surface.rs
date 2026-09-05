//! Slice 50 facade contract: evidence types and operations are public.

use fathomdb::{
    EvidenceErrorReasonV1, EvidenceRefV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    EvidenceSearchResultV1, ResolvedEvidenceV1,
};

#[test]
fn evidence_contract_is_reexported_by_the_facade() {
    let reference = EvidenceRefV1::new("opaque").unwrap();
    let _resolve_type: Option<EvidenceResolveRequestV1> = None;
    let _search_request_type: Option<EvidenceSearchRequestV1> = None;
    let _search_result_type: Option<EvidenceSearchResultV1> = None;
    let _resolved_type: Option<ResolvedEvidenceV1> = None;
    assert_eq!(reference.as_str(), "opaque");
    assert_eq!(EvidenceErrorReasonV1::EvidenceUnavailable.as_str(), "evidence_unavailable");
}
