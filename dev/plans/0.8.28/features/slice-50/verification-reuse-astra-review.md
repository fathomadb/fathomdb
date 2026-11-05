---
title: Process-local verification reuse — Astra-medium review
status: SOUND_DIRECTION_NEEDS_DESIGN; no implementation approval
observed_on: 2026-10-31
reviewed_head: 50488a05389b0c35ff3bfb5e9ad05114b13b8dbb
reviewer: gpt-6-astra
reasoning_effort: medium
---


> **SUPERSEDED BY SCOPE `seq-357` (2026-11-01).** This historical review concerns
> the withdrawn runtime/evidence-cache proposal and is not approval of the current
> [narrow design](built-in-provider-design.md). Its original findings are preserved;
> the [separate narrow-scope review](built-in-provider-narrow-review.md) records
> the current independent verdict.

# Process-local verification reuse review

Owner requested an independent Astra-medium review. Reviewer ran as
`/root/astra_medium_reuse_review`, with fresh context and read-only access.
It reviewed the slice checkout and MCP on dev/0.8.29 at `6c177cf90`.
No tests, edits or implementation were performed by the reviewer.

## Proposal reviewed

Full numeric verification on first relevant open for each store/runtime
combination within a process; typed failed open and explicit recovery on
failure. Retain positive verification metadata independently of the database
handle so MCP idle/switch closes do not inherently require another 45 probes.
Reuse requires both unchanged store verification inputs and an actual runtime
fingerprint (assets, pooling, normalization, build, effective backend and
execution settings). Restart or changed inputs force recheck. Unknown/custom
providers cannot inherit acceptance merely from their claimed identity; a new
instance requires verification without an adequate stability descriptor.
The proposal claimed models need not remain resident and explicitly was not
implementation-ready. This review tests that claim rather than assuming a
complete fingerprint is already available.

## Verdict

**SOUND_DIRECTION_NEEDS_DESIGN.** Positive-result reuse within a process can
preserve MCP eviction and avoid repeated probes. The required runtime
immutability contract does not exist today. Process-local scope prevents a
persisted verdict bypassing the first relevant check after restart; it does
not itself establish numerical stability inside the process.

## Ranked review

### P1 — Runtime descriptor must identify a frozen execution regime

The provider trait exposes only identity/embed/embed_batch; Send + Sync does
not prevent interior mutation or a remote provider changing behavior. BGE
pooling can change without identity changing. Installed Candle also exposes
`set_gemm_reduced_precision_f32`, a mutable process-global numerical setting.
Sampling a constructor descriptor cannot freeze that setting while checking
and serving.

Define supported runtime stability or versioning before allowing reuse. Process
membership, claimed name/revision or object address are not proofs. A fresh
check on a new custom instance does not solve mutation of an existing instance.
Forty-five successful probes are evidence for the specified check, not universal
numerical equivalence for arbitrary future text or changing providers.

Witnesses: `src/rust/crates/fathomdb-embedder-api/src/lib.rs:79–96`;
`src/rust/crates/fathomdb-embedder/src/candle_bge.rs:692–699`; installed
`candle-core-fathomdb-0.11.0/src/cuda_backend/mod.rs:2477–2489`.
The parent independently confirmed the mutable setter in the installed source.

### P1 — Store invalidation needs an explicit live-session policy

Open probes after mean recovery, and the store fingerprint includes mean,
references, fixture and thresholds. However, operator `recompute_mean` changes
the mean and requantizes during a live session without repeating verification.
The binary comparison itself depends on that mean.

Choose explicitly between verified-at-admission semantics and a continuing
verification guarantee. The latter needs invalidation/reverification before
further dense admission when verification state changes. Cover baseline
creation, projection registration, recovery and failed fresh diagnosis. Publish
positive reuse tokens only for committed state, never a prospective baseline
whose transaction can roll back. Read/hash/publish coherently with the state
admitted by open, preserving the existing store lock. Do not imply protection
from unsupported external SQLite modifications.

