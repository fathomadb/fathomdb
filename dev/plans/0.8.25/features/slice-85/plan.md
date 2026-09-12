---
title: Slice 85 — final verification, CI and non-publishing packaging
status: READY
depends_on: 80
---

# Slice 85 — final verification and packaging

## Planning reconciliation

The original brief landed at `b8a1a228` before Slices 76–80. The reviewed
baseline is `56e6d7fd`. The intervening work changes this slice as follows:

1. Slice 76 measured statement reuse and compilation/accounting behavior, then
   removed its experimental product diff. It contributes diagnostic evidence,
   not a shipping treatment or a release pass.
2. Slice 77 produced an inconclusive residual profile, selected no treatment,
   and restored the protected product/test tree. Its experiment matrix must not
   be repeated here.
3. Slice 79 shipped one startup-only runtime configuration owner plus bounded
   reader statement reuse. It added Rust/Python/TypeScript public surfaces,
   changed Engine/runtime initialization and the schema-26 upgrade fixture, and
   passed focused source and installed-binding tests. Those are final-candidate
   invalidation inputs and require broad plus installed-artifact coverage.
4. Slice 80 replaced retired AC-020 with accepted AC-081a/b/c, retained AC-072
   with host swap as diagnostic context, and accepted applicable protected 71B
   write evidence. Its 76/77 tooling and performance evidence are reusable
   only under their recorded input identities.
5. The typed verifier still registers 109 suites (106 fast, three heavy).
   Therefore the original 109-label expectation remains current; the full
   round must show real Python and TypeScript execution and no skip/exclusion.
6. Package metadata remains `0.8.24`. Slice 85 may rehearse local artifacts at
   that truthful version but cannot silently cut 0.8.25, publish, or claim a
   publish-ready dependent-crate rehearsal that requires the version cut.
7. The local `windchill3` host has Rust 1.95, Python 3.12, Node 25 and packaging
   tools. An unconfined device check resolves the registered Slice 72 RTX 3090
   as `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`; the Slice 72 baseline was
   recorded on this same host/device and CPU affinity. Existing uv-managed
   CPython 3.10 and 3.11 interpreters plus system CPython 3.12 provide the
   local runtime-floor executor. Node 25.9.0 is the sole Node target for this
   release because it is the runtime used throughout earlier 0.8.25 development
   and testing. Compatibility with other Node release lines is deferred. The
   matrix binds absolute interpreter paths. GPU checks must run with device
   access rather than treating a sandboxed driver failure as absence.

This reconciliation approves the draft direction with the adjustments below.
It rejects rebuilding the Slice 75 framework, rerunning the 76/77 experiments,
restoring AC-020, multiplying equivalent SDK/platform combinations, or using a
version cut as a packaging shortcut.

## Needs, requirements and acceptance

- **N85-1:** a release owner needs one auditable, deduplicated verdict for the
  final 0.8.25 candidate before deciding whether to publish. This is the final
  owner of draft N25-04 and R25-75/AC25-75 after Slice 75's checkpoint transfer.
- **R85-1:** every inherited Slice 75 cell and every Slice 79/80 addition has a
  final disposition bound to its relevant inputs and evidence.
- **R85-2:** the candidate passes one full local verification round plus the
  risk-weighted long, installed-artifact, model, platform and CI obligations;
  a missing, skipped, zero-test or stale result is not a pass.
- **R85-3:** exact artifact/source/toolchain identity is preserved across build
  and consumption, without editable/source fallback or registry writes.
- **R85-4:** closure changes only release evidence and state unless a failing
  test exposes a bounded product/runner defect; such a fix follows RED/GREEN
  and invalidates only affected evidence.
- **AC85-1:** the Slice 85 manifest accounts for all 26 legacy cell IDs plus
  runtime configuration and protected-write applicability, with no duplicate
  ID and a permitted final disposition for every row.
- **AC85-2:** `agent-verify --tier=all`, strict MkDocs, full workspace
  all-target check/clippy, selected long reliability/performance/model tests,
  and installed Linux SDK/CLI workflows pass with positive test/operation
  counts; output records exact commands and identities.
