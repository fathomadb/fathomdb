mod codec;
pub(crate) mod execution;
mod traversal;
mod types;

pub use codec::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1,
};
#[cfg(test)]
pub(super) use execution::read_graph_expand_in_tx;
pub(super) use execution::read_graph_expand_with_stop_words_in_tx;
#[cfg(feature = "test-hooks")]
pub(super) use execution::{count_graph_expand_sql_statement, GraphExpandReaderControlsForTest};
#[cfg(feature = "test-hooks")]
pub use execution::{
    GraphExpandMeasurementForTest, GraphExpandProjectionStateForTest, GraphExpandRendezvousForTest,
};
#[cfg(any(test, feature = "test-hooks"))]
pub(super) use traversal::explain_graph_neighbors_in_tx;
pub(crate) use traversal::SearchExpandHandlerError;
pub use traversal::SearchExpandResult;
pub(super) use traversal::{
    crossed_boundary_since_in_tx, graph_neighbors_in_tx, search_expand_in_tx,
    search_expand_on_snapshot,
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
    use crate::properties::{PropertyComparisonOp, PropertyPredicate, PropertyValue};
    use crate::temporal::ReadView;

    use super::execution::encode_graph_evidence_request;
    use super::{GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, TraversalDirection};

    fn request(token: &str) -> GraphExpandRequestV1 {
        let context = ReadContextV1::new(
            ReadView { valid_as_of: Some(41), ..ReadView::default() },
            SearchFilter::default(),
        )
        .unwrap();
        GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root"), IdSpace::logical("second")],
            },
            TraversalDirection::Both,
            GraphReadContextV1::Frozen {
                schema_version: 1,
                context: FrozenReadContextV1 {
                    schema_version: 1,
                    effective_valid_at: 41,
                    context,
                    token: token.into(),
                },
            },
            2,
            17,
            1_000,
        )
        .with_edge_kinds(vec!["supports".into(), "corrects".into()])
        .with_target_kinds(vec!["observation".into(), "claim".into()])
        .with_explanation(true)
        .with_evidence(true)
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

    #[test]
    fn graph_request_commitment_input_normalizes_predicate_order_and_detects_tampering() {
        let p1 = PropertyPredicate {
            key: "weight".into(),
            op: PropertyComparisonOp::Gte,
            value: PropertyValue::Float(-0.0),
        };
        let p2 = PropertyPredicate {
            key: "verified".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Bool(true),
        };
        let q1 = PropertyPredicate {
            key: "tier".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::String("gold".into()),
        };
        let q2 = PropertyPredicate {
            key: "rank".into(),
            op: PropertyComparisonOp::Gt,
            value: PropertyValue::Int((1_i64 << 53) + 1),
        };

        let unpredicated = request("tok");
        let left = request("tok")
            .with_edge_predicates(vec![p1.clone(), p2.clone()])
            .with_target_predicates(vec![q1.clone(), q2.clone()]);
        let right = request("tok")
            .with_edge_predicates(vec![
                p2.clone(),
                PropertyPredicate {
                    key: "weight".into(),
                    op: PropertyComparisonOp::Gte,
                    value: PropertyValue::Float(0.0),
                },
            ])
            .with_target_predicates(vec![q2.clone(), q1.clone()]);

        assert_eq!(encode_graph_evidence_request(&left), encode_graph_evidence_request(&right));
        assert_ne!(
            encode_graph_evidence_request(&unpredicated),
            encode_graph_evidence_request(&left)
        );

        // Moving a predicate from edge to target changes the commitment.
        let swapped = request("tok")
            .with_edge_predicates(vec![q1.clone(), q2.clone()])
            .with_target_predicates(vec![p1.clone(), p2.clone()]);
        assert_ne!(encode_graph_evidence_request(&left), encode_graph_evidence_request(&swapped));

        // Changing a single predicate operator or value changes the commitment.
        let mut tampered = left.clone();
        tampered.target_predicates[1].value = PropertyValue::Int((1_i64 << 53) + 2);
        assert_ne!(encode_graph_evidence_request(&left), encode_graph_evidence_request(&tampered));
    }

    #[test]
    fn graph_request_commitment_input_binds_history_tags_prefix_free_and_preserves_current_bytes() {
        let base = request("tok");
        let base_bytes = encode_graph_evidence_request(&base);

        let pred = PropertyPredicate {
            key: "weight".into(),
            op: PropertyComparisonOp::Gte,
            value: PropertyValue::Int(1),
        };
        let with_pred = base.clone().with_edge_predicates(vec![pred.clone()]);
        let with_cursor = base.clone().with_as_of_write_cursor(42);
        let with_cursor_zero = base.clone().with_as_of_write_cursor(0);
        let with_epoch = base.clone().with_as_of_epoch_ms(42);
        let with_pred_and_cursor =
            base.clone().with_edge_predicates(vec![pred.clone()]).with_as_of_write_cursor(42);
        let with_pred_and_epoch =
            base.clone().with_edge_predicates(vec![pred]).with_as_of_epoch_ms(42);

        let b_pred = encode_graph_evidence_request(&with_pred);
        let b_cursor = encode_graph_evidence_request(&with_cursor);
        let b_cursor_zero = encode_graph_evidence_request(&with_cursor_zero);
        let b_epoch = encode_graph_evidence_request(&with_epoch);
        let b_pred_cursor = encode_graph_evidence_request(&with_pred_and_cursor);
        let b_pred_epoch = encode_graph_evidence_request(&with_pred_and_epoch);

        // Current-time encoding has no trailing tag; history/predicate tags are disjoint (1, 2, 3).
        assert_eq!(b_pred[base_bytes.len()], 1);
        assert_eq!(b_cursor[base_bytes.len()], 2);
        assert_eq!(b_epoch[base_bytes.len()], 3);
        assert_eq!(b_pred_cursor[b_pred.len()], 2);
        assert_eq!(b_pred_epoch[b_pred.len()], 3);

        // All distinct combinations produce distinct byte sequences (no collision across tags or values).
        let all =
            [base_bytes, b_pred, b_cursor, b_cursor_zero, b_epoch, b_pred_cursor, b_pred_epoch];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "collision between encoding {i} and {j}");
            }
        }
    }
}
