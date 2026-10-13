# Competitive testing requirements and acceptance criteria

**Status:** Proposed experiment-infrastructure contract, 2026-10-13. These
`BT-*` identifiers belong to this document; they do not amend the locked
product acceptance criteria in `dev/acceptance.md`. Execution under the
[Program](PROGRAM.md) still needs a registered track and its
[Track Runner](TRACK-RUNNER.md) control. This document authorizes no corpus
acquisition, model/GPU spend, or competitor run.

## Purpose and scope

Establish three repeatable, receipt-backed scoreboards for FathomDB 0.8.27 and
later: **latency and capacity**, **retrieval quality**, and **answer quality**.
Two comparison axes consume those scoreboards:

1. **Product comparison:** FathomDB against embedded-scale databases with
   capabilities beyond SQL. Published competitor measurements on sufficiently
   described hardware are the first, context-only reference; a controlled
   local competitor run is secondary; reproducing a vendor benchmark is
   tertiary. A published row is never a paired performance win by itself.
2. **SDK comparison:** Python, TypeScript, and the dedicated `fathomdb_sdk`
   Rust crate invoke equivalent FathomDB engine source, features, and resolved
   shared dependencies through their public APIs.
   This measures SDK path cost and output parity, not a different database
   product. Each shared SDK operation is registered even if a given campaign
   does not benchmark it.

The first executable SDK slice is explicit full-text search. Hybrid, filtered,
projected, graph, evidence, read, lifecycle, and model/provider operations
enter as fixture and oracle contracts become ready. There is no claimed shared
vector-only query method in 0.8.27. A benchmark case is comparable only under
its stated workload, correctness, hardware, and timing conditions.

## Requirements

### Common experiment and evidence

- **BT-R01 — Definition as data.** One versioned experiment file defines the
  workload, comparator or SDK arms, immutable input/fixture digests, method and
  effective options, setup/readiness, hardware and device policy, sampling,
  metrics, and output schema. The command line selects only the file, a fresh
  external output root, and validate/run. No bespoke case command or hidden
  environment default may change a measured treatment.
- **BT-R02 — Typed, durable evidence.** Preflight rejects unknown or unused
  fields, version/digest drift, unsupported capabilities, and missing required
  inputs. One suite invocation expands into **cell attempts**, each identified
  by `(suite attempt, case, arm)` and aggregating its declared repetitions.
  Every cell attempt, including a timeout or blocked one, writes one safe
  `experiments.record.v1` receipt and one append-only
  `experiments.index-row.v1` row. An external suite manifest binds the
  configuration hash to its complete cell roster, run IDs, statuses, and
  result digests; it has no separate receipt/index row. External execution artifacts retain the full
  resolved configuration, driver/result hashes, samples, and logs. The safe
  receipt retains enough hashes and settings to verify the run without copying
  raw data or private paths.
- **BT-R03 — Honest comparison labels.** A row carries `published_context`,
  `local_descriptive`, or `paired_eligible` evidence status, plus a typed
  complete/blocked result status. A validator marks unreadable or mismatched
  evidence `invalid` in the renderer; it is not a valid result payload.
  A published reference retains its
  source URL, retrieval date, quoted metric definition, system version,
  workload and hardware description, and source snapshot digest. Missing
  details are recorded as unknown and keep it context-only. New published rows
  carry this metadata in a versioned source-evidence sidecar or successor
  schema; historical `competitive.result.v1` records stay valid. Local pairing
  requires matching inputs, workload semantics, and a predeclared eligibility
  rule; no renderer infers a winner from adjacent rows.
- **BT-R04 — Independent axes.** Product rows name deployment, engine/product
  version, comparator, and method. SDK rows name language, SDK/package/native
  build, driver, runtime, and the common FathomDB engine source revision. A result
  cannot be both a product comparator and an SDK-overhead arm. The renderer
  may show them in the same metric family but must group and label the axes
  separately.
