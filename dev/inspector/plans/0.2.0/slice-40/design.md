---
title: Inspector 0.2.0 Slice 40 — vector design
date: 2026-11-20
status: ACTIVE
desc: Exact stored vector observations and confined production Default serving
---

# Slice 40 — vector design

[Plan](plan.md) owns scope/delta; [HLD](../../../design/high-level-design.md) owns
architecture; [status](status.md) owns executed qualification. Engine format37,
APIs/pins and accepted production Default/None policy remain unchanged.

## Existing and new work

Reuse20 exact class/cursor/revision handles, transaction-local queued-erasure
classification, bounded observations and owned worker lifecycle. Reuse30 native
lexical composition and complete serialized2MiB response bound. New Inspector
`vector.rs` reads stored vectors/head spans and L2; `model.rs` supplies sealed
assets/confinement; worker/process extend typed serving and owned sink delivery;
DOM vector workspace uses existing observation/Back/refresh ownership.

Stored path reads `_fathomdb_embedder_profiles`, `_fathomdb_vector_kinds`,
`_fathomdb_vector_rows`, `_fathomdb_vector_spans` and physical `vector_default`.
Row/profile/source cursor/revision are separate facts; row ID is not logical ID.
Only supported window0/start0 is qualified. Recorded byte end/body length are
preserved, unknown end yields no inferred window text. No extra rows/windows.
Stored producer facts stay separate from current session/runtime facts.

Find similar requires an exact permitted stored seed. Within one read transaction,
scan mapping rowids after the requested key, point-read one compatible vector,
check exact origins/policy and compute unchanged sqlite-vec f32 L2. Validate
finite components, dimensions and finite distance output (f32 may overflow).
Bounded top-k heap uses distance then rowid; charge mapping, representation and
source work, maximum100k shared work and25s local work within30s request budget.
Return examined/visible/compared/returned counts, keyset window and continuation.
Nearest means nearest in this compared policy-visible population; stored inactive
history may participate under D01 and does not imply Engine eligibility.

## Supported cached CPU Default route

The production loader accepts pin-matching symlinks but reopens paths after
hashing; direct mutable-cache admission is unsafe. After closed-environment
preflight, require existing pinned profile, complete45 Engine probe references
and conservative materialization readiness. Stream existing assets into bounded
memfds; add and confirm WRITE/GROW/SHRINK/SEAL, then hash sealed bytes. A private
cache-shaped symlink view points only to retained `/proc/self/fd/N` descriptors.
A source replacement/race yields pinned immutable bytes or deliberate None,
never unverified model bytes. Descriptors outlive Engine/model teardown.

Before Engine threads, require LandlockABI3+ with mutation/refer/truncate rights;
write grants cover only canonical admitted DB parent and owned work directory,
with no ancestor grant covering private model storage. Architecture-checked
seccomp denies socket/socketpair/connect and x32 bypasses; only owned inherited
IPC survives exec. No ambient hints/credentials/proxies/GPU overrides enter.

Each new SQLite connection receives a write-denying authorizer for vector storage
and shadow tables, mapping/span tables, mean/profile state and probe embeddings,
including protected drops/alters. Exact SQLITE_INSERT on `_fathomdb_vector_kinds`
is the sole reviewed exception: Engine always prepares metadata-only INSERT OR
IGNORE, even when unchanged. Kinds update/delete/drop/alter stay denied. This
prevents persisted new embeddings despite readiness/admission races; it does not
claim zero Engine verification computation or zero permitted metadata effects.

Use one Engine admission lifecycle. Actual Default compatibility/open failure is
failure and never retries None. Missing assets/unqualified stored prerequisites
or unsupported confinement can select None before Default. Optional CE/GPU/test/
evaluation features are absent. No model download, ambient cache repair, chooser,
Caller provider, copied ranker or new Engine capability is introduced.

## Native serving and observations

Use public frozen explained hybrid search with actual default eligibility,
limit, no graph/CE and CPU provider. Preserve native typed ID, write cursor,
original payload window, representative branch, zero-based per-arm ranks,
fused/blended/CE values, structural inclusion, degradations and correlation.
Dense-positive qualification requires vector_hits>0 plus returned vector_rank;
representative branch alone is insufficient. Binary shortlist/conditional mean
centering and f32 L2/fusion describe native stages, not exposed per-hit distances.
Unexposed counts/distances/errors remain unknown; lexical-only output is never
labeled dense success. Frozen query context and later policy/origin context stay
separate.60 may pin these actual observations without implicit re-execution.

All values/window text/origin labels/copy/evidence pass fresh D01 checks. Source
queue caches die with each read transaction. Raw global mean values are not
exposed through a record whose aggregate disclosure scope is unestablished.

## Export, resources and interaction

Worker authorizes exact retained content/destination first. Supervisor opens the
verified parent directory, checks device/inode and uses openat with exclusive,
no-follow0600 creation. One regular writable sink FD is sent over owned IPC,
bound to one-use nonce/handle/destination. Unexpected/truncated descriptors close.
Worker repeats current policy checks while existing bounded streaming writes.
Canonical admitted cache roots and current cache hints, private resources and DB
files remain forbidden destinations; no broad directory grant is added.

Six-key env_clear policy is fixed before every session exec: owned work TMPDIR,
LANG/LC_ALL C.UTF-8, forced CPU, RAYON1 and private-view XDG_CACHE_HOME. Parent
resolves only an absolute cache candidate string; SQLite/assets are examined in
the child. A None decision before asset probing touches no source cache; a failed
bounded asset probe necessarily reads it. Canonical admitted DB paths drive
readiness, grants and attachment. Private work/view RAII cleans after child reap.

Native numeric bars have equivalent labeled values and keyboard controls;
serving scores/ranks have a separate list. Exact origin/Back restores permitted
original observations and focus. Shared navigation tokens reject late responses
before effect; eviction/current policy clears stale content. No tutorial, 2D/3D
geometry, extra provider helper or general export is introduced.

Fresh direct Sol-high design review approved the corrected composition. Fresh
code review found DB/cache alias mismatches; focused RED/GREEN and independent
recheck resolved both. Independent f64/sign-bit/bounds/identity facts, actual
Chromium, asset races, native process controls and Terra assess implementation.

Primary sources: [Landlock](https://docs.kernel.org/userspace-api/landlock.html),
[memfd seals](https://man7.org/linux/man-pages/man2/memfd_create.2.html),
[seccomp](https://man7.org/linux/man-pages/man2/seccomp.2.html),
[SQLite authorizers](https://www.sqlite.org/c3ref/set_authorizer.html),
[auto extensions](https://www.sqlite.org/c3ref/auto_extension.html).

## Build composition

The shipped package explicitly enables Inspector `cached-cpu`, forwarding only
Engine `default-embedder`. Bare Cargo development/workspace builds stay model-free;
they gate before readiness/cache probing even if another package unifies Engine
features. The build description hashes/reports the Inspector feature. Feature-off
no-cache admission, original SDK feature-off behavior and production cached dense
positives are separate required checks. Direct Sol-high review approved this
composition after the actual workspace SDK feature/cfg mismatch was observed.
