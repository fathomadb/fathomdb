#[cfg(test)]
mod tests {
    use super::*;

    const OUT_OF_MEMORY: u32 = 2;
    const NO_DEVICE: u32 = 100;

    #[test]
    fn early_init_runs_unless_every_cuda_capable_policy_is_cpu() {
        // (FATHOMDB_EMBED_DEVICE, FATHOMDB_RERANK_DEVICE, expected) with both
        // components built with CUDA. Unset means `auto`, as at open. A value
        // outside the exact grammar is refused at open; load still initialises
        // because only an exact `cpu` rules CUDA out.
        let cases = [
            (None, None, true),
            (Some("auto"), Some("auto"), true),
            (Some("cpu"), None, true),
            (None, Some("cpu"), true),
            (Some("cpu"), Some("auto"), true),
            (Some("cuda:0"), Some("cpu"), true),
            (Some("cpu"), Some("cuda:1"), true),
            (Some("cpu"), Some("cpu"), false),
            (Some("CPU"), Some("cpu"), true),
            (Some(" cpu"), Some("cpu"), true),
            (Some("cuda"), Some("cpu"), true),
            (Some(""), Some("cpu"), true),
        ];
        for (embed, rerank, expected) in cases {
            assert_eq!(
                early_cuda_init_wanted(true, true, embed, rerank),
                expected,
                "FATHOMDB_EMBED_DEVICE={embed:?} FATHOMDB_RERANK_DEVICE={rerank:?}"
            );
        }
    }

    #[test]
    fn a_component_built_without_cuda_never_asks_for_early_init() {
        // embed-cuda only: the reranker cannot use CUDA whatever its policy.
        assert!(!early_cuda_init_wanted(true, false, Some("cpu"), Some("cuda:0")));
        assert!(!early_cuda_init_wanted(true, false, Some("cpu"), None));
        assert!(early_cuda_init_wanted(true, false, None, Some("cpu")));
        // rerank-cuda only: the embedder cannot use CUDA whatever its policy.
        assert!(!early_cuda_init_wanted(false, true, Some("cuda:0"), Some("cpu")));
        assert!(early_cuda_init_wanted(false, true, Some("cpu"), None));
        // No CUDA at all.
        assert!(!early_cuda_init_wanted(false, false, None, None));
    }

    #[test]
    fn the_address_space_hint_needs_a_probe_failure_and_an_out_of_memory_cu_init() {
        assert!(fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY)).is_some());
        // cuInit was never attempted, succeeded, or failed for another reason.
        for last_cu_init in [None, Some(0), Some(NO_DEVICE), Some(1)] {
            assert_eq!(
                fragmented_address_space_hint("cuda_probe_failed", last_cu_init),
                None,
                "{last_cu_init:?}"
            );
        }
        // Every other refusal keeps its message whatever cuInit returned.
        for kind in [
            "no_visible_cuda_device",
            "cuda_incompatible",
            "cuda_not_compiled",
            "invalid_policy",
            "arm64_sbsa_unsupported",
        ] {
            assert_eq!(fragmented_address_space_hint(kind, Some(OUT_OF_MEMORY)), None, "{kind}");
        }
    }

    #[test]
    fn the_hint_names_the_cause_and_both_remedies() {
        let hint = fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY))
            .expect("hint for an out-of-memory cuInit");
        for needle in [
            "cuInit",
            "CUDA_ERROR_OUT_OF_MEMORY",
            "address space",
            "JavaScript heap",
            "import fathomdb before",
            "--import fathomdb",
        ] {
            assert!(hint.contains(needle), "{needle:?} missing from {hint:?}");
        }
    }

    #[test]
    fn the_refusal_message_keeps_its_prefix_and_appends_the_hint() {
        let base = "cuda:0 requested but unavailable: CudaProbeFailed";
        let message = refusal_message("cuda_probe_failed", base.to_owned(), Some(OUT_OF_MEMORY));
        let hint = fragmented_address_space_hint("cuda_probe_failed", Some(OUT_OF_MEMORY))
            .expect("hint for an out-of-memory cuInit");
        assert_eq!(message, format!("{base}; {hint}"));
        assert_eq!(refusal_message("cuda_probe_failed", base.to_owned(), Some(0)), base);
        assert_eq!(refusal_message("cuda_probe_failed", base.to_owned(), None), base);
    }
}
