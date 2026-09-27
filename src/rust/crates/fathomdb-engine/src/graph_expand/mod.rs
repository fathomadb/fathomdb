mod execution;
mod types;

use crate::{
    FrozenReadContextV1, IdSpace, ReadContextV1, StructuralDependencyStateV1,
    StructuralLifecycleStateV1,
};

pub(crate) use execution::read_graph_expand_in_tx;
#[cfg(feature = "test-hooks")]
pub(crate) use execution::{count_graph_expand_sql_statement, GraphExpandReaderControlsForTest};
pub use execution::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1,
};
#[cfg(feature = "test-hooks")]
#[allow(unused_imports)]
pub use execution::{
    GraphExpandCurrentRssSampleForTest, GraphExpandIsolatedProcessRssSampleForTest,
    GraphExpandMeasurementForTest, GraphExpandProjectionGenerationForTest,
    GraphExpandProjectionStateForTest, GraphExpandRendezvousForTest,
};
#[cfg(feature = "test-hooks")]
pub use types::graph_expansion_degradation_codes_for_test;
pub(crate) use types::{graph_expansion_degradation_codes, SCHEMA_VERSION};
pub use types::{
    GraphExpandRequestV1, GraphExpandResultV1, GraphExpansionDegradationCodeV1,
    GraphExpansionErrorReasonV1, GraphExpansionErrorV1, GraphExpansionExplanationV1, GraphOriginV1,
    GraphProjectionOriginV1, GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1,
    GraphSeedSourceV1, GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1,
    TraversalDirection,
};

#[cfg(test)]
use crate::{ReadView, SearchFilter};
#[cfg(test)]
use execution::encode_graph_evidence_request;

#[cfg(test)]
mod graph_evidence_request_tests {
    use super::*;

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
