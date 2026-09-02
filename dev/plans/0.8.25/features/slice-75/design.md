---
title: 0.8.25 Slice 75 — integrated closure and installed conformance design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
target_release: 0.8.25
depends_on: 70
readiness_gate: 0.8.25 Slice 7 completion
---

# Slice 75 — integrated closure and installed conformance design

## Authority and purpose

This design owns R25/AC25-75, the measurement half of Memex need 21, needs 22
and 24, and the integrated audit of need 23 and A25-01 through A25-07. It audits
evidence produced by Slices 10–70 and measures the assembled data plane. It does
not repair missing feature proof, tune a failed profile, hide nonexecuted
platforms, or mix answer-system results into a FathomDB claim.

The design is REVIEWED_BLOCKED_ON_SLICE_7 and cannot become READY until Slice 7 activates
architecture v2 and every owning feature slice is READY with its own
verification record.

## Requirements-to-design comparison

| Obligation | Existing material | Integrated decision |
| --- | --- | --- |
| Cross-SDK/wire parity | Binding and release designs provide package-level routes | Run one installed versioned conformance corpus across Rust, Python, TypeScript, and wire encodings. |
| Snapshot concurrency | WAL/stress tests exist; Slice 35 adds cross-operation snapshots | Measure cold/steady readers with concurrent writers and require typed snapshot outcomes. |
| New-contract performance | SCALE-02 covers accepted A0 through 50k; new features are unmeasured | Measure evidence, pages, dependencies, readiness, lifecycle, tracing, and graph/profile overhead separately. |
| Lifecycle closure | Earlier slices prove local correctness | Run integrated mutation-to-ready and erase-to-no-orphan flows with restart/fault receipts. |
| Retrieval-only evaluation | EARP exists; GLOBAL-01 mixed system layers | Add a native `Engine.search` witness with no answerer or semantic judge. |
| Installed artifacts | Release design requires post-publish smoke | Install isolated wheel/npm/crates/CLI artifacts and reject source-tree leakage. |

## Predecessor disposition

| Design or evidence | Disposition |
| --- | --- |
| `dev/design/0.8.23-scale-characterization-v2.md` | **Preserve/reuse method.** Retain sealed manifests, five repetitions, cold/steady separation, raw samples, semantic validation, and honest partial states. |
| `dev/design/earp.md` | **Reuse.** Owns versioned data-plane campaign inputs, metrics, blockers, and append-last durable receipts. |
| `dev/design/release.md` | **Preserve/reuse.** Owns registry install isolation and post-publish smoke principles. |
| SCALE-01/SCALE-02 receipts | **Historical baselines.** Do not rewrite them; use their exact A0 operating points and result-equivalence discipline. |
| GLOBAL-01 receipts | **Reclassify/reference.** They remain end-to-end evidence; the new native witness is separate retrieval-only evidence. |
| Slice 10–70 records | **Audit inputs.** Missing or contradictory proof blocks closure and is returned to its owner. |

## Integrated evidence contract

`IntegratedClosureManifestV1` is a strict, versioned, checked-in configuration.
It identifies the candidate commit, release/artifact versions, feature-design
and verification-record digests, registry/profile digests, corpus/gold/config
digests, environment and device requirements, workload cells, repetition
counts, thresholds, and permitted nonexecuted routes. Unknown or missing keys
fail validation.

`IntegratedClosureReceiptV1` records each command/cell, artifact identity,
actual Engine method, snapshot/projection/profile identities, raw sample file
digests, errors/timeouts/skips, latency/resource distributions, lifecycle
witnesses, and derived result. Result states are `passed`, `failed`,
`insufficient_samples`, `missing_prerequisite`, and `environment_invalid`.
Only `passed` supports a release claim. Partial samples are retained and never
pooled into a complete result.

Each receipt contains `measurement_layer`, `engine_search_executed`, shared and
differing components, operation/configuration digest, platform/device identity,
start/end time, child exit, and exact input/output paths. It is an evaluation
artifact, not a product API. It follows A25-05 closed-request and material-
unknown-variant rules. The experiment index is appended only after all
sidecars and content digests validate.

## Workload families

### 1. Pre-publication artifact conformance

Build once **per target/profile** from the same candidate commit, then test
isolated registry-equivalent Rust crate/CLI packages, Python wheels, npm
package/native-package pairs, and their wire fixtures. A sealed per-target build
manifest records candidate SHA, target triple, feature/profile set, toolchain,
command, artifact name/version, size, and SHA-256. Source paths, editable
installs, and a worktree-native module are forbidden. The same fixture
corpus exercises every public Slice 15–70 type, success response, typed error,
unknown request field/variant, additive response field, and material unknown
response variant. Rust/Python/TypeScript semantic results and canonical wire
JSON must agree after documented casing conversion.

