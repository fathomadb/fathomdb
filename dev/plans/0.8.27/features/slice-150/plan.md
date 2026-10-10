---
title: FathomDB 0.8.27 Slice 150 — integrated release qualification
status: REVIEWED_PLAN
target_release: 0.8.27
planning_baseline: 3a69eef82e0381641e5bfbf7dcd8ed25141d97ff
---

# Slice 150 — integrated release qualification

This plan reconciles the original Slice 150 paragraph in
[`plan-0.8.27.md`](../../../plan-0.8.27.md) with the release tree at
`3a69eef82` on 2026-10-10. It approves a bounded qualification scope, subject
to independent design review. Slice 150 changes no product behavior or SDK
contract. Its final execution waits for the owner's explicit confirmation that
Slice 140 is complete, then refreshes from `release/0.8.27` and verifies the
dependency and exact candidate from Git. Slice 160 owns publication.

## Reconciliation since the original draft

The master Slice 150 paragraph first appeared at `a3e6cff6f` on 2026-09-21.
The current release plan, release-state ladder, assigned features, prework test
approach, completed slice records, and allocated decisions were reviewed at the
planning baseline. These changes determine what remains here:

| Change or allocated work since `a3e6cff6f` | Slice 150 disposition |
| --- | --- |
| Slice 90 accepted engine runtime topology and retained D27, performance, Windows, Linux, installed-artifact and audit receipts. Its checkpoint artifacts include a 77.92 MiB host-local revalidation set. | Audit exact bindings and retention; run final-candidate routes. Historical receipts are context, not final-candidate passes. Do not repeat an entire diagnostic campaign without a concrete risk. |
| Slices 100–115 closed PyO3, NAPI, Tegra and warning/dead-code work. Slice 117 added a qualified but unpublished Jetson CUDA Node Pages tarball; Slice 160 now owns its publication and post-publication smoke. | Include applicable fresh package/platform routes and source-to-artifact hashes. No registry, Pages or Jetson publication claim. |
| Slices 120 and 130 decomposed TypeScript and Python. Slice 132 added the dedicated `fathomdb-sdk` Rust peer with 44/44 governed operation mappings, compile-negative controls and real-database behavior tests. It explicitly assigned Slice 150 a missing Rust SDK hidden-surface probe row. | Verify the three SDK surfaces and installed examples. Add the bounded `fathomdb-sdk` rustdoc row to the temporary hidden collector with a RED sensitivity test and reviewed additive capture; do not reimplement parity or resolve the external-provider ruling. |
| Slice 135 completed bounded 0.8.26 performance comparison and a 611-query CPU–CUDA sample at an earlier SHA, with raw licensed archives retained locally. | Review its limits and run the final-candidate performance/resource routes warranted by the changed source. Do not extrapolate its earlier sample to final acceptance. |
| Slice 140 is converging documentation, test-seam gates, generated navigation and the reviewed hidden-surface successor `f327c28c`. Its release-state entry remains `PLANNED` at this baseline. | Wait for explicit HITL completion confirmation and Git-verified closeout before execution. Re-capture surfaces and test inventory at the final SHA. No stale Slice 140 receipt becomes a Slice 150 pass. |
| The hidden-surface unit now requires a permanent feature-complete gate and test-target coverage, followed by oracle retirement after final comparison. | Run the heavy gate. Record manifest digests and comparison before retirement; preserve the permanent gates and matrix. |
| The owner made AC-037 live security an exact-final-candidate Slice 150 obligation. | Follow the HITL runbook and record all three pass lines, grant and revert; downgrade or missing host evidence blocks a PASS. |
| Slice 160 was added after the original draft for all publication destinations. Slice 137 consolidation is unruled proposal only. | Keep version bump, tag, registry, Pages, provider-policy and consolidation changes out of Slice 150. |

`dev/acceptance.md` remains locked. The IDs below are local to this release
slice; they neither rewrite historical criteria nor add product features.

