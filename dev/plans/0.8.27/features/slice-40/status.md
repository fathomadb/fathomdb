---
title: FathomDB 0.8.27 Slice 40 - implementation status
status: COMPLETE
implemented_on: 2026-09-22
baseline_entry_sha: 5f5c1798a3cffc1416467fd707587954ea75d9c6
implementation_candidate: fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae
closeout_commit: 8dcee1ba7eccdf03fcf89a8e623282274d0d70aa
---

# Slice 40 implementation status

## Completed scope

Slice 40 is complete on `release/0.8.27`.

- `errors.rs` owns the shared error taxonomy and carriers.
- `identity.rs` owns shared ID spaces, source/revision/dependency identities,
  canonical hash, validation, and stable/logical derivation.
- `temporal.rs` owns read views, frozen-view/clock support, temporal SQL, strict
  timestamp conversion, renderability, and normalization.
- `test_hooks.rs` owns the existing rendezvous, one-shot hooks, and projection
  pause support under their original cfg gates.

`Engine` remains at crate root. The four new modules are private, root public
paths are preserved, and only necessary crate-internal visibility changed.
Slice 20 erasure, later domain operations, schemas, bindings, package exports,
interfaces, and governed API behavior were not absorbed or changed.

## Requirements and evidence

- **AC27-40A:** all four modules are private, `Engine` stays rooted, and every
  batch plus the final candidate compares equal to the immutable 13-row Slice
  30 baseline with empty metadata and row diffs.
- **AC27-40B:** exhaustive error characterization, identity/source grammar,
  temporal boundary/shape/clock, facade, and focused owner suites pass.
- **AC27-40C:** default, `test-hooks`, `slice72-test-hooks`, and combined feature
  checks preserve the existing hook surfaces and ordering behavior; the direct
  projection-pause suites pass under `test-hooks`. The comparator's
  `cargo public-api` rows omit `#[doc(hidden)]` items, so every moved hook is
  invisible to them and the comparator is not evidence of hook-gate
  preservation. That evidence is the feature-matrix compile checks plus the
  adversarial review's hidden-items rustdoc JSON comparison (pre/post root name
  sets identical under default, `test-hooks`, `slice72-test-hooks`,
  `operator`, and `operator,test-hooks`). The comparator blind spot is tracked
  in the todos ledger; method and findings: `adversarial-review.md`.
- **AC27-40D:** the locked correction-safe erasure matrix is unchanged and
  passes 5/5; no later domain or delivery surface was moved.

The broad gate found and closed three repository checks whose physical-file
assumptions were invalidated by the legitimate refactor: one orphan doc
separator, AC-050c public re-export recognition, and C1 error/identity source
anchors. Each behavior-bearing gate correction has committed RED/GREEN
coverage and independent review. No contract was weakened or repinned.

## Final receipts and cleanup

At `fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`, independent verification passed:

- fresh surface capture: 13 rows, SHA-256
  `832bc78957c20bf8efd36ec97f6f91bbff098a821b3525bfae67bb2179ef6473`,
  equal with empty metadata/row diffs;
- canonical gate: 124 registered, run, and passed; zero failed, skipped, or
  excluded;
- security: 0 violations, 0 blockers, 0 downgrades;
- Python native candidate receipt: PASS;
- workspace Clippy with warnings denied and workspace check: PASS; and
- independent design review and final code review: PASS.

The temporary environment, native outputs, receipts, comparator cache, and
capture files were removed. This slice created no branch or worktree: the
user-provided durable `release/0.8.27` branch/worktree remains for Slice 50.
