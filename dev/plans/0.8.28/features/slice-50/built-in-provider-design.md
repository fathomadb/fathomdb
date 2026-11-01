---
title: Built-in provider — proposed admission, stability and verification reuse contract
status: proposed by Astra-medium; owner review required; no implementation authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider contract

## 1. Recommendation and authority

Support exactly the pinned BGE-small/Mean engine provider, plus explicit
no-provider admission, before 1.x. Close production caller injection; retain
only specifically contained test/evaluation seams. Keep the numerical
compatibility check. Avoid repeated inference on unchanged MCP reopens by
retaining bounded successful numerical evidence independently of model lifetime.

The owner ruled the pre-1.x scope and commissioned this design at steward
`seq-355`. Astra-medium developed the proposal read-only against `14dbbd57a`;
the main thread integrated requirements and records. Detailed mechanisms,
failed-open/recovery policy, dependency work and release placement remain
proposed. This is not permission to implement D50-1/D50-2 or change the ladder.

Companion documents: [needs, requirements and AC](built-in-provider-requirements.md),
[work plan](built-in-provider-plan.md), and the
[proposed open/recovery ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md).
The prior [Astra review](verification-reuse-astra-review.md) found the direction
sound but required these stability/invalidation rules. This revision changes
its metadata-only cache candidate to a bounded numerical-evidence cache.

**Reuse invariant:** a cached pass represents the specified finite test for
an exact committed store verification state and an immutable built-in execution
regime. It is not universal equivalence for every future input, tamper evidence,
proof of historical vector provenance or protection against arbitrary external
SQLite/process-memory modification.

## 2. Supported production surface and extension inventory

Source witnesses below are relative to `src/rust/crates/`, inspected at
`14dbbd57a`. Dispositions are proposed implementation work. No listed source
path has been changed by this design. Future user-provider integration is
eligible for separate design at 1.x or later, not promised for 1.0.

| Path / current behavior | Proposed disposition and containment |
| --- | --- |
| `fathomdb-engine/src/open.rs:1999` `EmbedderChoice::{Default,Caller,CallerWithDeviceResolution,None}`; dispatch `:525` | **Close:** production enum has only `Default`/`None`. Remove both caller variants and misleading recovery advice at `:1647`. Feature unification must not reintroduce them. Distinct hidden test/evaluation openers may inject providers, never mint production reuse evidence. |
| `fathomdb-embedder-api/src/lib.rs:79` public `Embedder` trait | **Retain, deferred integration:** low-level internal/test abstraction, not a supported user-provider engine contract. Annotate trait/crate docs and remove plugin instructions. No generic descriptor/stability API for third-party providers in this change. |
| `fathomdb-embedder/src/candle_bge.rs:539–697` public constructors, `LoadedWeights` paths, `with_pooling` | **Close engine escape paths:** only the internal pinned factory can construct an engine-admissible runtime/evidence descriptor. Public standalone/evaluation construction has no admission capability. Keep standalone CLS where needed; engine Mean cannot be overridden. Replace path-based provenance on the supported path with owned verified bytes. |
| `fathomdb-embedder/src/lib.rs:206`, `nomic.rs:19` Nomic available with ordinary default-provider feature | **Retain deferred:** move implementation/exports behind a named non-default evaluation feature. No production engine route, no production cache capability. Keep its research data. |
| `fathomdb-embedder/src/lib.rs:183–210`, `ort_bge.rs` and `onnx-embedder` feature | **Retain deferred:** offline/evaluation only, consistent with signed ONNX scope. Preserve comparison fixtures; remove engine caller injection and production configuration promises. No silent ONNX substitution. |
| `fathomdb-embedder/src/lib.rs:219` `NoopEmbedder` | **Retain test support:** gate public test export; ordinary features do not expose it as a production option. It is not a recovery provider. |
| `fathomdb-engine/src/open.rs:696` test opener; PyO3/N-API `test_support.rs` | **Retain contained:** explicit non-default test support and separate factory provenance. Shipping package/artifact feature tests prove no test exports. Fake identity equality cannot acquire the built-in production capability. |
| `fathomdb-engine/src/vector_storage.rs:643–645` `write_vector_for_test` | **Retain contained:** already gated by `cfg(any(test, feature = "test-hooks"))` at this HEAD. Preserve this gate and test shipped-feature closure; never describe it as a production vector import API. |
| `fathomdb-engine/src/projection_registry.rs:121` `ProjectionVector.embedder`; stored `vector_embedder`, destructive-change check `:1188` | **Close selection, retain stored metadata:** new caller declarations accept only omitted/null `embedder`; reject every string, including empty string, `default` and model names, before mutation. Keep field for truthful historical read metadata, annotate as reserved/unavailable for selection. Preserve old strings verbatim on read; unsupported historic read→configure is intentionally refused, not silently normalized. No identity strings move into vector configuration. |
| Rust SDK `standalone.rs:88,138`; PyO3 `embedding.rs:179,190`; N-API `embedding.rs:19,30` standalone `embed_batch_cls` | **Retain explicitly separate helper:** CLS output is not an engine-provider selector or a compatible producer of engine-managed Mean vectors. No Mean cache sharing. Update documentation that could imply interchangeability. |
| `fathomdb-embedder/src/loader.rs:316,373,564` configurable loader hooks; `tc5-benchmark` local-asset constructors | **Retain contained:** existing non-default loader-test/benchmark features serve tests/evaluation only. They cannot forge engine-admissible built-in provenance even when Cargo unifies features. Keep explicit device attestation and pinned benchmark asset rules. |
| SDK Rust `engine.rs:56`, PyO3/N-API engine open, MCP SDK open | **Retain:** default/none already fits. Never retry a failed provider-enabled open without a provider automatically. MCP ownership, idle close and max-open-store behavior remain intact. |
| CPU/CUDA policy and separately compiled device capabilities; reranker provider/runtime | **Retain supported policy:** device choice is not user embedder selection. No new backend is authorized here. Reranker remains a separate function; shared Candle settings must not affect immutable BGE execution. Other enabled execution backends require equivalent stability qualification before reuse. |
| `_fathomdb_embedder_profiles`, `_fathomdb_vector_kinds`, `_fathomdb_embed_probe`, profile/mean helpers keyed to names | **Retain data model:** inspect well-formed historic profiles without provider substitution. Built-in-enabled open requires matching supported identity/dimension and references. Do not relabel/delete old custom identities or assume a profile row proves every historic writer. Replace name-only capability assumptions with the internal supported-runtime record where runtime behavior is required. |

