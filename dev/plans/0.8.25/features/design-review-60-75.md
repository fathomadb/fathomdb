---
title: 0.8.25 Slices 60/65/70/75 independent design review
date: 2026-09-01
cycle: 0
reviewer: independent Codex design review
verdict: FIX-1
scope: read-only review; no design or implementation edits
---

# Independent design review: Slices 60, 65, 70, and 75

## Verdict

**FIX-1.** The slice ladder and semantic/data-plane boundary are sound, but the
designs have unresolved implementation-shaping P1/P2 issues. READY is blocked
until graph paging under bounds, terminal-node filtering, profile default and
usability semantics, deterministic/bounded PPR, artifact-install proof, and the
integrated workload contract are made normative.

## Findings

### P1 — DR-60-01: bounded traversal cannot safely keyset-page a partially discovered order

Slice 60 requires global sorting before paging and a cursor after the last tuple
([design.md:95](slice-60/design.md)), but also stops after 10,000 examined path states and may return a successful page with continuation
([design.md:104](slice-60/design.md)). Sorting the discovered prefix does not prove the global next tuple: an unexamined same-hop path can sort before the emitted cursor, causing omission or reordered pages.

**Required correction:** make traversal enumeration identical to the canonical
page order, or place an authenticated frontier/visited/candidate-heap and ordering
state in the continuation so the next global tuple is reproducible. Do not emit a
successful page at the ceiling unless every path capable of sorting into that page
has been resolved; otherwise return a typed bound failure with no continuation.
Add an adversarial fixture with more than 10,000 same-hop paths and shuffled
insertion order proving no duplicate, omission, or reordering across all pages.

### P2 — DR-60-02: target predicates are conflated with intermediate traversal eligibility

The contract calls `target_kinds` a target constraint ([design.md:59](slice-60/design.md)), while execution compiles target kind into the recursive traversal statement
([design.md:90](slice-60/design.md)). If this prunes the frontier, a valid path through a different-kind intermediate node can never reach a matching terminal target.

**Required correction:** normatively separate seed eligibility, intermediate
node/edge traversal eligibility, and terminal/output target predicates. Eligible,
live intermediate nodes remain traversable even when they do not match
`target_kinds`; terminal predicates execute before output truncation. If
intermediate filtering is desired, give it a separately named constraint. Add
`A(kind X) -> B(kind Y) -> C(kind X)` coverage with `target_kinds={X}`.

### P1 — DR-65-01: registry `default` state contradicts the immutable omitted-profile contract

The registry admits a `default` entry ([design.md:58](slice-65/design.md)), but `None` is normatively A0 ([design.md:65](slice-65/design.md)). The promotion section then permits a separate HITL decision to change a default
([design.md:86](slice-65/design.md)) without defining what that state changes or storing the required decision reference in the entry. This is either inert state or an implicit behavior change.

**Required correction:** for 0.8.25, either forbid `default` entries other than an
immutable built-in A0 and keep `None == A0`, or explicitly define a versioned
default-alias resolution contract. Any promotion-capable form must carry a
`default_decision_ref`, accepted receipt identity/digest, compatibility effect,
and builder validation. Add tests proving omitted-profile behavior and rejecting
missing, mismatched, or unauthorized default-decision references.

### P2 — DR-65-02: “alters answer context” is not mechanically decidable

Qualification requires answer no-regression only for profiles that “alter answer
context” ([design.md:93](slice-65/design.md)), although every selector capable of changing ordered hit identity, body, or evidence can alter the downstream context. Earlier REASON/GRAPH receipts demonstrate that retrieval gains can reduce correctness, groundedness, or attribution.

**Required correction:** define context-changing mechanically as any profile that
can change the ordered hit/evidence set relative to A0. Such a profile must pass
the registered correctness, groundedness, and attribution no-regression gate
before it is qualified for answer-bearing memory use. If retrieval-only
qualification is retained, give it a distinct registry state/capability that
cannot be represented as answer-qualified. Restrict retrieval-only exemption to
identity/order-preserving telemetry or packaging changes.

### P1 — DR-70-01: PPR is not deterministic enough for the promised replay digest

The design names floating-point constants and a tie-break ([design.md:102](slice-70/design.md)), then requires repeatable ordered identity digests across insertion order and execution environments
([design.md:175](slice-70/design.md)). It does not define the recurrence, numeric representation, edge orientation for `both`, node/edge accumulation order, convergence evaluation order, or near-tie policy. Equal mathematical graphs can therefore produce different mass and rank order.

