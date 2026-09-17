# Code organization for Codex and Claude

## Position

Code should be organized so that an agent can identify an owner, retrieve a
complete semantic unit, follow its dependencies, and verify a change without
loading unrelated implementation. This benefits Codex, Claude Code, human
reviewers, IDEs, and conventional static tooling.

Neither Codex nor Claude requires a universal maximum file length. Both are
less dependent on raw file boundaries when their harnesses provide search,
symbol indexes, bounded reads, and tests. File organization still matters
because current tools often use paths and line ranges as retrieval and edit
boundaries.

## Responsibility model

| Layer | Owner | Role | Enforcement strength |
| --- | --- | --- | --- |
| source architecture | repository maintainers | create semantic boundaries and stable interfaces | structural |
| project instructions | repository maintainers | explain architecture, commands, and rules | advisory |
| task prompt | user | state the goal and unusual constraints | advisory and session-local |
| navigation tools | harness owner | bound reads, rank symbols, cap search output | enforceable |
| permissions and hooks | harness and repository owners | deny unsafe actions and run required checks | enforceable |
| evaluation | repository and harness owners | measure success, tokens, latency, and regressions | empirical |

The user should not have to repeat “search first” on every task. A user can
correct an agent, add task-specific instructions, or choose a harness mode,
but cannot make an unconstrained file-read tool bounded. Reliable behavior
requires the harness to expose and enforce bounded operations.

## Portable source-organization standard

### Organize around owned responsibilities

- Give each module one explainable responsibility and an explicit owner of
  its state and invariants.
- Make dependency direction visible. Prefer a small façade calling lower-level
  modules over peer modules reaching into each other's internals.
- Use typed interfaces between modules. Avoid shared bags of mutable state and
  implicit initialization order.
- Keep public contracts in a deliberate façade. Internal file moves should not
  accidentally change public type paths or visibility.
- Separate production implementation, generated code, test support, and
  benchmarks. Label generated files and their generator so an agent does not
  edit generated output as source.

### Make semantic retrieval cheap

- Use descriptive, reasonably unique symbol names. Generic names such as
  `process`, `handle`, or `run` make repository search ambiguous.
- Begin substantial modules with a short statement of responsibility,
  invariants, and important neighboring modules. Do not maintain a prose copy
  of the implementation.
- Keep definitions close to the constants, errors, and focused unit tests that
  explain their contract.
- Prefer functions and types that can be understood with their local
  neighborhood. A method that requires several distant regions to interpret is
  a coupling signal even when each region is in a different file.
- Provide one typed build, lint, test, and verification entry point. Agents
  recover much more reliably from executable checks than from narrative
  descriptions of expected behavior.

### Use token budgets as triggers, not LOC laws

The following are proposed repository operating thresholds, not claims of
universal model limits:

| Hand-written source size | Required treatment |
| --- | --- |
| below 8K tokens | ordinary symbol search and local documentation |
| 8K–32K tokens | module outline and symbol-aware retrieval |
| above 32K tokens | never inject whole by default; require an ownership and decomposition review |
| above 100K tokens | presumptive decomposition unless the file is generated, vendored, or evidence shows unusually strong cohesion |

The thresholds deliberately align operational action with the external
evidence that long-code performance often worsens above 32K. They are not
quality grades. A 5K-token tangled module can be worse than a 40K-token table
of cohesive declarations.

## Shared instructions for both agents

Codex officially discovers `AGENTS.md` from the repository root toward the
working directory, with closer files taking precedence. Its default combined
project-instruction limit is 32 KiB. The current root `AGENTS.md` is 21,337
bytes, so it fits that default when no other discovered project instructions
consume the remaining budget.

Current official Claude Code documentation says Claude reads `CLAUDE.md`, not
`AGENTS.md`, and recommends a small `CLAUDE.md` containing `@AGENTS.md` when a
repository wants one shared instruction source. It also says behavioral
instructions are not a hard enforcement layer.

This conflicts with the current FathomDB `AGENTS.md` statement that Claude
Code reads `AGENTS.md` natively and that no `CLAUDE.md` is maintained. Before
changing repository policy, verify the exact Claude Code distribution used by
the project. If it is standard Claude Code with the documented behavior,
replace that claim and add this minimal compatibility adapter:

```markdown
@AGENTS.md
```

Do not duplicate the full instructions in two files. Duplication creates
drift and consumes context. Keep shared invariants in `AGENTS.md`; use
agent-specific additions only when a real tool or workflow differs.

