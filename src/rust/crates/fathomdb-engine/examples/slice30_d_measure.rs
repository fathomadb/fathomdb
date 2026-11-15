//! Private deterministic D workload; no model, GPU or performance verdict.
use fathomdb_embedder_api::Embedder;
#[path = "slice30_d_measure/provider.rs"]
mod provider;
use provider::VaryingEmbedder;
use std::process::ExitCode;

const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const QUERY_SEED: u64 = 0x0AC0_130D_EC0D_E000;
const SEED_BATCH: usize = 1000;
const QUERY_COUNT: usize = 500;
const NATURAL_CALLS: usize = 1000;

fn main() -> ExitCode {
    let identity = VaryingEmbedder.identity();
    println!(
        "{}",
        serde_json::json!({
            "complete":false, "state":"not_run", "reason":"D driver is not implemented",
            "provider":{"name":identity.name,"revision":identity.revision,"dimension":identity.dimension},
            "corpus_seed":CORPUS_SEED,"query_seed":QUERY_SEED,"seed_batch":SEED_BATCH,
            "query_count":QUERY_COUNT,"natural_calls":NATURAL_CALLS,"warmup_calls":0
        })
    );
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use fathomdb_embedder_api::EmbedderIdentity;

    #[test]
    fn fixed_provider_has_reproducible_signed_normalized_768d_vectors() {
        let provider = VaryingEmbedder;
        assert_eq!(provider.identity(), EmbedderIdentity::new("varying", "perf-gates-dense", 768));
        let empty =
            provider.embed("").expect("accepted deterministic768d generator is unavailable");
        for text in ["", "needle alpha", "needle beta", "雪"] {
            let vector = provider.embed(text).expect("fixed synthetic vector must exist");
            assert_eq!(vector.len(), 768);
            assert!(vector.iter().all(|coordinate| coordinate.is_finite()));
            assert!(vector.iter().any(|coordinate| *coordinate > 0.0));
            assert!(vector.iter().any(|coordinate| *coordinate < 0.0));
            let norm = vector.iter().map(|coordinate| coordinate * coordinate).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-5, "accepted f32 L2 normalization was not applied");
            let repeated = provider.embed(text).unwrap();
            assert_eq!(
                vector.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                repeated.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
            );
        }
        assert_ne!(empty, provider.embed("needle alpha").unwrap());
        assert_ne!(provider.embed("needle alpha").unwrap(), provider.embed("needle beta").unwrap());
    }
}