**Required correction:** specify the recurrence equation and whether `0.15` is
restart/teleport mass; require a numeric type; sort nodes and immutable edge
revisions canonically; define serial or stable summation order, direction-to-
transition mapping, dangling redistribution, full-iteration L1 convergence, and
near-tie comparison. Add hand-computable, shuffled-insertion, reopen, and near-tie
fixtures whose graph and ordered-result digests are stable on required platforms.

### P1 — DR-70-02: graph-cap exhaustion does not define which subgraph is scored

The associative profile caps the graph at 500 nodes and 2,000 edges
([design.md:97](slice-70/design.md)) and permits `bound_reached` as a qualified degraded outcome
([design.md:113](slice-70/design.md)), but does not define deterministic subgraph selection or whether diffusion runs on a partial graph. That can make rankings depend on SQLite/insertion order and can silently transform the algorithm.

**Required correction:** define the canonical bounded-subgraph selection order and
its digest, separately for depth, node, edge, and path-state ceilings. Either fail
closed before diffusion whenever a ceiling truncates the graph, or qualify a
specifically versioned degraded treatment whose deterministic truncated-graph
semantics and no-regression gate are preregistered. Never silently score an
unspecified prefix. Add over-cap permutation/reopen tests.

### P2 — DR-70-03: “supporting paths” overstates PPR score provenance

The response may return “exact supporting paths” beside PPR-ranked hits
([design.md:109](slice-70/design.md)), but PageRank mass is aggregated over the bounded graph; an exact witness path is not a complete causal explanation of a score.

**Required correction:** label these as exact, source-resolvable **witness paths**,
not complete score provenance. If score explanation is claimed, return a bounded,
deterministic contribution account (for example, top contributing predecessors)
plus omitted-mass/bound indicators. Preserve Slice 55’s material-degradation
signals and never imply one path explains the PPR score.

### P1 — DR-75-01: installed-artifact closure conflates candidate and post-publish proof

Slice 75 says to build artifacts once, test installed Rust/Python/npm/native/CLI
artifacts ([design.md:72](slice-75/design.md)), and run registry-installed smokes
([design.md:190](slice-75/design.md)). Publication requires separate authorization, Windows and Linux cannot share one binary build, and a Rust crate is not “installed” like a runtime package. A post-publish prerequisite would make release authorization circular.

**Required correction:** split the gates. Pre-publication closure uses locally
packed, registry-equivalent artifacts built once **per target/profile from the same
candidate commit**, hashed and installed/consumed without source-tree leakage.
Define Rust as a clean consumer fixture using the packaged `.crate` contents or a
local registry, and define wheel/npm/native/CLI isolation explicitly. Actual
registry-installed smoke is a post-publish gate run only after explicit publish
authorization; it cannot be a prerequisite to authorize publication. Record the
per-target build manifest and hashes.

### P2 — DR-75-02: the concurrency workload is not yet repeatable enough to close AC25-75

The matrix fixes sizes, readers, repetitions, minimum operation count, and writer
categories ([design.md:90](slice-75/design.md)), but leaves the operation mix, writer cadence, snapshot/page span, warm-up, timeout/retry policy, random seed, SQLite/cache settings, CPU placement, and resource sampling unspecified. Two valid runners can therefore produce materially different latency and contention distributions.

**Required correction:** require the sealed manifest to bind those workload
details per cell, including freshness/reset rules and exact failure accounting.
Define correctness/parity/lifecycle as zero-tolerance mandatory gates. Mark
latency/resource cells as either linked to a pre-existing accepted policy or
advisory before execution; advisory results cannot block or support a release
claim. Add manifest-schema negatives for each omitted repeatability field.

## P3 observations

- Dependency sequencing is correct: `55 -> 60 -> 65 -> 70 -> 75`, and Slice 75
  audits rather than backfills owning-slice evidence.
- Semantic policy remains external. The designs do not add query-intent routing,
  entity resolution, contradiction/truth selection, answer generation, or model
  choice to FathomDB.
- Historical receipts are treated correctly: protected graph and REASON
  treatments remain rejected; TEMPORAL external quality remains blocked without
  source-derived gold; GLOBAL-01 is not relabelled, and Slice 75 requires a fresh
  native `Engine.search` witness.
- Slice 15 wire rules are substantially carried through: versioned closed
  requests, material unknown-variant rejection, typed parity, immutable identity,
  frozen views, and opaque continuation/evidence capabilities are all explicit.

## Re-review gate

FIX-1 should be limited to the nine corrections above. Re-review should confirm
the normative contract changes and their mapped RED/GREEN fixtures; it should not
reopen accepted program scope, promote any historical treatment, or move semantic
policy into FathomDB.
