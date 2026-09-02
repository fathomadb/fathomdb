---
title: 0.8.25 Slice 65 — deterministic candidate selection and profile qualification design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
target_release: 0.8.25
depends_on: 60
readiness_gate: 0.8.25 Slice 7 completion
---

# Slice 65 — deterministic candidate selection and profile qualification design

## Authority and shared ownership

This design owns R25/AC25-65 and the candidate-selection half of Memex need 16.
It also defines the shared named-profile, qualification, receipt, and promotion
contract used by Slice 70. It does not build an intent router: callers select a
profile explicitly, and omitted profile selection remains accepted A0.

Architecture v2, A25-05, EARP, and the performance PROGRAM govern this design.
It is REVIEWED_BLOCKED_ON_SLICE_7 and cannot become READY before Slice 7 activates architecture
v2 or before Slice 60 is READY. Slice 70 may add algorithms, not a second
registry or qualification grammar.

## Requirements-to-design comparison

| Obligation | Current design/evidence | Required decision |
| --- | --- | --- |
| Named, bounded, opt-in profiles | Planner/router PSD is unreviewed and contains open routing questions | Define an explicit caller-selected profile reference; do not ship L2 routing. |
| Deterministic selection | RRF is shipped; MMR and coverage ideas exist | Define replayable stage inputs, tie-breaks, bounds, and explanation. |
| Entity/alias handling | No governed selector; semantic entity resolution is external | Match only caller-declared aliases through indexed exact attributes. |
| Duplicate/diversity/complementarity | Candidate designs and experiments exist | Define bounded treatments without inferring semantic requirements. |
| Qualification and promotion | EARP supports strict versioned campaigns | Add a profile qualification schema and fail-closed registry state. |
| Usability guard | REASON-01 improved recall while answer grounding regressed | Require downstream usability gates whenever a profile changes answer context. |

## Predecessor disposition

| Design or evidence | Disposition |
| --- | --- |
| `dev/design/earp.md` | **Reuse.** It remains the execution/receipt authority; this design adds a profile campaign contract. |
| `dev/design/planner-router-psd-0.8.x.md` | **Experimental input only.** Do not promote its L2 router, intent classifier, feedback loop, or unresolved cost policy. |
| `ADR-0.8.0-agent-memory-retrieval-and-identity.md` | **Reuse.** Fused RRF and its stable tie behavior remain the A0 basis. |
| `dev/design/0.8.4-closing-graphrag-gap.md` | **Candidate evidence.** MMR/coverage concepts may be treatments, not accepted defaults. |
| ANSWER-01, GLOBAL-01, GRAPH-01, and REASON-01 receipts | **Normative negative evidence.** More recall, context, or graph reach alone cannot qualify a profile. |

Historical designs and receipts remain unchanged.

## Shared profile contract

### Registry and public types

An installed, versioned `RetrievalProfileRegistryV1` contains immutable profile
entries keyed by `(name, revision)`. Each entry contains:

- canonical stage configuration with all defaults resolved;
- input capabilities and required projections/devices;
- hard candidate, context, latency, and memory bounds;
- compatibility minimums and registry digest;
- state: `qualified_retrieval_only`, `qualified_answer`, or `retired`;
- the accepted qualification receipt identity and decision-rule digest.

Experimental and rejected treatments are retained in `experiments/` receipts,
not in the installed production registry. A rejected name/revision can never be
resolved by the production API.

`RetrievalProfileRefV1 { schema_version: 1, name, revision }` is an additive
option on the governed search request. `None` resolves directly to the compiled,
immutable A0 configuration; there is no registry lookup and no `auto` value.
The request also declares `intended_use: retrieval_evaluation|answer_context`.
The latter rejects a profile not in `qualified_answer` state.
`ProfileExecutionV1`, available in opt-in explanation/receipts, contains the
registry digest, resolved profile, stage versions, candidate counts, bounds,
fallback/degraded outcomes, timings, and result-order digest. It contains no
query text or source bytes.

Unknown request fields/variants and unknown profiles fail typed before search.
Additive response fields may be ignored, but unknown variants that affect
ranking, visibility, continuation, or fallback yield `UnsupportedVersion`.
Rust, Python, TypeScript, wire, and Windows CPU/native behavior must agree.

### Qualification states and immutable default

A treatment moves through `preregistered -> executed -> accepted|rejected` in
an EARP `comparison` campaign. The configuration, corpus/gold digests, split,
fixed A0 comparator, changed knobs, metrics, paired uncertainty method/seed,
minimum sample/power rule, resource ceilings, and decision predicate are sealed
before execution. Validation rejects test-set tuning, a differing answerer/
judge, omitted failures, mixed corpus identities, or an unmeasurable metric.

Acceptance permits creation as `qualified_retrieval_only` or
`qualified_answer`. In 0.8.25 the builder rejects every registry entry or alias
whose state/name purports to be `default`, `A0`, or the omitted-profile target.
`None == compiled A0` is immutable for this release. A future default change
requires a successor design and release contract carrying a decision reference,
accepted receipt identity/digest, and compatibility effect; the 0.8.25 registry
has no promotion-capable form. Rejected and underpowered runs remain durable
evidence and cannot enter the installed registry. A changed stage or bound
mints a new revision and repeats qualification.

A profile is mechanically **context-changing** when its algorithm can change,
or an equivalence fixture observes a change to, ordered hit identity, body,
evidence identity, or evidence order relative to A0. Every candidate selector
in this slice is therefore context-changing. It can enter
`qualified_retrieval_only` after its retrieval gate, but enters
`qualified_answer` only after the registered answer correctness, groundedness,
and attribution no-regression predicates also pass. Only identity/body/evidence-
and-order-preserving telemetry or packaging changes may omit that answer gate.
Retrieval-only state is surfaced in the execution receipt and is rejected when
`intended_use=answer_context`. A campaign cannot relabel an answer metric as a
data-plane metric or vice versa.

