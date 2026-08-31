---
title: 0.8.24 Slice 50 — Windows Python SDK WAL review draft plan
status: POSTPONED-TO-0.8.26
target_release: 0.8.24
---

# Slice 50 — Windows Python SDK WAL behavior review

> **POSTPONED TO 0.8.26 — HITL ruling `seq-258`.** This retained draft is
> planning input only. No Memex evidence retrieval, Windows reproduction,
> diagnostic, workflow, or product action is authorized under 0.8.24.

## Planning boundary

This document plans an evidence and attribution slice. It does not declare a
FathomDB defect, change retry budgets, modify reader pools or binding lifetime,
rerun the Memex suite, or implement a product fix. The required first outcome
is exactly one typed disposition: **reproduced and attributed**, **not
reproduced**, or **insufficient evidence**.

An attributed product change, if any, receives a separately reviewed follow-on
plan and HITL decision. The review must not make a broad client observation
become an in-slice code change by momentum.

## Goal and outcome

Ground the 0.8.24 Windows WAL disposition in the actual completed Memex job and
the installed FathomDB Python path:

- preserve the external job's commit, package, environment, command, failure,
  and artifact provenance;
- compare it with existing FathomDB first-party Windows checkpoint and
  source/installed-wheel attribution controls;
- reproduce only the smallest real-SQLite client sequence needed for
  attribution, if existing evidence is insufficient;
- retain fail-closed `ErasureIncomplete` semantics throughout; and
- route an attributed fix proposal, durable no-change record, or explicit
  evidence gap to the correct next owner.

## Authority and inputs

- P24-13, R24-6/R24-12, A24-7, NEED-020a, and Slice 6 approval.
- Memex job
  <https://github.com/coreyt/memex/actions/runs/32587291032/job/97065598178>.
- `dev/notes/0.8.23-memex-windows-x64-wal-evidence-20260816.md`.
- `dev/design/0.8.23-{wal-attribution-investigation,windows-wal-checkpoint-reader-conflict}.md`.
- Current `ci.yml` Windows WAL jobs, engine WAL controls, and
  `src/python/tests/test_slice65_wal_attribution_{installed,typing}.py`.
- The native Windows VM record and memory `windows-vm-for-fathomdb-testing.md`
  as a possible first-party comparison environment.

The existing note and controls are supporting evidence. They do not substitute
for the linked job's actual result, nor does the job title establish cause.

## Scope

### In scope

- Retrieve and preserve available job logs/artifact metadata through read-only
  GitHub access; identify exact Memex source/client path and installed FathomDB
  version.
- Compare the client sequence with released and current FathomDB installed
  Python controls on a native Windows x64 environment.
- Use a small real SQLite database and process/lifetime sequence, not a mocked
  database or the full Memex suite, for any new comparison.
- Retain reader/connection/process/checkpoint timing and typed result evidence
  sufficient to distinguish FathomDB-owned, client-owned/external, and unknown.
- Produce a durable typed disposition and follow-on recommendation.

### Non-goals

- Retrying until green, treating a later pass as success of an earlier failed
  erase, widening checkpoint retries, changing GC/finalizers, or redesigning the
  reader pool without attribution.
- Importing Memex code or fixtures into FathomDB.
- Re-running hosted CI merely because old logs are inconvenient; use the local
  Windows environment when it can answer the same first-party question.
- Windows CUDA/package publication work, which belongs to Slices 40/60.
- Writing to a Memex ledger or repository.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, evidence availability, current-main SHA, and environment;
- `draft-contracts.md` — slice-local needs/requirements/acceptance drafts;
- `design.md` — evidence-attribution and minimal-reproduction design;
- `research.md` — primary SQLite/Python/GitHub questions and findings; and
- `disposition.md` — final typed outcome, confidence, evidence, and next owner.

### Prep tasks

1. Read the linked job metadata/logs/artifact list once available. Record the
   run/job SHA, event, runner OS, Python/FathomDB versions, exact failing test or
   command, error text, elapsed/retry/process evidence, and artifact digests.
2. If logs expired or access is unavailable, record precisely what is missing.
   Do not infer it from the 0.8.23 handoff note.
3. Restate existing NEED-020a and current fail-closed erasure requirements.
   Propose slice-local drafts:
   - **N50-DRAFT:** Windows Python clients need WAL-checkpoint failures
     attributed without false success or an unexplained reliability change;
   - **R50-DRAFT-1:** external and first-party evidence retain package/source,
     process, connection-owner, transaction, checkpoint, timing, and result facts;
   - **R50-DRAFT-2:** every conclusion uses one of the three typed dispositions
     and preserves `ErasureIncomplete` until a complete independent attempt;
   - **AC50-DRAFT:** a minimal installed-wheel comparison either reproduces
     with a named owner/mechanism, does not reproduce under recorded equivalent
     conditions, or names the exact missing evidence that prevents attribution.
