---
title: 0.8.25 Slice 55 independent implementation review — cycle 10
status: FAIL
---

# Slice 55 independent implementation review — cycle 10

## Review target

- Candidate HEAD: `65bba8f233aed6229f050b563584ec4b6ad1caef`
- Product commit: `80ac13636c92aadb7b6bbf09914d2bbf42894f6e`
- Branch: `release/0.8.25`
- Worktree: clean; 145 commits ahead of the remote branch
- `git diff --check`: PASS

The independent reviewer made no repository or Git changes.

## Verdict

**FAIL.** One P2 remains. The owner has authorized one final FIX/re-review
cycle after this verdict.

## P2 — oversized Python u64 strings leak `ValueError`

The strict Python dependency-trace decoder calls `int(value)` in
`_trace_u64` without first bounding the decimal string or translating a
conversion failure. On Python 3.12.3, canonical decimal strings of 4,301 and
5,000 digits therefore raised raw `ValueError` rather than the stable typed
`DependencyTraceError` with reason `trace_corrupt` and the exact field path.

This violates the design's above-`u64::MAX` rejection and typed-error
contracts. The shared helper affects every trace-response u64 field. The final
correction must preserve a genuine installed-decoder RED, then reject an
overlong decimal string before conversion or translate conversion failure.

## Resolved from cycle 9

- Rust encoding and decoding reject every dependency edge outside
  `1..=dependencyGeneration` at the indexed generation path.
- Python and TypeScript impose the same numeric relationship while preserving
  canonical decimal-string u64 representation.
- The canonical fixture and property generator no longer bless an edge at
  generation 1 against boundary generation 0.
- The two narrow Python test corrections restore the established string
  representation without weakening the zero/ahead refusal assertions.
- The plan's TypeScript verification uses the working package-local build and
  test route.
- No regression was found in the previously accepted integrity, trace,
  explanation, facade, CLI, ordering, privacy, or frozen Slice 10–50 behavior.

## Reviewer verification

- Rust Slice 55 and legacy matrix: 109 passed, 1 ignored.
- Facade and CLI: 5 passed.
- Exact installed-wheel generation checks: valid 1/1 accepted; zero and ahead
  rejected at the exact edge path.
- Exact installed-wheel oversized-u64 reproduction: 4,301- and 5,000-digit
  strings leaked `ValueError`; an ordinary 20-digit value above `u64::MAX`
  correctly produced the typed error.
- Focused compiled TypeScript edge-generation test: 1 passed.

Broad independent verification remains pending.
