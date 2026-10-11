// Private core owner for the TypeScript SDK package root.
import {
  write,
  registerSourceDependency,
  actuate,
  dependenciesForSource,
  dependencyForDerived,
  readDependencyClosure,
  transition,
  purge,
  eraseSource,
  ingestWithExtractor,
  consolidateWithProvider,
} from "./write.js";
import { configureProjections } from "./projection.js";
import {
  freezeReadContext,
  searchFrozen,
  searchExpandFrozen,
  search,
  searchTextOnly,
  searchProjectedText,
} from "./search.js";
import { traceDependency, searchWithEvidence, resolveEvidence, resolveGraphEvidence } from "./evidence.js";
import { denseDisabled, denseDisabledReason, vectorEquivalenceRefusalCount, openReport } from "./open.js";
import { embed } from "./embedding.js";
import {
  enableTelemetry,
  lastTelemetryQueryId,
  recordFeedback,
  counters,
  setProfiling,
  setSlowThresholdMs,
  attachSubscriber,
} from "./instrumentation.js";
import { native, type NativeEngine } from "./binding.js";

import { validateFfiString } from "./validation.js";

import {
  LifecycleState,
  WriteReceipt,
  SourceDependencyRegistrationV1,
  DependencySourceLookupV1,
  DependencyDerivedLookupV1,
  SourceDependencyV1,
  DependencyListV1,
  ClosureLookupV1,
  ClosureStatusV1,
  ActuationBatchV1,
  ActuationReceiptV1,
  EraseReport,
  IngestWithExtractorReceipt,
  ExtractDocument,
  ConsolidateReceipt,
  ConsolidateAxis,
} from "./write.js";

import { ProjectionSpec, ProjectionDelta } from "./projection.js";
import {
  SearchFilter,
  ReadContextV1,
  FrozenReadContextV1,
  FrozenSearchOptions,
  SearchOptions,
  SearchExpandOptions,
  Filter,
  SearchResult,
} from "./search.js";
import {
  DependencyTraceRequestV1,
  DependencyTraceResultV1,
  EvidenceSearchRequestV1,
  EvidenceResolveRequestV1,
  EvidenceSearchResultV1,
  ResolvedEvidenceV1,
} from "./evidence.js";
import { OpenReport, CounterSnapshot, SubscriberCallback } from "./open.js";

import { intercept } from "./native-call.js";

import { SearchExpandResult, GraphEvidenceResolveRequestV1, ResolvedGraphEvidenceV1 } from "./graph.js";

export interface EngineConfig {
  readonly embedderPoolSize?: number;
  readonly schedulerRuntimeThreads?: number;
  readonly provenanceRowCap?: number;
  readonly embedderCallTimeoutMs?: number;
  readonly slowThresholdMs?: number;
}

export const ENGINE_CONFIG_LIMITS = {
  embedderPoolSize: [1, 64],
  schedulerRuntimeThreads: [1, 64],
  provenanceRowCap: [0, Number.MAX_SAFE_INTEGER],
  embedderCallTimeoutMs: [1, 0xffff_ffff],
  slowThresholdMs: [0, Number.MAX_SAFE_INTEGER],
} as const;

export function snapshotEngineConfig(value: unknown): EngineConfig {
  if (value === undefined) return Object.freeze({});
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError("engineConfig must be an object");
  }
  const config = { ...value } as Record<string, unknown>;
  for (const [field, [min, max]] of Object.entries(ENGINE_CONFIG_LIMITS)) {
    const requested = config[field];
    if (requested === undefined) continue;
    if (typeof requested !== "number") {
      throw new TypeError(`engineConfig.${field} must be a number`);
    }
    if (!Number.isSafeInteger(requested) || requested < min || requested > max) {
      throw new RangeError(`engineConfig.${field} must be a safe integer in ${min}..=${max}`);
    }
  }
  return Object.freeze(config) as EngineConfig;
}

export interface EngineOpenOptions {
  engineConfig?: EngineConfig;
  /**
   * EU-6: opt-in to the engine's pinned default embedder
   * (`fathomdb-bge-small-en-v1.5`). On first use, weights are downloaded
   * from HuggingFace and cached under `~/.cache/fathomdb/embedders/`.
   * `false` (the default) opens without an embedder; vector writes
   * then fail with `EmbedderNotConfiguredError`. Caller-supplied
   * custom embedders are deferred to a future release per
   * ADR-0.6.0-embedder-protocol Invariant 3.
   */
  useDefaultEmbedder?: boolean;
}

