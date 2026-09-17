# Do large source files reduce coding-agent efficiency?

## Executive answer

Yes, when their contents enter model context or when their structure makes
relevant code difficult to retrieve. A large file sitting on disk consumes no
model context. The operational cost begins when a user, agent, or harness
returns the file, large excerpts, or repeated overlapping excerpts to the
model.

File length is therefore a risk multiplier, not an independent causal law.
The quantities that matter are retrieved tokens, relevance, ordering,
repetition, semantic coupling, and whether the harness can navigate by symbol
instead of treating a file as one prompt attachment.

There is no evidence for a universal 1,000-LOC limit. There is strong evidence
that indiscriminate long-context injection can reduce coding-agent success,
and direct evidence that bounded navigation can outperform full-file viewing.
In the SWE-agent ablation, a 100-line viewer resolved 18.0% of 300 SWE-bench
Lite tasks, compared with 12.7% for a full-file viewer. A 30-line viewer
resolved 14.3%, demonstrating that the goal is sufficient relevant context,
not the smallest possible excerpt.

FathomDB's engine file is an extreme case. The first reproducible snapshot in
this investigation was 27,700 lines and approximately 297,000 public-tokenizer
tokens. The clean active `release/0.8.26` snapshot checked on 2026-09-14 was
33,485 lines and 1.52 MB. Its size, rapid growth, test scaffolding, large
functions, and concentration of unrelated capabilities justify action without
waiting for a new experiment to decide whether action is warranted.

The recommendation is not merely to evaluate a possible refactor:

1. Enforce bounded, indexed navigation in the harness now.
2. Stop adding unrelated capabilities to the engine root file now.
3. Schedule a dedicated, behavior-preserving decomposition at the first
   available governed release slot.
4. Begin with a reproducible public-surface baseline and extraction of test
   support.
5. Move capabilities mechanically before separately decomposing oversized
   functions.

Measurement should tune the boundaries and report outcomes. It should not
reopen the already supported decision that the hotspot needs decomposition.

## Four quantities that must remain separate

1. **Repository size** affects indexing and search but does not have to enter
   the prompt.
2. **File size** affects editor, parser, search, and worst-case retrieval
   ergonomics.
3. **Retrieved-context size** directly affects input tokens, latency, cost,
   attention allocation, cache behavior, and compaction pressure.
4. **Architectural complexity** determines how many definitions and
   invariants must be understood together, regardless of file boundaries.

Splitting a file directly changes only the second quantity. It improves the
third when the harness uses semantic or file boundaries to retrieve less
irrelevant code. It improves the fourth only when the new modules establish
real ownership boundaries and reduce coupling.

## What long files do to context

Only returned content eats context. A local search tool can scan megabytes
without sending those bytes to the model. This investigation observed a warm
`rg` scan of the original 1.28 MB engine file completing in under one
millisecond, while the returned 100-line neighborhood was 791 tokens. The
timing is a local observation rather than a benchmark, but the execution
boundary is factual: tool-side bytes scanned are not model-side tokens.

When a harness returns a whole large file, it consumes context needed for
instructions, history, related files, diagnostics, tests, and output. The
original 297K-token snapshot was about 375 times the corresponding 100-line
view. A nominally large context window makes such a read possible; it does not
make it free or reliable.

Prompt caching can reduce the cost of repeated identical prefixes. It does not
remove the model's need to find relevant material among distractors. Likewise,
compaction can recover space but may discard exact code details. After
compaction, an agent should retain conclusions, revision and symbol anchors,
then reread the current source before editing.

## Context rot

The file does not create context rot merely by existing. Here, “context rot”
means declining task performance as more content, especially irrelevant or
weakly relevant content, is placed in the active prompt.

The evidence establishes a real risk but not a FathomDB-specific failure rate:

- LongCodeU reports marked degradation beyond 32K tokens on several long-code
  tasks, with cross-unit semantic relationships among the hardest.
- NoLiMa found that at 32K, 11 of 13 evaluated long-context models fell below
  half of their short-context baseline on nonliteral retrieval.