For every retained seam, add an annotation citing this contract, its test/eval
purpose, exact gating feature and unsupported engine-integration status. Keep
this table as the disposition record; source comments link here rather than
inventing independent roadmaps. Surface tests must exercise the shipping feature
sets and feature-unified builds: documentation hiding is not access control.

The implementation must update `dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md`,
`dev/design/{embedder,bindings}.md`, crate/feature docs and misleading examples.
Upon owner approval, the successor ADR narrows the public-provider promise in
`ADR-0.6.0-embedder-protocol.md` while retaining dispatch invariants; it records
the offline/evaluation-only ONNX boundary. It retains
`ADR-0.6.0-vector-identity-embedder-owned.md`: configuration is not identity.
No backward-compatibility shim is proposed for removed pre-1.0 admission APIs.

## 3. Construction and runtime stability

### Immutable built-in execution

The installed Candle fork consults mutable process-global
`gemm_reduced_precision_f32()` when selecting CUDA GEMM computation
(`candle-core-fathomdb-0.11.0/src/cuda_backend/mod.rs:2482,2488,2526`).
Resetting it at startup, checking before/after inference, or locking only
FathomDB callers cannot freeze it against concurrent outside calls.

**Propose separately qualified fork work:** an immutable per-runtime strict-F32
execution policy, propagated through device clones/tensors and every built-in
BGE matmul, including batched paths. The supported runtime must not consult the
legacy global flag. Leave unrelated Candle consumers' legacy behavior intact.
Audit the actual execution path for other ambient numerical controls; remove
their influence or fix them in the same immutable regime. A list of sampled
environment values is not a stability mechanism. No fork mutation, publication,
new dependency pin or expanded hardware qualification is authorized by drafting
this proposal. Follow the existing fork-first qualification process after
approval; no driver/toolkit changes are needed by this design.

### Construction-bound descriptor

A sealed internal record, created only by the supported factory, contains:

- Versioned descriptor recipe and actual consumed config/tokenizer/weight hashes.
- Fixed Mean pooling; tokenizer normalization and special-token/truncation
  behavior (512-token window); unit normalization and F32 numerical policy.
- Effective backend and execution regime, including effective CUDA device
  identity/capability and resolved runtime provenance where applicable.
- Implementation identity within the cache's loaded build/library domain.
  Fixed process implementation may be implicit in that domain; no executable
  rehash is required. `auto` is a request, not an effective backend identity.