Witnesses: engine `src/open.rs:851–873`, `:800–810`;
`src/vector_equivalence.rs:347–375`, `:675–704`; `src/mean.rs:414–456`.
Parent also inspected the mean recompute transaction/publication path.

### P1 — Preserve failed-open dispatch and cleanup ownership

Construct the post-probe resource guard before rejecting verification. Preserve
structured failure causes, original open error, SQLite/lock release and bounded
provider-only retention. Cache only completed positive checks. Failure, panic,
malformed output, timeout/cancellation and late completion cannot publish a pass.

Cache entries must not retain models, dispatchers, SQLite connections or locks.
The accepted degraded-open clauses still require explicit supersession before
implementation; this review does not approve a behavior change.

Witnesses: engine `src/open.rs:867–878`, `:121–154`;
`src/vector_equivalence.rs:661–665`; accepted 0.8.18 and 0.8.27 runtime ADRs.

### P2 — Cross-construction reuse requires actual asset provenance plumbing

Current code has the identity triple, expected asset hashes, private pooling
and device/model state, a device label, and most store fingerprint inputs.
It lacks a provider descriptor/stability contract, retained provenance bound
to the actual loaded assets, a canonical descriptor schema/version, and the
process cache itself.

`LoadedWeights` carries paths; construction later reads config/tokenizer and
maps weights from those paths. Hashing expected constants or separately
rereading files is not proof of the model actually loaded. Bind provenance to
construction. Include preprocessing/truncation, pooling, normalization, dtype,
implementation identity and effective execution regime. Fixed process build/
library identity may be implicit only under an explicit justified assumption;
avoid gratuitous repeated hashing.

Witnesses: embedder `src/loader.rs:68–77`, `:142–157`;
`src/candle_bge.rs:625–676`, `:702–714`.

### P2 — Bound the cache and define concurrency

Open-handle limits do not bound historical verification combinations. Use a
bounded positive-result cache; eviction is a safe miss and recheck. Hold cache
locks only during lookup/publication, not embedding or database work. Duplicate
concurrent checks are a defensible initial approach; single-flight adds
cancellation and lock-order complexity. Obsolete results cannot satisfy or
overwrite a newer generation.

Witness: MCP `src/rust/crates/fathomdb-mcp/src/registry.rs:722–755`.

## Smallest defensible refinement

Initially support cross-instance reuse **only for the built-in provider**, via
a versioned, construction-bound immutable runtime descriptor. Unknown/custom
providers get fresh checks unless an explicit stability contract is designed.
Keep bounded digest/result metadata after close, not resource-owning objects.
Preserve MCP's normal store eviction. Extending idle timeout alone cannot
address single-store A/B/A eviction.

Safe-reuse invariant: a hit is valid only when a completed successful check used
the same committed store verification state and frozen runtime regime now
being admitted, with no intervening invalidation. Missing evidence is a miss,
never acceptance. Live-session semantics must still be chosen explicitly.

## Minimum implementation-design tests

- Same descriptors reuse across close/reconstruction and A/B/A; process restart
  and capacity eviction force checks.
- Changes to mean/reference/fixture/thresholds, actual assets, Mean/CLS,
  CPU/CUDA and relevant numerical settings invalidate reuse.
- Equal claimed custom identities cannot share a pass; mutable-provider behavior
  has an explicit tested support policy.
- Recompute/recovery, baseline transaction commit/rollback and generations cannot
  publish stale tokens.
- Concurrent verification, failed checks, timeout and late completion cannot
  create a success entry; no cache lock spans inference/database work.
- Failed open releases database ownership for explicit recovery; cached metadata
  does not keep models or dispatch state alive.

## Disposition

Retain this as a design candidate, not an approved cache or implementation-ready
contract. The CPU reopen cost justifies pursuing bounded reuse, not a generic
provider fingerprint framework or a stronger equivalence claim. No new pooling
sweep, runtime change, cache policy, shared model ownership or release placement
is approved by this review. The proposal must resolve the P1 items before
implementation approval is requested.
