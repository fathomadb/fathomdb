# Slice 30 test and RED/GREEN map

Test new measurement behavior, not hypothetical tampering. Each row is an
independent behavior unit: stage/commit the real failing runtime assertion
with raw RED evidence in its body; separate GREEN keeps that test unchanged.
Do not claim an import/compile error as measurement RED or break already-correct
code. If a reused behavior already passes, retain it without a fabricated pair;
new missing behavior must be exercised through an existing compilable command
or adapter scaffold. Parent may audit RED shas in own detached trees.

| Unit | Human-intent oracle and planned runtime RED | Scoped GREEN verification |
| --- | --- | --- |
| T30-1 converter selection/report | Three literal sessions include a repeated session, repeated/empty text, uneven arrays and a prefix cutoff. Assert exact body/question order and counts; existing converter lacks explicit selection/report behavior. Malformed input must fail before producing a qualifying workload. | Converter tests plus inventory count cross-check; no full model/database. |
| T30-2 statistics | Literal samples[1,2,3,100] yield p50=2,p99=100,mean=26.5; separate error denominator and repeat/query pairing have explicit small tables. New analysis command must emit these exact values, not duplicated implementation formulas. | Fast Python tests; full input/output example. |
| T30-3 committed cell runner | A recording command writes its assigned run ID to an ordinary output file; expect explicit forward/reverse order, distinct per-repetition store paths and every cell's status. Resume reuses completed cells only; partial-cell rerun starts a fresh DB. Runtime mismatch, not command-not-found. | Fast runner tests with actual bounded subprocesses and no DB mocking. |
| T30-4 direct-text adapter | Tiny real DB alpha/beta literal records. Require exact logical IDs and text-only invocation, no provider work, real close/reopen/normal exit. Extend the current compilable driver scaffold's missing direct-text command/record behavior; expected IDs come from fixture literals. | Rust example/integration tests on real stores; source-bound release smoke. |
| T30-5 write schedule | Real store with ten known marker IDs; exact batch/search counts1/10/100, marker revision visibility immediately after commit and no seed reuse between repetitions. New driver schedule initially lacks these actions; assertions examine actual DB/results, not echoed schedule metadata. | Small1/10batch real-DB cases, lifecycle cleanup and canonical engine freshness controls unchanged. |
| T30-6 match attribution | Literal bodies and stopword/punctuation query exercise actual compiler; expected matched row IDs are hand specified. Runner must compare actual product results/compiled-FTS row count; absent/naive-token diagnostic fails this runtime assertion. | Small real-DB query fixture; no generated goldens/relevance claims. |

Choose test selectors and final source paths before each RED; record exact
commands/counts/exits, frozen test blob and source identity in the committed
experiment README/execution record. Existing-passing reuse needs no test-only
exception because no new coverage is represented as a required behavior change.
If existing behavior genuinely needs new passing coverage under strict policy,
stop that exact addition and ask parent for disposition; the20 question does
not confer a blanket30 exception.

The exact Rust driver is
`src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs`; current SDK dependencies
expose both engine and SDK adapters. Only workspace query and explicit engine-matching rusqlite0.41
bundled/fallible_uint dev dependencies (existing lock0.41.1) are needed for separate untimed match counts. No new public SDK API or runtime
observer belongs to initial P. Additional G/D observer/math/schedule cases get a
concrete reviewed extension after P; no queued implementation under this map.

[Acceptance](acceptance.md) maps T30-1→A30-1/2/5,
T30-2/3→A30-4/5/6, T30-4/5→A30-2/3/5 and T30-6→A30-2/4.
Relevant scoped Rust/Python lint and typecheck precede tests; run Markdown and
release-view checks only for applicable docs/parent metadata. Required final
full source gate runs once on a clean stable tree. No parent/sibling installed
artifact qualifies30; no canonical benchmark/threshold/oracle rewrite.

## Actual selected suites and correction units

The maintained runner selects14Python tool tests and6Rust example tests,
including20deterministic codec-property examples and real P1/P2 owned-child
checkpoint interruption. No helper is ignored and no natural100k workload is
launched by ordinary tests. Original5Rust oracle bodies remain unchanged.
Real three-cell resume, UTF8-prefix summary/property, and checkpoint retention
have distinct observed RED commits before separate GREEN; exact grouped units,
invalid124fixture history, diagnostics and oracle identities are in
[execution](execution.md). [Results](results-p.md) maps actual affected checks,
assembled source qualification and remaining holds; no test count implies full
whole30/fidelity/Windows completion.