Do not key ordinary reconstruction reuse on provider addresses, context allocation
IDs or caller-supplied device reports. Context loss is handled by invalidation.
The factory hashes and constructs from the **same immutable owned bytes/backing**
and checks the pinned expected hashes. Existing `LoadedWeights` paths followed
by later reads/mmap (`loader.rs:148`; `candle_bge.rs:625–676`) do not meet this
contract. Parsing, tokenization and tensor construction must use the owned
snapshot; mutable file-backed mappings cannot remain runtime authority.
Do not retain loader mappings/files in the evidence cache.

No new durable per-store full-runtime schema is needed for this process-local
optimization. Existing identity/profile/references remain authoritative data;
the runtime descriptor supplements rather than rewrites their identity. Future
persistent provenance/rotation is separate work. Re-measure complete construction
cost and transient memory introduced by owned snapshots; earlier timing did not
measure this proposed constructor.

## 4. Verification cadence and bounded evidence cache

### When checks happen

A relevant open has the provider enabled and either a registered/durable dense
arm or an existing probe baseline. An existing baseline still requires checking
when all kinds were removed, so later reenrollment cannot bypass its obligations.
Validate reference structure and current store state under admitted-store
ownership, with the compatibility-conditioned mean recovery ordering in §5.
It computes:

- Store verification digest over profile identity/dimension, reference rows and
  texts, live mean/centering state, compiled fixture, thresholds and recipe.
- Internal runtime descriptor digest and store scope. Hash the canonical absolute
  path, never an MCP alias; read current persisted comparison inputs under store
  ownership on every open. Different paths miss; a replacement at the same path
  with changed comparison inputs misses. Exact comparison-state replacement may
  reuse the same finite numerical verdict; this is not historical provenance or
  file-incarnation proof. No new schema UUID/file-identity mechanism is needed.

An exact positive hit can reuse the completed finite verdict and its evidence,
after confirming the current runtime/context is still valid.
A miss performs the full live check; changed mean is a verdict miss even when
an already-admitted engine can validate that proposed mean from its retained
numerical evidence. Changes of baseline, fixture, thresholds or recipe cannot
inherit an old verdict. No on-disk success marker alone authorizes acceptance;
retire TC-68's persisted-marker bypass, without requiring a schema migration or
removing historic data by a migration.

No-provider opens and truly vector-less opens (no registered/durable dense arm
and no probe baseline) perform no probes. Existing references
are still structurally validated where applicable; absence of a currently
registered arm is not permission to ignore known corrupt verification state.
Ordinary calls on an accepted engine do not perform 45 probes or hash model
assets. Later provider failures remain normal classified operation failures.

### Exact cache bounds and ownership

Use a process-local cache in one loaded engine-library domain. Independent
library copies may miss independently; no cross-library sharing is promised.
Maximum **16 positive entries** and **2 MiB accounted payload**, evicting least
recently used entries until both constraints hold. Account fixed keys, evidence
and entry data; separately bound container overhead by the 16-entry limit.
No dynamically retained path strings, negative entries, refresh workers or
unbounded tombstone/history maps.

Each entry retains 45 uncentered live probe vectors from the successful check:
`45 × 384 × 4 = 69,120 bytes`. Read reference vectors from the database when
needed; do not duplicate them in global entries. If implementation adds payload,
it must account for it rather than assuming all 16 entries fit. A hit must carry
the immutable evidence belonging to its exact runtime/reference/fixture domain.

Each active baselined engine keeps one immutable evidence reference, even if its
LRU entry is evicted: **69,120 bytes per active engine plus fixed metadata**.
This state is separately bounded per active engine and released on close; active
engine count remains the host's existing resource policy. Cache bookkeeping
must not retain engine/provider/dispatcher/device/SQLite/lock owners. Cached
evidence can outlive a model, not keep the model alive. First-enrollment sessions
hold no claimed verified evidence until they actually obtain it.

### Concurrency, publication and invalidation

Cache mutex covers only lookup, LRU bookkeeping, epoch update and publication.
No inference, SQLite operation, callback or cleanup runs while it is held.
Duplicate concurrent verification is acceptable; no single-flight initially.
Capture a cache-wide monotonic invalidation epoch before fresh checking. Publish
only after accepted comparison and matching baseline/arm durable commitment,
only if the epoch and relevant engine/store generation still match. Checked
generation overflow disables reuse rather than wrapping into an old validity.

A failed fresh diagnostic or runtime-invalidating event increments the epoch
and clears all positive entries. Whole-cache invalidation is conservative and
bounded. It fences already-running publication; a later independent fresh check
can pass normally. Diagnostic admission uses the same store ownership, so it
cannot conflict with a live engine of that store. It changes no database data.
No failure, panic, timeout, malformed output, cancellation or late worker result
can publish positive acceptance. A successful diagnostic may publish only a
coherent completed verdict; absent baseline cannot become acceptance.

