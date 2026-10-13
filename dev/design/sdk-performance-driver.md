---
title: Cross-SDK performance driver design
date: 2026-10-13
target_release: post-0.8.27 experiment infrastructure
desc: Versioned, repeatable Python, TypeScript, and Rust SDK workload driver
blast_radius: experiments; SDK benchmark artifacts; no product API change
status: PROPOSED
---

# Cross-SDK performance driver

## Decision and boundary

Build one configuration-driven experiment coordinator and three small,
language-native drivers. A driver invokes the public `fathomdb` Python,
`fathomdb` TypeScript, or `fathomdb_sdk` Rust API. The coordinator owns the
experiment definition, input validation, cell isolation, receipts, and
scoreboards; each driver owns SDK-specific type conversion and timing. An exact
experiment file pins an exact build and can be rerun without modification. To
measure a later release, revise its arm pins in a new configuration revision;
the command and driver protocol stay the same. A case names a **semantic
operation** and all effective options, never an arbitrary method name or
executable command.

This is an SDK-overhead and SDK-parity measurement lane, not a replacement for
the [competitive product scoreboards](../../experiments/COMPETITIVE-SCOREBOARDS.md)
or [Program](../performance-benchmarking/PROGRAM.md) decisions. It makes no
performance claim until real, qualified runs exist. Direct core calls, raw SQL,
and Rust's lower-level `fathomdb` facade are outside its SDK comparison.
The overall need and testable gates are in the
[competitive testing requirements](../performance-benchmarking/competitive-testing-requirements.md)
(`BT-R01`–`BT-R14`, `BT-AC01`–`BT-AC10`). This design implements their SDK
axis; the product-comparator runner retains its separate product axis.

### Shipped versus proposed

| Present at 0.8.27 | Needed for this design |
| --- | --- |
| Three public SDKs and the [governed parity map](../../src/conformance/governed-operation-parity.json), with 44 live canonical operations. | An experiment operation registry, initially derived from that map and checked against it, with workload-specific semantics and normalizers. |
| The [competitive runner](../../experiments/competitive_runner.py) verifies config/input/adapter hashes and calls a Python source adapter. Its committed execution witness and result locator contain absolute paths. | A fixed cross-language driver protocol and pinned Python/TS/Rust artifacts, plus a forward path-free product receipt/witness schema. The present runner cannot execute TS or Rust SDK cells and does not meet `BT-AC03`. |
| [`competitive.experiment.v1`](../../experiments/configs/competitive/experiment.example.json), `competitive.result.v1`, and `experiments.record.v1`. | A separate `sdk.experiment.v1` definition and `sdk.result.v1` payload within the unchanged common record/index envelope. Existing competitive records stay readable as legacy evidence; new product executions need a successor schema before meeting the safe-receipt gate. |
| A Python FTS adapter and smoke path. | Equivalent **explicit `search_text_only`** cells for all three SDKs. The existing adapter calls general `search`; its measurements cannot be relabeled as this cell. |

The proposed protocol and schemas below are design contracts, not implemented
interfaces. Implementing them under `experiments/` must follow that directory's
[agent instructions](../../experiments/AGENTS.md) and the Program's Track Runner
control.

## Three test families and two axes

The SDK driver is a common source of FathomDB calls and normalized evidence,
not a new answer API. The coordinator chooses exactly one test family per case:

| Family | Driver contribution | Result producer and comparison |
| --- | --- | --- |
| Latency and capacity (`BT-R08`) | Timed public calls, operation/error counts, readiness, resource and storage observations over declared size tiers. | Coordinator validates driver samples and emits `sdk.result.v1`. Compare SDK arms only under the same FathomDB engine revision and treatment; product rows remain separately labeled. |
| Retrieval (`BT-R09`) | Ordered IDs, scores and canonical-source mapping in an external evidence artifact; safe quality aggregates and call timing. | Coordinator applies pinned gold/oracle and emits `sdk.result.v1`. A wrong SDK binding fails parity; poor but valid retrieval yields its measured quality score. |
| Answer (`BT-R10`) | The retrieval evidence artifact identified by the configured upstream case/arm dependency and bound by digest after that attempt completes. | A shared answer pipeline consumes that evidence, with one frozen answerer/prompt/judge contract, and emits an answer-family `sdk.result.v1` linked to the retrieval result. The driver itself emits no answer score. |

