---
title: 0.8.30 prework Slice 3 product contract proposals
status: COMPLETE
slice: 3
target_release: 0.8.30
date: 2026-11-18
---

# Slice 3 — Product contract proposals

## Review basis

Read [needs](../../../needs.md), [requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), the [architecture](../../../design/fathomdb-data-plane-architecture-v2.md)
and maintainers' [ADR index](../../../adr/ADR-0.6.0-decision-index.md) at
`18427b7a8`. Reconcile the [master](../plan.md), [receiving allocations](../release-followups.md)
and Slice 2 dispositions. The accepted format 37, governed SDK, typed public
admission and same-file ownership constraints govern this draft. The .27 SDK,
.28 transactional visibility and .29 record-version identity decisions are
successors to older scope descriptions. Proposed text below is local to this
review. It changes no accepted ADR, canonical AC amount, ID or test threshold.

## Proposed needs, requirements and acceptance

Use the following wording in the owning release-local requirements after selection.
Acceptance statements are observable. Numeric gates already defined by the
canonical contract retain their explicit profiles and limits.

| Operation and owner | Need and draft requirement | Local draft acceptance |
| --- | --- | --- |
| Update applicability in 10 | Contributors need one way to find applicable requirements. “Retain stable identifiers as traceability. Use the current release selector for commissioned work, and accepted ADRs and maintained interfaces for semantics. The integrated database currently bootstraps or admits schema 37 and refuses older nonempty stores. The .30 plan is a proposal until activation.” | The selected pointers resolve and distinguish proposal, active and historical work. Prior ledger, AC and release outcomes are intact. |
| Create delivery diagnostics refinement in 20 under R30-CI/DIAG | Maintainers need useful failure information even when the run stops. “Retain the first complete diagnostic and final exit for each completed unit through later cancellation. A release reports requested and observed versions and the actual native resolution outcome.” | A selected unit emits >200 lines, fails, and a subsequent unit is cancelled. The retained result identifies the first failure, cancelled and unrun scope. Wrong or absent npm native resolution fails with the expected path, observed path and versions. |
| Create correctness refinement in 30 under R30-COR/S50 | SDK and agent callers need exact stored-record attribution. “At fixed state and inputs, supported reference and optimized retrieval agree exactly. Distinct canonical record versions retain their own ranks, source and extents even when their bodies or public refs agree. Otherwise-recordable feedback rejects an ambiguous named ref before writing any label.” | Prove exact IDs, fields and score bits against independent ranks, and reject a mixed unique/ambiguous feedback call without any telemetry label. A known vector endpoint of zero remains present. A NULL endpoint remains missing. A failed capture cannot publish staged counts. |
| Create latency refinement in 40 under R30-LAT | Applications need useful completions under real caller load. “Evaluate a treatment on matched state, inputs, workload and native pins. Report caller p50/p95/p99, useful completions per time, failures and resources. Adopt only a reproducible useful improvement that preserves correctness and the approved comparison limits.” | Interleaved baseline/treatment preserves expected hit/error identity. Record every attempt and constituent boundary observed. Insufficient attribution or benefit yields a recorded non-adoption, with unchanged canonical performance gates. |
| Create runtime refinement in 50 under R30-ROB | Applications need predictable cancellation, close and restart. “Refuse queued expiry distinctly from started timeout. Finish SQLite quiescence before timing the provider drain. Owner-approved opt-in model release may drop resources only after safe-drop proof for the exact consumed fork scope.” | Rendezvous proves a committed write and progressing reader, queued/started outcomes and capacity reclamation, and exact persistent state after fresh-process reopen. If B30-01 is selected, concurrent use, repeated explicit release and healthy reload are observed on affected Rust/Python/TypeScript paths. |
| Create dependency qualification refinement in 60 under R30-FORK | Consumers need reproducible supported kernels and exact dependency identity. “Record the upstream bases, retained fork deltas and exact resolved pins. Dispatch only the declared native/PTX combinations, and preserve honest unsupported outcomes and private-pool behavior.” | Resolve the actual downstream graph, inspect emitted targets and run independent output and unsupported controls on selected venues. Bind proof to exact artifact and pin identities. A missing device leaves the affected claim unqualified. |
| Create integrated qualification refinement in 70 under R30-QUAL/TEST | Consumers need readiness that reflects runtime proof. “Each required result names its relevant source, features, inputs, platform and execution exit. Each selected test improvement names the owning fix and a concrete wrong result its oracle rejects.” | Actual full, feature, strict security, long, performance and artifact rows are present or expressly disposed by the owner. No skipped or zero-selection result is called PASS. Valid scoped reuse shows applicable inputs. Selected consolidation retains test meaning and actual cost/overlap evidence. |
| Create delivery refinement in 80 under R30-REL | Users need exact installed releases with working storage and truthful public links. “Publish only after separate authorization for the accepted immutable candidate. Resolve required exact versions before dependents run. Promote only the main npm package via the qualified OIDC path after all required publishers, registry consumers and co-tagging pass.” | Eight public consumer jobs prove the exact packages. A deployed SDK/binding fixture writes and reads its body, checks a missing record, closes and exits, and reopens and reads in a separate process. Pages serves expected public links and bytes. A partial publish leaves release status open. |

