---
title: Slice 75 platform and artifact qualification
status: ACTIVE
slice: 75
target_release: 0.8.30
---

# Slice 75 — Platform and artifact qualification

Owner seq-407 allocates this scope from the prior 70 plan.
The whole slice is commissioned under the owner's direction to finish the
release; execution remains nonpublishing. Parent owns canonical state,
Slice70 closure and final integration/publication.

## Entry and ownership

Reconcile the commissioned checkout against
`9538272e638d6bca4547a61519a0ce9c98c3872d` and current `origin/main`.
Inventory relevant source, contracts, assigned Slice 3/4/5 and seq-406 drafts,
workflows, produced artifacts, platforms and pin changes. Decide which prior
evidence still applies, which claims need fresh proof and who owns blockers.
Use 10's approved environment, 20's consumer and diagnostics routes, 50's
selected runtime conclusions, 55's source fixture and CLI subset, 60's final
consumed pins, and 70's integrated source and behavior evidence.
[Master](../../plan.md), [coverage audit](../../coverage-audit.md).

Users need packages that load and behave correctly on the announced targets,
accurate support statements, and actionable failure evidence on the hosted
route. 75 qualifies those claims on the nonpublishing candidate. Slice 70
owns full source, features, security, long/canonical numerical and composed
behavior. Slice 80 owns protected production routing, registry installation,
publication and public readback. Preparation and independent rows can proceed
before a distinct platform or pin prerequisite closes.

## Requirements and acceptance

| Need and local requirement | Observable acceptance |
| --- | --- |
| R30-QUAL / supported artifact set | On Linux x64/ARM64, macOS x64/ARM64 and Windows x64, record Tier-1 Rust build/test and target-native installed candidate wheel, npm platform/main and CLI evidence as required. Check versions, package/tag/floor, native path and actual bytes. Exercise strict TypeScript and positive persisted write/read, missing-record, separate-process reopen, close and natural exit through maintained consumers at the appropriate API. Use the selected thin CLI doctor subset, not the deep 17-command matrix on every target. |
| R30-QUAL / downstream source archive | From a clean external consumer build the SDK and CLI against the exact source archive, preserving the producer lock, active source configuration and consumed pin identity. Show actual SDK and CLI behavior and immutable input/file-set checks. Source archive and local wheel/npm proof do not establish registry resolution. |
| R30-QUAL / real Windows and cancellation | On supported Windows, execute the maintained WAL checkpoint, local-verbatim-path and attribution controls with actual test IDs/runtime evidence and the applicable installed binding receipt. On an approved hosted runner, show an actual cancellation preserves first full redacted failure beyond 200 lines, exact completed outcomes and unrun later work. Consume Slice 20's offline control as separate evidence. |
| R30-QUAL / CUDA capability | Join final 60 pins and actual linkage, floor, kernel images/selection and installed results to the claimed target. Keep AC-082 driverless auto/CPU, forced typed CUDA refusal, approved positive device/PID/UUID witness and explicit CPU nonuse. Record actual supported and unsupported outcomes or hold the affected claim. Keep generic ARM64 CPU, approved classic Orin and x64 CUDA separate. |
| R30-QUAL / contracts, Node and renderer | Freeze the applicable contract and support metadata before building affected release proof. Verify interface/generated/public/changelog/version alignment and real installed behavior for any Node 26 public claim. Resolve the renderer advisory with a supported patched dependency graph and real compatible-browser startup and render/sanitizer results, or retain the exact unresolved hold. |
| R30-QUAL / delivery handoff | Produce one reviewed candidate proof map for 80 joining exact commit, artifact digests, scopes, original exits and remaining limits. Validate the approved nonpublishing release route. Required unknown evidence remains open or needs explicit owner disposition. |

## Architecture and qualification design

- Trace engine and governed SDK contracts through Python/Node and CLI into
  package build, platform resolution and the process that actually loads
  the native code. Verify the intended producer lock/features/toolchain and
  model/fixture identity. Use the maintained
  [native CI](../../../../../.github/workflows/ci.yml),
  [POSIX smoke](../../../../../scripts/release/smoke/smoke-local-native-artifacts.sh),
  [Windows smoke](../../../../../scripts/release/smoke/smoke-local-native-artifacts.ps1)
  and [receipt collector](../../../../../scripts/release/native-artifact-receipts.py).
  Reuse the [.29 archive/Q14 method](../../../0.8.29/features/slice-160/qualification.md)
  after checking current paths. Distinguish byte-identical reuse, proved
  same-source scope and new producers. Packaging or dependency changes can
  invalidate an installed receipt even when Rust code is identical.
- Confirm real Windows selectors, actual linked SQLite/WAL path and scope,
  and selected hosted job, later wait and cancellation trigger before running.
  A static workflow shape or local process kill is separate evidence.
  Reuse a maintained cancellation route if one is available. Return any
  missing or faulty workflow seam to 20. Retain earlier nonpasses and the
  actual hosted cancellation disposition. [Windows WAL design](../../../../design/0.8.23-windows-wal-checkpoint-reader-conflict.md),
  [Slice 20 status](../slice-20/status.md),
  [GitHub cancellation reference](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-cancellation).