`sdk.result.v1` declares `family`, `axis: sdk`, `producer` (`driver` or
`answer_pipeline`), and the upstream evidence/result hashes that connect its
stages. Its metric object is closed per family. Historical product results
continue under `competitive.result.v1` with `axis: product` supplied by the new
derived renderer and a legacy path-bearing label, not by rewriting old
receipts. New product runs use a successor path-free result/execution schema.
The renderer dispatches on payload
schema and axis, validates each, and presents three scoreboards with visibly
separate product and SDK sections. It never pools these axes or computes a
cross-row delta without a registered paired protocol. New published-reference
metadata required by `BT-R03` belongs in a versioned source-evidence sidecar
or successor product schema; the closed `competitive.result.v1` is unchanged.

## Surface coverage

The registry tracks **every shared public operation**, including operations
that are setup-only or presently unsuitable for a latency comparison. Seed its
canonical IDs and SDK spellings from the governed parity map, then extend it
with the common non-command members and standalone `embed_batch_cls` listed in
the [Rust SDK interface](../interfaces/rust-sdk.md). Compare the observed SDK
surface with the registry in CI: a new shared operation must be classified as
`measure`, `setup`, `observe`, or `excluded_with_reason`. Classification is
separate from selecting operations for a particular experiment. No new SDK
method may silently disappear from the inventory.

| Family | Common operations to register | Example measured shape |
| --- | --- | --- |
| Lifecycle and ingest | `engine.open`, `close`, `write`, `drain`, `actuate`, `transition`, `purge`, `erase_source`; `admin.configure` | Open/close; batch acknowledgement; drain-to-ready; lifecycle transition. Mutating cases get fresh database state per repetition. |
| Retrieval | `engine.search_text_only`, `search`, `search_projected_text`, `search_frozen`, `search_expand_frozen`, `search_with_evidence`, `resolve_evidence`, `resolve_graph_evidence` | Explicit FTS, configured hybrid, projected text, frozen and evidence paths, each a different case ID. |
| Graph | `graph.neighbors`, `graph.expand`, `graph.search_expand` | Bounded neighbor or traversal request with pinned direction, depth, limits, and work budget. |
| Read | `read.get`, `get_many`, `list`, `collection`, `mutations`, `canonical_page`, `operational_state`, `operational_state_page`, `crossed_boundary_since` | Point, batch, filtered, and paginated read; cursor state prepared outside the timed call. |
| Projection and readiness | `engine.configure_projections`; `read.projections`, `projection_status`, `projection_generation_status`, `embedding_readiness`, `mutation_projection_status` | Configuration, freshness, and status calls; readiness also gates retrieval cases. |
| Provenance and dependencies | `register_source_dependency`, `dependencies_for_source`, `dependency_for_derived`, `read_dependency_closure`, `trace_dependency`, `freeze_read_context` | Separate writes, lookups, trace, and frozen-context creation. |
| Models and providers | `engine.embed`, standalone `rerank`, `embed_batch_cls`, `ingest_with_extractor`, `consolidate_with_provider` | Separate model/provider cost cells with pinned artifact, device, and provider program. No hidden model download in a timed cell. |
| Observability and runtime | `open_report`, `config`, `counters`, dense-status queries, telemetry, feedback, profiling, slow-threshold, subscriber attachment, runtime configuration | Usually setup or observation. If measured, name a specific cost question and keep instrumentation state identical across arms. |

This classification covers the shared *surface* without pretending all calls
have the same cost model. The [0.8.27 parity ADR](../adr/ADR-0.8.27-rust-sdk-parity.md)
and interface docs govern API equivalence. The driver registry governs what a
benchmark invocation means. There is no shared public vector-only nearest
neighbor query verb at this baseline: dense retrieval is a configured part of
`engine.search`. A “vector-only” case must wait for a real shared API or be
explicitly labeled a non-equivalent diagnostic. `graph.expand` is bounded
traversal, not an unrestricted recursive graph query.

