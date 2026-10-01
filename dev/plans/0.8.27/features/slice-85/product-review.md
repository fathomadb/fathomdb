---
title: Slice 85 product review
status: COMPLETE
date: 2026-09-30
target_release: 0.8.27
reviewed_candidate: 6ec4194e7f2bc514922a1dcbf4514957f0c788b6
verdict: NO_NEW_CORRECTNESS_FINDINGS
acceptance: qualification gaps remain
---

# Slice 85 product review

**No new actionable correctness findings** in this bounded review. Reviewed the
full `8b2a9edaf..6ec4194e7` Slice 85 product diff and the separate
`df1ffd000..6ec4194e7` recovery diff. The review includes the original ownership
moves, not just the recovery fixes. Release acceptance remains unverified.

## Effort and area verdicts

Estimated active review effort followed **60% critical runtime / 30% other
touched runtime / 10% non-runtime**. These are attention estimates, not stopwatch
measurements; tool waits and unattended test runtime are excluded. Main reviewed
critical paths and tooling; a read-only subagent reviewed secondary runtime,
splitting its effort approximately 50% graph / 33% ML / 17% operational routes.

| Area | Effort | Verdict and checked behavior |
| --- | ---: | --- |
| Lifecycle and SQLite/WAL ownership | 20% | No finding. Close stops projection owners, joins readers, removes callbacks and writer connection, then releases profile userdata and the sidecar. Implicit Drop calls the same path. WAL activity, checkpoint classification and worker-zero targeting retain their prior behavior. |
| Read dispatch and snapshots | 20% | No finding. Typed factories preserve request fields and response channels; facade checks and read ordering remain. Page authentication, snapshot validation and filter checks stay on the reader transaction. Vector-stage boxing preserves by-value execution and the unchanged envelope bound. |
| Ordinary search | 20% | No finding. Hybrid/evidence/text-only constructors preserve all prior controls and defaults. Snapshot-local validation, eligibility before caps, ranking, sparse fallback and public error mappings remain. |
| Graph expansion | 15% | No finding. Facades preserve validation precedence, continuation binding and controls; handlers validate/filter before walking, assemble evidence in the same transaction and commit before response. Required facade access survives private traversal. |
| Optional ML/provider routes | 10% | No source finding; CPU evidence supported. Provider identity, vectors, generation/refusal state and ranking controls survive the moves. Embedding/reranking implementations and feature wiring are unchanged. GPU execution remains unverified. |
| Operational/operator routes | 5% | No finding. Operational reads preserve refusal precedence and keyset ordering; structural state retains same-snapshot provenance checks. Telemetry sink/accessor and erasure substitutions preserve behavior. |
| Gate, test infrastructure and records | 10% | No finding: 6% gate/tiers, 2% pause-hook synchronization, 2% records. Required forbids, ownership, Engine/root-helper reach, cfg guards and freshness remain; qualification stays explicit. Hook cancellation/token identity and documented API delta agree with code. |

## Findings and evidence

There are **no new P0–P3 correctness findings or recommended code changes**.
The previously fixed Engine-method item-identity defect is closed; its compiling
witness and original RED/GREEN evidence are retained. No hypothetical laundering
forms, frozen inventories or new review cycles were introduced.

Compared moved WAL method bodies with the pre-slice definitions; substantive
behavior is preserved, with the Engine-owned pause state added during recovery.
Checked read facade changes against the new factories and compared search
constructors field by field with the previous struct literals. Inspected close,
callback uninstall, worker teardown, reader handoff, filter conversions and
meaningful regression assertions. The topology ADR's bounded executors, admission
and deadlines are future Slice 90 requirements, not implemented Slice 85 behavior.

Reused source-bound evidence in [recovery-receipt.md](recovery-receipt.md):

- 81 engine library tests, including teardown, handoff, WAL and pause controls;
  41 serial hook-consumer tests across pre-KNN filtering, frozen races and evidence.
- 33 focused feature tests: 19 graph, four wire, one transaction release, two
  error routes, three vector-stage and four operator checks.
- 15 live CPU Rust and 13 live Python FFI/embedding/graph tests, without skips;
  fresh native artifact hash and receipt verifier.
- Gate units, wrapper/tier checks and 42 qualification cases, with 28 actual
  compiled architectural witnesses and the Engine-method cycle regression.

Git comparisons confirmed the engine tree matches the Phase B candidate
`6b8533344`, and `src`, `scripts`, `dev/tools` and `dev/interfaces` match source
candidate `294af94b5`. This supports reuse of those receipts; it does not bind
historical baselines to a new candidate. Relevant raw logs and RED/GREEN artifact
paths remain recorded in the recovery receipt. No runtime test was rerun: no
new defect or uncovered changed behavior justified additional execution.

## Remaining qualification gaps

- Official public/hidden comparisons and the hidden release probe have not run
  successfully: public capture requires 100 GB free and hidden CUDA preflight
  fails with the recorded NVML driver/library mismatch. Immutable baselines stand.
- GPU runtime evidence for applicable moved routes remains outstanding on a
  capable executor; CPU results and feature compilation do not replace it.
- The original combined verification remains FAIL. Its eight Python child-import
  failures passed a separate environment-corrected rerun. TypeScript and hermetic
  N-API executable suites skipped; their source was untouched.

Review is complete within its scope. Qualification gaps remain acceptance
requirements. No production code, tests, release bindings, baselines or hardware
settings changed; no landing, rebinding or Slice 90 settlement is implied.