- **BT-R05 — Integrity and safe storage.** Corpus, queries, gold, prompts,
  model artifacts, binaries, drivers, and resolved settings are pinned before
  execution and checked again during/after it. Databases, corpus content,
  queries, predictions, per-query traces, and secrets remain outside the repo.
  The safe receipt config is a deterministic content-free projection of the
  external execution config; their separate hashes and the result hash are
  linked by a validated execution witness. The current product runner commits
  absolute adapter/input/result paths in its witness and settings; it does
  **not** yet meet this rule. New path-free product receipts require a
  successor product result/execution schema and projection. Historical v1
  receipts remain readable but are flagged as legacy path-bearing evidence.
- **BT-R06 — Comparable cells.** Predeclare shared invariants: case/task,
  input/gold/question IDs, retrieval unit, top-K and eligibility semantics,
  cache and process state, concurrency, timing boundary, hardware rule, and
  metric definition. For **SDK** comparisons also require the same engine
  source revision and effective engine config/features, resolved shared engine
  and embedder dependency closure, semantic operation/version/options,
  model/device, and fixture digests. Pin the effective validity instant or
  read context for temporal-capable operations; a wall-clock-derived default
  is ineligible for a paired cell. Each language's native binary has its own
  recorded digest. SDK language/build/runtime is the
  treatment. For **product**
  comparisons record each product's own engine, method, options, and model as
  treatment identity rather than requiring equality. Require equivalent work
  and input access under a predeclared hardware/timing rule; a materially
  different task stays context-only. Unknown or mismatched shared invariants
  refuse pairing. No missing, failed, or unsupported cell is a zero.
- **BT-R07 — Repetition and uncertainty.** A smoke or one-run result is
  descriptive. A comparative latency/capacity or quality claim needs a
  registered sample unit, independent repetitions, deterministic ordering or
  balanced arm order, exclusions, summary method, uncertainty interval, and
  decision rule fixed before looking at results. Do not pool cold with steady,
  CPU with GPU, or unlike corpus-size tiers.

### Three test families

- **BT-R08 — Latency and capacity.** Distinguish ingest acknowledgement,
  ready-to-search, open/startup, query, answerer/judge, and storage/resource
  costs. Capture each SDK call from public method invocation through
  materialized result using its language-native monotonic clock; for TS await
  completion. Record counts, errors, sample distribution and units, p50/p95/
  p99 only where sample size supports them, throughput at declared concurrency,
  peak memory, storage, and corpus-size tier. Capacity is a scale curve or
  bounded envelope, not a single large-run QPS number. Setup and download time
  are separate unless the case explicitly measures them.
- **BT-R09 — Retrieval quality.** Same corpus and question IDs, gold/qrels,
  eligibility view and effective validity instant, retrieval unit, top-K, and
  metric definitions are required
  for a paired retrieval claim. Record ordered result IDs and canonical-source
  mapping externally, plus safe aggregate recall@K, MRR/nDCG where meaningful,
  negatives/abstention, and class-specific or all-support coverage where gold
  supports them. SDK parity checks ID/order and relevant fields before timing
  claims; numeric tolerances are declared per operation. A change in retrieval
  unit or gold denominator creates a different case.
- **BT-R10 — Answer quality.** A same-suite answer case declares its upstream
  retrieval case and SDK arm dependency in immutable configuration. The
  coordinator binds that attempt's completed retrieval-result and evidence
  digests in the answer execution witness and result before scoring. A
  separately imported retrieval artifact instead requires its digest pinned
  up front and is labeled as such. The answer test consumes the bound
  evidence, then uses the **same** answerer, prompt, context budget, decoding,
  evaluator/judge, gold, and question order across eligible arms. It reports
  answer correctness/F1 and groundedness, attribution, temporal or multi-hop
  slices only where the gold supports them, plus cost and answer/judge time.
  Retrieval, answer generation, and judging remain separate timed stages.
  SDK drivers export equivalent evidence; they do not implement a new answer
  method or choose different answerers. A retrieval-only gold set cannot
  produce an answer claim.

### SDK drivers and change over time

