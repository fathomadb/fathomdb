---
title: FathomDB 0.8.27 Slice 80 - implementation status
status: COMPLETE
implemented_on: 2026-09-27
historical_planning_commit: 9a31e979
follow_up_commit: 1d7f826c2125d5de6d852ddbc96d35fec2de5816
authoritative_binding: PASS
historical_reviewed_candidate: e9631b9761d292a4115d1beee95678801512f4f2
historical_closeout_commit: bca0c99d46e0111c1bd3906d78a77668801463bc
reviewed_candidate: b7403958a3839d371c1672335c517fa762a451cf
implementation_candidate: 1d7f826c2125d5de6d852ddbc96d35fec2de5816
closeout_commit: e8e603b7bc5fd3a87616382ffd82273f04931f4f
---

# Slice 80 implementation status

## Slice 80 complete on the release branch

The product extraction is complete on `release/0.8.27`: it moved the
root-owned read side into private semantic modules without changing behavior,
schema 34, SQL, statement ordering, snapshot or transaction scope, error
mapping, feature gates, public paths, or wire encodings. Overall Slice 80 is
**COMPLETE** after independent rereview and release-state binding:

- `fusion.rs` and `filter.rs` own ranking fusion and filtering;
- `search_types.rs`, `search.rs`, and `search_api.rs` own search carriers,
  execution, graph-arm integration, and the `Engine` search facades;
- `telemetry.rs` owns search observability and feedback;
- `read.rs` owns read verbs, canonical pages, and operational-state reads;
- `reader_pool.rs` owns the pool implementation and worker loop; and
- `graph_expand/` is split into types, codec, execution, and traversal.

Public and doc-hidden items keep their root paths through re-exports. Reader
data carriers remain at the root without field widening. The four search-owned
`ProjectionRuntimeShared` fields keep their shape and owner.

## TDD and implementation

Characterization commit `77439ba1` added three gap-filling tests on unchanged
production. Each passed on the baseline, failed against its specified
temporary mutant, and passed again after exact restoration:

- reader transactions release after an in-transaction refusal;
- the search graph arm applies the validity view; and
- projected-text search applies the validity view.

Fifteen mechanical extraction batches then moved the implementation. The final
structural batch was `f333926e`. Historical code-review fix `8e449963`
returned the shared reader-transaction primitive to root-private ownership and
narrowed unnecessary visibility; later production changes ended at
`0efa62c544af00858aa6975944e8f36c99f13218`.

Follow-up test commit `9700991f` closes the graph-result codec property gap.
Generated coherent results now prove typed decode→encode→decode equality across
linked fields, canonical integer strings, finite/null score, and optional valid
evidence. A second property proves generated nonzero first-entry evidence
positions are refused at the exact path. Both killed their specified temporary
production mutants and passed after exact restoration; production is unchanged
and no generated golden oracle was added.
Lint-only follow-up `31e78529` packages fixture inputs in one private test
carrier; test behavior and production remain unchanged.

## Review and verification

- **Design review:** the final boundary rereview returned PASS at `b7403958`
  after the recorded three cycles (`design-review.md`).
- **Code review:** the independent `gpt-5.6-sol` high-reasoning review first
  returned FAIL with two P2 architectural findings, closed historically in
  `8e449963`. The follow-up rereview covered the codec properties and Slice 85
  handoff in three cycles: Cycle 1 found one P1 and four P2s, Cycle 2 one P2,
  and Cycle 3 returned **PASS** at clean reviewed candidate `b7403958`
  (`code-review.md`).
- **Independent verification:** the separate read-only `gpt-5.6-terra`
  verifier returned PASS at historical clean candidate `3e60cc5d`
  (`review-verification.md`). Those receipts apply only to `3e60cc5d`;
  production subsequently changed through `0efa62c5`, so they do not verify
  later production or `e9631b97`.
  - canonical verification: 127/127;
  - workspace Clippy/check: PASS;
  - candidate-bound Python receipt: PASS;
  - public surface: 13 rows, exact;
  - hidden surface: 33 rows, additive only, with 261 additions, 0 changes,
    and 0 removals;
  - strict security: claimed 0 violations, 0 blockers, and 0 downgrades,
    including live AC-037. That claim at `3e60cc5d` is **UNEVIDENCED**: the pass
    lines and the grant/revert record were not captured. A HITL-granted re-run
    through `dev/release/ac-037-live-netns-hitl-runbook.md` then passed 0/0/0
    with both live layers at `66e27983`, and the grant was reverted
    (2026-09-27; `review-verification.md`). That run is historical only: it
    does not qualify later HEAD or the final candidate. By owner ruling, the
    Slice 150 alone must bind a new runbook receipt to the exact final
    candidate; and
  - feature-complete: 21 runs, 357 planned, 349 passed, 0 failed, and 8
    documented ignores.

