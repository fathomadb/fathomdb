---
title: FathomDB 0.8.27 Slice 70 - code review
status: PASS
target_release: 0.8.27
reviewed_range: a95b5b0f..b0289e04
---

# Slice 70 code review

The reviewer was an independent, read-only, adversarial subagent (Opus 5.5,
high effort). Its builds used an isolated target directory. Its verdict was
**PASS-WITH-FIXES**, with no P1 and no semantic change to production code.

## Verified

- **Verbatim move.** Line-multiset and per-item comparisons over all engine
  sources at `4df75e07` and `b0289e04` found that every moved item and
  `Engine` method is a contiguous substring of the baseline after normalizing
  visibility, whitespace, and link text. No item was dropped. There is no
  change to SQL text, constant values, statement order, lock or
  `commit_gate` order, or error mapping.
- **Attributes.** Each item's `cfg`, `cfg_attr`, and `doc(hidden)` set is
  identical. The `#[cfg(feature = "operator")]` on `mod projection_rebuild`
  hides nothing, because all four of its items were already operator-gated.
  `ce_rerank`'s nested `tc5_benchmark` and `slice72_test_hooks` paths resolve
  to the same modules.
- **Public surface.** The three reranker functions are the only public items
  in the new modules. They were already `#[doc(hidden)]` where they were
  before. No public item is missing from the root.
- **Inventory.** The moved and stay-at-root sets match `design.md`.
  `PROJECTION_CANDIDATE_PAGE` moved with its only users.
- **Singleton.** The only `static` is `reranker_singleton`'s function-local
  `OnceLock`, so there is still one instance.
- **Gates.** Manifest needle counts are unchanged. The slice35 audit passes
  6/6. The C1 gate passes 26/26, with its negative arms still able to fire.
  The `plan-0.8.20` citations are correct.

## Findings and resolutions

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P2 | The chronology claimed that the manifest bodies were byte-identical. Under the test's own `function_body`, six bodies had widened (next item `pub(crate) fn`, or a `concat!` file boundary). No needle fell in a widened tail. | The claim was corrected. The extractor now also ends at `pub(crate) fn`, and a sentinel `fn` follows each file. 13 of the 15 bodies equal the baseline. The other two are tighter or differ only in inter-item text. |
| 2 | P2 | The post-batch-10 public and hidden captures were missing. | Run and recorded. Public: equal. Hidden: additive residue tests only. |
| 3 | P3 | `0d2b336e` added a `redundant explicit link target` rustdoc warning. | The link is now a code span. |
| 4 | P3 | The residue test could hang on a panic before release, or if no failure was reported. | `residue()` returns a `Result`, release happens before unwrap, and the report wait has a 60 s timeout. The mutant was re-run, and both arms fail cleanly. |
| 5 | P3 | The rerank re-export carries a redundant `#[doc(hidden)]`. | Kept. The hidden capture shows no change. |

The fixes touch one doc comment and two test files. The focused suites,
crate Clippy with warnings denied, and `cargo fmt` pass. The scope did not
warrant another review cycle.