- **BT-R11 — Common surface registry.** Seed the registry from the 0.8.27
  [governed parity map](../../src/conformance/governed-operation-parity.json)
  and the documented common non-command members. Every shared public
  operation has a semantic ID/version, native binding, input/options schema,
  timing boundary, output normalizer, oracle, setup/readiness rule, and an
  explicit `measure`, `setup`, `observe`, or `excluded_with_reason` disposition.
  The suite chooses a subset; the inventory does not silently omit new SDK
  features.
- **BT-R12 — Capability and version negotiation.** Each pinned Python, TS,
  and Rust driver reports protocol, SDK/runtime/build identity, supported
  operation and option versions, and required features. A required mismatch
  blocks before measurement. Optional unsupported cases get a typed blocked
  result without metrics. No driver substitutes another method, drops a
  filter, downgrades a model/device, or uses the lower-level Rust facade.
  Additive operations require registry/binding/conformance changes, not a
  coordinator branch; changed semantics require a new operation version.
- **BT-R13 — State and process isolation.** Every FathomDB measurement cell
  uses a fresh database made by `prepare_test_database` and records doctor/
  device evidence. Pin its bootstrap engine revision to the measured arm.
  Each independent repetition uses a fresh driver process. Cold calls have no
  rehearsal or warmup on measured state; process-singleton model costs require
  a fresh process per sample. A mutating sample starts from reconstructed
  initial state. Correctness is checked after timing or on a separate
  equivalent state. Timeouts terminate and reap the driver and providers.
- **BT-R14 — One contract, separate payloads.** The existing
  `competitive.experiment.v1`/`competitive.result.v1` product runner remains
  readable as historical evidence. Its path-bearing v1 receipts are not
  eligible for new safe-receipt claims. New product executions need a
  successor path-free payload/witness; SDK runs use the separate proposed `sdk.experiment.v1`/
  `sdk.result.v1` and fixed `sdk.driver.v1` protocol from the
  [driver design](../design/sdk-performance-driver.md). Both nest under the
  unchanged experiment record/index envelope. A derived scoreboard validates
  and discriminates these payloads; it never treats an SDK row as a competitor
  or rewrites historical receipts.

## Acceptance criteria for the mechanism

These criteria test the *harness*. They do not assert that FathomDB wins a
comparison. Real scoreboards require the evidence eligibility below.

| ID | Acceptance evidence |
| --- | --- |
| **BT-AC01** | The same immutable suite config runs twice into different new external roots, including twice in one UTC minute. Each invocation has a distinct suite-attempt ID; each `(case, arm)` cell has a distinct run ID, the same safe treatment/config identity across invocations, and exactly one receipt/index row per cell. Both external suite manifests list the expected complete cell roster and independently hashed results. No receipt is overwritten or index row suppressed. Changing an input or binary digest before/during execution is rejected or recorded blocked, never complete. |
| **BT-AC02** | Schema tests reject unknown/missing fields, invalid metric families, unsupported operation/options, and any silently ignored setting. A required driver capability mismatch blocks all affected cells before timing; an optional mismatch is visible with no metric. |
| **BT-AC03** | Both new SDK and successor product receipt paths use secret-looking paths/content in external fixtures and prove none appears in committed `record.json`, `config.resolved.yaml`, index, or scoreboard. Recomputing safe and execution hashes plus the result/witness link passes; tampering any link fails validation. A historical path-bearing v1 product receipt is still readable but is labeled legacy, never presented as a pass of this gate. |
| **BT-AC04** | The registry check covers all 44 live governed 0.8.27 operations and documented common non-command members with an explicit disposition. Adding a new shared operation to the parity fixture fails until it has a descriptor, three bindings/capability declarations, and a conformance fixture. Old configs still resolve to the old semantic version. |
| **BT-AC05** | The first real-database fixture executes **explicit `search_text_only`** through Python, awaited TS, and `fathomdb_sdk` Rust with the same seeded queries/top-K/state and expected ordered IDs. Results carry identical normalized correctness and distinct SDK/runtime/build identities. No general `search` fallback qualifies. |
| **BT-AC06** | A controlled cold and steady fixture proves separate timing boundaries. A mutation fixture demonstrates reconstructed state for every sample; a lazy-model fixture proves fresh-process isolation. Injected driver timeout leaves no running child/provider and yields one blocked receipt with no latency metric. |
| **BT-AC07** | A deterministic latency/capacity fixture verifies counted operations, timed boundaries, declared cache/concurrency, units and percentile calculation, resource/storage attribution, scale-tier separation, and sample-count gating for tail statistics. Setup, query, answer, and judge times cannot be pooled by the renderer. |
| **BT-AC08** | A fixed labelled retrieval fixture yields expected recall/rank/negative and source-attribution aggregates. Altered top-K, result unit, filter, qrels digest, effective validity instant, or missing support labels refuses a paired retrieval row. A fixture crossing a validity boundary proves that omitted wall-clock defaults cannot pair. A wrong binding/normalization output invalidates its latency; a correctly executed low-quality retrieval receives its measured score. |
| **BT-AC09** | A deterministic answer-pipeline fixture feeds the same evidence and frozen answerer/judge contract to each arm and reproduces expected correctness, grounding, attribution, cost, and stage timing. Repeating one immutable suite config binds each attempt's different completed upstream digests to its answer witness/result without changing config identity; a missing or cross-attempt dependency blocks. A retrieval-only gold fixture refuses answer scoring; a changed prompt/context/model refuses a paired answer row. |
| **BT-AC10** | The renderer reads validated receipts and, for SDK suites, a verified complete suite manifest. A missing expected cell makes that suite incomplete rather than silently shrinking its denominator. Product and SDK rows have separate axis labels within each of the three families. Published references stay context-only, incomplete cells remain visibly blocked/invalid, and the renderer computes no cross-row win or delta without an eligible paired protocol. |

