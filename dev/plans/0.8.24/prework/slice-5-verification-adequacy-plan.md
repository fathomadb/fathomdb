---
title: 0.8.24 Slice 5 — verification-adequacy review plan
status: DRAFT
target_release: 0.8.24
---

# Slice 5 — verification-adequacy review plan

## Purpose

Determine whether the 0.8.24 drafts have complete, falsifiable verification
coverage and whether the release's critical user paths have adequate proof.
This slice maps requirements to acceptance signals and evidence owners; it does
not add or change tests.

## Required inputs

- The 0.8.24 plan and completed Slice 0–4 records.
- `dev/needs.md`, `dev/requirements.md`, locked `dev/acceptance.md`,
  `dev/test-plan.md`, and applicable accepted ADR/interface contracts.
- Existing Rust, Python, TypeScript, shell, PowerShell, workflow-contract,
  package-smoke, CUDA, WAL, performance, and publisher-idempotency tests.
- The executor, publication-topology, CI-control, and benchmark evidence
  records from Slice 0.

## Scope

Answer three questions separately:

1. Does every retained in-scope draft requirement have one or more falsifiable
   acceptance signals?
2. Does every acceptance signal have an appropriate named test, inspection,
   registry query, installed-package smoke, or retained environment witness?
3. Do the combined mechanisms adequately protect the release's product goals
   and critical paths, including negative/failure behavior?

The review covers R24-1 through R24-15 plus any Slice 3 additions or
adjustments. Existing tests receive credit only for the behavior they actually
assert; a nearby test name or structural workflow check is not completion
evidence.

## Non-goals and no-implementation boundary

- Do not edit tests, fixtures, workflows, scripts, source, contracts, package
  metadata, release state, or external configuration.
- Do not invent authoritative AC identifiers in the locked acceptance file.
- Do not run release builds, publish/dry-run jobs, hardware workflows,
  performance measurements, or the linked Memex workflow.
- Do not substitute source-tree compilation for installed-package evidence,
  a local static guard for target hardware, or Linux behavior for Windows.
- Do not require a new full hosted CI cycle for documentation or other narrow
  administrative work.

## Review method

### 5.1 Build the traceability matrix

For every retained draft requirement/criterion, record:

| Field | Required content |
| --- | --- |
| Need and requirement | Temporary label and owning slice |
| Acceptance signal | One observable pass/fail outcome |
| Existing evidence | Exact test/script/workflow/document path and what it proves |
| Gap | Behavior not currently proven |
| Proposed proof | RED-to-GREEN test, inspection, registry query, or real-environment witness |
| Environment | Local, hosted OS, self-hosted GPU, Jetson, remote Windows CUDA, or registry |
| Negative case | Failure/fallback/idempotent-retry behavior that must also be proven |
| Evidence owner | Slice 7, 10, 20, 30, 40, 50, 60, 70, or postponed |

An existing mechanism may be adequate, partially adequate, structurally useful
but non-proving, or absent. Record the distinction explicitly.

### 5.2 Review critical paths

At minimum, assess these paths end to end:

1. Generic CPU Python, npm, and applicable Rust/CLI artifacts remain
   installable and usable after CUDA publication changes.
2. A Jetson user explicitly installs the separate Tegra distribution and
   completes open/write/search/close/exit on the installed package with GPU
   engagement evidence.
3. A Windows user installs the supported CUDA SDK artifact built by the
   approved remote executor and completes the documented installed-package
   smoke without a local compile.
4. A partial multi-registry publish can be retried safely: existing valid
   artifacts are skipped, registry-query uncertainty fails closed, and missing
   artifacts can proceed.
5. The nominated engine change preserves correctness and satisfies the
   pre-declared retained-evidence decision rule without reopening a settled
   benchmark decision through an unrequested confirming run.
6. Windows Python-SDK WAL behavior is classified reproduce, not reproduced, or
   insufficient evidence before any product fix; fail-closed behavior is
   retained.
7. Main CI routes relevant changes to the existing proportional evidence and
   does not become a ceremonial release-wide gate.
8. Any accepted Slice 7 tooling or documentation change has a bounded local
   proof matched to its real blast radius.

### 5.3 Test-quality review

For proposed code behavior, require TDD with a meaningful RED that fails for
the intended reason. Database-path tests use a real database. Codec,
projection, recovery, and round-trip changes require property-based coverage
where applicable. Public-surface changes require functional SDK parity, not
symbol-presence tests.

For artifact and environment evidence, require provenance sufficient to tie
the result to the candidate SHA, package identity/version, target, toolchain,
and installed artifact. Artifact upload alone is not a smoke, and a runner
label alone is not hardware proof.

### 5.4 Route gaps

Allocate a verification gap to the same slice that owns the behavior whenever
possible. Cross-cutting installed-package and publisher proofs belong in Slice
60; final evidence completeness belongs in Slice 70. A gap that requires an
unapproved executor, registry identity, SDK surface, or contract decision stays
blocked on that decision rather than receiving a weaker substitute.

## Deliverable

`slice-5-verification-adequacy.md`, containing:

1. requirement → acceptance signal → evidence traceability matrix;
2. critical-path coverage matrix;
3. existing-evidence versus required-evidence findings;
4. test-quality and provenance requirements for each owning slice;
5. gaps, risk, and one primary allocation per gap; and
6. an explicit statement of what local/structural evidence cannot prove.

## Completion and verification

Slice 5 is complete when:

- every retained draft requirement has a falsifiable acceptance signal or an
  explicit missing-criterion finding;
- every acceptance signal has a named existing or proposed verification
  mechanism and environment owner;
- every critical path has success, failure, provenance, and installed-artifact
  coverage as applicable;
- every cited existing test/script/workflow path exists and its asserted claim
  has been checked in the file rather than inferred from its name;
- no test, workflow, product, contract, or release-state file changed; and
- scoped Markdown lint and `git diff --check` pass.

The review itself needs no full build, test suite, hosted workflow, hardware
run, or registry mutation.
