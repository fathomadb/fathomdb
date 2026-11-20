use super::*;
use crate::{CandleTinyBertReranker, RerankerDevicePolicyError};
use candle_core::cuda::cudarc::driver::{result, sys};

#[test]
#[ignore = "exclusive leased child; reset modes require separate concrete authorization"]
fn completed_models_preserve_healthy_or_coordinated_loss_drop_contract() {
    assert_eq!(std::env::var("FATHOMDB_REQUIRE_LIVE").as_deref(), Ok("1"));
    assert_eq!(std::env::var("FATHOMDB_EMBED_DEVICE").as_deref(), Ok("cuda:0"));
    assert_eq!(std::env::var("FATHOMDB_RERANK_DEVICE").as_deref(), Ok("cuda:0"));
    let mode = std::env::var("FATHOMDB_COORDINATED_RESET")
        .expect("explicit healthy/reset-refuse/reset-drop mode");
    assert!(matches!(mode.as_str(), "healthy" | "reset-refuse" | "reset-drop"));
    let embedder = CandleBgeEmbedder::new().expect("verified cached BGE on selected CUDA device");
    let reranker = CandleTinyBertReranker::try_load()
        .expect("verified cached reranker on selected CUDA device");
    assert!(embedder.device.is_cuda());
    let inputs = [
        "the cat sat on the mat",
        "a cat is on a mat",
        "quantum mechanics dictates the behaviour of subatomic particles",
    ];
    let single = embedder.embed(inputs[0]).expect("healthy CUDA single");
    assert_eq!(single.len(), 384);
    let vectors = embedder.embed_batch(&inputs).expect("healthy CUDA batch");
    for vector in &vectors {
        assert_eq!(vector.len(), 384);
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.).abs() < 1e-5, "existing EU-4 unit norm: {norm}");
    }
    let cosine = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let near = cosine(&vectors[0], &vectors[1]);
    assert!(near - cosine(&vectors[0], &vectors[2]) > 0.15);
    assert!(near - cosine(&vectors[1], &vectors[2]) > 0.15);
    assert!(reranker
        .score("What is the capital of France?", "Paris.")
        .expect("healthy CUDA rerank")
        .is_finite());
    embedder
        .device
        .as_cuda_device()
        .expect("selected CUDA device")
        .cuda_stream()
        .context()
        .synchronize()
        .expect("all primary-context streams completed");
    let lifetime = crate::cuda_lifetime::existing(0).expect("production shared generation");
    assert!(lifetime.loss().is_none());
    let before = crate::cuda_pool_policy::observe_pool();
    if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
        assert_eq!(before.status, "private");
        assert_eq!(before.live_models_after, Some(2));
        assert!(before.used_current_bytes.unwrap() < 8 << 30);
    }
    println!("completed-models: ready synchronized model-count=2 mode={mode} pool={before:?}");
    if mode != "healthy" {
        let ordinal = result::device::get(0).expect("selected device ordinal");
        // The sole test thread owns every completed model and clone. Calls above
        // returned and submitted work synchronized; no other CUDA module runs.
        unsafe { sys::cuDevicePrimaryCtxReset_v2(ordinal) }
            .result()
            .expect("process-primary reset");
        println!("completed-models: process-primary-reset-returned mode={mode}");
    }
    if mode == "reset-refuse" {
        let mut first_cause = None;
        let mut first_returned = None;
        for _ in 0..32 {
            let single = embedder_loss(embedder.embed("after reset").expect_err("lost single"));
            let expected = first_returned.get_or_insert_with(|| single.clone());
            assert_eq!(&single, expected);
            let actual = lifetime.loss().expect("authoritative first cause after single refusal");
            assert_eq!(single.0, lifetime.recorded_id());
            assert_eq!(single.1, actual.observed_id);
            assert_eq!(actual, *first_cause.get_or_insert(actual));
            let batch = embedder_loss(
                embedder.embed_batch(&["after reset", "second"]).expect_err("lost batch"),
            );
            assert_eq!(&batch, expected);
            let error = reranker
                .score("What is the capital of France?", "Paris.")
                .expect_err("lost reranker");
            let rerank = match reranker.cuda_pool_error(&error) {
                Some(RerankerDevicePolicyError::CudaContextLost {
                    recorded_context_id,
                    current_context_id,
                    driver_error,
                    ..
                }) => (recorded_context_id, current_context_id, driver_error),
                other => panic!("expected typed lost reranker, got {other:?}"),
            };
            assert_eq!(&rerank, expected);
            assert_eq!(lifetime.loss(), first_cause);
            assert_eq!(crate::cuda_lifetime::loss(0), first_cause);
        }
        let lost = crate::cuda_pool_policy::observe_pool();
        if before.status == "private" {
            assert_eq!(lost.status, "lost");
            assert_eq!(lost.context_id, before.context_id);
            assert_eq!(lost.pool_id, before.pool_id);
            assert_eq!(lost.live_models_after, Some(2));
            assert!(lost.used_current_bytes.is_none() && lost.reserved_current_bytes.is_none());
        }
        println!(
            "completed-models: typed-single-batch-rerank-refusal32 cached-loss={:?} pool={lost:?}",
            lifetime.loss()
        );
    }
    // reset-drop deliberately performs no operation/observation after reset.
    drop(embedder);
    drop(reranker);
    if mode == "healthy" {
        assert!(lifetime.loss().is_none());
    } else {
        assert!(lifetime.loss().is_some(), "drop must discover loss without a forward");
    }
    let after = crate::cuda_pool_policy::observe_pool();
    if before.status == "private" {
        assert_eq!(after.live_models_after, Some(0));
        assert_eq!(after.context_id, before.context_id);
        assert_eq!(after.pool_id, before.pool_id);
        if mode != "healthy" {
            assert_eq!(after.status, "lost");
            assert!(after.used_current_bytes.is_none() && after.reserved_current_bytes.is_none());
        }
    }
    drop(lifetime);
    println!("completed-models: models-dropped mode={mode}");
}

fn embedder_loss(error: EmbedderError) -> (u64, Option<u64>, String) {
    match error {
        EmbedderError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            ..
        } => (recorded_context_id, current_context_id, driver_error),
        other => panic!("expected typed CUDA loss, got {other:?}"),
    }
}