- Lost in the Middle found that relevant-information position affected
  retrieval and question-answering performance.
- Chroma's 194,480-call Context Rot study found broad degradation when fixed
  tasks were padded with distractors or weak semantic matches.

These studies span different models, tasks, and publication standards. They
show that advertised context length is not the same as effective context.
They do not prove that splitting identical code into more files automatically
improves results.

## When a large file helps

A cohesive large file can preserve locality, reduce import indirection, expose
an invariant in one continuous region, and make lexical search effective. A
poor split can make an agent chase definitions across many files or omit a
dependency that was formerly nearby.

Long Code Arena found that dependency-ranked project context outperformed
target-file-only context, with reported exact-match gains as high as 35% in
its repository-size groups. RepoCoder reported double-digit exact-match gains
from iterative repository retrieval. Relevant context often crosses file
boundaries.

The correct principle is semantic locality, not maximum or minimum file size.
A large file is beneficial when most of it is commonly needed together. It is
harmful when independent capabilities, tests, and long algorithms accumulate
behind one retrieval and ownership boundary.

## How agents should navigate code

An efficient harness should treat a repository as an indexed corpus:

1. Search paths, symbols, signatures, errors, and call sites.
2. Prefer compiler, language-server, AST, or symbol-index data when lexical
   results are ambiguous.
3. Return a token-budgeted repository or file map with signatures and anchors.
4. Read a bounded neighborhood around the selected definition.
5. Follow callers, callees, types, errors, and tests iteratively.
6. Report total search matches while capping and paginating returned results.
7. Hash observations and discard repeated overlapping source.
8. Preserve conclusions and source identity through compaction.
9. Reread the exact current symbol before editing.

Aider documents a 1,000-token default repository-map budget. SWE-agent uses a
100-line viewer and caps search output at 50 results. These are examples, not
universal thresholds. Their shared design is more important: repository-scale
computation happens outside the model, while the prompt receives a bounded,
ranked projection.

### What a user can and cannot override

A user can narrow a task to a symbol or subsystem, request search-first
navigation, forbid a full-file read, or explicitly authorize an exceptional
whole-file analysis. Repository instructions can express the same policy.

That is advisory control. A user cannot reliably implement truncation,
pagination, deduplication, context reservation, or token accounting through a
prompt. Those are harness responsibilities. The normal tool must enforce safe
defaults and expose a deliberate escape hatch; it should not require every
user to remember a special instruction.

## FathomDB evidence reconciliation

### Reproducible snapshots

The investigation now has four distinct snapshots. They must not be blended:

| Snapshot | Git/source status | Engine `lib.rs` |
| --- | --- | ---: |
| 2026-09-04 initial measurement | clean `release/0.8.25` at `79d296fa` | 27,700 |
| 2026-09-09 committed `c9d683ef` | Git blob | 32,930 |
| final `release/0.8.25` | Git blob at published branch tip | 33,292 |
| 2026-09-14 active `release/0.8.26` | clean at `db47f22e` when checked | 33,485 |

The separately supplied September write-up described `c9d683ef` as 33,031
lines. Git contains 32,930 lines at that commit. The 101-line difference
indicates that its count included uncommitted worktree content. Its exact
numbers are therefore historical and not reproducible from its stated commit,
although its structural conclusions remain useful.

The active 0.8.26 snapshot was 1,515,576 bytes with SHA-256
`c6647f6db43fa6eea3faf0722f854d1a15d38f4b205c4b4f3baa2ce1d9dd1e1c`.
The worktree was clean when checked; this does not imply that an active
worktree remains unchanged after the check.

Between final 0.8.25 and the checked 0.8.26 head, the endpoint grew by 193
lines. Commit-by-commit numstat recorded 1,199 additions and 1,006 deletions,
or 2,205 lines of gross churn across 29 commits touching the file. The old
instruction to refactor “immediately after 0.8.25” is stale because that
window has passed. The underlying urgency is stronger, not weaker.

### Durable structural findings

The current file still concentrates engine state, read and write paths,
search, graph traversal, projection runtime and registry behavior, validation,
SQLite operations, administration, and extensive test seams.

