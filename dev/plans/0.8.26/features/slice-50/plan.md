---
title: FathomDB 0.8.26 Slice 50 — integrated release verification
status: APPROVED
---

# Slice 50 plan — integrated release verification

## Outcome and authority

Complete `N26-05`: fresh, non-published `0.8.26` artifacts prove the combined
P0–P2 Memex profile, current cross-SDK V1 behavior, restart and fresh-database
boundaries, the five supported native CPU targets, and candidate-document
truth. Slice 50 establishes a release candidate and evidence packet; it does
not tag, publish, promote an npm dist-tag, create a GitHub release, or claim
post-publication registry evidence.

This plan applies the [lean slice execution contract](../../slice-execution-contract.md)
and the Slice 8 rulings at `seq-287` and `seq-288`.

## Reconciliation since the draft

The original plan was written at `183fe45a` and amended through `db47f22e`.
The following changes and assigned inputs now govern execution:

1. Slices 9, 10, 15, 20, 30, 35, 40, 45, and 46 completed. Their accepted
   functions are frozen explanation, exact graph evidence, immutable operator
   integrity, changed-in-place atomic derived-edge actuation, schema-34
   fresh-only admission, and current architecture/design truth.
2. Slice 20 added M20-03: installed-artifact graph evidence must cover explicit
   and query-derived seeds, outgoing/incoming/both directions, depth one and
   multihop, positional sidecars, and target plus winning terminal-edge
   resolution for ordinary and actuated edges.
3. The release-wide review after Slice 46 closed lifecycle disclosure, symlink
   lock identity, projection-generation authority, exact decoder, and
   release-profile gate defects through `6d952c9c`; those fixes are candidate
   inputs, not Slice 50 reimplementation work.
4. Axis W is still `0.8.25`. Building an exact `0.8.26` candidate therefore
   requires the normal mechanical `scripts/set-version.sh --workspace 0.8.26`
   step before artifact generation. Published-release prose remains `0.8.25`
   until publication.
5. P26-09 remains open: Python's source-derived `test-hooks` inventory and the
   Windows workflow/package checks keep separate handwritten module/symbol
   expectations. The existing retained N-API module manifest covers test
   modules and fixtures, not the private hook surface it is meant to exercise.
6. P26-13 remains open: registry propagation handling must retry only an exact
   requested version that is not yet visible. Authentication, malformed
   metadata, version/package mismatch, transport, install, import, and runtime
   failures must fail immediately.
7. P26-11 remains conditional. The exact candidate evidence is scanned first;
   an exact digest is registered only if a stable benign generated value is
   rejected, with a nearby credential-shaped mutation still rejected.
8. P26-04 is an execution condition: use checkout-owned tools, fresh install
   roots, a measured disk budget, and cleanup ownership. The host currently
   has about 67 GiB free while this worktree's generated `target/` uses about
   68 GiB, so a fresh-build directory may be created only after space is made
   safely.
9. The supported native matrix remains Linux x86-64/ARM64, macOS x86-64/ARM64,
   and Windows x86-64. The existing exact-SHA CI dispatch is its owner; local
   Linux evidence cannot be relabeled as another platform.
10. P26-10, P26-12, P26-14, broad dependency/tool upgrades, historical
    migration matrices, prebuilt CLI expansion, CUDA/Metal qualification, and
    installation of every cross-target on this host remain postponed or
    rejected.

## Plan dispositions

| Input | Disposition | Reason |
| --- | --- | --- |
| N26-05 and integrated Slice 10–40 profile | approve, make executable | This is the slice outcome. |
| M20-03 graph-evidence matrix | approve, retain exact breadth | It is the consumer evidence added after the first draft. |
| P26-09 Windows inventory | adjust | One machine-readable contract must drive source-derived hook-surface checks, installed-wheel symbol probes, and the retained package witness; runtime connection-count oracles remain unchanged. |
| P26-13 registry visibility | approve, narrow classifier | Poll only exact-version absence before smoke fan-out; every other failure is terminal. |
| P26-11 benign Gitleaks digest | keep conditional | Candidate output decides whether any change exists. |
| Axis-W `0.8.26` bump | add as required mechanical work | Exact-version package evidence otherwise cannot exist. |
| Five-target native matrix | approve on named executors | The existing exact-SHA workflow already owns the platform topology. |
| P26-10/P26-12/P26-14 and adjacent maintenance | reject from Slice 50 | No candidate evidence changes their prior ruling. |

## Requirements and acceptance criteria