## Reconciliation with the completed library sweep

The subsequent [Slice1 result](slice-1.md) proposes narrow Markdown/TLS-root/Tokio
fixes and a Node26LTS investigation because25 is end-of-life. Add a conditional
local requirement allocated to10 and70: “A selected runtime/security update
preserves accepted API, typed-error, target-native loading and persistence
behavior on the declared support set; claims change only after qualification.”
Acceptance is the selected lock/feature diff plus affected tools, MCP/async and
fresh native consumer tests, with negative outcomes and version ranges retained.
The owner selects the Node migration and exporter/rendering cohorts in9; no
current25 support statement is silently rewritten and no candidate is adopted.

## Withdrawals, labels and reserved decisions

- **Delete from new local wording only.** Drop any proposed unconditional
  natural-write score-bit comparison across separately moving reader states.
  Replace it with “capture candidate identities, ranks, tie order and reader
  state for an attributable comparison.” Preserve the actual .29 S50 failed
  result and fixed-state A3 exactness. Do not choose a tolerance.
- **Rename an assumption in local design.** Use “record-version key” for the
  internal fusion owner and “public reference” for the visible lookup label.
  Keep logical fetch latest-eligible under its view and preserve duplicate-ID
  context packing refusal. Exact per-result label representation remains a
  Slice 30 review and owner choice under seq-375.
- **Update product status prose after proof.** Remove the stale current
  published .28/role-of-Slice-95 language in v2 only after distinguishing
  integrated and published .29 evidence. Preserve the live discrepancy between
  the .29 state and docs/manifest claim following `42629c2a1`.
- **Keep historical criteria visible.** AC-020's ratio is retired. Report-only
  synthetic recall and stress do not replace real-corpus proof. Older migration
  fixtures remain internal. Public governance and erasure effect boundaries
  continue under their accepted ADRs.

## Draft architecture delta

- **Update the current-profile paragraph (10 if selected).** “An Engine uses
  one managed SQLite database, a primary mutation connection, projection writers
  that serialize commits and pooled reader transactions. Rust, Python and
  TypeScript wrap the governed Engine. The MCP adapter shapes and budgets
  responses at the adapter boundary.” Retain architectural history in place.
- **Create a release-evidence paragraph (70/80).** “The same-state retrieval
  witness belongs to the reader transaction. Committed coverage publication
  belongs to the writer token. Qualification follows the exact built native
  subject. Publication follows the immutable authorized tag and fresh public
  installations.” Check source fit in Slice 4 before turning this into design.
- **Delete no accepted infrastructure.** Preserve typed failure and explicit
  release authorities. Existing projection providers, cache and backend are
  sufficient to investigate selected cases. A replacement needs new evidence
  and an owner-approved design before scope changes.
- **Keep Inspector v0.1 separately governed (9).** The `dev/0.8.30` branch
  at `7d3637657` has its own needs, acceptance, v0.1 ladder, and pending
  independent checks. The database release proposal grants no Inspector
  change, package, browser support or integrated qualification claim.

This completes the review and local drafts. No authoritative product record
or executable artifact changed.
