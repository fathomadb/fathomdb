# Repository observations

## Snapshot discipline

The investigated release worktree was
`/home/coreyt/projects/fathomdb-worktrees/release-0.8.25`. It was treated as
read only because another session may be writing there. The initial
measurement was taken at Git commit
`79d296fa8da129bb310a3b7d1be25f12fe50c239`. `git status --short` was empty at
that instant.

The target was
`src/rust/crates/fathomdb-engine/src/lib.rs`. At the initial measurement it
had:

- SHA-256:
  `cbcdd51665f2cee48375e5af01a286fda47668f32ab0ff610e77883365e10a5e`
- modification time: `2026-09-04 07:50:26.861422929 -0500`
- 27,700 lines
- 129,838 whitespace-delimited words
- 1,282,691 bytes

These values identify the measured bytes; they are not a claim that the
worktree remained unchanged throughout the investigation. The closing
snapshot appears below.

## Context footprint

Using `tiktoken` 0.13.0 on the exact bytes above produced:

| Encoding | Tokens |
| --- | ---: |
| `cl100k_base` | 297,028 |
| `o200k_base` | 297,047 |

These are exact counts for those public encodings, not a guarantee of the
billable tokenizer used by every current model or coding product.

For comparison, a 100-line window centered on `validate_batch` was 791
`o200k_base` tokens and a 300-line window was 2,903 tokens. The whole file is
about 375.5 times the 100-line window. The bounded read is 0.266% of the
whole-file token count.

At current published GPT-5.6 Sol API limits, the estimate is 28.3% of its
1,050,000-token context window and exceeds the 272,000-token long-context
pricing boundary. If its billable input count equaled the `o200k_base`
estimate and the prompt contained only this file, uncached input would cost
about $2.38 at the published two-times long-context input rate, before system
instructions, history, tool output, or response tokens. This is an
illustration, not a Codex subscription-cost claim.

## Concentration

The file accounts for:

- 27,700 of 30,939 Rust lines in `fathomdb-engine/src`, or 89.53%
- 1,282,691 of 1,408,409 bytes in that directory, or 91.07%
- 22.43% of all 123,493 Rust lines in the release worktree
- 8.27% of the 334,853 lines across the measured Rust, Python, TypeScript,
  JavaScript, and shell source set

The worktree contained 252 Rust files, 17 of which exceeded 1,000 lines. The
broader measured source set contained 977 files, 46 of which exceeded 1,000
lines. Thus, 1,000 lines is not a rare enough boundary to diagnose a problem,
and the engine file is an extreme outlier even among those files.

The next-largest production-oriented files observed were approximately 4,362
lines for the Python binding `lib.rs`, 3,956 for the N-API binding `lib.rs`,
2,593 for a release-check shell script, 2,467 for the CLI `lib.rs`, and 2,057
for `fathomdb-engine/src/actuation.rs`.

## Structural map

The following segmentation is descriptive. It uses stable semantic anchors
found by search rather than asserting that these are already clean module
boundaries.

| Region | Lines | LOC | `o200k_base` tokens |
| --- | --- | ---: | ---: |
| headers and test hooks | 1–435 | 435 | 3,987 |
| runtime and reader types | 436–2,976 | 2,541 | 25,497 |
| public contract types | 2,977–6,925 | 3,949 | 43,223 |
| `Engine` implementation | 6,926–13,593 | 6,668 | 71,928 |
| ranking, search, and graph work | 13,594–17,009 | 3,416 | 39,725 |
| projection runtime | 17,010–19,007 | 1,998 | 20,993 |
| open, schema, and vector work | 19,008–21,136 | 2,129 | 24,251 |
| validation and dependency work | 21,137–22,732 | 1,596 | 13,585 |
| projection registry and application | 22,733–24,747 | 2,015 | 23,634 |
| commit and SQLite work | 24,748–25,939 | 1,192 | 12,945 |
| inline tests | 25,940–27,700 | 1,761 | 17,279 |

The segments sum to 297,047 tokens. Useful anchors include `impl Engine` near
line 6,932, `read_search_in_tx` near 15,033,
`projection_dispatcher_loop` near 17,010, `open_managed_connection` near
18,377, `validate_batch` near 21,145, `apply_projection_config` near 23,485,
`commit_batch` near 25,040, and the inline test module near 25,943.

Simple regular-expression inventories found 726 function declarations in the
file. The main `Engine` implementation spans roughly 6,650 lines and contains
195 methods, including 143 public functions. There were 102 function
definitions whose names ended in `_for_test`, 225 textual `_for_test`
occurrences, and 139 test-configuration markers. These counts are navigation
signals, not parser-derived language semantics.

The file header is valuable: it acts as a small human-readable map of storage,
projection, ingest, query, lifecycle, embedder, and retrieval responsibilities.
That benefit does not require an agent to read the remaining 27,000 lines.

## Growth and change locality

Snapshots from Git history show rapid growth on the current path:

| Date | Lines | Bytes |
| --- | ---: | ---: |
| 2026-06-01 | 5,740 | 241,389 |
| 2026-07-01 | 10,450 | 468,948 |
| 2026-08-01 | 19,824 | 963,501 |
| 2026-09-01 | 25,352 | 1,193,310 |
| initial release snapshot | 27,700 | 1,282,691 |

The current path was touched by 251 commits since 2026-06-01 and 90 since
2026-08-01. A path-local numstat aggregation over the former interval found
25,795 added and 3,838 deleted lines. Rename and path-history choices can
change those exact churn totals, so the defensible conclusion is that this is
a large, rapidly growing change hotspot.

## Navigation observations

A warm-cache `rg --stats 'fn validate_batch'` run scanned all 1,282,691 bytes,
found one match, reported 0.000139 seconds of search time, and took 0.000753
seconds of wall time in one local observation. This is not a benchmark. It
does establish an important distinction: a local tool can scan the complete
file without placing all scanned bytes into the model context. Only the small
returned match or subsequently requested line range needs to become prompt
content.

The practical navigation sequence used in this investigation was:

1. inventory paths and sizes;
2. use lexical search to locate symbols and semantic anchors;
3. request bounded line ranges around those anchors;
4. build a compact structural map;
5. inspect additional ranges only when a question required them.

Symbol indexes, AST queries, LSP references, dependency graphs, and
token-budgeted repository maps can make the same pattern more semantic. They
change retrieved-context size without requiring a source refactor.

## Closing snapshot

The closing check found that the release worktree had advanced to
`8db99b127e0935b0b027c6bc90df4a46b92cb1f8`. Four other files were modified:

- `src/rust/crates/fathomdb-engine/tests/slice25_fix1_red.rs`
- `src/rust/crates/fathomdb-napi/src/lib.rs`
- `src/rust/crates/fathomdb-py/src/lib.rs`
- `src/ts/src/validation.ts`

The investigated engine `lib.rs` was not listed as modified. Its SHA-256,
27,700-line count, 1,282,691-byte size, and modification time were identical
to the opening snapshot. The worktree moved, but the exact target bytes used
for the structural and token measurements remained stable across the two
checks.
