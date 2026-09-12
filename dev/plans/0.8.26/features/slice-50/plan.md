---
title: FathomDB 0.8.26 Slice 50 — integrated release verification
status: DRAFT
---

# Slice 50 plan — integrated release verification

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
enumerate candidate and allocation deltas, finalize release acceptance criteria
and evidence design, obtain design review, add RED/GREEN harness changes when
needed, obtain code review and independent verification, write status, and
clean up temporary workspaces without publishing.

## Outcome

Fresh non-published release artifacts prove the combined P0–P2 Memex profile,
cross-SDK compatibility, restart behavior, platform support, and documentation
truth before any release decision.

## Requirements

- Build fresh Rust, Python, TypeScript/native, and CLI artifacts from the exact
  candidate commit.
- Install into clean environments and execute the Slice 10–40 contract flows.
- Reopen the same real database across supported bindings where the public
  contract permits it.
- Run the target-platform matrix approved in Slice 8 and distinguish local
  proof from externally owned evidence.
- Produce a reproducible manifest binding commit, toolchain, artifacts,
  hashes, platform, commands, and outcomes.
- Perform no tag, registry publication, or release mutation.

## Execution

1. Run lint, typecheck, unit/property, integration, fault, and documentation
   gates in latency order.
2. Build packages once from a clean candidate state and record hashes.
3. Run clean installed-artifact witnesses for frozen explanation/evidence,
   graph artifact resolution, operator integrity, and atomic derived-edge
   replay/restart.
4. Run cross-SDK and selected platform witnesses.
5. Audit package metadata, public docs, changelog/release notes, and absence of
   unapproved SDK authority.
6. Record all evidence and unresolved external gates for the HITL release
   decision.

## Acceptance

All in-scope requirements trace to passing artifact-level evidence; no
source-only test substitutes for a package claim; platform claims match actual
execution; documentation matches the candidate artifact; and publication has
not occurred.

## Stop gates

Any package/source mismatch, cross-SDK incompatibility, target failure,
non-reproducible artifact, stale documentation claim, or missing high-risk
fault proof blocks release readiness.
