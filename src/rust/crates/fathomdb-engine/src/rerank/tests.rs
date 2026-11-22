//! 0.8.28 Slice 30 (R30-04a, F-1, F-2) — the cross-encoder's scoring and
//! singleton seams.

use std::sync::Mutex;

use fathomdb_embedder::{RerankerDeviceResolutionError, RerankerDeviceResolutionReason};

use super::*;

fn exhausted() -> RerankerDevicePolicyError {
    RerankerDevicePolicyError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 3 << 30,
        message: "rerank forward: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
    }
}

fn context_lost() -> RerankerDevicePolicyError {
    RerankerDevicePolicyError::CudaContextLost {
        recorded_context_id: 1,
        current_context_id: None,
        driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
        operation: "rerank forward".to_owned(),
    }
}

fn forced() -> RerankerDevicePolicyError {
    RerankerDevicePolicyError::Resolution(RerankerDeviceResolutionError::ForcedCudaUnavailable {
        ordinal: 0,
        reason: RerankerDeviceResolutionReason::CudaProbeFailed,
    })
}

type Outcome<T> = Result<T, Option<RerankerDevicePolicyError>>;

struct FakeScorer {
    batch: Outcome<Vec<f32>>,
    pairs: Mutex<Vec<Outcome<f32>>>,
    forced: Option<RerankerDevicePolicyError>,
}

impl FakeScorer {
    fn auto(batch: Outcome<Vec<f32>>, pairs: Vec<Outcome<f32>>) -> Self {
        Self { batch, pairs: Mutex::new(pairs), forced: None }
    }
}

impl CeScorer for FakeScorer {
    fn score_pair(&self, _query: &str, _passage: &str) -> Outcome<f32> {
        self.pairs.lock().unwrap().remove(0)
    }

    fn score_pairs(&self, _query: &str, _passages: &[&str]) -> Outcome<Vec<f32>> {
        self.batch.clone()
    }

    fn forced_cuda_runtime_error(&self) -> Option<RerankerDevicePolicyError> {
        self.forced.clone()
    }
}

#[test]
fn a_successful_batch_returns_its_logits() {
    let scorer = FakeScorer::auto(Ok(vec![1.0, -1.0]), vec![]);
    assert_eq!(score_pool(&scorer, "q", &["a", "b"]), Ok(vec![1.0, -1.0]));
}

#[test]
fn pool_exhaustion_of_the_batch_propagates_under_auto() {
    let scorer = FakeScorer::auto(Err(Some(exhausted())), vec![Ok(1.0), Ok(2.0)]);
    assert_eq!(score_pool(&scorer, "q", &["a", "b"]), Err(exhausted()));
}

#[test]
fn context_loss_of_the_batch_propagates_under_auto() {
    let scorer = FakeScorer::auto(Err(Some(context_lost())), vec![Ok(1.0)]);
    assert_eq!(score_pool(&scorer, "q", &["a"]), Err(context_lost()));
}

#[test]
fn an_ordinary_batch_failure_keeps_the_per_pair_neutral_fallback() {
    let scorer = FakeScorer::auto(Err(None), vec![Ok(2.0), Err(None)]);
    assert_eq!(score_pool(&scorer, "q", &["a", "b"]), Ok(vec![2.0, 0.0]));
}

#[test]
fn pool_exhaustion_of_one_pair_propagates_instead_of_a_neutral_score() {
    let scorer = FakeScorer::auto(Err(None), vec![Ok(2.0), Err(Some(exhausted()))]);
    assert_eq!(score_pool(&scorer, "q", &["a", "b"]), Err(exhausted()));
}

#[test]
fn a_forced_policy_turns_an_ordinary_batch_failure_into_its_refusal() {
    let scorer =
        FakeScorer { batch: Err(None), pairs: Mutex::new(vec![Ok(1.0)]), forced: Some(forced()) };
    assert_eq!(score_pool(&scorer, "q", &["a"]), Err(forced()));
}

#[test]
fn a_forced_policy_still_reports_pool_exhaustion_as_itself() {
    let scorer = FakeScorer {
        batch: Err(Some(exhausted())),
        pairs: Mutex::new(vec![]),
        forced: Some(forced()),
    };
    assert_eq!(score_pool(&scorer, "q", &["a"]), Err(exhausted()));
}

#[test]
fn ce_blend_retains_exact_key_and_representative_through_reordering() {
    use crate::fusion::{FusionHit, FusionOwnership};
    let make = |ownership, source: &str, score| FusionHit {
        ownership,
        hit: SearchHit {
            id: IdSpace::content("shared"),
            write_cursor: 7,
            kind: "doc".into(),
            body: "same".into(),
            score,
            branch: SoftFallbackBranch::Text,
            source_id: Some(source.into()),
            ce_score: None,
            properties: None,
        },
    };
    let hits = vec![
        make(FusionOwnership::CanonicalNode, "node", 2.0),
        make(FusionOwnership::CanonicalEdge, "edge", 1.0),
        make(FusionOwnership::OwnerlessNodeFts { rowid: 999 }, "ownerless", 0.01),
    ];
    let ranked = blend_ce_ranked(&hits, vec![0.0, 1.0], 1.0, 2);
    assert_eq!(ranked.len(), 3);
    assert_eq!(
        ranked.iter().map(|hit| hit.key()).collect::<Vec<_>>(),
        [hits[1].key(), hits[0].key(), hits[2].key()]
    );
    for (output, input) in [(0, 1), (1, 0), (2, 2)] {
        assert_eq!(ranked[output].id, hits[input].id);
        assert_eq!(ranked[output].write_cursor, hits[input].write_cursor);
        assert_eq!(ranked[output].kind, hits[input].kind);
        assert_eq!(ranked[output].body, hits[input].body);
        assert_eq!(ranked[output].source_id, hits[input].source_id);
        assert_eq!(ranked[output].ownership, hits[input].ownership);
    }
    assert_eq!(ranked[0].score, 1.0 / (1.0 + (-1.0_f64).exp()));
    assert_eq!(ranked[0].ce_score, Some(ranked[0].score));
    assert_eq!(ranked[1].score, 0.5);
    assert_eq!(ranked[1].ce_score, Some(0.5));
    assert_eq!(ranked[2].score, 0.01);
    assert_eq!(ranked[2].ce_score, None);
}
