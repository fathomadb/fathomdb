---
title: Issue 264 partial-span dependency trace investigation
status: COMPLETE
date: 2026-11-19
---

# Issue 264 — Partial-span dependency tracing

[FathomDB issue #264](https://github.com/fathomadb/fathomdb/issues/264) identifies
a concrete correctness defect: a valid derived artifact citing part of a source
can be written successfully but refused by dependency tracing. Recommend a
bounded correction in .30. The owner commissioned it in Slice30 at seq-403,
using the existing Slice30 agent and the complete slice execution template.
Implementation is integrated at `5b0348775279883fe2ffcd52f60c23c90171166e`;
[Slice30 status](../plans/0.8.30/features/slice-30/status.md) records RED/GREEN,
independent reviews, exact native verification and retained gate outcomes.

## Evidence

The issue was fetched on 2026-11-19: open, with no comments. Source baseline is
`release/0.8.30` at `5f4edd9831ddbb5ee12ebe594a3f6cd17fcc46b8`. The inspected
trace implementation, writer and existing trace suite are unchanged from
`v0.8.29`. The running Slice 30 branch had no changes to the trace implementation
or trace suite when checked. This is source and contract analysis; no regression
test or runtime reproduction was executed for this investigation.

- The accepted provenance design states that SHA-256 covers the entire canonical
  source revision, not just its span:
  `dev/plans/0.8.25/features/slice-15/design.md:230`.
- `src/rust/crates/fathomdb-engine/src/write_commit.rs:298` compares the supplied
  digest with `canonical_body_hash(&source_body)` and stores that digest with the
  locator. `tests/slice15_identity_provenance.rs:147` exercises a valid partial
  locator and verifies the stored whole-source hash at line 182.
- `src/rust/crates/fathomdb-engine/src/dependency_trace.rs:553` instead selects
  the locator's bytes and hashes that selection. For a strict partial span this
  differs from the whole-source digest and `source_link_valid` returns false.
  Trace callers consequently refuse otherwise valid provenance.
- `tests/slice55_dependency_trace.rs:53` builds derived fixtures with
  `SourceLocator::whole_body()`. Its passing whole-body cases cannot distinguish
  the two hash interpretations. The write-path partial-span test does not also
  call tracing, leaving the composition uncovered.

## Minimal correction

In `source_link_valid`, separate locator validity from source integrity:

1. Preserve the existing schema, identity, source revision/version, hash shape
   and canonical-source checks.
2. For `whole_body`, retain the absent-endpoint requirement. For `utf8_bytes`,
   retain ordered nonnegative endpoints, checked integer conversion, bounds and
   UTF-8 character boundaries. Invalid locators must still refuse.
3. After a valid locator, hash `canonical_body.as_bytes()` for either locator
   kind and compare that whole-body digest with the persisted digest.

Do not change the writer to store a span hash: that would contradict the
accepted contract, persisted rows and the evidence/dependency implementations.
No schema migration, API/protocol change or new provenance policy is needed.
Keep existing typed refusals and frozen/lifecycle/filter checks intact.

The helper is reused for canonical source validation and derived artifacts in
both trace directions. Review all its callers, including dependent admission
and root/source validation, rather than fixing only one public return path.

## Required regression and focused verification

- **RED then GREEN:** write a real canonical source and a registered derived
  artifact with a valid strict UTF-8 subspan and the whole-source digest. Use a
  multibyte body, for example `aéz` with byte span `[1,3)`. Require successful,
  reciprocal `ToSource` and `ToDependents` traces with exact source/derived
  identities and relation fields under the same frozen context. Inspect the
  stored locator and digest separately; the trace response has no locator field.
  Preserve the
  actual pre-fix refusal in the RED commit.
- Retain whole-body and full-length `utf8_bytes` controls. Exercise both derived
  node and edge call paths where existing coverage does not already do so.
- Preserve refusal of mid-codepoint, reversed, out-of-bounds and malformed
  locators. Invalid public writes should refuse without mutation; persisted
  corruption cases should still produce the existing trace refusal.
- Change source bytes outside the cited span in a controlled corruption fixture
  while retaining the stored digest: tracing must refuse. This discriminates
  whole-source integrity from merely checking the cited bytes.
- Reuse existing source/version mismatch, lifecycle, frozen drift, relation/work bounds
  and corruption tests. Add a short property over valid UTF-8 subspans only if it
  improves coverage beyond those cases. Run focused trace, provenance,
  dependency and evidence suites; exercise the affected installed SDK route.
  Apply the repository's final source-gate policy when implementing.

The trace also requires a nonempty span, whereas the writer accepts an empty
span. Reconcile that adjacent distinction explicitly with the contract; do not
assert that the public writer rejects empty spans or silently change range policy
while fixing the hash.

This issue is independent of the accepted extractor first-source policy and
the graph-arm seeding/documentation proposal. It restores an existing contract;
it does not require multi-source provenance or a new retrieval mode.

Related proposals: [extractor identity and provenance](ingest-with-extractor-memory-graph-proposal.md)
and [graph seeding and containment](graph-seeding-containment-consumer-proposal.md).
