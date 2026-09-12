---
title: Memex collaboration Slice 10 — frozen and evidence search documentation
status: DRAFT
depends_on: none
date: 2026-09-12
---

# Slice 10 plan

## Outcome and boundary

Publish a task-oriented Python guide for `freeze_read_context`,
`search_frozen`, `search_with_evidence`, and `resolve_evidence`. A reader must
be able to select the correct search API, implement the normal Memex retrieval
path, and handle drift or evidence refusal without consulting `dev/interfaces`.

This slice changes documentation and executable examples only. It does not
change search behavior, fix the 0.8.25 explanation defect, add a graph evidence
API, or prescribe Memex answer/citation policy.

The verbatim assessment that motivated the slice is preserved in
[`frozen-evidence-documentation-assessment-verbatim.md`](frozen-evidence-documentation-assessment-verbatim.md).
It describes the tagged 0.8.25 public reference. The collaboration baseline
contains a post-tag API inventory, so implementation must reconcile that newer
starting point rather than repeat the historical claim that the methods are
entirely absent.

## Documentation work

1. Add a comparison table to `docs/reference/python-api.md` covering ordinary,
   frozen, and evidence-bearing search: authority, evidence references, exact
   source resolution, explanation support, and intended use.
2. Add a runnable end-to-end example using this normal grounded-retrieval path:

   ```text
   freeze_read_context
   -> search_with_evidence
   -> resolve_evidence
   -> persist the consumer's use record
   ```

3. Explain that `search_with_evidence` already performs the frozen ranked
   search. Calling `search_frozen` first and joining results is unnecessary and
   creates avoidable race and association risks.
4. Document positional association, ranking equivalence for equivalent
   requests, exact UTF-8 span semantics, direct dependency scope, graph-origin
   limits, and which incomplete provenance conditions refuse evidence search.
5. Document the authority model: equivalent frozen context required; evidence
   references confer no authority; drift restarts the whole read attempt; a
   reference alone is not a durable audit record.
6. Add a bounded retry example that remints the frozen context and reruns search
   before resolving evidence. It must never retry resolution under a different
   context or fall back to non-frozen search silently.
7. Document when `search_frozen` remains useful: reproducibility diagnostics,
   retrieval-quality experiments, and non-evidence-consuming frozen reads.
8. Add a versioned known-issue note for the published 0.8.25 explanation paths,
   linking the local regression and upstream issue. Name the first fixed
   release only after a published artifact passes the same installed-artifact
   probe.
9. Add or update a public example test so method names, arguments, positional
   association, and error handling cannot drift from the Python SDK.

## Acceptance criteria

- **S10-AC1:** The public reference contains a side-by-side API table and a
  decision rule for all three search entry points.
- **S10-AC2:** A tested example demonstrates freeze, evidence search, positional
  resolution, and whole-attempt retry on `FrozenReadError`.
- **S10-AC3:** The guide distinguishes frozen authority from snapshot retention
  and evidence references from authorization.
- **S10-AC4:** The guide states that Memex's normal evidence-bearing path does
  not call `search_frozen` in addition to `search_with_evidence`.
- **S10-AC5:** The 0.8.25 explanation defect is documented without recommending
  non-frozen search as a substitute.
- **S10-AC6:** Public docs and example tests pass the docs, Markdown, and Python
  documentation gates.

## Verification

- Run the example against the locally built SDK and against the fixed published
  artifact before claiming a fixed-version boundary.
- Run `scripts/agent-lint-docs.sh`, the focused example test, and
  `scripts/agent-verify.sh`.
- Review every public claim against `dev/interfaces/python.md`,
  `dev/interfaces/rust.md`, and
  `dev/adr/ADR-0.8.25-compact-source-evidence.md`.

## Stop gates

Stop if documentation requires inventing retention guarantees, weakening
eligibility at resolution, treating a reference as authorization, promising
full graph-path evidence, or naming an unverified fixed release.
