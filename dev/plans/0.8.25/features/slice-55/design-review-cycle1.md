---
title: 0.8.25 Slice 55 independent design review — cycle 1
status: FIX_1_APPLIED_RE_REVIEW_REQUIRED
review_cycle: 1
reviewed_commit: 059618f79898e658e149d06013cedb72eb1fee7a
reviewed_on: 2026-09-05
verdict: FAIL
---

# Slice 55 independent design review — cycle 1

## Verdict

**FAIL.** The design at `059618f79898e658e149d06013cedb72eb1fee7a`
had six P1 implementation-shaping findings, two P2 verification/authority
findings, and one P3 readiness-record finding. Implementation was not
authorized. FIX-1 updates the design and plan, but does not change this verdict
or self-promote the design to READY; an independent cycle-2 review is required.

## Preserved findings

### C1-55-01 — P1 — Public API ownership collides with accepted operator contracts

Existing operator-only `Engine::check_integrity(CheckIntegrityOpts) ->
IntegrityReport` and `trace_source_ref` already occupy those names and accepted
contracts. Python and TypeScript explicitly expose no doctor methods. The draft
needed a code-grounded exists-versus-net-new table, exact non-colliding Rust/
Python/TypeScript names and namespaces, CLI mapping, facade exports, governed
allowlists, and explicit composition or separation from the legacy APIs.

### C1-55-02 — P1 — Integrity has no enforceable work bound or coherent snapshot rule

`max_findings` bounded output but not work. The design needed a hard
`max_work`, counted units, per-check and aggregate accounting, deterministic
order, cap-plus-one detection, exact `checked_count`, duplicate/empty-check
behavior, and all-or-error semantics. It also needed a versioned boundary and
either one-snapshot linearization or a precise generation-drift contract.

### C1-55-03 — P1 — Dependency reciprocity assumes a nonexistent reverse row

Slice 20 stores one normalized dependency table and uses the authoritative
source link for both directions. The draft's missing-reverse-row fixture was
fictional. Integrity needed to validate the actual dependency row, derived
owner, source link, canonical owner/node, source-version mapping, canonical
self-link, dependency generation, schema values, and roles, with injectable
real corruptions and stable finding codes.

### C1-55-04 — P1 — Reciprocal trace semantics and authorization are incomplete

The draft omitted complete trace node/edge/boundary schemas, exact API
mappings, context requirement/default, root role/direction behavior, root
inclusion, lifecycle/eligibility subjects, absent-versus-invisible precedence,
and a single read transaction. Eligibility applies to a registered relation's
endpoints and closure barriers; the relation itself has no active state.

### C1-55-05 — P1 — Explanation is not reconciled with the ratified carrier, correlation, or privacy

The draft needed a precise additive successor to
`Explanation { trace, per_hit }` and `PerHitExplain`, preserving query-level
versus positional association. It needed to distinguish ordinary explanation
from the evidence-authorized path, define one content-free correlation identity
and telemetry-off behavior, preserve unchanged default/off-path cost, remove or
define `operation_id`, prohibit raw caller artifact/source/dependency IDs in
telemetry, and enumerate the safe telemetry subset.

### C1-55-06 — P1 — A25-05 wire and error requirements are under-specified

Every type needed a complete schema and Rust/Python/TypeScript or operator-CLI
mapping, schema version, canonical field order, `u64` representation, finite
score rule, lower-snake discriminants, request/response unknown behavior,
validation order, exact RFC 6901 paths, typed error mapping, and nondisclosure
precedence.

### C1-55-07 — P2 — Projection and orphan checks lack an authoritative finding matrix

The design needed to enumerate authoritative tables/joins, the effective
instant, legitimate exclusions, closed finding codes, severities, and minimum
IDs. It needed to reuse Slice 40's physical-membership/completion classifier,
define the mutation-readiness scan, and map `ProjectionGenerationError`
without redefining that accepted public error.

### C1-55-08 — P2 — The plan lacks executable TDD and verification traceability

The plan needed numbered S55-R and S55-AC rows mapped to named RED tests, exact
commands, real fault fixtures, features, Windows jobs, fresh-package smokes,
evidence paths, default search/explanation nonregression, and real-database
property tests.

### C1-55-09 — P3 — Readiness metadata is stale

Slice 7 and Slice 50 were already complete. Metadata needed to say so while
remaining non-READY until the P1/P2 findings pass independent re-review.

## FIX-1 resolution map

| Finding | FIX-1 resolution | State pending re-review |
|---|---|---|
| C1-55-01 | `design.md` now inventories shipped APIs and assigns `trace_dependency` to the governed SDK surface and `check_data_plane_integrity`/`doctor data-plane-integrity` to the operator surface. Legacy reports and SDK doctor absence remain separate and unchanged. | PROPOSED_RESOLVED |
| C1-55-02 | Integrity now has `max_work_units <= 10000`, `max_findings <= 100`, exact candidate units, canonical check/key order, aggregate/per-check counts, indexed cap-plus-one probes, all-or-error behavior, and one deferred reader transaction with a versioned boundary. | PROPOSED_RESOLVED |
| C1-55-03 | The fictional reverse row is removed. The design enumerates the normalized dependency/source-link chain, ten closed corruption conditions, real test-hook targets, codes, severities, and IDs. | PROPOSED_RESOLVED |
| C1-55-04 | Complete request/node/lifecycle/edge/boundary/result schemas, required frozen context, role/direction rules, root-first ordering, endpoint/barrier eligibility, nondisclosure precedence, reciprocity, and one-snapshot linearization are specified. | PROPOSED_RESOLVED |
| C1-55-05 | The accepted carrier is amended only with `Explanation.correlation_id` and `PerHitExplain.structural`; positional association and existing ID meaning remain. One Engine-minted identity is reused by telemetry, evidence-only authority stays separate, the off path stays allocation-free, and safe/prohibited telemetry fields are explicit. | PROPOSED_RESOLVED |
| C1-55-06 | The design now pins every new schema, SDK/CLI mapping, version, canonical field order, scalar representation, finite-score rule, unknown behavior, validation order, error family/code, RFC 6901 path, and nondisclosure precedence. | PROPOSED_RESOLVED |
| C1-55-07 | Authoritative dependency and search/projection/orphan/readiness matrices name candidates, legitimate exclusions, stable codes, severities, IDs, effective instant, shared Slice 40 classifiers, mutation receipt scan, and `ProjectionGenerationError` translation. | PROPOSED_RESOLVED |
| C1-55-08 | `plan.md` maps S55-R1..R7 and S55-AC1..AC7 to named RED/property tests, exact focused/repository/package/Windows commands, features, real fault fixtures, evidence files, and nonregression probes. | PROPOSED_RESOLVED |
| C1-55-09 | Design and plan state that Slice 7 and prerequisites through Slice 50 are complete, use `DRAFT_FIX_1_REVIEW_REQUIRED`, and require cycle-2 PASS before READY. | PROPOSED_RESOLVED |

## Re-review gate

Cycle 2 must inspect the FIX-1 files against the current code and accepted
Slice 15/20/35/40/50 contracts. A P1/P2 finding keeps the design non-READY.
Only an independent PASS may change readiness metadata and authorize RED
implementation work.
