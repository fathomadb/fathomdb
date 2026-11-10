---
title: FathomDB 0.8.28 Slice 95 — completion
status: COMPLETE
---

# Slice 95 status

**COMPLETE. FathomDB 0.8.28 is published and its installed consumers are verified.**
Reviewed implementation
`3741dfb8a3d85bced4bfac6047083cc7e30da314` is integrated into `origin/main`,
`dev/0.8.28` and `release/0.8.28`. Astra design/code reviews and separate
verification close the implementation and qualification findings. The
[qualification record](qualification-evidence.json) preserves actual commands,
counts, source/artifact identities, failures, scoped resumptions and limits.

Qualification is assembled. The full CPU run at `4e6760744` remains **failed**;
the two contract/fixture defects were corrected and independently checked on
the affected targets. The full local run remains 204/206 with exit 1; restored
original corpus inputs and the reviewed Python boundary delta close its two
failed suites through scoped verification. No exact-final-SHA uninterrupted
full-matrix green claim is made. Production/Cargo inputs are unchanged from the
fully exercised v2 implementation; final changes concern test coordination,
explicit contract accounting and documentation.

## Acceptance and evidence

| Requirement | Completed preparation result |
| --- | --- |
| S95-1 coverage/truth | Full CPU CI42663174134: 34 jobs,32 success,1 heavy failure,1 conditional Markdown skip; fast199/199. Actual failures and advisory outcomes retained and resolved within their scopes. |
| S95-2 setup | Pinned cold/restored tool setup and wrong/missing-tool controls pass. Checkout-owned Python/native identity is verified. |
| S95-3 bounded parallel report | Actual report exit101,2390 passed/1 failed/65 ignored retained. The seed-before-pause fixture correction has Astra approval and independent28/28 parallel passes, including both ordering controls. |
| S95-4 installed consumers | All five CPU native rows/collector, distinct Windows WAL, installed Tegra Python/Node, genuine ARM driverless and x64 v2/v3 package consumers pass. External Rust archive scope remains separate from registry consumption. |
| S95-5 docs/contracts | Slice92 baseline and later deltas converge. Witness v2 interfaces/design/ADR and48 explicit Python declarations/12 exact removals retain frozen historical baseline and stub digest. |
| S95-6 final qualification | Original engine/feature/performance/reliability evidence retains its sources and limits; relevant input comparisons justify reuse. Restored60/60 graph documents and the unchanged real graph oracle pass; all3 canonical boundary tests pass. |
| S95-7 hosted execution | Failed full-run snapshots stay failed. Reviewed scoped fixes and source comparisons resolve their findings; no second full CI matrix was used for test-only corrections. |
| S95-8 review/handover | Astra approvals and independent checks complete. Code handover600beb06e preserves .29 MCP/filtered-body-FTS scope;39MCP+1SDK+6package controls and3 merge-boundary cases pass in their recorded scopes. Completion records follow the same forward-merge rule. |
| S95-9 harness robustness | Routing/tier/spill, truthful monotonic timing, namespace/PID and failure controls pass; final fast timings have no negative rows. |
| S95-10 evidence safety | SourceM scan0. Generated-text scan1328 files reports100 corroborated hashes/C identifiers,0 credentials/unresolved; actual exit1 preserved. History advisory1 and bounded71-match triage/caveat retained; policy unchanged. |
| S95-11 trusted rehearsal | Main-route dry-run42670315377 SUCCESS:15 successful jobs,1 non-gating history advisory failure,25 skipped. Actual four GPU preflight observations and both v2/v3 rehearsals pass; all23 publication-related jobs actually skipped. |

Raw local evidence is retained under
`target/slice95-final-format37/approved-ci/`; each independent scope has its own
verification record. The final state/views and completion commit are checked
with the existing scoped documentation validators and normal hooks.

## Measured baseline and architectural changes