## Operation registry and extension rule

Each language's public binding and environment attestation are specified in
the [Python](sdk-driver-python.md),
[TypeScript](sdk-driver-typescript.md), and
[Rust](sdk-driver-rust.md) driver designs. Their driver-specific requirements
and acceptance criteria are in the overall requirements document. The
coordinator accepts the same `sdk.driver.v1` repetition shape from each and
applies the same cell aggregation, receipt, and comparison rules.

Each registry entry is data plus one binding per SDK. It defines:

- Stable `operation_id` matching the parity map where possible, plus
  `semantic_version` for a change in call meaning or normalized result.
- Supported SDK bindings, feature prerequisites, allowed options and defaults,
  input schema, fixture requirements, and result schema. Options are normalized
  **before** dispatch; the resolved config stores them, including defaults.
  Temporal-capable operations require a fixed effective validity instant or
  read context; an implicit `now` default cannot qualify for pairing.
- Timing boundary (`sdk_call_to_materialized_result`, `open`, `write_ack`,
  `write_to_ready`, or a named composite), whether the call mutates state, and
  its isolation rule.
- A normalization/oracle policy: ordered IDs and ranks for search, typed IDs
  and canonical fields for reads, edge/path identity for graph, receipt fields
  for writes, and explicit numerical tolerances where floats are involved.
- A `setup` recipe and readiness predicate. These are predeclared recipes,
  never free-form shell fragments in the experiment file.

An experiment lists registry IDs and versions. The coordinator asks each
driver for a capability manifest containing its protocol version, driver and
SDK artifact digests, SDK/runtime versions, supported operation versions,
option schema versions, build features, and resolved shared engine/embedder
dependency provenance from native build manifests. It rejects unknown IDs/options,
missing required capabilities, and mismatched versions before creating a
measurement cell. A declared optional case may produce a typed
`blocked_prerequisite` receipt with reason `unsupported`; it produces no
latency sample. There is no fallback from `search_text_only` to `search`, or
from a hybrid treatment to lexical search. Conditional code belongs in a
binding module for the relevant operation, not in the coordinator.

When the SDKs add a feature, update the governed parity map and interface
contract as required, add one registry entry/version and three bindings,
provide a cross-SDK fixture and normalizer, and add capability and conformance
tests. The coordinator and receipt envelope should need no new operation
branch. Additive registry entries preserve old experiment meanings; changes to
defaults, option interpretation, or normalized output require a new operation
semantic version. Breaking protocol or result-envelope changes require a new
schema version and an explicit renderer migration, never silent reinterpretation
of old receipts.

## Configuration contract

One checked-in `sdk.experiment.v1` JSON file defines the entire run matrix.
The file names immutable fixture manifests by absolute path and SHA-256, three
SDK arm manifests, operations, setup, sampling, and output policy. The
coordinator resolves defaults, validates every consumed field, hashes the
canonical resolved **execution config**, and writes it to the external
artifact directory. It derives a separate path-free receipt config (below).
Environment overrides may change only an explicitly declared `environment`
field; they are captured in the resolved config and result. The command line
supplies only the config path, new external output root, and a validate/run
verb. It cannot supply a workload, method, or settings override.

One invocation creates a random `suite_attempt_id` outside both configuration
hashes and a new external suite directory. Expansion is deterministic: one
**cell attempt** per `(suite_attempt_id, case_id, SDK arm)`; the cell owns all
declared independent repetitions and exactly one aggregate result/receipt.
The external suite manifest lists the full expected cell roster and binds each
cell's run ID, status, result hash, and the suite config hash. It is finalized
only when all cells are complete or explicitly blocked. A validation-only
invocation creates no suite attempt, cell, or receipt.

