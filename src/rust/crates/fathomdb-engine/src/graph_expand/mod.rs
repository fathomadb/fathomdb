mod codec;
pub(crate) mod execution;
mod traversal;
mod types;

pub use codec::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1,
};
pub(super) use execution::read_graph_expand_in_tx;
#[cfg(feature = "test-hooks")]
pub(super) use execution::{count_graph_expand_sql_statement, GraphExpandReaderControlsForTest};
#[cfg(feature = "test-hooks")]
pub use execution::{
    GraphExpandMeasurementForTest, GraphExpandProjectionStateForTest, GraphExpandRendezvousForTest,
};
pub(crate) use traversal::SearchExpandHandlerError;
pub use traversal::SearchExpandResult;
pub(super) use traversal::{
    crossed_boundary_since_in_tx, explain_graph_neighbors_in_tx, graph_neighbors_in_tx,
    search_expand_in_tx, search_expand_on_snapshot,
};
#[cfg(feature = "test-hooks")]
pub use types::graph_expansion_degradation_codes_for_test;
pub(crate) use types::SCHEMA_VERSION;
pub use types::{
    GraphExpandRequestV1, GraphExpandResultV1, GraphExpansionDegradationCodeV1,
    GraphExpansionErrorReasonV1, GraphExpansionErrorV1, GraphExpansionExplanationV1, GraphOriginV1,
    GraphProjectionOriginV1, GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1,
    GraphSeedSourceV1, GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1,
    TraversalDirection,
};

#[cfg(test)]
mod graph_evidence_request_tests {
    use crate::filter::SearchFilter;
    use crate::frozen_read::{FrozenReadContextV1, ReadContextV1};
    use crate::identity::IdSpace;
    use crate::temporal::ReadView;

    use super::execution::encode_graph_evidence_request;
    use super::{GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, TraversalDirection};

    fn request(token: &str) -> GraphExpandRequestV1 {
        let context = ReadContextV1::new(
            ReadView { valid_as_of: Some(41), ..ReadView::default() },
            SearchFilter::default(),
        )
        .unwrap();
        GraphExpandRequestV1 {
            schema_version: 1,
            seed: GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root"), IdSpace::logical("second")],
            },
            direction: TraversalDirection::Both,
            edge_kinds: vec!["supports".into(), "corrects".into()],
            target_kinds: vec!["observation".into(), "claim".into()],
            context: GraphReadContextV1::Frozen {
                schema_version: 1,
                context: FrozenReadContextV1 {
                    schema_version: 1,
                    effective_valid_at: 41,
                    context,
                    token: token.into(),
                },
            },
            max_depth: 2,
            result_limit: 17,
            max_work_units: 1_000,
            include_explanation: true,
            include_evidence: true,
        }
    }

    #[test]
    fn graph_request_commitment_input_normalizes_sets_and_excludes_context_token() {
        let left = request("first-token");
        let mut right = request("second-token");
        right.edge_kinds.reverse();
        right.target_kinds.reverse();
        assert_eq!(encode_graph_evidence_request(&left), encode_graph_evidence_request(&right));
    }

    #[test]
    fn graph_request_commitment_input_preserves_explicit_seed_order() {
        let left = request("same-token");
        let mut right = left.clone();
        let GraphSeedV1::Explicit { logical_ids, .. } = &mut right.seed else { unreachable!() };
        logical_ids.reverse();
        assert_ne!(encode_graph_evidence_request(&left), encode_graph_evidence_request(&right));
    }
}
