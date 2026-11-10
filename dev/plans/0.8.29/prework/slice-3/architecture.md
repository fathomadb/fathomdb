# Slice 3 proposed architecture reconciliation

## Ownership and accepted constraints

The [active v2](../../../../design/fathomdb-data-plane-architecture-v2.md) owns
mechanisms; the caller owns semantic policy. Engine owns canonical eligibility,
rank/provenance, format37 visibility, persistent span truth and pure hints.
SDK/native layers preserve typed outcomes, options and bytes. MCP owns admission,
registry, handles, adapter-only facts, response shaping/budgeting and rendering.
No proposal document becomes an additional runtime authority.

Accepted authorities: [eligibility/frozen reads](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md),
[format35 spans/native eligibility](../../../../adr/ADR-0.8.28-store-format-35.md),
[format37 coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md),
[hints](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md),
[provider admission](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md),
[CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) and
[current operator maintenance](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md).
Seq-368 retains existing37/no migration; seq-367 limits140 to required repairs.

## Source-grounded built versus residual seams

November10 at product6edc1155f; source reading is not execution. Paths below are
repository-relative. Exact test-body evidence/limits are in [acceptance](acceptance.md).

| Seam / actual inspected source | Built fact → proposed residual / owner |
| --- | --- |
| `src/rust/crates/fathomdb-schema/src/lib.rs`; v2 profile; canonical applicability openings | SCHEMA_VERSION37 versus format36 prose; Cargo12 members/maintained53 live governed endpoints versus v2 historical11/44.10 corrects applicability/profile only, retaining actual historical decisions and ID criteria.6 supplies audience wording. |
| `src/rust/crates/fathomdb-engine/src/fusion.rs` `fuse_three_arms`; `search.rs` body rank/score maps; `dev/interfaces/rust.md` direct-text contract | Hash collision handling compares full bodies; per-arm credits and representative/provenance still body-keyed.50 must select a semantic successor, update explanations/graph seeds/evidence identity consistently, then optimize. A faster old-body result is not new-identity equivalence. |
| `src/rust/crates/fathomdb-engine/src/vector_spans.rs`; `tests/slice60_store_format.rs` | Current physical guard requires head window0/start0, vector rowid=cursor, UTF-8 end/length and mapping completeness. Future multiwindow storage allowance is not enabled production chunking.80/90 reporting reuses this, not a migration or new spans writer. |
| `src/rust/crates/fathomdb-mcp/src/output.rs` Envelope/Unavailable; `tools.rs`; `edge.rs` | Mandatory unavailable state/verdict and dual schema/structured content remain; default_store rows/hints are already present.60 owns formats;80/90 implement accepted omission plus proven residual fields. |
| `src/rust/crates/fathomdb-mcp/tests/search.rs`; `src/rust/crates/fathomdb-engine/src/hints.rs` | Adapter explicitly refuses text filters; pure no_results/degraded/more_results hints already built.60 changes adapter exposure using current direct engine route,80/90 preserve hint engine/adapter split. |
| `src/rust/crates/fathomdb-embedder/src/cuda_pool_policy_tests.rs` reset parent/child | Feature/architecture/venue-gated characterization expects context_lost and SIGSEGV at drop.110 owns positive remedy;140 only required fork repair,130 final affected proof. This is not a request to rerun a destructive GPU fault now. |

## Identity choice: explicit50 prerequisite

Existing logical_id-alone active uniqueness is canonical identity, not by itself
a fusion key choice. Typed IdSpace, physical write cursor, logical revisions,
nodes/edges, ownerless/NULL logical IDs and equal-body distinct sources must be
censused. Alternatives include current body semantics, record-cursor identity,
or another explicitly specified typed record key. State which same-record
multiarm contributions accumulate and which twin records remain distinct; freeze
representative/score/tiebreak/provenance/graph-seed/explanation rules together.

Accepted role-tagged bytes (decision-index row65) remain stored bytes: no
normalization may silently merge records or change coverage coordinates.
Existing body-dedup tests are intentional, so semantic replacement requires
accepted successor ADR/interface/test-intent change before RED→GREEN.50 provides
settled identity to80;80 does not require50's completed optimized route.

## Omission and coverage: settled versus open

Hints ADR(f) already requires omission while state/verdict can only be
unavailable and orient disclosure in not_in_this_release. Compact design's
restoration implication must be reconciled to that accepted decision.80 specifies
both-format schemas/rendering;90 applies accepted residuals. Neither row
creates a calibrated verdict or weak_results threshold.

Span endpoints are exclusive original-body UTF-8 offsets. Body, selected snippet
windows with separators, and escaped/framed rendered bytes are different spaces.
80 must choose and document: coverage population/denominator, eligible/frozen view
and projection generation, known-empty/full/partial versus unknown/pending/missing,
nonvector-result behavior, scored intersection representation and budget truncation.
Proposed invariant: intersect only evidence belonging to the exact returned
record/revision; never infer full coverage from NULL or borrow equal-body spans.
The representation/aggregation choices remain unaccepted.90's tests use the
approved rule rather than letting arithmetic/serialization define policy.

## Cache, degradation and fault fit

40 keys/invalidates cached facts using actual reader snapshot/database/branch,
view, effective time and37 visibility semantics, including external writes,
rollback and time-only transitions. Retain native eligibility before caps and
separate dependency fallback.30 measures write-interleaved suitability first;
reject/defer is valid. No automatic generation-counter/schema alternative.

70 proposes only bounded missing degradation reasons, precedence and supported
remedies under accepted failed-open/numeric provider admission. Preserve Default/
None and semantic-task transport; do not reintroduce caller numeric providers.
100 compares existing real erasure/recovery/crash coverage before adding reached
fault seams, preserving explicit maintenance/fences and survivor/history truth.
The fully read slice85_crash child already exits73 at eight named before/after
commit positions with reached-boundary/reopen/retry assertions. Only demonstrated
remaining transaction/checkpoint gaps need new qualification.
110 requires safe resource lifetime/teardown, not only an error label.

Required120 policy/toolchain exposure review distinguishes compiler, emitted
PTX, dynamic libraries, driver floor, device architecture and product artifact.
R580/primary12.6 remain fixed, no K620/reflash. Fork-local required repair proof
precedes pin integration; historical CUDA13 library success is not product,
TegraQ5 or installed-consumer success. Shared engine/interface/manifest writes
serialize; conditional edges and receiving public surfaces are in [allocation](allocation.md).
