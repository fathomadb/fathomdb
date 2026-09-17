# Source ledger

This ledger favors primary papers and official product documentation. It
separates measured results from implications for FathomDB. No cited study
establishes a universal file-length threshold, and none isolates a 1,000-line
file boundary.

## Code and repository tasks

### LongCodeU

- Source: [LongCodeU: Benchmarking Long-Context Language Models on Long Code
  Understanding](https://arxiv.org/html/2503.04359v1), Li et al., 2025
  preprint.
- Design: nine models, eight tasks, 116 real Python repositories, and 3,983
  examples across context buckets through 128K tokens. The repositories were
  created between June and November 2024.
- Result: the authors report marked degradation beyond 32K tokens. In the
  64–128K bucket, some dependency and semantic-relation tasks approach 10% or
  0%; cross-unit relationships were especially difficult. GPT-4o achieved 4%
  Success@1 on the reported RepoTransBench setting.
- Relevance: direct evidence that long code context and cross-unit reasoning
  can challenge models, even when the text fits the nominal window.
- Limits: Python-only, preprint, mainly 2024-era models, and not an agentic
  patch benchmark. It does not separate one large file from the same code
  divided into modules.

### RepoQA

- Source: [RepoQA: Evaluating Long Context Code Understanding](https://arxiv.org/abs/2406.06025),
  Liu et al., ICML 2024 workshop.
- Design: 500 natural-language-to-function retrieval tasks from 50
  repositories in Python, C++, Rust, Java, and TypeScript; 33 models; a fixed
  16K CodeLlama-token context.
- Result: the leading models exceeded 90% retrieval accuracy, while results
  varied substantially by model and language. The benchmark demonstrates that
  selective repository retrieval can work well within a bounded context.
- Limits: function retrieval and reproduction are narrower than editing or
  multi-step debugging, and the fixed 16K setup does not measure very long
  prompts.

### Long Code Arena

- Source: [Long Code Arena: a Set of Benchmarks for Long-Context Code
  Models](https://openreview.net/pdf/ddc0b751a313fdd40dc54e5ca26abb0c4ec8008e.pdf),
  Bogomolov et al.
- Design: repository-level code-completion experiments compare target-file
  context with a dependency-ranked 16K project context.
- Result: for CodeLlama-7B exact match, file-level to path-distance-ranked
  context changed from 0.35 to 0.37 on small repositories, 0.39 to 0.49 on
  medium, 0.35 to 0.47 on large, and 0.39 to 0.45 on huge repositories. The
  reported relative gains were 5%, 26%, 35%, and 17%.
- Relevance: a whole target file is not necessarily the best context. Relevant
  code elsewhere in the project can matter more than irrelevant nearby code.
- Limits: older model and code completion rather than autonomous repair. It
  does not compare a monolith with an equivalent modular source tree.

### RepoCoder

- Source: [RepoCoder: Repository-Level Code Completion Through Iterative
  Retrieval and Generation](https://aclanthology.org/2023.emnlp-main.151.pdf),
  Zhang et al., EMNLP 2023.
- Design: iterative similarity retrieval and generation for repository-level
  completion.
- Result: moving from in-file context to two retrieval iterations raised line
  exact match from 40.56 to 56.81 for GPT-3.5 and from 34.56 to 47.06 for
  CodeGen-6B. API exact match rose from 34.06 to 49.19 and from 26.19 to 38.88,
  respectively. The paper reports gains above 10 absolute exact-match points
  and eight edit-similarity points across settings.
- Relevance: retrieval policy and iteration can dominate the raw file
  boundary.
- Limits: completion benchmark, possible repository duplication, unstable
  gains in later iterations, and retrieval-generation latency was outside the
  evaluation.

### SWE-agent

- Source: [SWE-agent: Agent-Computer Interfaces Enable Automated Software
  Engineering](https://arxiv.org/html/2405.15793v3), Yang et al., NeurIPS
  2024.
- Design: a coding-agent interface with a line-range file viewer, summarized
  search output, and bounded observation history. The file viewer exposes at
  most 100 lines and searches return at most 50 results.
- Result: in a 300-task SWE-bench Lite ablation with GPT-4 Turbo, a 100-line
  viewer resolved 18.0% of tasks versus 14.3% with 30 lines and 12.7% with the
  full file. Full-history context resolved 15.0% versus 18.0% for the most
  recent five observations. Summarized search resolved 18.0%, iterative raw
  search 12.0%, and no search 15.7%. The paper reports a 64% relative gain for
  the full agent-computer interface over a shell-only baseline.
- Relevance: this is the strongest direct quantitative evidence here that
  bounded file views and controlled tool output can improve an actual coding
  agent. More file content was worse in that experiment.
- Limits: one harness, an older model, and ablations that alter interface
  behavior rather than source-file architecture. “Full file” refers to files
  opened by the benchmark agent, not specifically to a 27,700-line file.

## General long-context behavior

### NoLiMa

- Source: [NoLiMa: Long-Context Evaluation Beyond Literal
  Matching](https://proceedings.mlr.press/v267/modarressi25a.html),
  Modarressi et al., ICML 2025.
- Design: nonliteral needle retrieval for 13 models advertising at least 128K
  context.
- Result: at 32K, 11 models fell below 50% of their short-context baseline.
  GPT-4o fell from 99.3 below 1K to 69.7 in the reported comparison.
- Relevance: fitting text into a context window is not equivalent to using it
  reliably.
- Limits: non-code, synthetic or semi-synthetic retrieval; latent knowledge
  can affect results.

### Lost in the Middle

- Source: [Lost in the Middle: How Language Models Use Long
  Contexts](https://aclanthology.org/2024.tacl-1.9/), Liu et al., TACL 2024.
- Result: multi-document QA and key-value retrieval were commonly strongest
  when relevant information appeared near the beginning or end and weaker
  when it appeared in the middle.
- Relevance: the location of a needed symbol inside an injected large file may
  matter, not just total tokens.
- Limits: older models and non-code tasks; it supports a risk mechanism, not a
  FathomDB performance prediction.

### RULER

- Source: [RULER: What’s the Real Context Size of Your Long-Context Language
  Models?](https://arxiv.org/abs/2404.06654), Hsieh et al., COLM 2024.
- Design: 13 synthetic tasks, 500 examples per task at each of 4K, 8K, 16K,
  32K, 64K, and 128K.
- Result: the paper reports that only about half of evaluated models retained
  satisfactory performance at 32K; GPT-4 declined 15.4 points between 4K and
  128K in its aggregate comparison.
- Limits: synthetic tasks and rapidly changing model versions. The paper's
  abstract and body reflect different experiment revisions, so model-count
  claims should be read with care.

### Context Rot

- Source: [Context Rot: How Increasing Input Tokens Impacts LLM
  Performance](https://www.trychroma.com/research/context-rot), Chroma,
  July 2025 technical report.
- Design: 18 models, eight context lengths, 11 target positions, and 194,480
  calls; 69 calls, or 0.035%, were refusals. The tasks were held fixed while
  irrelevant content was added.
- Result: performance generally degraded as context length increased, with
  weaker semantic matches and distractors causing larger losses.
- Relevance: “context rot” is a useful operational label for quality loss from
  additional prompt content. It describes the prompt, not the file on disk.
- Limits: vendor report, not peer reviewed, mostly non-code tasks, and some
  scoring used a judge model.

## Systems costs and mitigations

### FlashAttention

- Source: [FlashAttention: Fast and Memory-Efficient Exact Attention with
  IO-Awareness](https://papers.neurips.cc/paper_files/paper/2022/hash/67d57c32e20fd0a7a302cb81d36e40d5-Abstract-Conference.html),
  Dao et al., NeurIPS 2022.
- Result: standard self-attention has quadratic time and memory complexity in
  sequence length. The IO-aware exact implementation reported three-times
  faster GPT-2 training at 1K and 2.4-times speedup on 1–4K Long Range Arena
  sequences.
- Relevance: more injected tokens have physical compute and memory costs even
  when optimized kernels reduce them.
- Limits: training-era kernel results do not imply that current hosted-model
  inference latency grows by an observable exact quadratic formula.

### PagedAttention and vLLM

- Source: [Efficient Memory Management for Large Language Model Serving with
  PagedAttention](https://arxiv.org/abs/2309.06180), Kwon et al., SOSP 2023.
- Result: the paper describes large, dynamically growing KV caches and reports
  two-to-four-times higher vLLM throughput at comparable latency than its
  evaluated serving baselines.
- Relevance: serving systems mitigate prompt-memory waste and share cache
  blocks, but they do not make additional input tokens free.
- Limits: system throughput rather than coding accuracy or source layout.

### Aider repository map

- Source: [Aider repository map documentation](https://aider.chat/docs/repomap.html).
- Method: build a concise map of classes, functions, and signatures; rank
  symbols using repository dependencies; include the most relevant map
  portions within an active token budget. The documented default map budget is
  1,000 tokens.
- Relevance: agents can navigate a large repository through a cheap semantic
  index and request exact files or ranges only when needed.
- Limits: official product documentation, not a controlled evaluation.

## Current OpenAI product facts

### Model window and pricing

- Source: [GPT-5.6 Sol model documentation](https://developers.openai.com/api/docs/models/gpt-5.6-sol).
- Current fact as checked on 2026-09-04: 1,050,000-token context window,
  128,000 maximum output, $4 per million uncached input tokens, $0.40 per
  million cached input tokens, and $20 per million output tokens. Prompts above
  272,000 input tokens are charged at two-times input and 1.5-times output
  rates; cache writes are 1.25-times the uncached rate.
- Relevance: a roughly 297K-token source file fits nominally but is not
  economically or operationally small. Product limits and prices can change.

### Prompt organization and compaction

- Source: [OpenAI model-selection and prompting guidance](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-5.5)
  and [GPT-4.1 long-context guidance](https://developers.openai.com/api/docs/guides/latest-model?model=gpt-4.1).
- Guidance: put stable content first to improve prompt-cache reuse; benchmark
  token use and end-to-end latency; use compaction to preserve decisions,
  actions, tool outcomes, blockers, and the next goal. The GPT-4.1 guidance
  explicitly warns that multi-item retrieval and graph search can degrade in
  long contexts.
- Relevance: long-lived agents need an explicit context-management policy;
  merely increasing the maximum window does not remove selection and
  compaction work.
- Limits: official operational guidance rather than an independent benchmark,
  and model-specific behavior evolves.

### Codex project instructions

- Source: [Codex custom instructions with `AGENTS.md`](https://developers.openai.com/codex/guides/agents-md).
- Current fact as checked on 2026-09-05: Codex builds a project-instruction
  chain from the repository root toward the working directory, with closer
  guidance later in the chain. The documented default combined project-file
  limit is 32 KiB. `AGENTS.override.md` can replace `AGENTS.md` at a given
  level.
- Relevance: a repository maintainer or user can provide durable behavioral
  guidance and scoped overrides. This affects agent choices but does not
  implement token limits in file tools.
- Limits: product behavior and defaults can change; a new run is required to
  rebuild the instruction chain.

### Claude Code project instructions and enforcement

- Source: [Claude Code project memory and instruction loading](https://docs.anthropic.com/en/docs/claude-code/memory).
- Current fact as checked on 2026-09-05: official Claude Code documentation
  says Claude reads `CLAUDE.md`, not `AGENTS.md`, and recommends importing
  `AGENTS.md` from a small `CLAUDE.md` when both agents share a repository. It
  explicitly distinguishes behavioral instructions from settings and hooks
  enforced by the client.
- Relevance: this supports the report's distinction between user or repository
  guidance and coded harness enforcement. It also identifies a version-sensitive
  conflict with the current FathomDB instruction-file claim.
- Limits: the installed FathomDB Claude harness may include behavior beyond the
  standard documented product and should be verified before repository policy
  changes.

### Claude long-context and agent guidance

- Source: [Claude prompting best practices](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/prompt-templates-and-variables).
- Current guidance: Anthropic treats 20K-plus input as long context, recommends
  deliberate document placement and grounding, and recommends explicit state,
  tests, and executable verification tools for work across context windows.
  Its page reports up to a 30% quality improvement in tests when the query is
  placed after long multi-document input.
- Relevance: source organization should expose retrievable semantic units and
  verification commands rather than rely on persistent bulk context.
- Limits: prompt-layout guidance does not establish a source-file size limit,
  and the reported improvement is for long-document prompting rather than
  repository refactoring.

## Independent research pass

The requested `gpt-5.6-terra` subagent at high reasoning effort independently
searched for quantitative evidence and returned the LongCodeU, RepoQA, Long
Code Arena, RepoCoder, SWE-agent, NoLiMa, Lost in the Middle, RULER,
FlashAttention, PagedAttention, and repository-map leads used above. The main
agent checked the material figures against the primary sources before
including them. The subagent did not modify repository files.