## Needs, requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-150A | The candidate and dependency are unambiguous. | AC27-150A: after HITL confirmation, Slice 140 is recorded closed in release state with a reachable SHA on `release/0.8.27`; the Slice 150 checkout is refreshed to that branch. A clean reviewed candidate SHA, source tree hash and candidate manifest bind every result. A source change invalidates affected receipts and requires a new candidate. |
| R27-150B | Public and hidden contracts remain governed. | AC27-150B: a fresh immutable Slice 30 public capture is compared with its fixed baseline; intended inherited differences have explicit approved provenance. Add a `fathomdb-sdk` rustdoc hidden-surface row with a RED sensitivity test, then compare the final hidden capture and per-feature test inventory with the current reviewed successor. Review the additive SDK row without rewriting existing baseline entries. Every other difference is resolved or blocks acceptance; hashes and exact row counts are recorded before oracle retirement. |
| R27-150C | All required code and feature routes pass. | AC27-150C: `scripts/test-feature-complete.sh`, full `scripts/agent-verify.sh`, full-workspace Clippy and check, and separately applicable default/operator/test-hooks/slice72/benchmark/debug/release/CUDA combinations pass at the candidate. Record actual test counts, skips, exclusions, commands, exits and platform limits; no scoped run is called a full pass. |
| R27-150D | Candidate packages work outside source checkouts. | AC27-150D: build fresh Rust, Python and Node packages from the candidate; hash them; run clean, non-editable consumer smokes for the three SDKs, native registration, CPU/driverless paths and warranted CUDA paths. Run the frozen `dev/plans/0.8.27/features/slice-20/memex-consumer-oracle.py` unchanged against the candidate artifact. Windows and Jetson routes are executed where available or recorded unavailable without a pass claim. |
| R27-150E | Security has live exact-candidate proof. | AC27-150E: `STRICT=1 bash scripts/agent-security.sh` runs under the temporary per-binary namespace grant in `dev/release/ac-037-live-netns-hitl-runbook.md`, reports both required AC-037 pass/catch lines and `0 violation(s), 0 blocker(s), 0 downgrade(s)`, and the grant/revert witness is recorded against the candidate SHA. Repeat the complete grant/check/revert cycle after oracle retirement at the final executable commit. Later receipt-only commits prove the tested source and gate-input trees unchanged; an executable-input change requires requalification. |
| R27-150F | Retained evidence is accountable. | AC27-150F: inventory the Slice 90 77.92 MiB checkpoint revalidation set separately from other audit/qualification records. Survey objects larger than 100 decimal GB in both the 0.8.27 release evidence scope and the remainder of repository-associated data; record path, measured bytes, checkpoint owner, receipt binding and retain/cleanup decision, or explicit zero per scope. Do not delete data as part of the survey. |
| R27-150G | Qualified evidence is independently reviewed and the temporary oracle is retired safely. | AC27-150G: independent `gpt-6-sol` high code/evidence review and Terra verification report no unresolved material finding. After A–F pass, record final hidden/test-inventory digests in the hidden-surface status, prune owned scratch/cache through its tool, remove only the temporary oracle and its stated fixtures/baselines/fast-tier wiring, update cadence text, and pass scoped and full gates appropriate to that executable deletion. A failed or incomplete qualification leaves the oracle in place. |

## Planned execution, including RED/GREEN handling

1. Finish this plan and [design](design.md); obtain first design review from
   `gpt-6.1-sol` high. Use `gpt-6-sol` high for any later design reviews. Record
   findings and exact resolutions in `design-review.md`.
2. **Wait for HITL confirmation of Slice 140 completion.** Refresh from the
   release branch, run preflight with `--expect-closed 140`, verify the status
   and SHA from Git, and record the final source candidate.
3. Commit a RED collector test proving the absent SDK row, add only the
   Slice 132-assigned row, and capture/review its additive result. Run the
   comparators and qualification gates in the design's order, stopping
   at a real failure. Build and hash packages before installed consumer tests.
   Preserve raw commands, exits, counts and artifact identity in the manifest.
4. If a product or qualification-tool defect is found, preserve a genuine RED
   failing test or exact reproducer, assign the fix to its owning slice, review
   the fix, and re-run only the affected blast radius before the final gate.
   Tests and baselines are never weakened to accept a defect. Mechanical
   retirement of the temporary oracle is the planned code deletion; the SDK
   probe is the planned collector addition. Its existing tests and final
   capture serve as the pre-change proof, and
   post-deletion coverage/feature gates must remain GREEN. Update both
   cadence step 4 and the master Slice 150 paragraph to cite the final
   hidden-surface status digests.
5. Obtain `gpt-6-sol` high code/evidence review and independent Terra
   verification on the exact candidate. Resolve findings with focused checks.
   When all pre-retirement criteria pass, retire the oracle last, verify the
   resulting tree and repeat the complete live AC-037 runbook cycle on that
   final executable commit. Later record-only commits must prove unchanged
   tested source and gate-input trees. Then
   write [status.md](status.md), merge the Slice 150 branch into
   `release/0.8.27`, and remove the temporary worktree/branch. If closeout
   changes source or executable scripts, bind any affected gate anew.

The manifest is a factual record, not a publication authorization. Unavailable
hosts, an incomplete AC-037 grant, insufficient disk, or a real failing gate
remain explicit blockers rather than provisional PASSes.