```json
{
  "schema_version": "sdk.experiment.v1",
  "program_track": "SDK-01",
  "suite_id": "fts-small-v1",
  "fixtures": {
    "manifest": {"path": "/external/fixtures/fts-small.json", "sha256": "<64 hex>"}
  },
  "arms": [
    {"sdk": "python", "artifact": "<pinned local manifest ID>"},
    {"sdk": "typescript", "artifact": "<pinned local manifest ID>"},
    {"sdk": "rust", "artifact": "<pinned local manifest ID>"}
  ],
  "cases": [
    {
      "id": "fts-top10",
      "family": "latency_capacity",
      "operation_id": "engine.search_text_only",
      "operation_version": 1,
      "required": true,
      "arguments": {
        "query_set": "fts-queries", "limit": 10,
        "view": {"valid_as_of": 1700000000}
      },
      "setup": "canonical-nodes-v1",
      "boundary": "sdk_call_to_materialized_result"
    }
  ],
  "sampling": {
    "warmup_per_query": 2,
    "repetitions": 5,
    "seed": 7,
    "cache_state": "steady_after_declared_warmup",
    "concurrency": 1,
    "order": "balanced"
  },
  "output": {"result_schema": "sdk.result.v1", "metrics": ["p50_ms", "p95_ms"]}
}
```

The example is illustrative; `SDK-01` is a placeholder track ID until the
Program registers a real track. A production schema must pin all setup and
measurement controls, including corpus/query/gold digests and counts, SDK
package and native-library hashes, Rust profile/features, Python/Node/Rust
versions, model/quantization/device policy, FathomDB config, timeout, cache
state, thread count, concurrency, and error policy. Values belong in the
resolved config even when inherited from a named fixture or arm manifest.
Manifest IDs resolve to pinned, hash-checked local files; they do not permit
arbitrary executables or network access. Keep paths to sensitive input data
out of committed receipts.
An answer-family case additionally pins its upstream retrieval **case ID and
SDK arm relation**, evidence transformation, answerer and judge artifacts/
prompts, context budget, decoding, cost ceiling, and gold. The upstream case
must belong to this suite config; its result digest cannot be known yet. After
that retrieval cell qualifies, the coordinator binds its completed result and
evidence digests into this attempt's answer execution witness and result,
without changing either immutable config hash. The answer cell consumes only
the upstream retrieval cell for the **same suite attempt and SDK arm**; it does
not create another database or launch an SDK driver. A missing, blocked, or
cross-attempt upstream cell blocks answer scoring. A separately imported prior
retrieval artifact is an explicit descriptive mode with its digest pinned in
the config, never silently mixed with same-suite pairing. The answer stage
does not call a driver-specific answer method.

## Driver protocol and run lifecycle

Use a fixed executable per language, with a versioned JSON request/response
protocol (`sdk.driver.v1`). The coordinator launches only those pinned driver
artifacts through a fixed argument shape; no per-case command strings. The
request names one **repetition of one cell**, resolved operation, fixture IDs
and verified local paths, setup controls, sampling order, and a fresh external
output directory. The response reports the capability actually used, effective
options, readiness, correctness summary, counts, sample artifact locator/hash,
repetition metrics, and typed status/error. The coordinator aggregates all
declared repetitions into the one cell result. Standard output is protocol-only; diagnostic
logs go to hashed external files. Pre/post hashes catch input or driver drift.
The coordinator treats a malformed/timeout/crashed response as a blocked
attempt, not a successful zero or partial metric. It owns the driver process
group, requires an engine-close and process-exit witness before the next repetition,
and terminates and reaps the driver and any provider descendants after a
timeout or crash. An orphaned process blocks subsequent measurements.

Each driver has a small dispatcher keyed by `operation_id` and version:
decode the neutral request into native SDK types, call the public method,
await TypeScript promises, materialize the result, normalize it, and measure
with that language's monotonic clock. Serialize results and write logs **after**
the timed call. Python and Rust synchronous calls and TS asynchronous calls
therefore share the same public-call-to-materialized-result boundary; host
scheduling and marshalling remain part of the measured SDK cost. Separate
cells may measure process startup/open, model load, writes, and readiness.
Start a fresh driver process for each independent repetition, apply any
process-wide runtime configuration **before its first engine open**, and
record its effective mode. Cold model cells use a fresh process per sample so
process singleton caches cannot carry over; steady cells may perform multiple
queries within one process after the declared warmup. Process startup is
outside a query timer unless the case explicitly measures startup.