Rust proof unpacks `cargo package` `.crate` archives into a clean local registry
or consumes them from a clean external fixture whose dependency resolution is
restricted to that registry; it is not described as `cargo install`. Python
creates a fresh venv, installs one hashed wheel with no dependency/source
fallback, and proves both Python and native module paths are inside it. Node
creates a fresh project, installs the packed JavaScript and matching native
tarballs with scripts/network disabled, and proves resolved package/native
paths. CLI proof executes the packaged binary or installs it from the local
crate registry in a clean cargo home.

Windows x64 CPU/native Rust, Python, and Node evidence is mandatory. Windows
CUDA remains explicitly deferred and cannot be implied by Linux CUDA. Linux
CUDA runs every accepted dense/rerank profile and records `nvidia-smi`, runtime,
model revision, allocation witness, and CPU-equivalence result. Pre-publication
CLI doctor and packed-artifact open/write/search/lifecycle smokes are mandatory.

Actual PyPI/npm/crates.io registry-installed smokes are a separate
**post-publication** gate. They run only after explicit publish authorization
and successful publication, compare registry hashes/versions to the per-target
build manifest where the registry permits, and close the published-release
receipt. They are not a prerequisite for authorizing publication and cannot
retroactively satisfy failed pre-publication conformance.

### 2. Concurrency and snapshot consistency

Use fresh databases at 10,000 and 50,000 canonical records with reader counts
1, 4, and 12 plus one bounded writer. Each cell has five process-cold and five
steady repetitions. Steady repetitions retain at least 1,000 operations per
reader configuration. Writers execute version, dependency, lifecycle, and
projection mutations from a deterministic schedule.

The sealed manifest binds every repeatability input per cell:

- exact corpus/fixture and mutation-trace digests; fresh database creation and
  process restart for every repetition; `process_cold` means the first measured
  operation in a new process after open, not an asserted empty OS page cache;
- reader operation mix by deterministic 100-operation cycle: 40 A0 searches,
  15 three-page ordered walks (`page_size=25`), 15 top-10 evidence resolutions,
  15 constrained graph pages, and 15 reciprocal trace reads; each journey uses
  one snapshot, spans at most three continuations, and has a 2-second lifetime;
- writer trace as a checked-in ordered list of fully resolved semantic-batch,
  dependency, lifecycle, erasure/recreate, and projection-generation operations,
  issued on a 25 ms monotonic schedule; no runner-selected mutation is allowed;
- 100 unreported warm-up operations per reader before 1,000 retained steady
  operations; cold has exactly one retained first operation and no percentile;
- seed `20260901` combined with the cell tuple by the manifest's pinned hash
  function; operation choice, IDs, bodies, and mutation order derive only from
  that seed/trace;
- 5-second per-operation and 120-second per-cell timeouts, no runner retry, and
  exact accounting of success, typed refusal, timeout, invariant failure,
  process exit, and unrun operations;
- shipped SQLite journal/synchronous/cache/mmap/temp-store/reader-pool settings,
  captured from the opened engine; any requested/observed mismatch invalidates
  the environment rather than being tuned during the run;
- an explicit ordered CPU-affinity list: writer on the first allowed physical
  core and readers round-robin over the next distinct allowed physical cores;
  insufficient cores are a missing prerequisite, not oversubscription; and
- monotonic latency timing and 100 ms RSS/CPU/I/O/GPU sampling, with sampler
  version and GPU availability recorded. Sampler overhead is measured in a
  preregistered no-sampling calibration cell.

Every multi-operation read must either reproduce its bound snapshot or return
the exact typed unavailable/drift/expiry outcome. Mixed visibility, duplicate/
omitted cursor rows, stale evidence bytes, searchable erased dependents, or an
untyped busy/timeout is a correctness failure. Report p50/p95/p99 latency,
throughput, writer wait, snapshot-outcome counts, errors/timeouts, RSS, CPU,
GPU when applicable, database/WAL size, and open/close time.

### 3. Feature overhead and lifecycle timing

At 10,000, 17,272, 25,000, 40,000, and 50,000 records, run A0 and the applicable
new operation in paired fresh databases. Report absolute and paired overhead
for snapshot mint/use, eligibility, ordered pages, evidence create/resolve,
dependency write/trace, semantic batch, projection generation/readiness,
constrained expansion, and each accepted profile. Retrieval-bearing pairs must
also prove exact expected identity/order or their preregistered qualified
result—not latency alone.

Compact deterministic lifecycle cells measure semantic-batch commit,
mutation-to-projection-ready, supersede/invalidate propagation, erase-to-fence,
erase-to-no-orphan, restart/resume, integrity scan, governed repair, and full
projection rebuild. Record affected row/dependency counts, generation changes,
storage deltas, and fault phase. Completion with an active/searchable orphan or
wrong generation always fails regardless of speed.

Correctness, SDK/wire parity, snapshot consistency, lifecycle closure, and
receipt integrity are zero-tolerance mandatory gates. Latency/resource cells
are explicitly `policy_gate` or `advisory` in the sealed manifest. A
`policy_gate` must link an already accepted policy and threshold; `advisory`
results can neither block nor support a release claim.

