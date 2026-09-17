# Long-file investigation plan

## Objective

Determine whether very large source files, especially the approximately
27,700-line `fathomdb-engine/src/lib.rs` in the `release/0.8.25` worktree,
materially reduce the efficiency or effectiveness of LLMs, coding agents, and
their harnesses. Separate direct evidence from inference and turn the result
into practical guidance for FathomDB.

## Scope and safeguards

- Write only under `dev/long-file-investigation/` in the local `main`
  checkout.
- Treat `/home/coreyt/projects/fathomdb-worktrees/release-0.8.25` as
  read-only.
- Assume files in the release worktree can change during the investigation.
  Record the Git revision, worktree status, file size, line count, and
  modification time with each local measurement; do not claim a stable
  snapshot when the worktree is dirty or changes between reads.
- Do not recommend a refactor merely because a file exceeds an arbitrary LOC
  threshold. Evaluate navigation, semantic cohesion, coupling, change
  locality, testability, and measurable agent cost or quality.

## Questions

1. Does file length itself consume model context, or only content selected by
   the harness, agent, or user?
2. How do input length and position affect accuracy, latency, token cost,
   attention, retrieval, and compaction?
3. What evidence exists specifically for source-code and repository tasks,
   versus general long-context retrieval?
4. How do coding agents navigate large files using text search, ranged reads,
   symbol indexes, ASTs, LSPs, repository maps, chunking, and iterative tool
   calls?
5. When can a large cohesive file help by preserving locality and avoiding
   cross-file indirection?
6. Which properties of the FathomDB file create actual costs: raw tokens,
   mixed responsibilities, symbol count, dependency density, edit collision,
   generated content, or frequently co-needed distant regions?
7. What observations or controlled measurements would justify retaining,
   indexing, or splitting the file?
8. Which source-layout and instruction conventions work across Codex and
   Claude, and which navigation guarantees require coded harness controls?

## Method

1. Inventory source files over 1,000 LOC and characterize the largest file by
   bytes, approximate tokens, symbol/module structure, test concentration,
   responsibility clusters, and change history.
2. Inspect repository and harness conventions that determine how agents find
   and load code. Use bounded or targeted reads rather than loading the full
   changing worktree file into one prompt.
3. Collect primary, quantitative research on long-context degradation,
   input-length latency/cost, code retrieval, repository-level coding, and
   context management. Record methods and limitations, not just headline
   numbers.
4. Use the requested independent `gpt-5.6-terra` subagent at high reasoning
   effort for a separate web-research pass. The subagent is read-only and will
   return a source ledger; the main agent will verify material claims before
   using them.
5. Synthesize a causal model distinguishing repository size, file size,
   retrieved-context size, and architectural complexity.
6. Produce FathomDB-specific recommendations and a measurement protocol. Mark
   any proposed refactor as conditional unless evidence establishes a current
   problem.
7. Separate user prompts, repository instructions, source architecture, and
   harness enforcement. Define a portable code-organization target for Codex
   and Claude.

## Planned artifacts

- `plan.md` — this plan.
- `repository-observations.md` — reproducible measurements and structural
  observations from the release worktree.
- `source-ledger.md` — source-by-source evidence, quantitative results,
  limitations, and URLs.
- `report.md` — conclusions, confidence levels, FathomDB implications, and
  recommendations.
- `agent-compatible-code-organization.md` — portable source structure,
  instruction compatibility, enforceable harness controls, and a FathomDB
  target decomposition.
- `README.md` — short index and reading order if the investigation produces
  more than the plan and report.

## Verification

- Re-check every quantitative citation against the primary source.
- Clearly label extrapolations from synthetic long-context tests to coding
  agents and from general code benchmarks to this repository.
- Re-run release-worktree metadata checks at the end and disclose drift.
- Run the repository's Markdown-specific lint on the new artifacts, then run
  the proportionate documentation verification gate available for this
  directory.