- Consume [Slice 60](../slice-60/plan.md) for fork qualification, supported
  targets and selected native/PTX dispatch controls. Inspect actual final
  bytes with the [CUDA artifact contract](../../../../../scripts/release/cuda-artifact-contract.sh).
  For the [.29 Tegra delivery](../../../release-state-0.8.29.json), reuse
  verified historical public proof only within its own scope. Require actual
  Orin for selected Orin claims, and keep the established driver/toolkit
  policy. Preserve safe post-loss drop proof for conditional 50 release.
  Real reset needs separate authority and the selected venue; no CUDA 13
  promotion in .30. [Receiving allocations](../../release-followups.md).
- Reconcile interfaces and public claims, including actual graph-seeding
  behavior, Node engine range, Python/npm supported platforms, CLI guidance,
  Axis W/E, changelog and generated maps. Keep developer Node 26 and the
  held release/Tegra tool roles distinct until selected proof establishes
  the final support. The Linux packed Node 26 result from 10 is scoped.
  Treat current `>=25 <26` package metadata as a held public claim and
  validate any proposed change on packed bytes via the owned route before
  release. Use `check-sdk-surface-parity.py`, scoped Markdown/docs checks,
  generated-doc/state checks and `mkdocs build --strict` where applicable.
  [Slice 10 evidence](../slice-10/status.md),
  [release design](../../../../design/release.md).
- Check renderer locks and transitive dependency graph against the cited
  advisory and the browser pairing for the chosen Puppeteer version. The
  earlier 21-control render and sanitizer pass retained an affected Zod path
  through Puppeteer. Require a supported patched graph, resolution/audit
  output, actual compatible browser startup, render and sanitizer controls.
  Coordinate an executable or dependency repair with the tooling owner.
  Keep any unavailable browser result open and obtain separate download
  authority where required. [Renderer design](../slice-10/design.md#exporter-and-renderer-dispositions),
  [Puppeteer browser support](https://pptr.dev/supported-browsers).
- Resolve hard questions before freezing the candidate. Which wheel tags
  and floors describe the actual extension bytes? Which ABI and optional
  addon does the installed Node process resolve? Which metadata enters
  the archive, tarball, wheel or trust subject? Study primary packaging
  and loader documentation, then check fit against current scripts, lock
  and producer. [PyPA compatibility tags](https://packaging.python.org/en/latest/specifications/platform-compatibility-tags/),
  [Node-API ABI stability](https://nodejs.org/api/n-api.html#implications-of-abi-stability).

## Review, execution and closure

Have direct Astra-medium review the design and needed contract deltas, then
critically review the updated plan, with at most one FIX-1 and re-review.
Commit after PASS. Proceed only when commissioned and the selected venue,
route and inputs are available. Freeze clean source, contracts/support,
generated views and tools before affected proof. Execute existing platform
and artifact selectors and the applicable [nonpublishing checks](../../../../../scripts/verify-release-gates.sh).
Retain actual nonzero cases, exits, skips, first failures and artifact
identity. Record docs-only changes as such with evidence of unchanged affected
inputs. Rebuild or rerun only invalidated scopes.

Return defects to 20 for workflow/consumer fixes, 30 or 55 for correctness
and data integrity, 50 for runtime/resource behavior, 60 for pins/kernels,
or the relevant tooling owner. Genuine behavior repairs need a failing
RED and separate GREEN/refactor with direct Sol6.2-high code review. 75 uses
existing tests and defect-owner fixes. Direct Terra verifies the actual
platform execution, artifact binding and reuse claims. Reuse 70's unchanged
full source gates and run scoped rechecks after narrow fixes.

Record exact candidate and limits once in status and authorized state. Deliver
the reviewed artifact/receipt map, open dispositions and nonpublishing route
evidence to 80. Integrate the owning branch and clean only owned inactive
worktrees and scratch after retaining evidence. No permission to dispatch,
download, use a GPU, deliver a fault, spend or publish follows from this draft.

## Commissioned preparation at 0f5db6cc

Current origin/main `b86b19847852e9beaeea32b98dae03b537d8fea4` is already
an ancestor. No unrelated primary-checkout changes enter this candidate.
Since the proposal anchor,40 supplied measured nonadoption and a narrow search
repair;55 supplied composed integrity/fuzz/crash/WAL/thin fixtures;60 changed
Cudarc/Candle/cudaforge and published its five fork packages;50 added explicit
module-model release across Rust/Python/Node;70 repaired MCP serialization and
private benchmark/fixture execution. Consume their exact statuses rather than
replaying passing campaigns. Broad coverage-guided fuzzing and power-loss proof
are allocated to.31 by seq414.70 still owns the two current-pin CLS/shared-GPU
source rows; unavailable discrete telemetry does not close them.

Approve the prework3/4/5 installed persistence, native identity, diagnostics and
contract proposals in this slice. Retain the existing Engine/SDK ownership and
operator-only CLI. Reject a new qualification framework and duplicate deep
17-doctor matrix. Adjust prework's70 artifact ownership to the explicit75 split;
canonical source numerical/security results remain70-owned. Current source
format37, exact final60 pins, AxisE and PTX0 producer default remain fixed.

The local design and execution sequence are in [design](design.md). Direct
Astra-medium reviews both before implementation. Candidate-producing changes
are reserved to this worktree until freeze; the parent confirmed no imminent
source integration. One final exact candidate is pushed to a separate
`release/0.8.30-qualification` branch for existing manual CI. This leaves the
parent's canonical release ref under its sole ownership.