Both products support directory-local instruction layering. Use it sparingly
for truly scoped build commands or invariants. Loading every subsystem's rules
at repository startup spends context and increases conflicts. Claude's
path-scoped rules and nested memory files can defer guidance until matching
files are read; Codex can discover nested `AGENTS.md` or
`AGENTS.override.md` files on the path to the working directory.

## Harness requirements

These controls belong in code, not in a user reminder:

1. Build a symbol index from rust-analyzer, tree-sitter, or compiler metadata,
   with lexical search as a fallback.
2. Make “read symbol with context” the primary operation. Expand to its type,
   callers, callees, errors, and tests within an explicit token budget.
3. Default raw reads to bounded ranges. Refuse or require an explicit
   exceptional operation before returning more than 32K tokens from one file.
4. Cap search results by count and bytes, preserve total-match counts, and
   support pagination or query refinement.
5. Hash observations and deduplicate repeated source. Invalidate cached ranges
   when the file hash changes.
6. Record path, symbol, line range, revision, and hash with every retrieved
   excerpt so edits target current source.
7. Reserve context for instructions, diagnostics, tests, and output instead of
   allowing source retrieval to consume the whole window.
8. Compact conclusions and anchors, not bulk code. Reread exact current code
   before editing after compaction.
9. Expose telemetry for retrieved tokens by file and operation, cache hits,
   repeated reads, tool latency, compactions, and task success.
10. Evaluate retrieval policy and source layout together on representative
    repository tasks.

An `AGENTS.md` rule can tell Codex to use these tools. A `CLAUDE.md` rule can
tell Claude the same. Neither instruction can guarantee compliance if a raw
tool still permits an unlimited dump. The harness must implement the limit and
provide a deliberate escape hatch for the uncommon task that genuinely needs
the complete file.

## FathomDB target structure

The measured engine file exceeds the proposed presumptive-decomposition
threshold by almost three times. Its responsibilities and growth provide an
independent architectural basis for decomposition.

A behavior-preserving target should keep `fathomdb-engine/src/lib.rs` as a
small public façade and move implementation behind semantic modules. Candidate
ownership boundaries from the observed file are:

```text
src/
├── lib.rs                    public façade and deliberate re-exports
├── engine/
│   ├── mod.rs                Engine state, construction, core invariants
│   ├── reader.rs             reader pool and read transactions
│   ├── ingest.rs             validation and batch preparation
│   ├── commit.rs             SQLite commit and transactional writes
│   └── test_support.rs       cfg-gated hooks, not production behavior
├── search/
│   ├── mod.rs                public search coordination
│   ├── ranking.rs            scoring and reranking
│   └── graph.rs              graph expansion and dependency traversal
├── projection/
│   ├── runtime.rs            dispatcher and execution lifecycle
│   ├── registry.rs           configuration and registration
│   └── validation.rs         dependency and provenance checks
└── tests/                    integration and contract tests
```

This tree is a starting hypothesis, not an implementation specification.
Actual boundaries must follow the Rust dependency graph and accepted public
interfaces. Multiple inherent `impl Engine` blocks can live with the modules
that own their operations while the type remains in `engine/mod.rs`.

The decomposition should be scheduled after `0.8.25` and performed one
boundary at a time under failing tests. Preserve public paths deliberately,
avoid compatibility shims, and require the normal interface or ADR update if
the public contract changes.

## Acceptance criteria

The reorganization is successful when:

- no ordinary hand-written production file exceeds 32K tokens without a
  documented exception;
- the public API and behavior remain unchanged unless separately approved;
- a localized task usually retrieves one owning module plus explicit
  dependencies and tests;
- repository verification remains green after each extraction;
- controlled task replay reduces source tokens or latency without lowering
  task success; and
- human merge conflicts and review navigation do not regress.

The controlled replay validates and tunes the decomposition. It should not be
used to postpone the already justified first extraction indefinitely.

## Product documentation

- [Codex custom instructions with `AGENTS.md`](https://developers.openai.com/codex/guides/agents-md)
- [OpenAI model instruction-following guidance](https://developers.openai.com/api/docs/guides/latest-model)
- [Claude Code project memory and instruction loading](https://docs.anthropic.com/en/docs/claude-code/memory)
- [Claude prompting and long-horizon context guidance](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/prompt-templates-and-variables)