The post-fix production changes were covered by focused tests, affected-route
Clippy, and an exact rustdoc surface diff. There is no post-fix full canonical
PASS and no official post-fix public or hidden capture. Attempts at exact
historical candidate `e9631b97` were blocked by the public capture's 100 GB
free-space minimum, `nvidia-smi` exit 9 for the hidden capture, and the
advanced release reference making one historical-ref `agent-verify` fixture
inapplicable.

An unconfined diagnostic `agent-verify` at live `24813b8e`—with `src/`
identical to `31e78529` and `e9631b97`—passed strict security 0/0/0 including
AC-036 and both AC-037 layers, and passed `test-rust` in 516,820 ms. It
registered 127 suites: 125 ran, 123 passed, 2 failed, 2 skipped, and none were
excluded. Both failures came from the missing local `fathomdb._fathomdb`
module and the consequent native-receipt failure. It is not a canonical PASS
or release-qualifying AC-037 receipt.

The final independent three-cycle rereview then covered the strengthened
result-codec property and the corrected Slice 85 boundary design through
`b7403958`. Cycle 1 found one P1 and four P2s, Cycle 2 found one P2 in manual
board status prose, and Cycle 3 returned PASS. This review result prepares the
separate release-state binding; it does not make the historical, partial, or
blocked verification evidence above a current pass.

## Slice 85 and Slice 90 handoff

Planning-only Slice 85 owns the carrier and dependency boundary work recorded
in the master plan and `design.md`:

- establish final homes: `read_api.rs`, `graph_api.rs` for every graph
  `Engine` facade including `explain_graph_neighbors_for_test`, existing
  `search_api.rs`, leaf `structural_state.rs`, leaf `reader_transaction.rs`,
  top-level `wal_attribution.rs` for `WalAttributionCollector`, every
  `Reader*Pause` alias, and related attribution phases/helpers, pool protocol
  in `reader_pool.rs`, search carriers in `search.rs`, and `TelemetrySink` in
  `telemetry.rs`. Fields stay private and Slice 90 must not redesign these
  boundaries. `graph_expand/execution.rs` retains only private non-`Engine`
  helpers and `*ForTest` carriers, while typed private pool factories cover
  the existing `VectorStage` and `ExplainGraphNeighbors` requests without
  exposing senders or the request protocol;
- eliminate all four Slice 80 cycles: `search` ↔ `graph_expand`,
  `read` ↔ `reader_pool`, `graph_expand` ↔ `reader_pool`, and
  `graph_expand` ↔ `search_api`; and
- enforce the exact dependency graph through compiler-visible ownership and a
  non-vacuous `syn` AST gate in normal lint. It must derive/compare the full
  in-scope module and governed Engine-field sets, allowing only named reviewed
  exclusions; only the three inherited earlier-slice cycles named in
  `design.md` are initially eligible for a shrink-only allowlist; and
- before any move, obtain successful exact-baseline canonical verification,
  native receipt, and official public and hidden captures on a capable host.

Slice 90 consumes those settled boundaries, then owns the runtime facade,
reader-loop WAL/diagnostic arms, reader open-path helpers, remaining
search-index projectors, and final placement of the four search-owned
`ProjectionRuntimeShared` fields.

These are handoffs. Slice 85 is planned but uncommissioned; no Slice 85 feature
directory or implementation was created.

Production is unchanged after `0efa62c5`. The implementation candidate is the
test-only `1d7f826c`; the independently reviewed candidate is `b7403958`, and
this closeout is `e8e603b7`. The earlier `e9631b97`/`bca0c99d` binding is
historical, as are `8e449963`, `3e60cc5d`, and `4299511c` where their original
evidence is discussed. Current review binding does not enlarge the historical
or partial verification evidence.

## Cleanup

No new branch or worktree was created. The verifier removed its disposable
Python 3.12 environment and generated artifacts, restored the tracked
`src/python/fathomdb/_fathomdb.pyi`, and left the tree clean. It reported
using an already-active AC-037 capability without changing AppArmor state;
that claim is unevidenced (see above). Tags,
registries, publication, and later slices were not touched.

## Post-hoc design review fix-1 (2026-09-27)

A post-hoc adversarial design review returned PASS-WITH-FIXES:

- one P1: four struct fields had widened;
- three P2s: the dependency invariant was false at module level, the Slice
  140 handoff was missing, and the live AC-037 layer was unevidenced;
- seven P3s.

Fix-1 made the following changes, with no behavior change:

- restored the private carrier fields and narrowed visibility (`b8af4d86`);
- corrected the design, review, and plan records;
- marked live AC-037 UNEVIDENCED.

The per-finding record is in `design-review.md`.
