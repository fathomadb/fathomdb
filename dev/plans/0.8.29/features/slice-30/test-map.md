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

## G/D extension candidate

The [G/D continuation](continuation-gd.md) adds ordered G-T1..10 and separately gated
D-T1..4 runtime RED/GREEN proposals with literal real-store/count/branch/time/
ownership oracles. P tests/evidence remain unchanged. Candidate test names and
ordinary/long-run registration must be finalized before each actual RED; this
proposal is not implementation, executed coverage or a Slice20 exception.

## M extension candidate

The [proposed M continuation](continuation-m.md) specifies small runtime
RED/GREEN units for the new wire recorder, call ledger, schedule/accounting,
lifecycle and private-helper outcomes. It separates normal CLI real-process
controls from nondefault existing-Seams diagnostics. No M source/runtime or
blanket passing-coverage exception follows; P/G tests and freezes stay unchanged.

## F/Q8 configuration admission

| Unit | Human-intent oracle and planned RED | Scoped GREEN verification |
| --- | --- | --- |
| T30-F1; R30-7/9, A30-6/7 | In `tests/experiments/test_tc5_gpu_v2.py`, use the existing `_candidate_config` literal fixture with release/candidate/package 0.8.29 and complete existing artifact fields. Assert `load_config` returns those exact versions and supplied candidate binding; current release allowlist raises instead. Reject missing binding, stale candidate version and wrong or malformed package versions. | Keep the RED assertions unchanged; run the affected parser tests and existing TC5 runner test file. Existing historical release/package cases remain unchanged. No native import, database open, model/GPU, frozen-pack validation or actual campaign runs in these controls. |

Limit implementation to `experiments/tc5_gpu_v2.py` release/package admission.
This unit changes no bridge inputs, metric math, resource caps or runtime
permission. Native/model/GPU qualification and the 900s versus historical 7200s
bound remain separate and held. No new hash-inventory or tamper tests.

## Private Storage diagnostic control

The [one-case diagnostic](continuation-gd.md#one-private-sqlite-log-storage-diagnostic)
adds a fresh-process real SQLite-error callback RED/GREEN and literal bounded
capture/loss controls under R30-4..7/A30-4..6. The source-qualified helper remains
outside ordinary D populations. Its single100k runtime must retain raw extended
codes and pre/post outer cgroup memory counters through the existing charged
guardian; missing capture never becomes a Storage diagnosis or D completion.

## One interrupted300k C continuation

The [single-cell amendment](continuation-gd.md#one-interrupted300k-held-c-r1-continuation)
implements R30-3..8/A30-3..7 through the existing explicit replacement path only.
Freeze real harmless-child REDs for this exact C-r1 selection: old2552606408977ns
plus conservative4320484082400ns fits the selected7500000000000ns cumulative
ceiling while the fresh whole attempt remains capped at4500000000000ns. The
unchanged old TERM row has no attempt identity fields; verify its exact bytes,
unique actual raw path/config and all existing qualified pins/charges survive.
Observe durable attempt0002 consumption and owned interruption/re-entry without
attempt0003; refuse mismatched history/raw identity, other cells, inadequate
headroom and any continuation after refusal/failure. Keep the frozen100k pair
and default-resume controls; run only affected source checks before one coherent
implementation review. Scientific execution and final cohort result review stay
separate; no passing native replay or partial-cohort PASS is implied.

## M-L short-process GPU observation correction

The [M-L-only repair](continuation-m.md#short-lifecycle-gpu-observation-repair)
adds source controls for R30-4..7/A30-4..6. Through existing payload/execute paths,
make a controlled late same-PID memory/positive-utilization witness available
only after the literal required RPC sequence and before recording-transport
closure; old sequencing must fail a runtime qualification assertion. Freeze the
exact RPC order/counts and first/immediate/lifecycle clock literals before GREEN.
Test wrong PID, no witness, observer/query failure, deadline/probe-timeout bounds,
close on exception, output/shutdown refusal and unchanged R/C defaults. GPU and
transport boundaries may be controlled; no DB integration is mocked. These
controls prove collector behavior only, not GPU work or scientific completion.