4. Read bindings/engine architecture and the actual checkpoint, reader-pool,
   Python ownership, and existing test code. Write an exists-versus-net-new map.
5. Verify every current test assertion in the full test body. Existing job
   success proves its exact control only, not the Memex outcome.

## Evidence and reproduction design

### Evidence ladder

Use the least expansive evidence that answers the attribution question:

1. Existing Memex job logs/artifacts and source-controlled audit record.
2. Existing FathomDB released/current installed-wheel Windows attribution
   artifacts for equivalent sequence and package version.
3. A bounded native-Windows real-SQLite reproduction of the exact client
   close/open/recovery-read/erase or purge sequence.
4. Only if steps 1–3 identify a missing first-party observable, a test-only
   diagnostic addition with a separately reviewed design.

Do not start with the full Memex suite or a retry-budget experiment.

### Outcome predicates

- **REPRODUCED_AND_ATTRIBUTED:** the equivalent installed path reproduces and
  evidence names the responsible surviving owner/mechanism with a falsifiable
  first-party control.
- **NOT_REPRODUCED:** equivalent recorded conditions execute without the
  behavior across the predeclared bounded comparison; this does not assert the
  historical external failure was unreal.
- **INSUFFICIENT_EVIDENCE:** a named missing artifact, condition, owner
  inventory, or equivalent environment prevents either conclusion.

Each outcome states confidence and alternative explanations. “Intermittent” is
not a fourth outcome.

### Challenging aspects and research plan

- Consult SQLite's official WAL/checkpoint result and transaction-lifetime
  documentation for BUSY/frame semantics; do not infer connection ownership
  from a checkpoint count.
- Consult PyO3/Python object-lifetime documentation only if actual evidence
  points at binding ownership; do not begin with a GC hypothesis.
- Use GitHub's official artifact/log retention/API docs to explain unavailable
  evidence.
- Compare Windows NTFS/process behavior through a native Windows control; Linux
  or WSL behavior is not an equivalent substitute.

### Architectural-fit review and revision

Review the design against `dev/design/bindings.md`, engine lifecycle/reader
architecture, existing WAL designs, and actual code. Revise away any proposed
production instrumentation, retry, API, or pool change not required to reach a
typed disposition. If a public error/interface change appears necessary, stop
and draft an ADR/interface follow-on.

## Planned execution after prep approval

1. Preserve read-only external evidence and its provenance.
2. Construct the smallest equivalent installed-wheel comparison plan with
   fixed attempts and no success-masking retry.
3. If new diagnostic behavior is required, commit a meaningful RED test before
   test-only instrumentation; production code stays unchanged unless separately
   authorized.
4. Run the comparison on the native Windows x64 environment and retain raw plus
   redacted result artifacts. A single predeclared run matrix replaces ad-hoc
   repeated attempts.
5. Write `disposition.md` with one typed outcome and evidence table.
6. Route results:
   - reproduced/attributed product gap → draft reserved follow-on Slice 51 for
     HITL review, with design/TDD scope;
   - not reproduced → durable no-change finding and Slice 70 handoff;
   - insufficient → exact prerequisite/evidence request assigned to the next
     reviewer, not a weaker substitute.

## Verification and evidence

- Source/job/package/environment provenance is complete or explicitly missing.
- The comparison uses a public/released or candidate installed wheel in a fresh
  Windows environment, never `pip install -e` from a worktree.
- Real SQLite WAL behavior is exercised; no database mock.
- Attempt count, process boundaries, owner inventory, checkpoint outcome,
  elapsed time, and typed error/success are retained.
- Existing first-party Windows WAL contract tests remain green for any
  test-only diagnostic edit.
- Scoped Rust/Python/workflow checks apply only if their files change;
  otherwise document lint and `git diff --check` are sufficient.

No hosted full CI is automatically required. If an external-only Windows fact
cannot be reproduced locally, the plan must name that fact before requesting a
hosted job.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| Job title/note is mistaken for raw evidence | Preserve actual logs/artifacts or mark insufficient. |
| Retries erase the failure signal | Fixed attempts; retain every failed outcome; never relabel. |
| Client behavior is blamed on FathomDB without ownership evidence | Require installed first-party control and owner/transaction inventory. |
| Test-only diagnostics alter production | Keep seams test-only and verify production transparency. |
| Investigation expands into a broad Windows redesign | Stop at typed disposition; route attributed fix to Slice 51. |

## Decisions and prerequisites for the next reviewer

The reviewer approves the evidence-equivalence matrix, fixed attempt budget,
redaction/provenance record, and outcome predicate before execution. Any missing
Memex artifact, native Windows access, or installed package is recorded as a
prerequisite. A production fix is never implicit in Slice 50 approval.

## Definition of done

Slice 50 closes when the actual available Memex evidence and a truthful
first-party installed Python comparison yield one durable typed disposition,
fail-closed semantics remain intact, no un-attributed product change is made,
and any follow-on is explicitly scoped for the next reviewer.
