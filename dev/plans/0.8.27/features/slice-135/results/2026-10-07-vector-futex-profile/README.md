# Slice 135 vector/hybrid futex profile — 2026-10-07 UTC

**Status:** source-bound diagnostic profile, separate from the unprofiled
[E01–E12 paired comparison](../2026-10-07-e12-paired-current/README.md).
The profile used the exact preserved baseline and candidate query binaries
(SHA-256 in [summary.json](summary.json)), rebuilt no product source, and
executed two alternating B–C–C–B runs. Each run made 1,000 materialized
calls in each of `text`, `vector_stage` and `hybrid`, with one warm-up per
cell. Every attempt matched the frozen ordered IDs and branches. The four
compressed raw outputs, `strace` summaries and process streams are retained;
[SHA256SUMS](SHA256SUMS) checks every retained file.

| Run | Version | Whole-process `futex` calls | `futex` errors | Sum of traced thread time |
| --- | --- | ---: | ---: | ---: |
| 1 | 0.8.26 | 12,922 | 248 | 2.314 s |
| 2 | 0.8.27 | 32,524 | 2,614 | 5.600 s |
| 3 | 0.8.27 | 32,034 | 2,415 | 5.010 s |
| 4 | 0.8.26 | 13,072 | 307 | 1.993 s |

The candidate made about **2.48 times** as many traced `futex` calls per
block. This supports a source-level hypothesis: candidate
`search_api.rs` routes embedding through `dispatch_embed_vector`, which
queues a request and waits for its reply; 0.8.26 called the provider directly.
The same dispatcher serves both vector-bearing query shapes, while text-only
search avoids it. The [paired latency receipt](../2026-10-07-e12-paired-current/README.md)
shows vector/hybrid p50 losses of +8.6/+8.8% and near-parity text p50.

This trace **does not isolate** query calls from setup/projection or split
vector from hybrid. `strace` perturbs scheduling and latency, and its syscall
seconds sum over threads rather than measuring client wall time. The traced
per-call durations are not a replacement for the frozen unprofiled deltas.
The result is an attribution lead, not proof that every extra syscall causes
the observed loss. Any optimization must preserve bounded provider capacity,
timeouts, close cancellation and recovery; the focused
[provider/close receipt](../2026-10-07-provider-close-current/README.md)
covers some of those system invariants. Stage-level instrumentation or a
query-only profile is the next diagnostic before changing the dispatcher.
