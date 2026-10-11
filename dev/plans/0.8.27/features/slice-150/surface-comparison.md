---
title: FathomDB 0.8.27 Slice 150 — public and hidden surface comparison
status: INDEPENDENTLY_VERIFIED
target_release: 0.8.27
---

# Public and hidden surface comparison

The exact-source capture commit is `c3239879e760bf8881f376a61e512a77703676e8`.
Both tools captured that committed tree with their pinned versions. The later
merge commit `04a024ed1abd1927ee22095b195f1cac6df70433` adds only the
Slice 140 board handoff Markdown; Git object IDs for `src/`, `scripts/`,
`dev/tools/`, `.github/`, `Cargo.toml`, `Cargo.lock` and
`src/ts/package.json` are identical at the two commits. The saved manifests
and diffs are in the host-local
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-150/`
directory. They are retained, not substituted for the fixed baselines.

| Evidence | SHA-256 |
| --- | --- |
| Slice 30 public baseline | `06212f662b2a3897447fe294d65d87ae93255b9b511d9fdd57eab3cf8490a723` |
| `public-c3239879e.json` | `31f329b280b3df7ed43d8df1f8643257b758422e7b9b1ef626b83d27b1c13f13` |
| `public-baseline-diff.json` | `525a44dcb79cfbab1f32316483ab50b8ce45b621d155f3434e6d6402a5ddafd3` |
| Reviewed hidden successor `baseline-f327c28c.json` | `e42c87f0684ee45e3cc5e51c30f82592999654ed62087cc213260595cd5b7109` |
| `hidden-c3239879e.json` | `2531c44d7dd2531f21c093cee8c1d8a8017f2fc23f8a5edc1846b96859f04428` |
| `hidden-baseline-diff.json` | `87c9d1b0d10bccd33c7d7695477830b2a78809e18448d1c11616b60a494d8d29` |

The public comparator captured 13 rows. Its fixed-baseline comparison exits
1 for previously reviewed inherited differences, with matching metadata and
row identities. Its ten changed-row `(added, removed, changed)` counts exactly
match Slice 140's
`dev/plans/0.8.27/features/slice-140/public-surface-comparison.md` table:

| Row | Added | Removed | Changed |
| --- | ---: | ---: | ---: |
| `napi-production` | 1 | 2 | 1 |
| `python-native-stub` | 3 | 3 | 3 |
| `python-wrapper-declarations` | 5 | 4 | 4 |
| `rust-engine-default` | 107 | 0 | 0 |
| `rust-engine-operator` | 203 | 0 | 0 |
| `rust-engine-operator-test-hooks` | 279 | 0 | 0 |
| `rust-engine-test-hooks` | 183 | 0 | 0 |
| `rust-facade-default` | 2 | 0 | 0 |
| `rust-facade-operator` | 5 | 0 | 0 |
| `typescript-declarations` | 3 | 4 | 2 |

The other three public rows compare equal. Git changes since the completed
release source affect only the Slice 150 hidden collector and its RED/GREEN
test, so there is no Slice 150 public contract delta. The independent
`gpt-6-sol` high verifier recomputed the saved comparator result and found no
unexplained row difference.

The hidden comparator captured 38 rows versus the reviewed successor's 37.
All **36 shared legacy rows** have identical entry lists and identities.
The prior `tests-req-fathomdb-sdk--none` row and new `tests-sdk-default` row
each list the exact same 38 test entries in the same order. The new
`sdk-default` rustdoc row lists 439 entries. Corresponding row-identity
metadata and capture SHA are the only metadata changes. This is precisely the
Slice 132-assigned SDK probe addition; no legacy baseline was rewritten. The
independent verifier recomputed this diff and found no unexplained change.

The SDK parity checker reports Rust 44/44 mapped operations and its 28 checker
tests pass. The RED test is commit `d80f43bcc`; the GREEN collector row is
`c3239879e`. The separate `gpt-6-sol` high code review approved that change.