| Requirement | Acceptance criterion |
| --- | --- |
| R26-50A: exact candidate identity | AC26-50A: Axis W is `0.8.26`; one immutable implementation candidate SHA, toolchains, artifact names, sizes, SHA-256 values, commands, platforms, outcomes, and explicit external/not-run states are recorded in a schema-validated manifest. |
| R26-50B: clean installed-artifact integration | AC26-50B: a fresh wheel and npm/native package install without checkout imports and execute their applicable Slice 10–40 SDK success/refusal/restart flows on real schema-34 databases; the source-built CLI executes only version identity plus immutable integrity inspection against the quiescent fixture. |
| R26-50C: M20-03 consumer evidence | AC26-50C: installed artifacts cover both seed forms, all three directions, depth-one and multihop traversal, ordinary and Slice-40-actuated edges; every selected result retains positional identity and resolves target and terminal-edge evidence under the same frozen authority without ranked-only fiction. |
| R26-50D: cross-SDK persistence and fresh-only admission | AC26-50D: public bindings exchange only persisted public state, reopen the same current database where permitted, and a representative schema-33 database is refused before byte or sidecar mutation; no historical migration matrix is added. |
| R26-50E: Windows inventory authority | AC26-50E: one versioned machine-readable contract owns the complete private Python `test-hooks` module/symbol surface; source-derived structural, typing/static, installed-wheel clean-import, and workflow checks consume it, and adding/removing a Rust hook or omitting package consumption makes the focused test fail. Existing WAL connection/state oracles stay intact. |
| R26-50F: exact registry visibility classification | AC26-50F: hermetic tests prove exact-version absence retries within recorded bounds and exact presence succeeds for PyPI `fathomdb` plus npm main and all five manifest-derived platform packages at Axis W, while auth, transport/5xx, malformed JSON, wrong package/version metadata, and exhausted bounds fail distinctly. The gate precedes smoke fan-out, omits npm only for the existing `0.8.20` recovery path, leaves the tiered crates.io waiter unchanged, and publishes nothing in this slice. |
| R26-50G: platform truth | AC26-50G: the exact-SHA native-artifact workflow passes on the five supported CPU targets and emits one validated artifact receipt per target; its Windows arm runs the retained N-API package modules. The distinct exact-candidate Windows WAL job proves the disposable `test-hooks` installed-wheel inventory and emits its own receipt. Any unavailable executor is external/unresolved, never inferred green. |
| R26-50H: release-surface truth and nondisclosure | AC26-50H: no parallel V2 or SDK operator authority appears; changelog, candidate-only docs, package metadata, governed surfaces, and release design agree; current published-release prose remains truthful until publication. |
| R26-50I: security and non-publication | AC26-50I: candidate-generated evidence passes current-tree Gitleaks without a new suppression, or only the exact required benign digest plus mutation proof is added; no tag, registry write, dist-tag promotion, GitHub release, or publication receipt is created. |
| R26-50J: repository gate and closure | AC26-50J: focused RED/GREEN suites, full `agent-verify`, package witnesses, design review, code review, independent verification, release-state views, and worktree cleanliness pass; platform and ptrace claims cite capable executors. |

## Design and TDD implementation plan

1. Finalize the companion design and obtain independent read-only design
   review before behavioral edits.
2. **RED — Windows authority:** add focused tests that reject the separate
   handwritten Python/workflow hook lists and require the source structural
   check, installed-wheel probe, and retained package witness to consume one
   versioned JSON contract.
3. **GREEN — Windows authority:** add the compact contract and derive the
   Python structural/clean-import probe, typing checks, and Windows workflow
   checks from it. Preserve every existing WAL managed-connection, native-
   state, BUSY, completion, mutation, and real-Windows oracle.
4. **RED — registry classifier:** extend hermetic registry fixtures for exact
   absence/presence plus auth, 5xx/transport, malformed, mismatch, and timeout
   cases; assert the post-publish fan-out is gated by the classifier.
5. **GREEN — registry classifier:** implement the bounded PyPI/npm visibility
   helper and wire a pre-smoke visibility job for the manifest-derived package
   set, with the existing `0.8.20` npm omission. Leave crates.io's tier waiter
   unchanged; do not change publish behavior or retry install/import/runtime
   failures.
6. **RED/GREEN — candidate manifest and M20-03:** add a schema/validator and an
   installed-artifact profile whose initial test fails for missing matrix
   cells, then implement the ordinary/actuated graph cases and candidate
   evidence emission without embedding artifact binaries in Git.
7. Apply the mechanical Axis-W bump with `scripts/set-version.sh`; verify all
   owned manifests/lockfiles and retain Axis E unchanged.
8. Run focused tests and inspect the actual diff in an independent code review;
   resolve findings without weakening oracles.
9. Establish the implementation candidate SHA, build once into fresh output
   roots, run local Linux wheel/npm/CLI witnesses and the manifest validator,
   then scan the generated output directory directly with the pinned Gitleaks
   configuration before running the tracked-tree scan. Each platform job
   emits and uploads a compact receipt containing candidate SHA, target,
   artifact names/sizes/hashes, and harness outcome; the durable manifest is
   assembled only from those receipts.
10. Push/dispatch only the existing `release/0.8.26` branch needed for the
    exact-SHA five-target workflow. Record the run and per-target outcomes; do
    not invoke the publish workflow.
11. Run the full canonical gate on a capable executor, obtain an independent
    verification review, write the status/evidence records, complete the
    release-state entry, regenerate declared views, and verify the clean
    release worktree.

## Stop gates

Stop release readiness on any source/package/version mismatch, missing matrix
cell, cross-SDK incompatibility, target failure, nondeterministic or unbound
manifest, parallel V2 surface, earlier-database mutation, stale candidate doc,
unresolved high-risk fault proof, Gitleaks finding, or attempted publication.
An external target or ptrace executor that did not run is `UNRESOLVED`, never
an inferred pass.
