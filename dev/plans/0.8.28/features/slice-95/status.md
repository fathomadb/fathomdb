---
title: FathomDB 0.8.28 Slice 95 — preparation completion
status: COMPLETE
---

# Slice 95 status

**PREPUBLICATION COMPLETE. Publication held.** Reviewed implementation
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

## Publication hold, retained limits and cleanup

The next action is explicit owner publication authorization. No tag, registry
or Pages publication occurred. Publisher jobs being skipped does not show their
publish/dry-run bodies executed. The original three local leaf dry-runs retain
their unchanged-input scope. Registry-installed end-to-end smoke follows actual
publication. Required rehearsal `pending_external` CLI observations remain
unexecuted scope, not passed or waived. D28-09/10/11 and populated-open deferrals
retain their existing dispositions. The literal durability ADR/evidence
discrepancy remains documented; process-kill tests are not machine-crash or
power-cut proof.

Task worktrees/branches are merged and removed;129 worker evidence files were
preserved with equal hashes. Temporary GPU runner29 is stopped/deregistered,
its private tmp2 directory removed, and the original selector restored. All13
restaged corpus copies were removed after original/copy checks and the real
graph pass; originals and qualification evidence remain. Original Windows disk,
XML, standing runner/service/policies and original project edits are preserved.
The exact oldqcow2 task disk was deleted under its explicit approval; runner30
auto-removed and the VM is off. Sealed Tegra diagnostic exports and the original
profile/recovered ELF remain retained evidence backups.

The optional Windows task cache `C:\fathomdb-s95-ci-20261109` remains retained
(4,527,596,302 logical bytes). Automatic approval review rejected its deletion
because the recorded deletion approval named only the qcow2 disk; an exact owner
question is pending. No deletion or workaround occurred. This cleanup decision
confers no permission to delete the directory.
