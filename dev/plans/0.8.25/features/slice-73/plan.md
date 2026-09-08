---
title: 0.8.25 Slice 73 — Windows Node/N-API CI coverage
status: DRAFT
depends_on: 72
design: design.md
---

# Slice 73 draft plan

## Outcome and reconciliation

Add focused Windows Node/N-API CI coverage for the retained Slice 15–60
contracts. The post-Slice-60 handoff at `88d21040` is evaluated as historical
input: its focused Windows coverage belongs here, while its final exact-head
hosted-CI release gate and integrated matrix remain in Slice 75. No work is
duplicated.

## Scope

- Audit the existing Windows native-artifact job and structural workflow tests
  before adding coverage.
- Define an explicit, deterministic list of retained Node/N-API test modules;
  do not use a glob whose membership can silently drift.
- Build/package the Windows native module, install packed npm artifacts in an
  isolated consumer, prove resolved paths are outside the source tree, and run
  the selected success/error/unknown-version, search, projection, lifecycle,
  evidence/dependency, and constrained-graph modules that apply on Windows.
- Preserve workflow structural tests proving the job, artifact dependency,
  shell, package identity, and each explicit test module cannot silently
  disappear.
- Keep CUDA, live-model, publication, and the final full release PR CI out of
  scope. Slice 75 consumes this receipt and runs the final exact-head CI gate.

## Acceptance and verification

TDD first adds failing structural and behavior checks. GREEN makes the
smallest workflow/harness change. Run only workflow validation, the structural
test, relevant Node/N-API package tests, and the focused Windows job. Obtain
independent design/code review and verification, retain exact commit/run URLs
and artifact hashes, write `status.md`, and hand the receipt to Slice 75.