export class Engine {
  readonly #native: NativeEngine;
  readonly config: EngineConfig;

  private constructor(inner: NativeEngine, config: EngineConfig) {
    this.#native = inner;
    this.config = config;
  }

  static async open(path: string, options: EngineOpenOptions = {}): Promise<Engine> {
    validateFfiString(path);
    // Read caller-owned accessors once so native open receives the validated snapshot.
    const requestedConfig = options.engineConfig;
    const config = snapshotEngineConfig(requestedConfig);
    const useDefaultEmbedder = options.useDefaultEmbedder;
    const nativeOptions = requestedConfig === undefined
      ? { useDefaultEmbedder }
      : { useDefaultEmbedder, engineConfig: config };
    const inner = await intercept(() => native.Engine.open(path, nativeOptions));
    return new Engine(inner, config);
  }

  /**
   * Write a batch of items. `sourceId` is MANDATORY on every canonical item
   * (0.8.20 R-20-E3) — a row written without it can never be erased by
   * `eraseSource`. Every field below accepts BOTH the camelCase and the
   * snake_case spelling.
   *
   * A node item is `{ kind, body, sourceId, logicalId?, state?, reason?,
   * validFrom?, validUntil? }`; an edge item is `{ edge: { kind, from, to,
   * sourceId, ... } }`.
   *
   * `validFrom` / `validUntil` (0.8.20 Slice 15b, TC-34) author the node's
   * WORLD-TIME validity window as INTEGER epoch SECONDS. The window is
   * HALF-OPEN — `validFrom` is inclusive, `validUntil` is exclusive — and an
   * OMITTED bound means unbounded on that side, so omitting both (the default)
   * makes the node valid at every instant. Read it back with
   * `read.get`/`read.list`'s `validAsOf` view, or ask which nodes crossed a
   * boundary with `read.crossedBoundarySince`.
   *
   * Because the window is half-open, `validFrom >= validUntil` describes a
   * window no instant can satisfy; that pair is rejected with
   * `WriteValidationError` rather than silently stored. A non-integral bound
   * rejects with `WriteValidationError` too — it is never truncated or
   * coerced. One family for the whole write-validation boundary.
   *
   * **BREAKING (0.8.20 Slice 22, decision #18).** The unsatisfiable-window
   * pair used to reject with `InvalidArgumentError` (napi code
   * `FDB_INVALID_ARGUMENT`) carrying both bounds in `message`. It is now
   * `WriteValidationError` (`FDB_WRITE_VALIDATION`) with the fixed message
   * `"write validation error"` and `data: null` — the offending bounds are no
   * longer recoverable from the error, so validate the pair before calling.
   */
  async write(batch: unknown[] = []): Promise<WriteReceipt> { return write(this.#native, batch); }

  /** Register one pinned dependency; exact replay is a no-op success. */
  async registerSourceDependency(
    request: SourceDependencyRegistrationV1,
  ): Promise<SourceDependencyV1> { return registerSourceDependency(this.#native, request); }

  /** Atomically apply a bounded set of caller-decided memory operations. */
  async actuate(request: ActuationBatchV1): Promise<ActuationReceiptV1> { return actuate(this.#native, request); }

  /** Return at most 100 dependencies in stable derived-revision order. */
  async dependenciesForSource(request: DependencySourceLookupV1): Promise<DependencyListV1> { return dependenciesForSource(this.#native, request); }

  /** Return the dependency for one derived revision, or `null`. */
  async dependencyForDerived(
    request: DependencyDerivedLookupV1,
  ): Promise<SourceDependencyV1 | null> { return dependencyForDerived(this.#native, request); }

  /** Return current closure status, or `null` for an absent opaque ID. */
  async readDependencyClosure(request: ClosureLookupV1): Promise<ClosureStatusV1 | null> { return readDependencyClosure(this.#native, request); }

  /**
   * OPP-12 Phase-1 (0.8.19 Slice 10) — `transition` lifecycle verb. Moves a
   * governed node between existence states per the engine-enforced
   * legal-transition table (promote `pending→active`, reject `pending→deleted`,
   * soft-delete `active→deleted`, undelete `deleted→active`). Promote/undelete
   * CLEAR `reason`; reject/soft-delete SET it. Keys on the bare `logicalId`
   * (`l:` only) — a non-`l:` id throws `NotLifecycleAddressableError`; an illegal
   * move throws `IllegalTransitionError`. Thin pass-through (no client logic).
   */
  async transition(
    logicalId: string,
    toState: LifecycleState,
    reason?: string | null,
  ): Promise<void> { return transition(this.#native, logicalId, toState, reason); }

  /**
   * OPP-12 Phase-1 (0.8.19 Slice 10) — `purge` lifecycle verb. Irreversible,
   * deleted-first, idempotent hard-erase of a governed node across every
   * row-owned target (all versions + FTS/vector shadows + touching edges,
   * cascade-removed). A SEPARATE verb from `transition` (NOT a recovery-denylist
   * name). Keys on the bare `logicalId` (`l:` only) — a non-`l:` id throws
   * `NotLifecycleAddressableError`; a non-`deleted` node throws
   * `IllegalTransitionError`.
   */
  async purge(logicalId: string): Promise<void> { return purge(this.#native, logicalId); }

  /**
   * 0.8.20 Slice 5d (R-20-E4) — the `eraseSource` lifecycle verb. Erases every
   * canonical row carrying `sourceId`, together with its row-owned projections
   * (FTS5, vec0, `search_index_v2`), and finishes the erasure at rest
   * (telemetry redaction + WAL truncation).
   *
   * The COMPANION to {@link Engine.purge}, not a duplicate of it. `purge`
   * addresses a *governed* node by `logicalId`; `eraseSource` addresses
   * *anonymous* content — rows written with no `logicalId`, which `purge`
   * cannot reach at all. Together they make every canonical row erasable from
   * the SDK alone, with no CLI on `PATH`.
   *
   * Idempotent: erasing an absent or already-erased source is a zero-count
   * success, so an interrupted erasure obligation can be retried without a
   * pre-check.
   *
   * Throws `WriteValidationError` for an empty, whitespace-only or reserved
   * (`_`-prefixed) `sourceId`. The engine's reserved namespace (`_engine:*`
   * substrate and the `_legacy:pre-0.8.20` migration cohort) is reachable ONLY
   * through the CLI recovery seam `fathomdb recover --excise-source`; a single
   * governed call against it would erase every pre-0.8.20 anonymous row.
   *
   * NOT a recovery-denylist name (`recover`/`restore`/`repair`/`fix`/`rebuild`),
   * so AC-041 is unaffected.
   */
  async eraseSource(sourceId: string): Promise<EraseReport> { return eraseSource(this.#native, sourceId); }

  /**
   * 0.8.20 Slice 15d (R-20-PR / C-1) — the `configureProjections` governed verb.
   * Declaratively apply projection declarations: the engine is the SOLE
   * projection authority and diffs `specs` against the durable registry,
   * backfilling the difference in one transaction. Cheap projections
   * (`filterable`, `searchable→FTS`) build same-transaction; `rankable` and the
   * `searchable→vector` sub-target are persisted-but-deferred (F9 / Slice 20).
   *
   * `drop` is EXPLICIT: omitting a live projection from `specs` does NOT drop
   * it; removal requires naming it in `drop`. A destructive change to a live
   * projection (a role removal or a tokenizer/embedder change) that is NOT in
   * `drop` throws a typed `FDB_PROJECTION_DESTRUCTIVE` error carrying
   * `{ name, delta }` — never silent data loss. Re-applying an unchanged spec
   * returns `{ unchanged: true }`.
   *
   * Pair with `read.projections` to inspect current state first.
   */
  async configureProjections(specs: ProjectionSpec[], drop?: string[]): Promise<ProjectionDelta> { return configureProjections(this.#native, specs, drop); }

  /** Mint a restart-stable read context bound to this database state. */
  async freezeReadContext(context: ReadContextV1): Promise<FrozenReadContextV1> { return freezeReadContext(this.#native, context); }

  /** Trace one reciprocal registered dependency under a frozen context. */
  async traceDependency(request: DependencyTraceRequestV1): Promise<DependencyTraceResultV1> { return traceDependency(this.#native, request); }

  /**
   * Search under an Engine-authenticated frozen context. `explain: true` returns a
   * finalized nonempty correlation identity.
   */
  async searchFrozen(
    query: string,
    context: FrozenReadContextV1,
    options: FrozenSearchOptions = {},
  ): Promise<SearchResult> { return searchFrozen(this.#native, query, context, options); }

  /**
   * Attach one evidence reference per frozen hit. `includeExplanation: true`
   * finalizes the nested correlation identity.
   */
  async searchWithEvidence(request: EvidenceSearchRequestV1): Promise<EvidenceSearchResultV1> { return searchWithEvidence(this.#native, request); }

  /** Resolve exact source bytes under an equivalent frozen context. */
  async resolveEvidence(request: EvidenceResolveRequestV1): Promise<ResolvedEvidenceV1> { return resolveEvidence(this.#native, request); }

  /** Resolve one exact artifact disclosed by frozen graph expansion. */
  async resolveGraphEvidence(
    request: GraphEvidenceResolveRequestV1,
  ): Promise<ResolvedGraphEvidenceV1> { return resolveGraphEvidence(this.#native, request); }

  /** Search and graph-expand on one frozen reader transaction. */
  async searchExpandFrozen(
    query: string,
    context: FrozenReadContextV1,
    depth: number,
    options: SearchExpandOptions = {},
  ): Promise<SearchExpandResult> { return searchExpandFrozen(this.#native, query, context, depth, options); }

  async search(
    query: string,
    filter?: SearchFilter | Filter,
    rerankDepth?: number,
    useGraphArm?: boolean,
    alpha?: number,
    poolN?: number,
    explain?: boolean,
    /**
     * 0.8.20 Slice 15b fix-2 (R-20-NV / R-20-RV) — optional validity view, the
     * same options object the five read verbs take. Omitted is the strict view:
     * active-only, non-superseded, and valid AT QUERY TIME. `{ includeOutOfWindow:
     * true }` returns hits whatever their `[validFrom, validUntil)` window;
     * `{ validAsOf: t }` evaluates validity at the bound instant `t`.
     *
     * The EXISTENCE flags (`includeSuperseded` / `includeInactive`) are REFUSED
     * on the search path with a typed `InvalidArgumentError` — search hydrates
     * from projection indexes that are not version-complete, so they have no
     * truthful answer here. They are refused rather than silently ignored.
     */
    view?: SearchOptions,
  ): Promise<SearchResult> { return search(this.#native, query, filter, rerankDepth, useGraphArm, alpha, poolN, explain, view); }

  /**
   * 0.8.18 Slice 5 (#5 vector-equivalence probe) — the explicit text-only /
   * FTS-only search path. It does NOT embed the query and NEVER throws
   * `VectorEquivalenceMismatchError`, so it stays serviceable when the engine
   * opened in the degraded `denseDisabled` state. It does not invoke vector
   * recall, CE reranking, or the graph arm. Matching node- and edge-body FTS
   * candidates are deterministically body-deduplicated and ranked before
   * `limit` is applied. For one immutable selection and effective validity time,
   * smaller accepted limits are prefixes of larger limits; this does not extend
   * to hybrid search.
   */
  async searchTextOnly(query: string, view?: SearchOptions): Promise<SearchResult> { return searchTextOnly(this.#native, query, view); }

  /** Search exactly one declared `searchable` property-FTS projection. */
  async searchProjectedText(
    query: string,
    name: string,
    filter?: SearchFilter,
    view?: SearchOptions,
  ): Promise<SearchResult> { return searchProjectedText(this.#native, query, name, filter, view); }

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — `true` iff the engine opened degraded (the #5
   * self-check found a vector-equivalence divergence and every dense arm is
   * refusing). Mirrors `OpenReport.denseDisabled`.
   */
  denseDisabled(): boolean { return denseDisabled(this.#native); }

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — the human-readable reason for the degraded state,
   * or `null` when dense is healthy.
   */
  denseDisabledReason(): string | null { return denseDisabledReason(this.#native); }

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — telemetry counter: query-time dense-arm refusals
   * raised because the engine opened degraded.
   */
  vectorEquivalenceRefusalCount(): number { return vectorEquivalenceRefusalCount(this.#native); }

  async close(): Promise<void> {
    await intercept(() => this.#native.close());
  }

  async drain(timeoutMs: number): Promise<void> {
    await intercept(() => this.#native.drain(timeoutMs));
  }

  /**
   * G11 (Slice 15) — BYO-LLM ingest. Spawns an external extraction harness
   * speaking the `fathomdb.extract.v1` NDJSON-over-stdio protocol, sends
   * documents for extraction, and writes the resulting entities and fact-edges.
   *
   * @param cmd - argv to spawn (first element = program, rest = args).
   * @param documents - array of `{ sourceDocId, body }` objects to extract from.
   */
  async ingestWithExtractor(
    cmd: string[],
    documents: ExtractDocument[],
  ): Promise<IngestWithExtractorReceipt> { return ingestWithExtractor(this.#native, cmd, documents); }

  /**
   * 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation / recency. Spawns a
   * caller-supplied harness speaking the `fathomdb.consolidate.v1`
   * NDJSON-over-stdio protocol (the SAME transport as `ingestWithExtractor`).
   * For each `{ subjectLogicalId, relation }` axis FathomDB assembles the
   * competing fact-edge cluster deterministically and applies the harness
   * verdicts as supersession/recency METADATA — edge bodies are never rewritten
   * and no row is ever deleted (ADR-0.8.12 §2.1).
   *
   * @param cmd - argv to spawn (first element = program, rest = args).
   * @param axes - array of `{ subjectLogicalId, relation }` axes to consolidate.
   */
  async consolidateWithProvider(
    cmd: string[],
    axes: ConsolidateAxis[],
  ): Promise<ConsolidateReceipt> { return consolidateWithProvider(this.#native, cmd, axes); }

  /**
   * Embed `text` with the engine's pinned default embedder
   * (`fathomdb-bge-small-en-v1.5`) and return the raw vector.
   *
   * Read-path primitive (mirror of the Python `Engine.embed`) for callers
   * that need vectors under the engine's own embedder identity (e.g.
   * coverage-index clustering) rather than a parallel, possibly-divergent
   * embedder. Rejects with `FDB_EMBEDDER_NOT_CONFIGURED` if the engine was
   * opened without an embedder (`useDefaultEmbedder: false`).
   */
  async embed(text: string): Promise<number[]> { return embed(this.#native, text); }

  /**
   * 0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a JSONL
   * `sinkPath`. Off by default; local file only (no egress). Once enabled, each
   * `search` records a query→result event keyed on the stable id, and
   * `recordFeedback` appends correlated agent labels. The query text and
   * `sourceId` are NEVER written (privacy, ADR §C).
   */
  async enableTelemetry(sinkPath: string): Promise<void> { return enableTelemetry(this.#native, sinkPath); }

  /**
   * 0.8.8 Slice 15 — the most-recent captured `queryId` (for `recordFeedback`),
   * or `null` when telemetry is off / no query has been captured yet.
   */
  lastTelemetryQueryId(): string | null { return lastTelemetryQueryId(this.#native); }

  /**
   * 0.8.8 Slice 15 — attach agent relevance labels for a previously captured
   * `queryId`. `relevantIds` / `irrelevantIds` are the stable identity carrier
   * (== `SearchHit.id`); `labelSource` is the caller-declared label origin
   * (e.g. `"agent:hermes"`). Rejects when telemetry is off.
   */
  async recordFeedback(
    queryId: string,
    relevantIds: number[],
    irrelevantIds: number[],
    labelSource: string,
  ): Promise<void> { return recordFeedback(this.#native, queryId, relevantIds, irrelevantIds, labelSource); }

  counters(): CounterSnapshot { return counters(this.#native); }

  openReport(): OpenReport { return openReport(this.#native); }

  setProfiling(enabled: boolean): void { return setProfiling(this.#native, enabled); }

  setSlowThresholdMs(value: number): void { return setSlowThresholdMs(this.#native, value); }

  attachSubscriber(callback: SubscriberCallback): void { return attachSubscriber(this.#native, callback); }

  /** @internal — handle to the napi-rs binding, used by `admin.configure`. */
  get _native(): NativeEngine {
    return this.#native;
  }
}