All other thresholds are sealed per operation before execution and must
link to an owning AC or accepted benchmark policy. The integrated runner cannot
invent or relax a threshold after observing data. An advisory measurement may
report without a release pass only when the manifest labels it advisory before
execution.

### 4. Native retrieval-only global witness

Run the frozen GLOBAL-01 held-out input through a named native `Engine.search`
call with A0, a fresh database, and resolved configuration. Record returned
identities, source coverage, duplicates, arm contributions, latency/resources,
and whether gold is sufficient for each requested metric. No map-reduce,
answerer, judge, semantic controller, or generated claim is present. The
receipt classifies every metric as data-plane and records
`engine_search_executed: true`.

The witness establishes execution and descriptive retrieval behavior only.
It does not retroactively convert either GLOBAL-01 path into a retrieval-only
comparison and cannot claim global answer comprehensiveness without suitable
gold.

## Flow, isolation, and failure policy

1. Validate candidate cleanliness, exact commit, Slice 10–70 evidence closure,
   manifest/schema, artifact versions, data roots, device/platform availability,
   and spend/network declarations.
2. Build immutable artifacts once per target/profile from the same commit;
   write the per-target build manifest; consume each registry-equivalent package
   in an isolated environment; prove imports/binaries resolve there.
3. Create a fresh database per repetition and run cells in manifest order.
   Cold and steady samples, platforms, record sizes, and repetitions never pool.
4. Checkpoint raw results atomically after every cell. Respect provider backoff
   only for a preregistered live-model route; malformed exhausted cells become
   failures and execution continues where safe.
5. Validate file closure, digests, sample counts, cross-object equality, and
   status derivation before calculating summaries or appending the index.

Missing platform/device/data/artifact evidence yields `missing_prerequisite`,
not a pass or zero. Candidate drift, dirty inputs, digest disagreement, mixed
snapshots, corrupt raw files, or contradictory SDK outputs yield
`environment_invalid` or `failed` with no passing summary. Failed child
processes retain logs and the remaining matrix is explicitly unrun.

## Invariants and compatibility

- Slice 75 audits; it does not backfill an owner's test, implementation, or
  review record.
- Correctness, visibility, erasure, parity, and receipt integrity have zero
  tolerance and precede performance interpretation.
- A0/default behavior and historical measured values remain unchanged.
- Answer-system metrics remain separately labelled and cannot satisfy a
  data-plane criterion.
- Every public/persisted feature has Rust/Python/TypeScript/wire and applicable
  Windows CPU/native proof; pre-publication registry-equivalent packages, not
  source checkouts, supply authorization evidence. Actual registry installs
  supply the separate post-publication release-close evidence.
- CUDA and ptrace execution are allowed where required, but authorization is
  not evidence that a route ran.

## Mapped RED/GREEN and verification routes

| Acceptance boundary | Required synthetic RED/real GREEN proof |
| --- | --- |
| Manifest/receipt closure | Missing/extra cell, bad digest, pooled repetition, unknown key, partial asserted complete, and status-forgery fixtures reject. Omit each operation mix, cadence/trace, snapshot/page span, warm-up, timeout/retry, seed, SQLite/cache, CPU-affinity, freshness/reset, sampling, or failure-accounting field in turn and require schema rejection. |
| Artifact isolation | Editable/source-tree module, wrong commit/target/profile/hash, version skew, wrong native package, and wire mismatch fail before workload execution; clean Rust/Python/npm/CLI consumers resolve only packed artifacts. |
| SDK/wire parity | Same success/error fixtures across Rust/Python/TS, including unknown-field/variant evolution and Windows CPU/native. |
| Snapshot concurrency | Scheduled writer races prove one view or typed failure at 1/4/12 readers and 10k/50k. |
| Lifecycle closure | Fault/restart, stale projection, source-set removal, erasure fence, orphan injection, and repair authority tests. |
| Performance | Five cold/steady repetitions retain raw distributions and resource/storage data for every declared cell; threshold drift rejects. |
| Evaluation boundary | A mixed-layer receipt and a witness bypassing `Engine.search` reject; the native search-only witness validates. |
| GPU/platform honesty | Missing CUDA/Windows evidence is explicit; CPU/CUDA equivalence and device witnesses are required where selected. |

Run repository fast, heavy, all, all-feature, and operator gates;
pre-publication packed Python/npm/native/CLI/Rust-consumer and wire conformance;
Windows CPU/native Rust/Python/Node; Linux CUDA; strict ptrace stress; lifecycle
fault suites; and every live-model route accepted by Slices 65/70. After
separate publish authorization, run actual registry smokes as the
post-publication close gate. Record N/A and unavailable separately; Windows
CUDA is N/A by release decision.

An independent review may require at most three FIX-n cycles. Unresolved P1/P2
findings, an owning-slice evidence gap, a source-installed artifact, mixed-layer
claim, pooled repetition, missing mandatory platform, or lifecycle correctness
failure blocks READY and release closure.