## Python driver requirements and acceptance

The [Python driver design](../design/sdk-driver-python.md) implements these
criteria in addition to `BT-R01`–`BT-R14` and `BT-AC01`–`BT-AC10`.

- **BT-PY-R01 — Public installed SDK.** Import `fathomdb` through its package
  root, allowing the import-time co-installation guard to run. Pin the Python
  interpreter, distribution/version and installed package, PyO3 extension,
  driver source, dependency environment, and resolved shared engine/embedder
  dependency closure by digest. Refuse an unexpected import path or a
  second/shadowed distribution. Native `_fathomdb` is never
  called directly by a measured operation.
- **BT-PY-R02 — Exact Python semantics.** Dispatch canonical operation IDs to
  `Engine`, `read`, `graph`, `admin`, `rerank`, and `embed_batch_cls` public
  members. Normalize keywords and dataclass/request objects explicitly; FTS
  invokes `Engine.search_text_only(query, view=..., limit=...)`, not `search`.
  Convert the shared integer `timeout_ms` to Python `drain(timeout_s=...)`
  only when the native seconds-to-milliseconds conversion round-trips to the
  requested integer; otherwise block before timing. Record requested seconds
  and effective native milliseconds.
- **BT-PY-R03 — Timed boundary and typed failures.** Use
  `time.perf_counter_ns` immediately around the public synchronous call and
  its returned, fully materialized SDK value. Fixture parsing and neutral
  normalization occur outside the timer. Map documented Python exception
  classes and host `TypeError`/`ValueError` to typed driver failures without
  treating a refusal as a zero-latency success.
- **BT-PY-R04 — Process and model state.** Run each independent repetition in
  its pinned interpreter process; configure SQLite runtime before the first
  open. Keep cold model and destructive cells isolated as in `BT-R13`, record
  `open_report`/readiness and effective device, and close the Engine in
  `finally` when close itself is not the operation under test.

