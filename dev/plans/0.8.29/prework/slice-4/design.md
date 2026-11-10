# Slice 4 review method and ownership

## Authorities and selected paths

[Current design ownership](../../../../design/README.md) and its
[current-owner map](../../../../design/current-owner-authority.json) distinguish
maintained explanations from accepted decisions and implementation witnesses.
Current source demonstrates implementation; dated qualification needs its own
receipt. A source comment is not a new contract or successful execution.

- Open/config validation and production Default/None admission are in engine
  `open.rs`; failed-open cleanup follows the accepted vector-compatibility
  successor. Engine-owned dispatch/runtime stays synchronous at the public API;
  embed workers own no SQLite connection. Projection workers retain connections
  and serialize publication under `commit_gate`. Caller extraction/consolidation
  subprocess transport is a separate semantic-task boundary.
- Canonical `write_commit.rs` and `projection_commit.rs` own IMMEDIATE transactions
  and visibility coalescing. `frozen_read.rs` binds visibility to snapshot state.
  Cache designs must use those actual readers/commits rather than only the
  writer connection's observed generation or an MCP open epoch.
- `search.rs` uses native KNN lifecycle/validity filtering plus source-dependency
  fallback; direct-node eligibility remains relevant to the deferred text route.
  `fusion.rs`, explanation maps and graph exclusion currently use body identity.
  Existing narrow hybrid deferred-identity search still loads every matched body
  before fusion and hydrates final identity later; deferred content is new work.
  The existing pure snippet selector consumes body-byte match ranges and produces
  different rendered-text coordinates; coverage cannot be inferred from them.
- MCP `registry.rs` admits work; `bridge.rs` transfers its StoreGuard to blocking
  SDK work and retains admission while abandoned work finishes. MCP owns adapter
  knowledge, handles, wire schemas, paging and budgets. Engine hints remain pure
  facts without record text; supported engine options do not imply MCP accepts them.
- `projection_commit.rs` publishes physical head span tuples; provider
  `truncation_offset` defines exclusive UTF-8 body-byte endpoints with unknown
  default. Reporting/carriers are residual work. SDK/native layers preserve typed
  outcomes and coordinates; adapter re-snippeting cannot redefine stored coverage.

This names inspected seams, not every caller, mutator or configuration path.
The [matrix](alignment-matrix.md) and [test mapping](test-mapping.md) bound claims.

## Predecessor fit and allocation

| Actual input | Slice4 disposition / receiving owner |
| --- | --- |
| Completed0 owned setup/state/capacity proposal | Reuse10 ownership and missing-tool/native-identity stops; do not provision or turn dated resources into readiness. |
| Completed1 dependency/survey proposal | Reuse10 exact discovery→dispatch→hash-parser and contained lock updates. Preserve qualified SQLite/fork/MCP baselines and independent E version; no new dependency design here. |
| Completed2 Keep dispositions | Keep query scaffold, package inputs, generator inputs, historical/failed evidence, refs/worktrees and live contracts. Only proven applicability/profile corrections feed10 via3/6. |
| Actual3 CRUD/authority allocation | Challenge identity, snapshot, fallback/remediation and coverage semantics against source. Completed3 final import/review log reread at2d5041795; no disagreement remains. Later4/5 findings feed9, never gate3 completion. |
| Completed6 documentation proposals | D6-3 adapter rationale, D6-6 format37 applicability and D6-11 workspace/operation count go to selected10 scope; behavior-coupled changes remain50/60/70/80/90. Avoid duplicating correction rows. |
| Completed7 delivery controls | Owned setup, contained editable-install repair, exact .29 tag-route scope and truthful failed/partial evidence stay10/20/160. No broad CI redesign or presumed runner access. |
| Completed8 reserved | No source path, inferred requirement or work allocation. |
| Slice5 concurrent oracle review | This slice supplies exact witnesses and bounded challenges;5 evaluates wider test adequacy. No declaration that5 is complete. |

## Contract-to-alignment trace

Actual3 R29-3P obligations map as follows; O3-n has the same number. This is a
review allocation, not a second acceptance specification.

| R29-3P suffix / D29 row | Alignment receiver |
| --- | --- |
|01 PREP,02 DEPS,03 SURVEY,04 DOCS,05 ROUTE | A4-01/13; reuse completed0/1/2/6/7 proposals and10 ownership. |
|06 BLK1 | A4-02;20 actual queue/hygiene residual. |
|07 MEASURE,08 CACHE | A4-04;30 inputs,40 conditional adoption. |
|09 SEARCH | A4-05;50 accepted identity and exact content/snippet route. |
|10 MCP1,11 MCP2 | A4-06;60 accepted surface exposure and final parity. |
|12 SDK | A4-03/07;70 bounded reasons and supported remediations. |
|13 B2-DESIGN,14 B2-BUILD,15 SPAN | A4-08;80/90 residuals and new public translations. |
|16 FAULT,17 RESET | A4-09/10;100 concrete gaps,110 positive teardown. |
|18 ARTIFACT,19 GPUQUAL,20 FORK | A4-11;120/130/140 qualified repair/artifact chain. |
|21 EXP,22 CLOSE | A4-12/13;150 selected studies and160 assembly. |

F4-2 also reaches engine `EmbedderRequired`/`EmbeddingReadiness.blocked`, facade/
SDK re-exports, PyO3/N-API errors and readiness conversions in
`src/rust/crates/fathomdb-py/src/errors.rs`,
`src/rust/crates/fathomdb-py/src/projection.rs`,
`src/rust/crates/fathomdb-napi/src/errors.rs` and
`src/rust/crates/fathomdb-napi/src/projection.rs`, Python
`src/python/fathomdb/read.py` and TypeScript `src/ts/src/errors.ts`/`read.ts`.
Sampled conversions copy the Rust remediation list into readiness/error payloads;
they do not filter the unsupported word.70 owns the contract and regression
coverage through both drain errors and readiness reports. Update affected
`dev/interfaces/{rust,rust-sdk,python,typescript}.md` with the behavior PR.
These paths identify receiving surfaces, not a completed binding audit.

## Alternatives and stop conditions

Prefer applicability-only corrections where source and accepted contract agree.
For semantic disagreement, retain current behavior until its owning successor is
accepted; do not silently reinterpret a body dedup contract as a record contract.
For cache/performance proposals, retain uncached/reference behavior if30 shows no
benefit or snapshot safety is unresolved. For unknown coverage, report its distinct
absence rather than infer full coverage;80 decides final public representation.
For faults, use existing controls where they reach the required seam; add a
bounded control only for a demonstrated gap after comparison. Inclusion100/110
is settled; actual fault venues and remedy design still gate affected execution.

Source/header stale wording can be corrected with evidence. Unsupported policy,
identity, public-field or error-precedence changes return to9 or owning design;
accepted ADR successors and public interface updates land with behavior PRs.
No storage-format change, driver/primary-CUDA change, erasure enhancement,
chunking/ANN work, new weak-result threshold or optional promotion is created.