At the checked 0.8.26 snapshot, a conservative textual inventory found 140
function definitions ending in `_for_test` and 39 direct `#[cfg(test)]`
markers. These counts do not include every possible test-gated item, but they
confirm that test support is a substantial, separable responsibility.

The September write-up also identified multiple functions hundreds of lines
long, including a search transaction function above 1,000 lines in its
working snapshot. Moving such a function intact can improve discoverability
and ownership. It does not reduce the function's internal reasoning burden.
Function decomposition is a separate change with a different risk profile.

Existing sibling modules demonstrate that the crate can support capability
extraction. The write-up's count of eight modules totaling about 14K lines was
incomplete and low: `pagination.rs` was also present, and the eight named
modules totaled roughly 17.5K lines at the stated commit. The architectural
pattern is still valid; the dated inventory is not.

The older
[`refactor-background-check/engine-decomposition.md`](../plans/refactor-background-check/engine-decomposition.md)
material does not supersede the September analysis. It was committed on
2026-07-25 and describes a 17,910-line engine file. It remains useful design
input, especially its Rust visibility probe, but it is neither a current
snapshot nor an item on the checked 0.8.26 release board.

### Claims that require qualification

- **Build speed:** Cargo invokes rustc for crate targets, so source-file splits
  do not create independently compiled crates. Rust incremental compilation is
  query-graph based, however, so “zero incremental-build benefit” is too
  absolute. Do not justify the refactor as a build optimization; use Cargo
  timings if build impact matters.
- **Language-server behavior:** no evidence collected here proves that
  rust-analyzer handles a 33K-line file without latency or feature degradation.
  Treat that as a measurement question, not a fact.
- **Behavior risk:** module moves with stable root re-exports are low-risk, not
  zero-risk. Privacy, `super` imports, nested modules, feature gates, macros,
  path-relative assets, docs, and tests can break.
- **Merge conflicts:** a single-maintainer, worktree-isolated workflow reduces
  the weight of merge-conflict arguments. Navigability, ownership, review
  scope, and context control are the primary reasons to decompose.

## Code organization for Codex and Claude

The source tree should expose the same semantic structure to a human, Codex,
Claude, a language server, and a simple lexical-search harness:

- one capability has one predictable owning module;
- filenames use domain terms that appear in APIs, ADRs, tests, and errors;
- public façades and re-exports are explicit and small;
- implementation, focused tests, and test support are colocated by concern;
- large functions expose named phases or helpers rather than comment-only
  sections;
- module headers document only non-obvious invariants and cross-module edges;
- generated symbol maps supplement this structure instead of replacing it;
  and
- directory-local agent instructions are reserved for genuine local rules.

For the engine, keep `Engine` at the crate root during the first mechanical
extractions, or make every field-accessing capability module a descendant of
the module that defines it. Moving `Engine` into one child while leaving its
implementation modules as siblings would force private-field visibility
changes. Multiple inherent `impl Engine` blocks can live in descendant
capability modules while root API paths remain stable.

Detailed cross-product guidance is in
[`agent-compatible-code-organization.md`](agent-compatible-code-organization.md).
Instruction files can guide behavior, but neither Codex nor Claude repository
instructions substitute for harness-enforced retrieval limits.

## Recommended execution plan

### Track 1: harness controls now

1. Make symbol or targeted search the normal entry point.
2. Default raw reads to approximately 100–300 lines.
3. Require an explicit exceptional operation before one file returns more
   than 32K tokens. Treat this as a tunable operating threshold, not a model
   limit.
4. Cap search output by matches and bytes, report total matches, and paginate.
5. Record revision, file hash, symbol, and line range with each observation.
6. Deduplicate repeated excerpts and invalidate them when source changes.
7. Reserve context for instructions, related code, diagnostics, tests, and
   output.
8. Measure retrieved tokens, repeated reads, compactions, latency, and task
   success.

### Track 2: prevent further concentration now

New capabilities should not be appended to the engine root by default. A new
root addition should require a brief explanation of why it is inseparable
from root state or façade behavior. This is a repository rule; enforcing it
with a structural lint is preferable to relying on reviewer memory.