For each FathomDB SDK cell attempt (latency/capacity or retrieval):

1. Validate config, manifests, digests, registry/capabilities, SDK build and
   runtime pins, device policy, and available resources. Record the selected
   runner and driver hashes. A capability or environment mismatch blocks the
   case with a reason.
2. Create a **new database per SDK arm and repetition** using
   `experiments.fathomdb_test_setup.prepare_test_database`; retain its explicit
   device policy and safe `doctor` evidence with external artifacts. Pin the
   helper's Python bootstrap and doctor binary to the **same FathomDB engine
   revision** as the measured SDK arm, or block the cell. Apply the declared
   fixture and projection setup, then drain and prove the case-specific
   readiness condition. Setup timing is separate.
3. Select the declared cache state. `steady_after_declared_warmup` uses a
   **separate equivalent state** for correctness rehearsal. Read-only cases
   then run the declared warmup on the measured state. Mutating cases warm a
   sacrificial state and reconstruct their initial state for the timed call.
   `first_pass_after_populated_reopen` starts a new driver
   process and takes its first query pass after opening the populated database,
   with **no prior query or model call** on that process/state. Cold model load
   similarly takes the first model call in a fresh process. A cold/open/close
   or mutating call must never be consumed by a correctness rehearsal.
   Recreate the declared initial state before **each timed mutating sample**;
   use a separate equivalent state for its oracle rehearsal. Execute the
   seeded, balanced order applicable to that state and time each call.
   Normalize and check outputs **after** each timed call against the fixture
   oracle or registered reference, outside the timer. Validate count, errors,
   operation identity, and normalized results; a mismatch invalidates that
   cell's latency rather than becoming a fast success.
4. Finalize one external `sdk.result.v1` artifact and safe
   `experiments.record.v1` receipt plus one `experiments.index-row.v1` row for
   **each cell attempt**, including blocked cells. Finalize the suite manifest
   after all expected cells have receipts. The aggregate renderer reads
   validated receipts and the complete roster, never loose logs or a manually
   edited table.

The fresh database rule has one deliberate consequence: `engine.open` on an
empty database is a distinct cell from opening a prepared populated database.
The setup helper's own open cannot be counted as an SDK-arm open sample. For
mutations, each timed sample starts from the same declared state; do not
replay writes or purges against progressively changed data and pool them as
one distribution. The registered setup recipe must specify how a fresh state
is recreated and how its oracle is checked; otherwise the case is ineligible.
Provider and model cells additionally pin local artifacts,
device resolution, and provider build; any download or provisioning occurs
before the cell and is recorded as setup.

## Result and comparison contract

`sdk.result.v1` is a validated aggregate for one cell attempt across its
declared repetitions. It contains
`suite_id`, `case_id`, family/axis/producer, operation ID/version, status/reason, **execution-config
and receipt-config hashes**, fixture hashes, SDK/driver/runtime/build identity,
hardware and device observations,
requested **and observed** options, database/setup/readiness witness hashes,
work counts, normalized-correctness digest and oracle verdict, sample artifact
hash, error counts, elapsed setup/open/query partitions, and p50/p95/p99 plus
spread/uncertainty appropriate to the registered sampling plan. Raw requests,
corpus text, hits, model output, and per-query samples remain external. The
answer-family result additionally binds the declared upstream case/arm to
that same attempt's retrieval-result and evidence digests; the answer witness
hash covers that binding. These per-attempt digests are excluded from both
immutable config hashes.
The safe receipt contains summary numbers, content hashes, and opaque artifact IDs,
never an external absolute path. Blocked attempts include a typed reason and no
performance metric. The runner records its own wall-clock and exit witness;
that is not substituted for driver call latency.