- **AC85-3:** the five-row native matrix, CUDA/Tegra routes and required hosted
  CI pass at the exact candidate, or an already-authorized unavailable cell is
  named as a non-pass. AC-034c is the only pre-authorized unavailable cell;
  the CUDA Engine comparison uses the accepted disposition in the
  [CUDA Engine p95 exception](ce-engine-p95-exception.md).
- **AC85-4:** retained AC-081, AC-072, 71B, Slice 72 CE and Slice 73 Windows
  receipts are reused only when an explicit relevant-input comparison proves
  applicability; otherwise their current route is rerun.
- **AC85-5:** final status distinguishes release readiness from publication and
  records any blocking non-pass without relaxing an oracle.

R25-75/AC25-75 close only through this explicit mapping:

| Allocated outcome | Required Slice 85 evidence |
| --- | --- |
| Cross-SDK and wire parity | Full Rust/Python/TypeScript round, installed Linux/native matrix, the shared frozen-context/database fixture and wire-equivalent values, plus risk-weighted mutation, erasure, eligibility, lifecycle and runtime-mode behavior. |
| Snapshot concurrency and lifecycle closure | `final-interactions`, populated `schema26-upgrade`, AC-021, AC-059b, AC-034a/b, AC-081c and the installed frozen/dependency/lifecycle workflows. |
| Predictable cold/steady performance and resource costs | AC-076, accepted AC-081/AC-072 distributions, rerun CE CPU/CUDA profile, rerun protected 71B workloads, EU7 AC-073/075, and recorded cold/steady latency, throughput, RSS/VRAM and uncertainty where the owning protocol defines it. Descriptive resource values are not new limits. |
| Retrieval-only evaluation | Installed GLOBAL-01 native search and EU7 Engine evidence with positive execution/model counts; no answer-quality or spend claim. |
| Missing platform/lifecycle evidence fails | Final manifest validator plus exact-candidate five-platform, CUDA/Tegra and hosted-CI receipts; AC-034c remains the sole authorized unavailable cell, and the CUDA Engine comparison links the [accepted disposition](ce-engine-p95-exception.md). |

The detailed architecture and execution ownership are in [the design](design.md).

## Outcome and authority

Prove one final implementation candidate across required functional,
performance, SDK/API, installed-artifact, platform and CI contracts.
This replaces Slice 75 as final verification owner, preserving rather than
discarding its work. Closure is release readiness, not publication.

Slice 80 is complete; consume its [current evidence and handoff](../slice-80/current-evidence.md).
The [execution matrix](execution-matrix.md) reconciles the retained
[Slice 75 manifest](../slice-75/slice75-closure-manifest.json) with that outcome.
Before READY, fill the candidate/executor/artifact paths and actual reuse
decisions, then review the matrix. Do not execute old campaigns blindly or
remove an obligation merely because it is costly. No new validation framework
is required to maintain this inventory.

## Required reconciliation

For every original manifest cell, record original SHA/artifact/log digest,
actual count/status, relevant inputs, diff-to-final, and disposition:
reuse with proof, rerun because invalidated, run because missing/unexecuted,
or explicit pre-existing unavailable/out-of-scope. Missing receipts are not
passes. Approved Slice 75 oracle corrections remain in force; do not repin
again without a genuine separately approved contract change.

Keep legacy Slice 75 manifest/tests as historical execution contracts.
The new manifest/validator gets focused RED tests for missing coverage,
zero-test/skip, stale artifact, relaxed threshold and false reuse. Do not
rewrite old result files to look like the final candidate.

## Minimum coverage inventory

1. One deduplicated full default workspace/SDK verification round:
   strict workspace all-target check/clippy, actual Rust/Python/TypeScript
   runtime tests, lint/security and strict MkDocs. Use the repository typed
   runner and inventory what it covers before adding commands.
   Do not stack all + fast + heavy or scripts/check.sh duplicates.
   Preserve legitimate model exclusions only when separate live-model cells
   positively execute the excluded bodies.