### Track 3: first dedicated refactor slice

Use small, independently verifiable changes. Do not combine source movement
with algorithmic redesign.

1. Build or adopt a reproducible public-symbol inventory before moving public
   items. The July background plan assumed such a gate, but no implementing
   tool was found in the repository.
2. Extract test support first, including `_for_test` methods and their owning
   gated helpers. Add a structural guard against placing new test hooks in the
   root implementation.
3. Extract cohesive contract and error types where root re-exports preserve
   intentional public paths.
4. Extract feature-gated operator or administrative behavior as an early
   production seam.
5. Move one engine capability at a time: reader pool, search coordination,
   projection runtime, projection registry, write and commit paths, erasure,
   open and probe behavior, and other validated ownership clusters.
6. Keep the `Engine` definition and shared invariants in the correct ancestor
   module; do not broaden field visibility merely to satisfy a desired tree.
7. Run the normal full-workspace verification gate after each meaningful
   extraction and refresh stale source-path documentation in the same change.

### Track 4: function decomposition

After a capability has a stable owner, review its oversized functions. Split
only around real data-flow or invariant boundaries. Add failing tests before
behavior changes, preserve transaction and frozen-view ownership, and avoid a
large context object whose sole purpose is to disguise existing coupling.

Search execution is a good example of why the tracks differ: moving a long
search function intact makes it easier to find and keeps related ranking and
graph code together; decomposing its phases changes control and data flow and
deserves a separate review.

## Measurement without a new decision gate

The earlier four-arm A/B/C/D study is too elaborate for deciding whether this
specific hotspot warrants action. That decision is already supported by
current size, responsibility concentration, and continued churn.

Use ordinary before-and-after telemetry on real work instead:

- source tokens returned by file and operation;
- number of search and read operations;
- repeated or stale reads;
- compaction events;
- time to the first correct target and to task completion;
- tests and acceptance results; and
- review defects or missed dependencies.

When a matched historical task is cheap to replay, compare the current
bounded-navigation baseline with the relevant extracted state. Do not require
a priced multi-run experiment before the first extraction, and do not treat a
null LLM-efficiency result as a reason to reverse modules that improve clear
human ownership and reviewability.

## Decision rule

Do not adopt an LOC ceiling. Use token size as an operational trigger and
semantic ownership as the architectural criterion.

- **Index and retain** a large file when it is cohesive and targeted retrieval
  reliably returns the complete relevant neighborhood.
- **Split** when it contains independently changing capabilities, test seams,
  or algorithms that are rarely needed together.
- **Decompose functions** when a single symbol contains multiple data-flow or
  invariant phases; moving that symbol to a new file is not enough.
- **Stop splitting** when new boundaries add navigation and dependency cost
  without improving ownership, retrieval, or review.

For FathomDB, the engine file meets both the operational trigger and the
architectural criteria. Enforce bounded navigation now, prevent further root
concentration, and schedule the first decomposition slice at the earliest
governed opportunity. The first slice should establish the public-surface
baseline and extract test support; it should not redesign search or another
large algorithm at the same time.

## Confidence and limits

- **High confidence:** only injected content consumes model context; the file
  is a context hazard if retrieved whole; bounded file views can outperform
  full-file views in an agent benchmark; no universal 1,000-LOC threshold is
  established.
- **High confidence:** the current engine file is an architectural hotspot
  that warrants planned decomposition. This rests on size, distinct
  responsibilities, test-support concentration, and continued churn.
- **Moderate confidence:** better semantic modules will reduce source tokens
  and target-selection effort under competent harness retrieval. FathomDB has
  not yet run a controlled task comparison.
- **Low confidence without measurement:** a file split alone will improve task
  success, rust-analyzer latency, or Rust build time.

Quantitative research and limitations are recorded in
[`source-ledger.md`](source-ledger.md). The original reproducible 0.8.25
measurements are in
[`repository-observations.md`](repository-observations.md). Current snapshot
values in this report were rechecked directly from Git and the clean
`release/0.8.26` worktree on 2026-09-14.
