---
title: FathomDB 0.8.26 Slice 6 — build and local verification evidence review
status: DRAFT
---

# Slice 6 plan — build and local verification evidence review

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is evidence-only: reconcile newly available transcripts and repository work,
independently review the failure model, verify root-cause claims with focused
reproduction where safe, write status, and implement no fix.

## Purpose

Review available evidence, including relevant transcripts, for failures that
occur before or during successful local product verification. Identify the
smallest pragmatic corrections to environment setup, preflight behavior,
developer workflow, or repository code without implementing them.

## Evidence scope

- current and historical agent transcripts available to the project;
- preflight output and failure records;
- `agent-build`, `agent-lint`, `agent-typecheck`, `agent-test`, and
  `agent-verify` output and spilled diagnostic logs;
- release boards, run records, issue/PR discussion, and durable failure notes;
- platform-specific build evidence for Linux, macOS, Windows, x86_64, and
  AArch64 where available; and
- the current scripts, toolchain contracts, and documented invocation rules.

Do not infer absence of failure from missing or expired logs. Record evidence
availability and provenance explicitly, and do not copy secrets or unrelated
transcript content into the findings.

## Review method

1. Build a chronology from environment setup through preflight and the typed
   local verification verbs.
2. Distinguish product defect, build-system defect, script defect,
   environment/platform mismatch, incorrect invocation, stale documentation,
   sandbox limitation, and transient external failure.
3. Confirm whether `agent-verify` was invoked correctly, whether its component
   failure was preserved verbatim, and whether required unconfined reruns used
   the unchanged strict command rather than weakening the gate.
4. Group repeated symptoms by root cause and identify time lost, workaround,
   recurrence, and present status.
5. Propose the least invasive durable correction and an executable regression
   witness for each actionable root cause.
6. Recommend delivery in Slice 9, a reserved post-10 hardening slice, Slice 50,
   or postponement; Slice 8 makes the decision.

## Pragmatism rules

- Fix deterministic recurring causes before optimizing rare inconvenience.
- Prefer clearer fail-fast diagnostics to broad environment automation when
  both prevent the same delay.
- Do not relax gates, skip tests, hide compiler diagnostics, or add retries that
  mask a reproducible failure.
- Treat incorrect agent usage as a documentation/usability problem when a
  reasonable user can repeat it.
- Preserve platform-specific truth; do not claim host-only evidence covers the
  release matrix.

## Deliverables and exit criteria

Produce a sourced failure register with chronology, classification, root cause
confidence, recurrence, impact, candidate correction, regression proof, risk,
effort, and proposed slice. All available relevant evidence has been reviewed
or its absence recorded. No environment, script, product, or documentation
change is made.