Context/device loss additionally invalidates the **affected live runtime**, not
just global cache entries. Its context identity plus shared-by-clones health and
generation are engine-owned. Validate that context at admission/cache hit,
provider execution boundaries and before accepting numerical evidence for a
state transition. A lost/replaced context rejects the next affected operation
regardless of which engine first detected it; idle engines need no eager
broadcast. Stop admitting work on the invalid runtime, discard obsolete results,
report the typed cause, and clear process positives through the epoch. No global
provider-retaining device→engine registry. Reconstruction is allowed only if
existing context policy permits it; otherwise require process restart. Never
silently detach/replace the provider or clear durable work. Recoverable ordinary
allocation errors do not alone prove numerical drift. General context-boundary
validation is net-new; measure its overhead and preserve the existing CUDA pool
policy's process-poisoning rules.

## 5. Store transitions and live mean rules

| Event | Required behavior |
| --- | --- |
| Relevant open with existing baseline | Exact hit or fresh check against current committed state, followed by compatibility-conditioned candidate recovery before dense admission. |
| Missing baseline at relevant open | Collect/stage complete baseline, confirm it, then publish baseline/arm and positive evidence only after acceptance and durable commitment. No partial/rejected baseline. |
| Vector-less open, then first enrollment with no baseline | Session is **unbaselined, built in this session**. Fixed built-in runtime may produce vectors and initial mean; no cached/verified claim. Preserve async canonical writes and the accepted next-relevant-open baseline residual. |
| Existing baseline when new kinds are enrolled | Same supported runtime and current accepted mean/reference domain; no new model choice. Enrollment transaction cannot publish a prospective token before commit. |
| Automatic pin in a baselined session | Compare retained live outputs to stored references under the candidate mean with unchanged P1/P2 rules. Accept and commit it, or reject only the candidate mean and continue projecting under the previously accepted mean/uncentered state. No model inference under transaction/commit locks. |
| Explicit mean recompute in a baselined provider session | Same candidate-mean validation, atomic update and requantization. On failure rollback, return typed compatibility failure and retain old valid state. |
| Baseline-bearing ordinary no-provider open | Defer automatic mean recovery because it cannot validate a candidate. Do not install a previously rejected mean through this route. Existing current state remains inspectable if integrity permits. |
| No-provider mean recompute | Approved metadata maintenance may proceed atomically; no numeric pass. Clear process acceptance and require checking current state at next provider-enabled relevant open. |
| Open-time automatic mean recovery with existing baseline | Verify the current committed state first, then condition the proposed recovery mean on the same prospective comparison. A rejected candidate cannot be committed by reopening. Invalid current state still fails open. Other startup effects retain their own contracts; no whole-open rollback promise. |
| Reference replacement/backend rotation | No ordinary API for blessing a new function. Restore compatible runtime/backup or reconstruct a separate store; full rotation requires a separate approved design. |

Successful candidate-mean comparison reuses the live numerical outputs observed
under the immutable runtime; it does not make new live-inference observations.
P1 recomputes sign bits with the proposed mean; P2 remains the uncentered L2
comparison. Publish new engine store digest/evidence only after commit, then
optionally publish global reuse. Rollback retains old mean and verdict. Existing
atomic foundations are `projection_commit.rs:325–358`, `mean.rs:414–456`,
and open recovery at `open.rs:1284–1294`.

**Automatic mean pin becomes compatibility-conditioned.** This explicitly
supersedes the unconditional automatic threshold-pin rule on rejection only;
normal successful pin behavior and threshold stay unchanged. A rejected optional
candidate does not mean the current accepted embedding/mean state failed.
Do not turn it into a terminal projection failure or stop otherwise valid work:
that would recreate the measured impossible-progress drain/mutation state.
Continue committing projections under the old accepted mean (including None),
without marking the rejected candidate accepted or requantizing under it.

Latch one bounded `MeanPinRejected` advisory per active baselined engine accepted
state token (runtime/reference domain plus current mean). Expose it as a typed
event and queryable current-engine status with candidate rejection detail and
actual mean-pinned state. Suppress further automatic candidates under that token,
even as corpus size/candidate means change. Explicit recompute can retry on
operator request; successful state renewal clears the advisory. Close/reopen is
a new admission attempt and may try once after validating current state. No
silent timer retry, unbounded candidate history or new durable latch/schema.
Document that an unpinned mean can remain unpinned, with its measured centering
benefit unavailable until an admissible update. Drain/readiness follow actual
projection completion under the accepted state; they do not claim the rejected
mean was installed. Canonical data and non-embedding operations remain usable.