Before the accepted throughput repair, AC011a measured999.651557 commits/s and
failed its unchanged1000 minimum. The final qualified e090 observation is
1215.559/s for1KB and291.286/s for100KB, against1000/100 floors. Separate
827.267649 and932.474344 experiment failures remain recorded; these campaigns
are not a causal percentage comparison. The
[visibility-coalescing ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
and qualification record retain the before/after design and measurement limits.
Current source comparisons preserve those original optimized measurements;
no new throughput/profile campaign or WAL/NORMAL policy change was made.

The [witness successor ADR](../../../../adr/ADR-0.8.28-attributable-cuda-allocation-witness.md)
replaces the shared UMA counter with checked private-pool live USED accounting
on integrated/private devices. Current installed Python and Node each prove
USED0→132848640, a touched67108864-byte control and exact release, with real
384-dimensional forwards. Global diagnostics9797632 and14508032 remain their
actual values. Four x64 observations retain global128MiB model growth and1GiB
control arithmetic with kernel-derived host/container PID binding. Historical
v1 failures remain unchanged. This changes the opt-in witness contract; allocator
policy, model pins, store format37 and ordinary opens retain their behavior.

The compiled patched dependencies were authenticated: cudarc-fathomdb0.19.10
atb777f8d94 and Candle core/nn/transformers0.11.0 plus kernel K9 at182ceab054.
All499 packaged files and25 kernel files match their source objects/archives;
local fork runtime code matches the qualified pins. No missing runtime patch
or dependency update was identified in the bounded branch/transcript audit.

## Publication authorization, retained limits and cleanup

Published tag `v0.8.28` remains `cdd41e2630a0084a5782128aa68d4de913aca90b`.
Production run42729768792 passes all required publishers, eight registry-smoke
jobs and co-tagging. Its overall cancellation retains the stopped non-gating
history advisory; the legacy npm promotion failed401. OIDC recovery42739775249
is SUCCESS: npm `latest` is0.8.28 and the public GitHub Release has five wheels.
Tegra Pages42735747672 is SUCCESS, including live Python/Node CUDA consumers.

Engine recovery42737348645 fixes the versioned self dev-dependency with a
path-only test dependency; real package RED101 became normal package GREEN0,
with default test-hooks consumer1/1. SDK recovery42738291355 uses the existing
API token after OIDC403. Both publish source `4b9e9fef89089bef9e03cd788ba4c136a09fcee0`;
runtime source is unchanged. Native builds were reused. The SDK trusted-publisher
configuration needs an operator check before the next release; no speculative
authentication change was made. Astra approved the focused recoveries.

Independent fresh registry Python, Node and Rust SDK consumers pass persisted
write/read, missing-record controls, separate-process reopen, close and exit.
Published CLI installation/version/row counts pass. Its conservative retained-term
certificate warning returns65; on a disposable copy, documented completion64
then doctor0 passes and the original record survives. The original remains unchanged.

Maintained documentation is current and Markdown-linted. All five published
Tegra artifacts are retained by digest. Temporary recovery code and task
worktrees/branches are removed; standing GPU/Windows/Tegra runners remain active
and the original GPU selector is restored.

Skipped original rehearsal publisher jobs do not show their publish/dry-run
bodies executed. The original three local leaf dry-runs retain
their unchanged-input scope. Registry-installed end-to-end smokes subsequently passed after actual
publication. Required rehearsal `pending_external` CLI observations remain
unexecuted scope, not passed or waived. D28-09/10/11 and populated-open deferrals
retain their existing dispositions. The literal durability ADR/evidence
discrepancy remains documented; process-kill tests are not machine-crash or
power-cut proof.

The prepublication task worktrees/branches were merged and removed;129 worker evidence files were
preserved with equal hashes. Temporary GPU runner29 is stopped/deregistered,
its private tmp2 directory removed, and the original selector restored. All13
restaged corpus copies were removed after original/copy checks and the real
graph pass; originals and qualification evidence remain. Original Windows disk,
XML, standing runner/service/policies and original project edits were preserved
by that cleanup. The later owner-directed runner restoration is recorded below.
The exact oldqcow2 task disk was deleted under its explicit approval; runner30
auto-removed and the VM was off after that cleanup. Sealed Tegra diagnostic exports and the original
profile/recovered ELF remain retained evidence backups.

The owner-authorized Windows task directory `C:\fathomdb-s95-ci-20261109`
was deleted and independently verified absent:97,482 files and4,527,596,302
logical bytes. No reparse points or active task processes were present. The
C-root entry diff contains only that directory; standing runner/configuration
hashes, Auto/Stopped service, execution policies and original VM XML are
unchanged. The VM was off after cleanup; the old task qcow2 remains absent. Receipt:
`target/slice95-final-format37/approved-ci/ci26-windows-cleanup/verification.json`.
The earlier deletion rejection is resolved by seq-366 and is historical.

CI26 prepared a fresh private local GPU executor under
`/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`, with public runner bits and
private Rust/tool caches. At that stage it was unregistered with no listener;
preparation changed no standing services, driver580 or primary CUDA12.6. CI28
below activates this installation as the restored standing runner. Receipt:
`target/slice95-final-format37/approved-ci/ci26-gpu-preparation/preparation.json`.

## Self-hosted runner readiness follow-up

On 2026-11-09 the owner requested running local runners and actual GitHub CI
reachability. The existing Windows VM suffices; no new VM/disk was created.
Its deleted registration24 and broken public junctions were repaired through
an Astra-approved bounded script, using an ephemeral repository registration
token on encrypted stdin. Registration31 is online; the existing Auto service
runs as NETWORK SERVICE. The official runner updated itself to2.338.2. A
controlled idle service restart and fresh GitHub session pass. The deleted
Slice95 task directory remains absent, and original VM XML/policies are
unchanged. The initial local console collector failed decoding Windows output;
no remote repair exit0 is invented and no mutation was replayed. Actual state
and subsequent CI execution independently prove the scoped result. SSH alias
existence checks remain inconsistent with successful service/worker execution;
the underlying cause is not claimed.

Manual readiness run42715234671 at
`af37af56f460b4a3c17a77b11b0f9ff0e769cb15` actually completed Windows
job129494925358 on runner31/`MEMEX-AFE8V4I00` and Jetson job129494925495 on
runner22/`ubuntu`, both SUCCESS. Tegra toolchains/CUDA/pinned model caches and
disk space were checked. The CUDA job129494925601 had no assigned runner and
was cancelled; the overall run is CANCELLED, not a three-platform pass. The
original organization CUDA runner6 in restricted group3 then had a deleted
registration; its endless restart loop was stopped without changing its unit,
driver580, primary CUDA12.6 or Coral service. Organization runner/group APIs
returned403 with the then-current CLI scopes. CI28 below resolves restoration
and proves group3 job delivery with authorized organization access; no
repository-runner replacement or protection-policy bypass was used.

The manual/main-only identity probe has empty permissions, fixed labels and
three-minute job limits; it performs no checkout, build or inference. Astra
code review approves `bb5ea9b7e6b59ab6f4679d1426c7e808b6053aee`; its three
reviewed objects match integrated `af37af56f`. RED and corrected RED precede
GREEN; the existing focused scope test, actionlint and normal workspace clippy
hooks pass. The one automatic proportional CI run is42715120008; heavy and
platform build rows were skipped. The run was cancelled during the remaining
bootstrap to limit Actions spend, with Astra approval:15 jobs succeeded,1 was
cancelled and10 skipped. No rerun or full CI-green claim is made.
The owned implementation worktree/branch was removed after preserving ten
evidence files with equal hashes. Evidence root:
`target/slice95-final-format37/approved-ci/ci27-runner-readiness/`.

## GPU restoration and publication preflight

The owner completed organization authorization and requested restoration in the
existing restricted group. The same organization/name was registered as
runner168 in group3. The existing enabled user service now runs the prepared
2.338.2 installation at `/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`,
with private Cargo/Rust/tool caches. **This is the active standing installation;
retain it.** The old installation is preserved for rollback. Astra approved
the bounded unit changes: installation paths/environment and
`Restart=on-failure`, preventing the demonstrated terminal-registration
restart storm while the official wrapper handles retryable/update exits.
Driver580, primary CUDA12.6 and the Coral service remain unchanged.

Actual run42726255745 at
`a3a364d99d83eba5f5d4f84ab3f4e70b4ab1fdaf` is **SUCCESS**: GPU168 on
`windchill3`, Windows31 on `MEMEX-AFE8V4I00`, and Jetson22 on `ubuntu` all
executed identity jobs successfully. Only the exact main-only probe workflow
was temporarily added to group3 for this proof; it was removed afterward.
Independent verification confirms the entire original group policy is
restored exactly. All three runners remain running. This proves identity and
job delivery; no inference, package build or publication is claimed.

At the pre-tag checkpoint, metadata/Axis-W/main-ancestry checks passed; the
protected route refused the then-missing entry and package endpoints returned404.
The owner subsequently approved the exact entries; publication is complete above.
Evidence and independent verification:
`target/slice95-final-format37/approved-ci/ci28-gpu-restoration/`.
