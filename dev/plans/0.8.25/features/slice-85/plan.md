---
title: Slice 85 — final verification, CI and non-publishing packaging
status: DRAFT
depends_on: 80
---

# Slice 85 — final verification and packaging

## Outcome and authority

Prove one final implementation candidate across required functional,
performance, SDK/API, installed-artifact, platform and CI contracts.
This replaces Slice 75 as final verification owner, preserving rather than
discarding its work. Closure is release readiness, not publication.

This is a planning brief pending the Slice 80 design and invalidation map.
Before READY, independently review and seal an executable matrix derived
from the retained [Slice 75 matrix](../slice-75/plan.md) and
[manifest](../slice-75/slice75-closure-manifest.json). Do not execute their
entire campaigns blindly or remove an obligation merely because it is costly.

## Required reconciliation

For every original manifest cell, record original SHA/artifact/log digest,
actual count/status, relevant inputs, diff-to-final, and disposition:
reuse with proof, rerun because invalidated, run because missing/unexecuted,
or explicit pre-existing unavailable/out-of-scope. Missing receipts are not
passes. Approved Slice 75 oracle corrections remain in force; do not repin
again without a genuine separately approved contract change.

Keep legacy Slice 75 manifest/tests as historical execution contracts.
Any new manifest/validator must get focused RED tests for missing coverage,
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
   permits; do not rerun the 76/77 experimental matrix.
4. Both protected 71B 10k candidate workloads when invalidated, with retained
   guards and no historical baseline reruns. Keep ack and drained-total
   measurements distinct and do not infer write performance from AC-020.
5. Risk-weighted API/SDK coverage manifest: public Rust, Python, TypeScript,
   wire/error codecs, configured features and native bindings. Give every
   surface an evidence/disposition row; name high-risk mutation, erasure,
   authorization/eligibility, lifecycle and concurrent-read interactions.
   Symbol/allowlist parity alone is not SDK behavior coverage.
6. Installed Linux x64 wheel/N-API/CLI candidate artifacts, Python 3.10/3.11/
   3.12 and Node 18/release-current runtime reuse, no editable/source fallback,
   cross-SDK frozen/dependency/lifecycle workflows and packaged GLOBAL-01
   native-search witness without answer-quality claims or model spend.
7. Exact-candidate native validation on Linux x64/ARM64, macOS x64/ARM64,
   Windows x64 CPU; Linux x64 combined CUDA packages; Jetson/Tegra Python
   CUDA with positive allocation/model-forward evidence. Reconcile actual
   supported feature sets rather than asserting all-feature Cartesian parity.
   Slice 72 CE and Slice 73 Windows deep receipts require input applicability;
   core runtime changes may invalidate them even without wrapper changes.
8. Exact-head required hosted CI with job/matrix/run SHA identities and
   explicit path-conditioned/skipped/advisory dispositions. Remote workflow
   dispatch needs the normal execution authority; this planning edit does not
   authorize a push merely to make the candidate available.
9. If runtime/linkage changed: artifact-specific ELF/Mach-O/PE symbol/import
   proofs, Rust co-tenant plus installed Python/Node initialization-order
   compatibility, same-file safety disposition, memory policy and extension
   binding checks. These are mandatory when applicable, not inferred from
   Linux prototype bytes. If no runtime change, retain the usage-risk audit
   and test/document only the approved resolution.

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

## Completion

Every inherited and new required obligation has verified passing or explicitly
authorized non-pass disposition; the Slice 80 successor has applicable passing
acceptance. AC-020 remains retired/superseded, not retroactively passing.
Publish durable final manifest/results/reviews under
dev/plans/runs/0.8.25-slice-85/, reconcile release-state generated views and
the package inventory, and hand off remaining publishing authority separately.