2. Retained final interactions and populated schema-26 upgrade/reopen;
   affected eligibility, frozen context, lifecycle/erasure, projection,
   WAL/checkpoint, concurrency/DDL and cursor/commit contracts.
   Selected long AC-021, AC-059b and AC-034a/b retain original protocols;
   AC-034c remains explicitly unavailable, not passed.
3. Slice 80 AC-081a/b/c absolute read-performance and reader-independence
   successor on its registered executor
   (seq-277; see [design](../slice-80/design.md)); AC-020 is retired, not a
   continuing ratio gate. AC-072 10k/384d at
   p50 <=80 ms/p99 <=300 ms; AC-076 text latency; real AC-073 stress and
   AC-075 vector-stage fidelity with positive execution/model counts.
   Consume exact Slice 80 recovery evidence where final-input identity
   permits; do not rerun the 76/77 experimental matrix. AC-072 host-wide swap
   remains diagnostic under the approved Slice 80 policy, not an automatic
   invalidation. The synthetic warm hybrid-search gate does not replace the
   separately required real-corpus stress/fidelity evidence.
4. Both protected 71B 10k candidate workloads are invalidated by the Slice 80
   `fathomdb-engine/src` change and must rerun, with retained
   guards and no historical baseline reruns. Keep ack and drained-total
   measurements distinct and do not infer write performance from AC-020. The
   intervening Engine edits are `debug_assertions`-gated and do not themselves
   establish a release-path regression; this conservative bounded rerun is
   retained because the recorded broad invalidation policy still matches.
5. Risk-weighted API/SDK coverage manifest: public Rust, Python, TypeScript,
   wire/error codecs, configured features and native bindings. Give every
   surface an evidence/disposition row; name high-risk mutation, erasure,
   authorization/eligibility, lifecycle and concurrent-read interactions.
   Symbol/allowlist parity alone is not SDK behavior coverage.
   Include `admin.configure_runtime` / `admin.configureRuntime` across Rust,
   Python and TypeScript: both modes, startup ordering, conflicting/late calls,
   and intended statistics/heap-limit behavior. Reuse Slice 79 proofs where
   applicable and exercise final installed bindings in fresh processes.
6. Installed Linux x64 wheel/N-API/CLI candidate artifacts, Python 3.10/3.11/
   3.12 and Node 25 runtime reuse, no editable/source fallback,
   cross-SDK frozen/dependency/lifecycle workflows and packaged GLOBAL-01
   native-search witness without answer-quality claims or model spend.
7. Exact-candidate native validation on Linux x64/ARM64, macOS x64/ARM64,
   Windows x64 CPU; Linux x64 combined CUDA packages; Jetson/Tegra Python
   CUDA with positive allocation/model-forward evidence. Reconcile actual
   supported feature sets rather than asserting all-feature Cartesian parity.
   Slice 72 CE and Slice 73 Windows deep receipts require input applicability;
   core runtime changes may invalidate them even without wrapper changes.
8. Exact-head required hosted CI with job/matrix/run SHA identities and
   explicit path-conditioned/skipped/advisory dispositions. After local
   verification and code review, fast-forward the GREEN candidate into the
   durable local `release/0.8.25` worktree, request explicit push authority,
   push that release branch, verify its remote head equals `FINAL_SHA`, and
   only then dispatch. This candidate-placement merge is not closure: the
   Slice 85 worktree remains until evidence/status-only commits are reviewed
   and merged. If push authority is withheld, hosted verification is blocked.
9. Verify final runtime/linkage behavior against the approved Slice 79 contract.
   Performance mode intentionally disables accounting/heap-limit facilities in
   its SQLite runtime; diagnostics restores them at process restart. Rust does
   not promise private SQLite isolation. Reuse applicable compatibility/linkage
   proofs and check changed artifacts as needed; do not require unchanged host
   statistics in performance mode, invent a shared-runtime compatibility mode,
   or reopen private-runtime isolation or database-file connection guards.

## Packaging, not publishing