| ID | Python-specific acceptance evidence |
| --- | --- |
| **BT-PY-AC01** | An installed wheel/checkout-local package fixture verifies interpreter, distribution, imported package, and loaded PyO3 extension identities. A shadowed or unexpected package path blocks before measurement; the public co-install guard is exercised. |
| **BT-PY-AC02** | A fresh database fixture runs explicit FTS, a typed read, and a bounded graph call through public Python methods, comparing normalized IDs/results with the registry oracle. A validity-boundary fixture proves the fixed `valid_as_of` reaches a temporal-capable call. The spy/contract fixture proves no call to general `search` or direct `_fathomdb` on the FTS path. |
| **BT-PY-AC03** | A fake monotonic-clock fixture brackets only the public call; serialization/normalization are outside. A `drain` fixture includes the 1001 ms rounding boundary and proves the public seconds argument survives native truncation as exactly 1001 ms, or blocks an unrepresentable deadline before timing. The result records requested and effective deadlines without altering the other SDKs' deadline. |
| **BT-PY-AC04** | Engine/argument error fixtures yield typed blocked outcomes without samples; a controlled close/mutation/cold-model fixture verifies fresh state/process and closure after failures. Its safe result and receipt satisfy `BT-AC01`–`BT-AC03`. |

## TypeScript driver requirements and acceptance

The [TypeScript driver design](../design/sdk-driver-typescript.md) implements
these criteria in addition to the overall contract.

- **BT-TS-R01 — Pinned ESM and native binding.** Launch the pinned supported
  Node version; import the public `fathomdb` ESM package. Record the main
  package/version, lockfile, resolved platform triple and **actually loaded**
  `.node` binding path/digest, driver bundle digest, and resolved shared
  engine/embedder dependency closure. Refuse an unsupported
  platform, package/binding version mismatch, ambiguous binding, or a local
  development binary not declared by the arm manifest.
- **BT-TS-R02 — Exact TS semantics.** Dispatch through public `Engine`, `read`,
  `graph`, `admin`, `rerank`, and `embedBatchCls`. FTS calls and **awaits**
  `Engine.searchTextOnly(query, { ...readViewFields, limit })`; the SDK's
  `SearchOptions` is a *flattened* `ReadView` plus `limit`, not `{view, limit}`.
  It never invokes `search`. Convert neutral names to camelCase shapes and preserve
  filter/view/limit and graph work bounds. Neutral integer fields use decimal
  strings where needed; any public `number` outside safe-integer range blocks
  instead of rounding.
- **BT-TS-R03 — Promise timing and failures.** Use
  `process.hrtime.bigint()` before the public method call and after its Promise
  resolves to the mapped JS result. Await `open`, `drain`, query, write, and
  `close` operations. Exclude JSON parse/stringify and neutral normalization
  from the timer. Record typed SDK errors and host argument errors without
  silently retrying or falling back.
- **BT-TS-R04 — Process and loader isolation.** Start one Node process per
  independent repetition, configure runtime before first open, and attest
  `openReport`/readiness. Use a new process for cold singleton-model calls;
  await close and process exit, and let the coordinator reap the process tree
  after timeout. A proposed concurrent cell must record its Promise launch
  schedule and separate per-call latency from batch throughput.

| ID | TypeScript-specific acceptance evidence |
| --- | --- |
| **BT-TS-AC01** | A loader fixture verifies the pinned Node/package/lockfile and the loaded `.node` digest, including local-binary precedence. A missing, mismatched, or undeclared binary blocks before a timed cell. |
| **BT-TS-AC02** | A real fresh database fixture runs awaited `searchTextOnly`, read, and graph operations with the same oracle as Python/Rust. A fake Promise that resolves late proves the clock stops after resolution and the FTS path cannot route through `search`. |
| **BT-TS-AC03** | Conversion fixtures test camelCase options, `limit`, fixed `validAsOf`, view/filter, graph bounds, `drain(timeoutMs)`, maximum safe integers, and decimal-string u64 fields without precision loss. A validity-boundary fixture rejects implicit `now` pairing. Unsupported values fail before a call. |
| **BT-TS-AC04** | A rejecting Promise, typed SDK error, missing native package, timeout, and close failure produce typed blocked receipts with no successful timing sample. A cold process fixture proves no retained model state. |

## Rust driver requirements and acceptance

The [Rust driver design](../design/sdk-driver-rust.md) implements these
criteria in addition to the overall contract.