This requires changing open-time recovery ordering for baseline-bearing stores:
current-state verification precedes any optional mean recovery commit. Otherwise
a subsequent open could commit the rejected candidate before checking it.
No-provider deliberate recompute remains explicit maintenance with no verdict;
its committed mean must earn admission at the next provider-enabled open.

The unbaselined state preserves an explicit accepted residual: the runtime that
establishes the next baseline does not retroactively prove which runtime built
earlier rows. Do not promise otherwise or inject synchronous probes into the
first canonical write. If a baseline already exists, dropping the last kind and
re-enrolling cannot discard its verification obligations. Frozen/read paths
must retain coherent mean/vector snapshots across accepted mean transactions;
reuse the existing commit/snapshot ordering and test concurrent readers.

## 6. Open failure and deliberate recovery

Retain the proposed successor ADR's typed mismatch / cannot-verify / invalid-state
classification and failed-open unwind. Establish cleanup ownership before an
error return; release SQLite owners and store lock, close dispatch admission,
and retain only provider/request/accounting resources within the existing
bounded drain contract. No late publication and no automatic mode switch.

Explicit no-provider access uses stored profile metadata, including well-formed
unsupported historic profiles, without inventing a BGE provider. It remains
ordinary mutable access subject to integrity checks. Text/canonical/lifecycle/
erasure work and truthful pending dense state remain available. Known corruption
still refuses admission. An unsupported historic identity never gets BGE dense
execution merely because metadata inspection succeeds.

Dense rebuild requires a supported, compatible provider before dense invalidation,
even with zero backlog; pure-text repair stays provider-free. Add deliberate
built-in CLI selection and truthful completion, including CPU feature forwarding.
CLI numeric diagnosis must run without successful provider-enabled admission,
force fresh inference, preserve database/reference state and respect WAL/store
ownership. Unsupported historic providers receive structural inspection plus
unsupported-provider detail, not an invented numeric verdict. Failed fresh
diagnosis revokes in-process reuse through the epoch mechanism above.

These are proposed public behavior changes, not already-approved safety-policy
supersession. No bypass flag, automatic rebaseline or in-place provider rotation.

## 7. Existing foundation, implementation sequence and verification

| Exists today | Net-new after contract approval |
| --- | --- |
| Pinned default loader, Mean constructor, identity/profile gates | Sealed engine-admissible factory, owned consumed asset snapshot and actual-runtime descriptor; close injection/selection gaps. |
| Finite P1/P2 check, references, store admission lock, TC-68 marker | Retire marker authority; bounded numerical evidence and commit/epoch-fenced positive reuse. |
| Mean pin/recompute atomic requantization | Prospective-mean numerical validation, engine evidence publication, compatibility-conditioned automatic recovery/pin and bounded rejection advisory. |
| Engine-owned dispatch, deadlines, late-result discard, unwind guards | Stable per-runtime Candle policy, live runtime invalidation and typed failed-open conversion through guards. |
| Explicit no-provider selectors, structural doctor and operator rebuilds | Correct historic-profile no-provider admission, live diagnosis and supported-provider dense rebuild orchestration. |

Proposed dependency order: surface closure → independent fork stability
qualification → construction provenance → store-state/evidence transitions →
bounded cache/concurrency → failed-open/recovery/binding integration. Each
behavior gets a failing RED before GREEN; intentional old-contract test
supersession is identified explicitly. Do not write a passing coverage commit
and call it RED. Keep test/eval fake providers distinct from real-provider proof.

The [acceptance matrix](built-in-provider-requirements.md#proposed-acceptance-criteria)
covers release-feature closure, actual bytes, concurrent precision toggles,
45-inference first relevant open versus unchanged reconstruction, MCP idle and
A/B/A, restart/eviction, exact bounds/resource release, mean transitions,
rollback, failed diagnostics, context loss and late publication. Property tests
cover digest encoding and state/commit/eviction interleavings; real SQLite and
built-in CPU/CUDA tests supply integration evidence. Other enabled backends
require equivalent execution-policy qualification before reuse.

Re-run complete factory/open and warm reconstruction measurements after the new
constructor exists, including snapshot transient memory, cache memory, mean
check time and MCP reopen scenarios. Keep all tools/data committed. The prior
0.16–0.18 s GPU / 2.37–2.56 s CPU added verification medians motivate reuse;
they do not predict this constructor's total latency or production reopen rate.
No new pooling sweep is needed: Mean is settled.

Scoped validators during design/code work; independent code review and full
strict `agent-verify.sh` on the final code tree after checkout-owned tooling
preflight. This document makes no new behavioral pass claim. Owner review of
this contract and release placement is the next gate before implementation.
