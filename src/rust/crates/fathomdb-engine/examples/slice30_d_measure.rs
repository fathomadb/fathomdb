//! Private deterministic D workload; no model, GPU or performance verdict.
use fathomdb_embedder_api::Embedder;
#[cfg(test)]
#[path = "slice30_d_measure/capture.rs"]
mod capture;
#[cfg(test)]
#[path = "slice30_d_measure/capture_overlap_tests.rs"]
mod capture_overlap_tests;
#[cfg(test)]
#[path = "slice30_d_measure/capture_tests.rs"]
mod capture_tests;
#[cfg(test)]
#[path = "slice30_d_measure/entry.rs"]
mod entry;
#[cfg(test)]
#[path = "slice30_d_measure/entry_tests.rs"]
mod entry_tests;
#[path = "slice30_d_measure/provider.rs"]
mod provider;
#[cfg(test)]
#[path = "slice30_d_measure/schedule.rs"]
mod schedule;
#[cfg(test)]
#[path = "slice30_d_measure/schedule_tests.rs"]
mod schedule_tests;
#[cfg(test)]
#[path = "slice30_d_measure/workload.rs"]
mod workload;
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
    fn fixed_workload_has_held_out_500_queries_cycled_twice_without_warmups() {
        let corpus = workload::corpus_bodies(1001).collect::<Vec<_>>();
        assert_eq!(corpus.len(), 1001, "accepted corpus stream omitted generated rows");
        assert_eq!(
            corpus,
            workload::corpus_bodies(1001).collect::<Vec<_>>(),
            "fresh executions must reproduce the same ordered corpus"
        );
        assert_eq!(
            workload::corpus_bodies(2).collect::<Vec<_>>(),
            corpus[..2],
            "corpus size must not change its deterministic prefix"
        );

        let queries = workload::query_bodies();
        assert_eq!(queries.len(), 500, "accepted held-out query population differs");
        assert_eq!(queries, workload::query_bodies(), "query stream must restart reproducibly");
        assert_ne!(queries, corpus[..500], "queries must use the distinct held-out seed");
        for body in corpus.iter().chain(queries.iter()) {
            let tokens = body.split(' ').collect::<Vec<_>>();
            assert!((55..=74).contains(&tokens.len()), "accepted body token count differs");
            for token in tokens {
                assert_eq!(token.len(), 7, "accepted vocabulary token width differs");
                let bytes = token.as_bytes();
                assert!(bytes[..3].iter().all(u8::is_ascii_lowercase));
                assert!(bytes[3..].iter().all(u8::is_ascii_digit));
                let index = token[3..].parse::<usize>().unwrap();
                assert!(index < 1024, "token is outside the accepted vocabulary");
                assert_eq!(
                    &bytes[..3],
                    &[
                        b'a' + ((index / 26 / 26) % 26) as u8,
                        b'a' + ((index / 26) % 26) as u8,
                        b'a' + (index % 26) as u8,
                    ],
                    "token spelling does not match its vocabulary rank"
                );
            }
        }
        let calls = workload::natural_queries(&queries).collect::<Vec<_>>();
        assert_eq!(calls.len(), 1000, "natural requests must have no extra warmup pass");
        let expected = queries.iter().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(calls[..500], expected);
        assert_eq!(calls[500..], expected, "second500 requests must retain the first order");
    }

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