- **BT-RS-R01 — Dedicated SDK binary.** Build a pinned non-publishable driver
  executable depending on `fathomdb-sdk`, never on the lower-level `fathomdb`
  facade or `fathomdb-engine` for measured calls. Record its binary digest,
  Cargo.lock, compiler/target/profile, resolved crate source revision,
  dependency source/checksum closure, and feature set. The standalone Cargo
  root must carry the reviewed workspace patch set because root patches do
  not propagate to downstream projects. The crate's default feature set is
  empty; model/reranker/CUDA
  cells require explicit matching features and effective-device attestation.
- **BT-RS-R02 — Typed translations.** Dispatch registry operations to
  `Engine`, `read`, `graph`, `admin`, `rerank`, and `embed_batch_cls` using the
  re-exported typed DTOs and option structs. FTS calls
  `Engine::search_text_only(..., TextSearchOptions { view, limit })`.
  `SearchFilterArg`, frozen requests, graph bounds, and optional views are
  translated per the [Rust SDK interface](../interfaces/rust-sdk.md); no
  method default is inferred from a missing neutral field.
- **BT-RS-R03 — Clock, errors, and process.** Use `Instant` around the public
  synchronous call returning an owned result; decode, normalization, and
  serialization are outside. Map `Error::kind()` and typed payloads to driver
  failure categories rather than parsing `Display`. Configure SQLite runtime
  before open and use a fresh process per repetition and per cold-model sample.
- **BT-RS-R04 — Build equivalence.** The Rust arm's engine source revision,
  resolved shared engine/embedder dependency closure, and effective
  configuration/model/device match the Python and TS arms. Distinct native
  binary hashes remain visible. Unknown or mismatched dependency provenance
  prevents a paired SDK-overhead claim. A build without required features is
  blocked, not substituted with identity rerank or lexical search.

| ID | Rust-specific acceptance evidence |
| --- | --- |
| **BT-RS-AC01** | A locked build manifest and runtime self-report agree on binary, Rust compiler/target/profile, crate revision, feature flags, and the reviewed root patch set. A dependency-resolution fixture demonstrates that omitting a root patch changes the resolved shared dependency closure and prevents pairing with Python/TS. A no-feature build refuses model/rerank cells requiring those features instead of reporting an identity-path timing. |
| **BT-RS-AC02** | A real fresh database fixture runs `search_text_only`, read, and graph through `fathomdb_sdk` with the same oracle. A source/API conformance gate rejects a direct `fathomdb`/`fathomdb_engine` measured call or a Rust-only overload. |
| **BT-RS-AC03** | Typed conversion fixtures cover `TextSearchOptions`, `SearchOptions`/`SearchFilterArg`, fixed temporal validity, frozen/graph/read options, and `drain(timeout_ms)` with explicit defaults; bad neutral inputs fail before public calls. A validity-boundary fixture rejects implicit `now` pairing. A clock fixture excludes request construction and output normalization. |
| **BT-RS-AC04** | `Error::kind()` classification, close/mutation isolation, cold singleton reset, feature mismatch, and timeout yield typed blocked results without successful samples. Its safe result and receipt satisfy `BT-AC01`–`BT-AC03`. |

## Eligibility for a reported comparison

Before a reported product or SDK comparison, register the case, gold, sample
unit, repetitions, hardware matching rule, exact metric definitions,
uncertainty/decision rule, and license/authorization boundary. A source can
support only the claim its data actually measures. A published competitor
reference can show a useful target hardware and metric, but stays descriptive
unless a controlled pairing establishes input and measurement equivalence.
SDK rows may show language overhead only when engine source revision,
resolved shared dependencies, effective features/config, temporal eligibility,
and workload are the same, with each native build
identified; answer rows may show answer parity only when the
external answer pipeline is shared. The [Program](PROGRAM.md) remains the
authority for whether a result changes a product treatment or default.

The acceptance fixtures above are deterministic harness checks. They are
not benchmark observations and must never populate a competitive scoreboard.
