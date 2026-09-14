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
cross-SDK current V1 behavior, restart behavior, the fresh-database boundary, platform
support, and documentation truth before any release decision.

## Requirements

- Build fresh Rust, Python, TypeScript/native, and CLI artifacts from the exact
  candidate commit.
- Install into clean environments and execute the Slice 10–40 contract flows.
- **M20-03 consumer evidence:** From those installed artifacts, produce a
  Memex-consumable graph-evidence matrix covering explicit and query-derived
  seeds; outgoing, incoming, and both-direction traversal; and depth-one and
  multihop results. For every selected result, preserve positional sidecar
  identity and resolve both the graph target and winning terminal-edge
  references to their intrinsic artifact/source evidence. Record graph-route
  selection provenance separately; do not synthesize ranked-search projection
  or ranking-contribution fields when they are not applicable.
- Create the database with 0.8.26 artifacts and reopen that same real database
  across supported bindings where the public contract permits it.
- Prove one representative earlier database is refused before mutation. Do not
  run or claim a historical migration matrix.
- Run the target-platform matrix approved in Slice 8 and distinguish local
  proof from externally owned evidence.
- Derive Windows structural and installed-binding inventories from one
  machine-readable contract, and prove the package witness consumes it.
- Add bounded polling/retry only for exact-version-unavailable registry
  responses in the non-publishing release-smoke logic.
- Register an exact benign Gitleaks digest only if candidate-generated evidence
  proves it necessary; do not add a generalized suppression.
- Produce a reproducible manifest binding commit, toolchain, artifacts,
  hashes, platform, commands, and outcomes.
- Perform no tag, registry publication, or release mutation.

## Execution

1. Run lint, typecheck, unit/property, integration, fault, and documentation
   gates in latency order.
2. Build packages once from a clean candidate state and record hashes.
3. Run clean installed-artifact witnesses for frozen explanation/evidence,
   graph artifact resolution, operator integrity, and atomic derived-edge
   replay/restart. Exercise the M20-03 matrix with an ordinary persisted edge
   and at least one edge committed through Slice 40 actuation, and bind each
   case, returned sidecar, resolved target/terminal-edge evidence, command, and
   outcome into the reproducible candidate manifest.
4. Run the no-parallel-V2-surface and earlier-database refusal witnesses, then
   cross-SDK current V1 and selected platform witnesses using the consolidated
   Windows inventory.
5. Audit package metadata, public docs, changelog/release notes, and absence of
   unapproved SDK authority.
6. Record all evidence and unresolved external gates for the HITL release
   decision.

## Acceptance

All in-scope requirements trace to passing artifact-level evidence; no
source-only test substitutes for a package claim; platform claims match actual
execution; documentation matches the candidate artifact; and publication has
not occurred. The M20-03 matrix is complete across seed, direction, and depth
variants; its target and winning terminal-edge references resolve under the
same frozen authority; and its recorded output distinguishes route-specific
selection provenance from intrinsic artifact/source evidence without inventing
ranked-only fields.

## Stop gates

Any package/source mismatch, cross-SDK incompatibility, target failure,
non-reproducible artifact, parallel V2 execution, earlier-database mutation,
stale documentation claim, or missing high-risk fault proof blocks release
readiness.
