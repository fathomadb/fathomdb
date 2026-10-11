# Slice 135 mixed-workload baseline noise pilot — 2026-10-07 UTC

**Status:** five valid baseline-only blocks. This is a noise estimate, not a
0.8.26 versus 0.8.27 performance result. The paired first-results protocol
is frozen separately in
[first-results-protocol.json](../../first-results-protocol.json).

## Identity and timing boundary

The baseline is peeled `v0.8.26` source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`, source-tree SHA-256
`d25750313a18352c8f81c34fef8c160b7d6b884c4156c143ea12afc11d7fc133`.
The exact release-profile workload binary is stored as
[baseline-workload.gz](baseline-workload.gz); after decompression its SHA-256
is `3eccc98816fa39d93ef7c3a560c091133d34e62de65038b9463214c0bd8c20db`.
The runner bundle SHA-256 is
`ffe180144a94894e25815e97ed16255d303bef8803199978f9870503c2cb4018`.
The [build provenance](build-provenance/) includes the compiler output,
resolved lock, source identity and exact runner bundle. These hashes bind the
runner to the baseline binary; the compressed copy is retained so the
validator's artifact check can be repeated after `/tmp` is cleared.

Each unprofiled block used one warm-up and 100 valid observations for each of
three real-engine cells on this host: a 32-row text query, fresh database
close with reopen verification, and a whole mixed sequence. The mixed timer
includes open, governed write, projection drain, materialized text retrieval,
source erasure, close, reopen and post-erasure search. Semantic assertions
were made for each attempt. Stage attribution and sampled stacks are separate
diagnostics and do not enter these latency values. This is an engine-only,
CPU/no-embedder workload subset; installed SDK and vector/graph cells remain
for the full Phase 1 campaign.

## Independent raw recomputation

Every [block 01](block-01/) through [block 05](block-05/) records `raw.json`,
`summary.json`, exact commands, source/protocol/artifact hashes, environment
snapshots and GNU Time resource observations. All have status
`VALID_BLOCK_RECEIPT`, 100 valid and zero invalid attempts per cell. An
independent Python standard-library pass verified each raw SHA-256 against
`summary.json`, checked all 1,500 attempt-level semantic flags, sorted the
100 nanosecond values per cell and reproduced nearest-rank p50/p95 at indices
49 and 94. No p99 is supported by these 100-sample cells.

| Block | Text p50 / p95 ns | Close p50 / p95 ns | Whole sequence p50 / p95 ns | Raw SHA-256 prefix |
| --- | ---: | ---: | ---: | --- |
| [01](block-01/) | 178758 / 531414 | 5381813 / 5919970 | 39145784 / 40386449 | `b3afa55eaf27` |
| [02](block-02/) | 177936 / 534029 | 5223794 / 5789464 | 39255902 / 40697497 | `7221b345cedc` |
| [03](block-03/) | 175081 / 526054 | 5303946 / 5884453 | 39393552 / 41260992 | `18c580d6df5d` |
| [04](block-04/) | 176714 / 529862 | 5392743 / 5972419 | 38969269 / 40827522 | `922973006a11` |
| [05](block-05/) | 177195 / 517809 | 5303705 / 5801245 | 38750596 / 40549205 | `de53a826647d` |

Across blocks, p50 range divided by the median block p50 was 2.08% for text,
3.19% for close and 1.64% for the whole sequence. The corresponding p95
spreads were 3.06%, 3.11% and 2.15%. All blocks reported zero child swap
events and zero major faults, with peak child RSS 20,576–21,412 KiB. Block 03
recorded 26 host-wide swapped pages and zero child swaps as a warning; it
remains visible rather than being silently excluded. The host swap counter
does not identify which other process caused that activity. Baseline-to-
baseline variation and the earlier [two-cell noise series](../2026-10-07-noise-pilot/)
motivate paired alternating blocks and uncertainty reporting; they do not
establish a regression threshold or a candidate result.

## Invalid attempts and limits

The [invalid-attempt register](invalid-attempts/) retains a stale-runner-hash
reuse refusal, two runs rejected under the original any-host-swap rule (one
and three pages), and one run rejected because GNU Time emitted a trailing
blank line that the validator initially treated as malformed. Test-first
fixes now reject actual child swaps and malformed resource content, preserve
host-only drift as an exact warning, and accept trailing blank lines. No
invalid run was recast as a valid block; each valid block used the final
runner and validator hashes above.

The environment inventory checks named heavy jobs only at block boundaries
and does not continuously monitor thermal state, CPU effective frequency or
every host process. The query p95 can vary beyond this five-block spread on
another day, as the earlier quiet series showed. Pair-level uncertainty and
all warning-bearing blocks must be reported in the candidate comparison.