Build each distinct final feature/platform artifact once and reuse identical
bytes across applicable installation/runtime tests. Bind archives, wheel,
native module and package-tree hashes to source/toolchain/flags. No registry
downloads of older dependent crates may masquerade as the candidate.

Retain the Slice 75 release workspace/CLI build and independently packageable
leaf-crate rehearsal. For dependent crates, seal a local/staged resolution
strategy and label any version/path overlay explicitly; if exact publish-ready
packaging requires the separately authorized version cut, record that boundary
and its uncompleted publish-rehearsal obligation. Do not bypass verification
or publish dependencies as a convenience.

No version cut, tag, release creation, registry staging/write, first-party-index
upload, publication, post-publication smoke or merge to main. Use
publish_to_pages=false on the Tegra evidence route. Actual publication remains
a distinct owner decision after this slice.

## Efficient execution and stop policy

- Preflight dependencies/models/platform access before expensive execution.
- Seal exact commands, environment, timeout, positive counts, thresholds,
  evidence paths and input sets before READY. Do not reuse stale numeric suite
  totals blindly; reconcile added tests without changing substantive oracles.
- Freeze the candidate after focused Slice 80 recovery/review. Run cheap
  checks first and short-circuit genuine failures. Serialize timing against
  builds and other performance work.
- Inspect each changed runner/artifact route and run its short relevant smoke
  before the longer suite. Fix ordinary runner/setup defects directly and
  rerun only affected checks; a stopped cell does not stop other useful work.
- One final broad round is planned. An exceptional second requires owner
  authorization with the changed-input/coverage reason. A narrow fix reruns
  only invalidated cells; do not restart unaffected broad work.
- Preserve all failed/invalid evidence. A changed candidate requires explicit
  applicability review, not relabeling old artifact receipts.
- No new performance optimization campaign here. A material product defect
  returns to a bounded implementation decision; no oracle relaxation.
- Separate independent implementation/tooling review from the evidence audit.
  Missing platform/model/receipt, unresolved successor acceptance or a material safety
  finding blocks release-readiness closure.

## Implementation and TDD sequence

1. **RED:** add focused validator tests proving the final manifest rejects a
   missing legacy/additional row, duplicate ID, forbidden disposition, passing
   row without evidence, zero-test/skip, stale candidate or artifact identity,
   relaxed thresholds, false reuse, and AC-034c represented as a pass. Commit
   the RED before the validator/manifest implementation. Add a runner RED that
   requires three installed Python and three installed Node runtime-mode cases.
2. **GREEN:** implement the smallest manifest validator, candidate manifest and
   installed-runtime smoke addition needed for AC85-1/2. Do not build a
   scheduler or replace existing cell runners.
3. Run cheap route/test-list/preflight smokes, then freeze the candidate. Source
   runtime-configuration tests positively counted by the broad round satisfy
   their rows; run the focused selectors only for missing counts or diagnosis.
4. Execute the matrix in dependency/cost order, recording immutable raw logs
   and hashes. Fix only actual product or route defects; add a reproducing RED
   first and rerun only invalidated cells after GREEN.
5. Obtain independent code review of the validator/manifest. After the local
   gate passes, stage the reviewed GREEN commit on `release/0.8.25`, obtain
   explicit push authority, push, confirm the remote branch head, and collect
   exact-head hosted evidence.
6. Obtain independent verification of the final evidence, then update release
   state and status. Merge any evidence/status-only commits into the already
   staged release branch, verify exact ancestry/tree and clean status, then
   remove the Slice 85 worktree and branch. Do not merge to `main`.

## Completion

Every inherited and new required obligation has verified passing or explicitly
authorized non-pass disposition; the Slice 80 successor has applicable passing
acceptance. The CUDA Engine comparison disposition is recorded in the
[CUDA Engine p95 exception](ce-engine-p95-exception.md). AC-020 remains
retired/superseded, not retroactively passing.
Publish durable final manifest/results/reviews under
dev/plans/runs/0.8.25-slice-85/, reconcile release-state generated views and
the package inventory, and hand off remaining publishing authority separately.