## Candidate-selection treatments

Every treatment is a deterministic stage over a bounded candidate list and may
be evaluated separately or in a preregistered composite. No stage chooses
query intent or creates semantic aliases.

1. **`declared_alias_exact_v1`.** Normalize query tokens and caller-declared
   alias values with the repository's pinned Unicode normalization and casefold
   implementation, then perform exact membership through a declared attribute
   projection. Alias ownership and entity identity remain caller policy.
2. **`content_dedup_v1`.** Group candidates by canonical content hash plus exact
   source locator. Retain the lowest original rank, then immutable revision ID.
   Cross-source semantic paraphrases are not collapsed.
3. **`mmr_diversity_v1`.** Greedily select by the preregistered relevance/
   cosine-diversity weight from a dense-ready pool. Ties use original fused rank
   then immutable revision ID. Dense-unavailable behavior is refusal unless the
   profile explicitly declares a measured soft fallback.
4. **`structural_coverage_v1`.** Greedily maximize only declared structural keys
   (`source_id`, canonical kind, retrieval arm, and caller-declared alias key),
   then relevance. It does not synthesize requirements or claim semantic
   comprehensiveness.
5. **`rrf_fusion_v1`.** Fuse named candidate arms using RRF with `k = 60`; ties
   use best contributing rank then immutable revision ID. New arms must be
   preregistered and independently explainable.

Profile definitions bind the candidate input cap (`1..=1000`), output limit
(`1..=100`), stage order, parameters, and fallback. The production executor
accepts no caller parameter outside the immutable profile entry; callers who
need custom experimental parameters use EARP, not an ungoverned SDK map.

## Persistence, flow, and receipts

The installed registry is a package asset validated at build/open and addressed
by SHA-256. No per-database profile tables or schema migration are added. A
database stores only projection declarations it already needs; profile choice
is request state and receipt evidence, not semantic memory.

Execution validates profile/capabilities/snapshot, collects eligible candidates
through named existing arms, applies stages in registry order, truncates only
after eligibility and stage rules, and emits compact hits plus optional profile
explanation. Replay with the same database snapshot, registry digest, request,
and device-equivalent deterministic operators must produce the same ordered
identity digest. CUDA kernels may use their documented numeric-equivalence
boundary; the receipt records device/runtime resolution.

## Invariants, failures, and boundaries

- A0 and omitted profile behavior remain unchanged; the installed registry
  cannot override, alias, or shadow the compiled A0 default.
- No stage makes an entity, truth, temporal, or intent judgment.
- Hard eligibility and lifecycle visibility precede candidate selection.
- Bounds are immutable profile data and enforced before allocation/execution.
- A soft fallback is allowed only when named, measured, and explained by the
  accepted profile; otherwise missing capabilities fail closed.
- A rejected, retired, unknown, digest-mismatched, or unsupported profile never
  executes as another profile.

Typed failures include unknown/retired profile, registry digest/version
mismatch, unsupported stage/version, missing projection/capability, snapshot
failure, invalid bound, deterministic replay mismatch, and resource ceiling.
Malformed/exhausted evaluation cells are retained as quality failures under the
registered continuation policy; they do not abort or disappear from aggregates.

## Compatibility, lifecycle, and performance

Profiles operate only on candidates already visible under the bound snapshot
and eligibility envelope. Superseded, erased, or dependency-dead records cannot
be restored by fusion, diversity, alias matching, or fallback. The default
compact `SearchHit` remains unchanged; profile execution detail is opt-in.

Qualification reports paired retrieval quality, duplicate rate, source/arm
coverage, answer-usability metrics when applicable, p50/p95/p99, throughput,
RSS/VRAM, context size, failures, and uncertainty. A speed or recall gain cannot
offset a failed registered correctness, lifecycle, grounding, attribution, or
resource boundary.

## Mapped RED/GREEN and verification

| Acceptance boundary | Required proof |
| --- | --- |
| Registry closure | Schema negatives for unknown keys/stages/states, digest mismatch, duplicate names, rejected installation, and immutable revision drift. |
| Determinism | Tie-heavy, duplicate, shuffled-input, reopen, CPU, and CUDA-equivalence fixtures yield the declared identity digest. |
| Treatment correctness | Exact-alias projection, exact-content dedup, MMR tie/order, structural coverage, and RRF k=60 golden properties on real databases. |
| Eligibility/lifecycle | Ineligible, superseded, erased, and stale candidates never reappear through a selector or fallback. |
| Qualification | Failed, underpowered, mixed-layer, mismatched-arm, and held-out-tuned fixtures cannot become registry entries. |
| Default guard | Omitted profile resolves byte-identically to compiled A0; `default`/`A0` entries, aliases, and unauthorized shadowing fail schema/builder validation. |
| Usability | Every ordered-hit/body/evidence-changing treatment is retrieval-only until correctness, grounding, and attribution pass; `answer_context` rejects retrieval-only profiles. Identity/order-preserving telemetry fixtures prove the sole exemption mechanically. |

Run focused selector and registry tests, EARP schema/real-SDK comparison tests,
heavy held-out campaigns, all/all-feature, applicable Windows CPU/native,
CUDA dense/rerank/fusion equivalence, and registry-installed smokes. A
live-model route is run only when its sealed campaign requires an answer-
usability gate; operator routes are not applicable.

An independent review may require at most three FIX-n cycles. Unresolved P1/P2
findings, an implicit router, mutable production parameters, a missing
usability guard, or any registry-controlled default behavior blocks READY.