The external execution config contains checked absolute paths and any private
setup details. The committed **cell** receipt config is a deterministic, typed
projection of it: suite-config hash, case/arm identity, operation/settings,
and **content hashes** remain, but local
paths, credentials, corpus text, and private identifiers are removed. Generated
output paths, database paths, timestamps, and attempt IDs are excluded from
both config hashes; they vary per attempt and belong in the execution witness.
Before writing a receipt, the coordinator validates that the projection recomputed
from the execution config equals the proposed receipt config. It passes only
that safe projection to `experiments._lib.write_record`, whose `config.sha256`
hashes the canonical JSON representation of the same config value stored in
`record.json` and `config.resolved.yaml`; the run ID uses that hash's prefix
and a timestamp. A fresh 128-bit suite nonce and case/arm identity also
produce a distinct cell-attempt discriminator in the experiment slug. The SDK
wrapper checks that the run ID is absent from the index and creates its run
directory exclusively **before** calling `_lib.write_record`; collision fails
instead of reusing or overwriting a receipt. The nonce is outside both config
hashes, so identical treatments retain identical config identity. A
separately named `execution_config_sha256` hashes
the full external config and appears in the result and hashed external
execution witness. The receipt points to that witness by digest and an opaque
artifact ID resolved through the external suite manifest; validation checks
the witness's execution hash, receipt-config hash,
and result hash together. No full execution config or sensitive path is passed
to the common record writer.

The SDK scoreboard groups only cells with the same case semantics, fixture
digests, engine revision/config, resolved shared engine/embedder dependency
closure, operation version, top-K/filter/view and fixed effective validity
instant or read context, model and device, cache state, concurrency, hardware
class, and timing boundary. Unknown or mismatched dependency provenance or
wall-clock-derived temporal eligibility leaves a cell descriptive.
Keep cold and steady, CPU and GPU, and Python/Node/Rust build profiles separate.
Use the balanced order and independent repetitions to show dispersion; do not
declare a winner from one smoke. Correctness/parity is a gate alongside the
latency table. Search cases report rank/ID agreement and retrieval metrics
against the same gold when available. Answer quality is scored by the separate
answer pipeline, using the same retrieved-evidence contract; the driver does
not invent an answerer. Missing or unsupported cells render as such, never as
zero or as a pooled comparison.

## Implementation and acceptance

| Contract | Design mechanism | Acceptance gate |
| --- | --- | --- |
| `BT-R01`–`BT-R07` | Closed configuration, hash-checked inputs, safe receipt projection, isolated attempts, comparison key. | `BT-AC01`–`BT-AC03`, `BT-AC10` |
| `BT-R08`–`BT-R10` | Family-specific metric schemas and timing stages; shared answer pipeline linked to retrieval evidence. | `BT-AC07`–`BT-AC09` |
| `BT-R11`–`BT-R14` | Registry, capabilities, three fixed drivers, fresh state/process, distinct product and SDK payloads. | `BT-AC04`–`BT-AC06`, `BT-AC10` |

1. Freeze a schema and registry fixture against the 0.8.27 parity map and
   actual SDK interfaces. Add contract tests for unknown operations/options,
   capability mismatch, version drift, blocked receipts, and new-operation
   registration. Keep the existing competitive schema and receipts untouched.
2. Build the fixed protocol and Python/TS/Rust drivers. Start with one
   `engine.search_text_only` fixture; test each binding against a real fresh
   database and the same expected IDs/ranks. Ensure the TS call is awaited and
   the Rust driver uses `fathomdb_sdk`, not the core facade.
3. Add configured `engine.search` hybrid, projected-text, read, and graph
   cases in that order, then mutation, evidence, and provider cases as their
   fixtures and oracles are ready. The registry retains an explicit disposition
   for every shared SDK operation throughout.
4. Add a shared answer stage that consumes the same suite attempt's declared
   upstream retrieval evidence from each SDK arm and binds its completed
   digests in the answer witness/result. Its deterministic fixture must
   reproduce answer/gold scoring and reject missing answer gold,
   cross-attempt evidence, or an answerer/prompt/judge mismatch.
5. Verify that the same experiment file can run twice to different external
   roots, produces complete safe receipts and an append-only index, detects
   altered inputs/binaries, and renders only truly comparable complete cells.
   Only then collect scoreboards over time.

This design needs a registered Program track and implementation review before
its first Program measurement. It does not authorize a new corpus, GPU/model
download, paid service, or external benchmark run.
