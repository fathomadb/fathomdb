import type { NativeEngine } from "./binding.js";
// Private search owner for the TypeScript SDK package root.
import {
  type NativeFrozenReadContextV1,
  type NativePerHitExplain,
  type NativeSearchResult,
} from "./binding.js";
import { FathomDbError, FrozenReadError, InvalidArgumentError, InvalidFilterError } from "./errors.js";
import { intercept } from "./native-call.js";
import { validateFfiString } from "./validation.js";
import type { SearchExpandResult } from "./graph.js";
import type { Predicate, ReadView } from "./read.js";

export type SoftFallbackBranch = "vector" | "text" | "text_edge" | "graph_arm";

export interface SoftFallback {
  branch: SoftFallbackBranch;
}

/**
 * C-2 (0.8.19 / OPP-12 Phase-1, TC-8) — the typed id-space carrier for
 * `SearchHit.id`. `space` is the lowercase discriminant (`"logical"` |
 * `"content"` | `"passage"`), mirroring the engine's `IdSpaceKind` enum (the
 * C-2 binding — a typed carrier, not a magic-prefixed string). `value` is the
 * bare id (id-space prefix stripped). The prefixed form is `${prefix}${value}`
 * (`l:`/`h:`/`p:`) — byte-identical to the pre-0.8.19 `stableId`. Only `logical`
 * ids are lifecycle-addressable.
 */
export interface IdSpace {
  space: string;
  value: string;
}

export interface SearchHit {
  /**
   * C-2 (0.8.19 / TC-8) — the typed, non-null, id-space-total hit id
   * (`{ space, value }`). Governed hits are `logical` (`"l:"`), doc-seeded hits
   * `content` (`"h:"`), synthetic passages `passage` (`"p:"`). Its `value` is
   * the bare (prefix-stripped) id; the prefixed form (`{prefix}{value}`) equals
   * the pre-0.8.19 `stableId` (which this subsumes) so cross-session
   * real-gold keying continues on `id`; it survives re-ingest and never
   * participates in ranking. The pre-C-2 positional `write_cursor` id is
   * engine-internal and no longer surfaced.
   */
  id: IdSpace;
  kind: string;
  body: string;
  /**
   * G9 RRF-fused relevance (`Σ 1/(RRF_K + rank)`; higher = more relevant),
   * optionally recency-reweighted. Raw `vec_distance_l2`/`bm25()` are fused on
   * rank, never compared raw.
   */
  score: number;
  branch: SoftFallbackBranch;
  /**
   * G0 Phase-2 — source-document provenance, the identifier `eraseSource`
   * consumes. TC-31 (0.8.20): populated on EVERY hit path, not just the graph
   * arm. Node hits (text/vector) carry the node's own `source_id`; edge hits
   * (edge-FTS, vector edge-fact) carry the edge's own; graph-arm hits carry the
   * traversed edge's (unchanged). `null` only when the stored row really has
   * NULL provenance: written before 0.8.20, or a governed row spared by the
   * step-21 backfill under the TC-11 pin.
   */
  sourceId: string | null;
  /**
   * 0.8.5 (EXP-0) — per-candidate CE score (`ce_norm = sigmoid(ce_logit)`) for
   * hits inside the reranked pool; `null` otherwise (out-of-pool, the identity
   * path, or no CE model loaded).
   */
  ceScore: number | null;
}

/**
 * G10 — closed metadata filter for `engine.search(query, filter?)`. All fields
 * optional; an all-`undefined` filter (or omitted) is the unfiltered path. A
 * closed shape, not an open DSL. `createdAfter` is a `created_at >= bound` lower
 * bound in unix seconds. `status` filters the vec0 `status` metadata column,
 * which ships an empty-string sentinel only (no real population source yet), so
 * a `status: "open"`-style filter prunes every row until a population slice
 * lands. Mirrors the Python `SearchFilter` (cross-binding parity).
 */
export interface SearchFilter {
  sourceType?: string;
  kind?: string;
  createdAfter?: number;
  status?: string;
  /** Ordered AND equality predicates over declared `filterable` projections. */
  attributes?: [string, string][];
}

/** Versioned validity and eligibility request for a frozen read. */
export interface ReadContextV1 {
  schemaVersion: 1;
  view: ReadView;
  eligibility: SearchFilter;
}

/** Database-local, authenticated read context minted by an Engine. */
export interface FrozenReadContextV1 {
  schemaVersion: 1;
  effectiveValidAt: number;
  context: ReadContextV1;
  token: string;
}

/** Per-call ranked-search controls that cannot weaken a frozen context. */
export interface FrozenSearchOptions {
  rerankDepth?: number;
  useGraphArm?: boolean;
  alpha?: number;
  poolN?: number;
  explain?: boolean;
  limit?: number;
}

/** Options shared by ranked search methods. `limit` defaults to 10 and is 1–100. */
export type SearchOptions = ReadView & {
  limit?: number;
};

/** Options for the initial ranked hits returned by {@link graph.searchExpand}. */
export interface SearchExpandOptions {
  searchLimit?: number;
}

export function validateRankedResultLimit(name: string, limit: number | undefined): number {
  const resolved = limit ?? 10;
  if (!Number.isInteger(resolved) || resolved < 1 || resolved > 100) {
    throw new InvalidArgumentError(`${name} must be an integer in 1..=100; got ${resolved}`);
  }
  return resolved;
}

export function splitSearchOptions(options: SearchOptions | undefined): {
  limit: number;
  view: ReadView | undefined;
} {
  if (options === undefined) return { limit: 10, view: undefined };
  const { limit, ...view } = options;
  return {
    limit: validateRankedResultLimit("limit", limit),
    view: Object.keys(view).length === 0 ? undefined : view,
  };
}

export function mapNativeSearchResult(r: NativeSearchResult): SearchResult {
  const branch = r.softFallback?.branch;
  const e = r.explanation;
  return {
    projectionCursor: r.projectionCursor,
    softFallback:
      branch === "vector" || branch === "text" || branch === "text_edge" || branch === "graph_arm"
        ? { branch: branch as SoftFallbackBranch }
        : null,
    results: r.results.map((hit) => ({
      id: { space: hit.id.space, value: hit.id.value },
      kind: hit.kind,
      body: hit.body,
      score: hit.score,
      branch:
        hit.branch === "vector" || hit.branch === "text_edge" || hit.branch === "graph_arm"
          ? (hit.branch as SoftFallbackBranch)
          : "text",
      sourceId: hit.sourceId ?? null,
      ceScore: hit.ceScore ?? null,
    })),
    explanation: e ? mapCandidateNativeExplanation(e, r.results) : null,
  };
}

export function assertKnownKeys(value: object, allowed: readonly string[], name: string): void {
  const unknown = Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .sort();
  if (unknown.length > 0) {
    throw new InvalidArgumentError(`${name} has unknown field ${unknown[0]}`);
  }
}

export function validateReadContext(context: ReadContextV1): void {
  assertKnownKeys(context, ["schemaVersion", "view", "eligibility"], "ReadContextV1");
  if (context.schemaVersion !== 1) {
    throw new InvalidArgumentError("ReadContextV1.schemaVersion must be 1");
  }
  assertKnownKeys(
    context.view,
    ["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
    "ReadContextV1.view",
  );
  assertKnownKeys(
    context.eligibility,
    ["sourceType", "kind", "createdAfter", "status", "attributes"],
    "ReadContextV1.eligibility",
  );
}

export function nativeFrozenContext(context: FrozenReadContextV1): NativeFrozenReadContextV1 {
  return context;
}

/**
 * 0.8.11 Slice 40 (#17) — one term of the unified `Filter` grammar (G4 + G10),
 * a discriminated union mirroring `fathomdb_engine::FilterTerm`
 * (ADR-0.8.11, Option A). Exactly five variants: the four G10 shorthand metadata
 * fields plus the general G4 json-path `Predicate` (`json`). The `json` term is
 * accepted on `read.listFilter` but **typed-rejected** on `search` (D3: an
 * arbitrary json-path predicate is never demoted to a post-KNN `json_extract`).
 */
export type FilterTerm =
  | { term: "source_type"; value: string }
  | { term: "kind"; value: string }
  | { term: "created_after"; value: number }
  | { term: "status"; value: string }
  | { term: "json"; predicate: Predicate };

/**
 * 0.8.11 Slice 40 (#17) — the unified, closed `Filter` contract. ONE typed
 * surface (implicit-AND `terms`) dispatched to two backends: the vec0-metadata
 * indexed pre-KNN `WHERE` for `search`, and `json_extract` over
 * `canonical_nodes.body` for `read.listFilter`. The shipped `SearchFilter` (G10)
 * and `Predicate` lists (G4) re-express as sugar that lowers into this type (D4).
 * Mirrors the Python `fathomdb.filter.Filter` (cross-binding parity, X1).
 */
export interface Filter {
  terms: FilterTerm[];
}

export const VEC0_JSON_REJECT =
  "arbitrary json-path predicate not supported on search_filtered; it would " +
  "require a post-KNN json_extract that defeats the indexed pre-KNN filter " +
  "(ADR-0.8.11 D3 no-demotion guarantee)";

export function isUnifiedFilter(f: SearchFilter | Filter | undefined): f is Filter {
  return f !== undefined && Array.isArray((f as Filter).terms);
}

/**
 * vec0 (`search`) backend lowering of the unified `Filter` to the shipped
 * `SearchFilter` sugar. Typed-rejects a `json` term with `InvalidFilterError`
 * (D3 no-demotion guarantee); the lowering is canonical-order-independent.
 */
export function filterToSearchFilter(filter: Filter): SearchFilter {
  const sf: SearchFilter = {};
  for (const t of filter.terms) {
    switch (t.term) {
      case "source_type":
        sf.sourceType = t.value;
        break;
      case "kind":
        sf.kind = t.value;
        break;
      case "created_after":
        sf.createdAfter = t.value;
        break;
      case "status":
        sf.status = t.value;
        break;
      case "json":
        throw new InvalidFilterError(VEC0_JSON_REJECT);
    }
  }
  return sf;
}

/** D4 sugar: re-express a shipped `SearchFilter` as the unified `Filter`. */
export function searchFilterToFilter(sf: SearchFilter): Filter {
  if (sf.attributes !== undefined && sf.attributes.length > 0) {
    throw new InvalidFilterError(
      "projected attribute predicates are not supported by the unified Filter grammar",
    );
  }
  const terms: FilterTerm[] = [];
  if (sf.sourceType !== undefined) terms.push({ term: "source_type", value: sf.sourceType });
  if (sf.kind !== undefined) terms.push({ term: "kind", value: sf.kind });
  if (sf.createdAfter !== undefined) terms.push({ term: "created_after", value: sf.createdAfter });
  if (sf.status !== undefined) terms.push({ term: "status", value: sf.status });
  return { terms };
}

/**
 * 0.8.8 EXP-OBS (Slice 10) — query-level retrieval trace (mirror of the Rust
 * `QueryTrace`). Present only on the opt-in `search(..., { explain: true })` path,
 * inside `Explanation.trace`. `queryChars` is the query LENGTH only (never the
 * text); `embedderId` is `"name@rev (dim=N)"` (`""` when none). Field
 * names/order mirror the Python `QueryTrace` (cross-binding parity).
 */
export interface QueryTrace {
  queryChars: number;
  k: number;
  rerankDepth: number;
  poolN: number;
  alpha: number;
  useGraphArm: boolean;
  recency: boolean;
  embedderId: string;
  ceActive: boolean;
  vectorHits: number;
  textHits: number;
  graphHits: number;
  /** Edge-FTS candidates excluded only by a node-scoped attribute predicate. */
  droppedEdgeHits: number;
}

/**
 * 0.8.8 EXP-OBS (Slice 10) — per-hit provenance + score breakdown (mirror of the
 * Rust `PerHitExplain`); parallel to (and same order as) `SearchResult.results`.
 * `id` is the engine-internal positional `write_cursor` (a `number`) — NOT the
 * typed `SearchHit.id` (`IdSpace`). Correlate an explain entry to its
 * `SearchHit` by ARRAY POSITION (`perHit[i]` ↔ `results[i]`), not by id.
 * `fusedScore` is the RAW post-recency, pre-CE RRF score (not normalized).
 * `importance`/`confidence` (0.8.16 Slice 5 / F9) are the node importance / edge
 * confidence applied to this hit's contribution (`null` = graceful-absent /
 * neutral); they mirror the Python `PerHitExplain` additive fields.
 */
export interface PerHitExplain {
  id: number;
  arm: SoftFallbackBranch;
  vectorRank: number | null;
  textRank: number | null;
  graphRank: number | null;
  fusedScore: number;
  ceScore: number | null;
  blended: number;
  importance: number | null;
  confidence: number | null;
  structural?: StructuralInclusionV1;
}

export type StructuralInclusionStateV1 = "included" | "degraded";
export type StructuralProjectionOriginV1 =
  "synchronous_body_fts" | "current_dense_generation" | "graph_traversal";
export type StructuralDependencyStateV1 = "not_applicable" | "not_registered" | "registered";
export type StructuralLifecycleStateV1 =
  "node_pending" | "node_active" | "node_deleted" | "edge_valid";
export type StructuralDegradationCodeV1 =
  | "soft_fallback_text"
  | "soft_fallback_text_edge"
  | "projection_legacy_unverified"
  | "projection_blocked"
  | "projection_deferred"
  | "graph_bound_reached";

export interface StructuralInclusionV1 {
  schemaVersion: 1;
  inclusionState: StructuralInclusionStateV1;
  projectionOrigin: StructuralProjectionOriginV1;
  dependencyState: StructuralDependencyStateV1;
  lifecycleState: StructuralLifecycleStateV1;
  degradationCodes: StructuralDegradationCodeV1[];
}

/**
 * @internal — map one native per-hit explain object into the public
 * {@link PerHitExplain}. Factored out of `Engine.search` so the mapping is
 * unit-testable against a fake native object without the compiled `.node`
 * (0.8.16 Slice 5 / F9, codex §9 fix-2). `importance`/`confidence` are the
 * additive F9 fields (node importance / edge confidence applied to this hit's
 * contribution; `null` = graceful-absent / neutral), symmetric with the Python
 * `_map_per_hit_explain` wrapper.
 */
export function mapPerHitExplain(p: NativePerHitExplain): PerHitExplain {
  const invalid = (path: string): never => {
    throw new FathomDbError(`invalid explanation response at ${path}`);
  };
  const optionalU32 = (value: number | null | undefined, path: string): void => {
    if (
      value !== null &&
      value !== undefined &&
      (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff)
    ) {
      invalid(path);
    }
  };
  const finite = (value: number | null | undefined, path: string, optional = false): void => {
    if (optional && (value === null || value === undefined)) return;
    if (typeof value !== "number" || !Number.isFinite(value)) invalid(path);
  };
  if (!Number.isSafeInteger(p.id) || p.id < 0) invalid("/id");
  if (p.arm !== "vector" && p.arm !== "text" && p.arm !== "text_edge" && p.arm !== "graph_arm") {
    invalid("/arm");
  }
  optionalU32(p.vectorRank, "/vectorRank");
  optionalU32(p.textRank, "/textRank");
  optionalU32(p.graphRank, "/graphRank");
  finite(p.fusedScore, "/fusedScore");
  finite(p.ceScore, "/ceScore", true);
  finite(p.blended, "/blended");
  finite(p.importance, "/importance", true);
  finite(p.confidence, "/confidence", true);
  const result: PerHitExplain = {
    id: p.id,
    arm: p.arm as SoftFallbackBranch,
    vectorRank: p.vectorRank ?? null,
    textRank: p.textRank ?? null,
    graphRank: p.graphRank ?? null,
    fusedScore: p.fusedScore,
    ceScore: p.ceScore ?? null,
    blended: p.blended,
    importance: p.importance ?? null,
    confidence: p.confidence ?? null,
  };
  if (p.structural !== undefined) {
    const structural = p.structural;
    if (structural === null || typeof structural !== "object") invalid("/structural");
    if (structural.schemaVersion !== 1) invalid("/structural/schemaVersion");
    if (structural.inclusionState !== "included" && structural.inclusionState !== "degraded") {
      invalid("/structural/inclusionState");
    }
    if (
      structural.projectionOrigin !== "synchronous_body_fts" &&
      structural.projectionOrigin !== "current_dense_generation" &&
      structural.projectionOrigin !== "graph_traversal"
    ) {
      invalid("/structural/projectionOrigin");
    }
    if (
      structural.dependencyState !== "not_applicable" &&
      structural.dependencyState !== "not_registered" &&
      structural.dependencyState !== "registered"
    ) {
      invalid("/structural/dependencyState");
    }
    if (
      structural.lifecycleState !== "node_pending" &&
      structural.lifecycleState !== "node_active" &&
      structural.lifecycleState !== "node_deleted" &&
      structural.lifecycleState !== "edge_valid"
    ) {
      invalid("/structural/lifecycleState");
    }
    if (!Array.isArray(structural.degradationCodes)) {
      invalid("/structural/degradationCodes");
    }
    const degradationOrder = [
      "soft_fallback_text",
      "soft_fallback_text_edge",
      "projection_legacy_unverified",
      "projection_blocked",
      "projection_deferred",
      "graph_bound_reached",
    ] as const;
    let previous = -1;
    structural.degradationCodes.forEach((code, index) => {
      const ordinal = degradationOrder.indexOf(code as (typeof degradationOrder)[number]);
      if (ordinal < 0 || ordinal <= previous) {
        invalid(`/structural/degradationCodes/${index}`);
      }
      previous = ordinal;
    });
    if ((structural.inclusionState === "included") !== (structural.degradationCodes.length === 0)) {
      invalid("/structural/inclusionState");
    }
    result.structural = {
      schemaVersion: 1,
      inclusionState: structural.inclusionState as StructuralInclusionStateV1,
      projectionOrigin: structural.projectionOrigin as StructuralProjectionOriginV1,
      dependencyState: structural.dependencyState as StructuralDependencyStateV1,
      lifecycleState: structural.lifecycleState as StructuralLifecycleStateV1,
      degradationCodes: structural.degradationCodes as StructuralDegradationCodeV1[],
    };
  }
  return result;
}

export function mapCandidateNativeExplanation(
  value: NonNullable<NativeSearchResult["explanation"]>,
  nativeResults: NativeSearchResult["results"],
): Explanation {
  const invalid = (path: string): never => {
    throw new FathomDbError(`invalid explanation response at ${path}`);
  };
  const u32 = (candidate: unknown, path: string, positive = false): number => {
    if (
      !Number.isInteger(candidate) ||
      (candidate as number) < 0 ||
      (candidate as number) > 0xffff_ffff ||
      (positive && candidate === 0)
    ) {
      invalid(path);
    }
    return candidate as number;
  };
  const trace = value.trace;
  if (trace === null || typeof trace !== "object") invalid("/trace");
  const queryChars = u32(trace.queryChars, "/trace/queryChars");
  const k = u32(trace.k, "/trace/k", true);
  if (k > 100) invalid("/trace/k");
  const rerankDepth = u32(trace.rerankDepth, "/trace/rerankDepth");
  const poolN = u32(trace.poolN, "/trace/poolN");
  if (typeof trace.alpha !== "number" || !Number.isFinite(trace.alpha)) {
    invalid("/trace/alpha");
  }
  if (typeof trace.useGraphArm !== "boolean") invalid("/trace/useGraphArm");
  if (typeof trace.recency !== "boolean") invalid("/trace/recency");
  if (typeof trace.embedderId !== "string") invalid("/trace/embedderId");
  if (typeof trace.ceActive !== "boolean") invalid("/trace/ceActive");
  const vectorHits = u32(trace.vectorHits, "/trace/vectorHits");
  const textHits = u32(trace.textHits, "/trace/textHits");
  const graphHits = u32(trace.graphHits, "/trace/graphHits");
  const droppedEdgeHits = u32(trace.droppedEdgeHits, "/trace/droppedEdgeHits");
  if (
    value.correlationId !== undefined &&
    (typeof value.correlationId !== "string" ||
      !/^(?:q[0-9]+|x[0-9a-f]{32})-(?:0|[1-9][0-9]*)$/.test(value.correlationId))
  ) {
    invalid("/correlationId");
  }
  if (!Array.isArray(value.perHit) || value.perHit.length !== nativeResults.length) {
    invalid("/perHit");
  }
  const seenIds = new Set<number>();
  const allowedArms = new Set(["vector", "text", "text_edge", "graph_arm"]);
  const perHit = value.perHit.map((nativeExplain, index) => {
    const prefix = `/perHit/${index}`;
    let mapped: PerHitExplain;
    try {
      mapped = mapPerHitExplain(nativeExplain);
    } catch (error) {
      const marker = "invalid explanation response at ";
      if (error instanceof FathomDbError && error.message.startsWith(marker)) {
        invalid(`${prefix}${error.message.slice(marker.length)}`);
      }
      throw error;
    }
    if (seenIds.has(mapped.id)) invalid(`${prefix}/id`);
    seenIds.add(mapped.id);
    const nativeHit = nativeResults[index]!;
    if (!allowedArms.has(nativeHit.branch)) invalid(`/results/${index}/branch`);
    if (typeof nativeHit.score !== "number" || !Number.isFinite(nativeHit.score)) {
      invalid(`/results/${index}/score`);
    }
    const hitCeScore = nativeHit.ceScore ?? null;
    if (
      hitCeScore !== null &&
      (typeof hitCeScore !== "number" ||
        !Number.isFinite(hitCeScore) ||
        hitCeScore < 0 ||
        hitCeScore > 1)
    ) {
      invalid(`/results/${index}/ceScore`);
    }
    if (mapped.arm !== nativeHit.branch) invalid(`${prefix}/arm`);
    if (mapped.blended !== nativeHit.score) invalid(`${prefix}/blended`);
    if (mapped.ceScore !== hitCeScore) invalid(`${prefix}/ceScore`);
    return mapped;
  });
  return {
    trace: {
      queryChars,
      k,
      rerankDepth,
      poolN,
      alpha: trace.alpha,
      useGraphArm: trace.useGraphArm,
      recency: trace.recency,
      embedderId: trace.embedderId,
      ceActive: trace.ceActive,
      vectorHits,
      textHits,
      graphHits,
      droppedEdgeHits,
    },
    perHit,
    correlationId: value.correlationId,
  };
}

/**
 * 0.8.8 EXP-OBS (Slice 10) — opt-in retrieval explanation sidecar (mirror of the
 * Rust `Explanation`): a query-level `trace` + a per-hit breakdown. Returned when
 * ordinary or frozen retrieval explicitly requests explanation; `null` (default)
 * keeps the result byte-identical to the pre-0.8.8 shape.
 */
export interface Explanation {
  trace: QueryTrace;
  perHit: PerHitExplain[];
  correlationId?: string;
}

export interface SearchResult {
  projectionCursor: number;
  softFallback: SoftFallback | null;
  results: SearchHit[];
  /**
   * Opt-in explanation sidecar; `null` unless the retrieval requests explanation.
   */
  explanation: Explanation | null;
}

export async function freezeReadContext(nativeEngine: NativeEngine, context: ReadContextV1): Promise<FrozenReadContextV1> {
    validateReadContext(context);
    const frozen = await intercept(() => nativeEngine.freezeReadContext(context));
    if (frozen.schemaVersion !== 1 || frozen.context.schemaVersion !== 1) {
      throw new FrozenReadError(
        "unsupported_schema_version at /schemaVersion",
        "unsupported_schema_version",
        "/schemaVersion",
      );
    }
    return {
      schemaVersion: 1,
      effectiveValidAt: frozen.effectiveValidAt,
      context: {
        schemaVersion: 1,
        view: frozen.context.view,
        eligibility: {
          sourceType: frozen.context.eligibility.sourceType,
          kind: frozen.context.eligibility.kind,
          createdAfter: frozen.context.eligibility.createdAfter,
          status: frozen.context.eligibility.status,
          attributes: frozen.context.eligibility.attributes?.map(
            (pair) => [pair[0]!, pair[1]!] as [string, string],
          ),
        },
      },
      token: frozen.token,
    };
  }

export async function searchFrozen(nativeEngine: NativeEngine, query: string, context: FrozenReadContextV1, options: FrozenSearchOptions = {}): Promise<SearchResult> {
    const nativeContext = nativeFrozenContext(context);
    await intercept(() => nativeEngine.validateFrozenReadContext(nativeContext));
    validateFfiString(query);
    validateReadContext(context.context);
    assertKnownKeys(
      context,
      ["schemaVersion", "effectiveValidAt", "context", "token"],
      "FrozenReadContextV1",
    );
    assertKnownKeys(
      options,
      ["rerankDepth", "useGraphArm", "alpha", "poolN", "explain", "limit"],
      "FrozenSearchOptions",
    );
    if (options.rerankDepth !== undefined) {
      if (options.rerankDepth < 0) {
        throw new InvalidArgumentError(`rerankDepth must be >= 0; got ${options.rerankDepth}`);
      }
      if (!Number.isInteger(options.rerankDepth) || options.rerankDepth > 0xffffffff) {
        throw new RangeError(
          `rerankDepth must be an integer in 0..=4294967295; got ${options.rerankDepth}`,
        );
      }
    }
    if (options.useGraphArm !== undefined && typeof options.useGraphArm !== "boolean") {
      throw new TypeError(`useGraphArm must be a boolean, got ${typeof options.useGraphArm}`);
    }
    if (
      options.alpha !== undefined &&
      (typeof options.alpha !== "number" || !Number.isFinite(options.alpha))
    ) {
      throw new RangeError(`alpha must be a finite number, got ${options.alpha}`);
    }
    if (options.poolN !== undefined) {
      if (options.poolN < 0) {
        throw new InvalidArgumentError(`poolN must be >= 0; got ${options.poolN}`);
      }
      if (!Number.isInteger(options.poolN) || options.poolN > 0xffffffff) {
        throw new RangeError(`poolN must be an integer in 0..=4294967295; got ${options.poolN}`);
      }
    }
    if (options.explain !== undefined && typeof options.explain !== "boolean") {
      throw new TypeError(`explain must be a boolean, got ${typeof options.explain}`);
    }
    validateRankedResultLimit("limit", options.limit);
    const result = await intercept(() =>
      nativeEngine.searchFrozen(
        query,
        nativeContext,
        options.rerankDepth,
        options.useGraphArm,
        options.alpha,
        options.poolN,
        options.explain,
        options.limit,
      ),
    );
    return mapNativeSearchResult(result);
  }

export async function searchExpandFrozen(nativeEngine: NativeEngine, query: string, context: FrozenReadContextV1, depth: number, options: SearchExpandOptions = {}): Promise<SearchExpandResult> {
    const nativeContext = nativeFrozenContext(context);
    await intercept(() => nativeEngine.validateFrozenReadContext(nativeContext));
    validateFfiString(query);
    validateReadContext(context.context);
    assertKnownKeys(
      context,
      ["schemaVersion", "effectiveValidAt", "context", "token"],
      "FrozenReadContextV1",
    );
    assertKnownKeys(options, ["searchLimit"], "SearchExpandOptions");
    if (!Number.isInteger(depth) || depth < 0 || depth > 3) {
      throw new InvalidArgumentError(
        `searchExpandFrozen depth must be an integer between 0 and 3; got ${depth}`,
      );
    }
    validateRankedResultLimit("searchLimit", options.searchLimit);
    const result = await intercept(() =>
      nativeEngine.searchExpandFrozen(query, nativeContext, depth, options.searchLimit),
    );
    return {
      searchHits: result.searchHits.map((hit) => ({
        id: { space: hit.id.space, value: hit.id.value },
        kind: hit.kind,
        body: hit.body,
        score: hit.score,
        branch:
          hit.branch === "vector" || hit.branch === "text_edge" || hit.branch === "graph_arm"
            ? hit.branch
            : "text",
        sourceId: hit.sourceId ?? null,
        ceScore: hit.ceScore ?? null,
      })),
      expanded: result.expanded,
      allLogicalIds: result.allLogicalIds,
    };
  }

export async function search(nativeEngine: NativeEngine, query: string, filter?: SearchFilter | Filter, rerankDepth?: number, useGraphArm?: boolean, alpha?: number, poolN?: number, explain?: boolean, view?: SearchOptions): Promise<SearchResult> {
    validateFfiString(query);
    // 0.8.11 Slice 40 (#17) — accept the unified Filter on the vec0 search path;
    // lower to the SearchFilter sugar (typed-rejects a `json` term, D3).
    if (isUnifiedFilter(filter)) {
      filter = filterToSearchFilter(filter);
    }
    // G10 filter strings cross the FFI like `query` and must clear the same
    // AC-068a/AC-068b guard. napi-rs lossily replaces lone UTF-16 surrogates
    // with U+FFFD before the Rust-side guard runs (see validation.ts), so —
    // exactly like write/configure — the surrogate check must happen JS-side.
    // `createdAfter` is numeric (no string validation).
    if (filter !== undefined) {
      if (filter.sourceType !== undefined) validateFfiString(filter.sourceType);
      if (filter.kind !== undefined) validateFfiString(filter.kind);
      if (filter.status !== undefined) validateFfiString(filter.status);
      if (filter.attributes !== undefined) {
        for (const pair of filter.attributes) {
          if (!Array.isArray(pair) || pair.length !== 2) {
            throw new InvalidFilterError(
              "attribute predicates must be [name, canonicalText] pairs",
            );
          }
          validateFfiString(pair[0]);
          validateFfiString(pair[1]);
        }
      }
    }
    // 0.8.1 R1: rerankDepth validation (must be a non-negative integer <= u32::MAX).
    // FIX-5: changed TypeError → RangeError for non-integer (consistency with
    //   validateLimit and graph depth checks).
    // FIX-5: added u32::MAX upper-bound guard (napi_get_value_uint32 wraps mod 2^32).
    // FIX-7: removed `?? undefined` no-op (rerankDepth is already `number | undefined`).
    if (rerankDepth !== undefined) {
      if (!Number.isInteger(rerankDepth)) {
        throw new RangeError(`rerankDepth must be an integer, got ${typeof rerankDepth}`);
      }
      if (rerankDepth < 0) {
        throw new RangeError(`rerankDepth must be >= 0, got ${rerankDepth}`);
      }
      if (rerankDepth > 0xffffffff) {
        throw new RangeError(`rerankDepth must be <= 4294967295 (u32 max), got ${rerankDepth}`);
      }
    }
    // 0.8.1 R3 (Slice 30): useGraphArm validation.
    if (useGraphArm !== undefined && typeof useGraphArm !== "boolean") {
      throw new TypeError(`useGraphArm must be a boolean, got ${typeof useGraphArm}`);
    }
    // 0.8.5 (EXP-0): alpha is a finite number (clamped to [0,1] in the engine);
    // poolN is a non-negative integer <= u32::MAX (mirrors the rerankDepth guard).
    if (alpha !== undefined && (typeof alpha !== "number" || !Number.isFinite(alpha))) {
      throw new RangeError(`alpha must be a finite number, got ${alpha}`);
    }
    if (poolN !== undefined) {
      if (!Number.isInteger(poolN)) {
        throw new RangeError(`poolN must be an integer, got ${typeof poolN}`);
      }
      if (poolN < 0) {
        throw new RangeError(`poolN must be >= 0, got ${poolN}`);
      }
      if (poolN > 0xffffffff) {
        throw new RangeError(`poolN must be <= 4294967295 (u32 max), got ${poolN}`);
      }
    }
    // 0.8.8 EXP-OBS (Slice 10): explain validation (mirrors useGraphArm + the
    // Python `search` guard, cross-SDK parity).
    if (explain !== undefined && typeof explain !== "boolean") {
      throw new TypeError(`explain must be a boolean, got ${typeof explain}`);
    }
    const searchOptions = splitSearchOptions(view);
    const r = await intercept(() =>
      nativeEngine.search(
        query,
        filter,
        rerankDepth,
        useGraphArm,
        alpha,
        poolN,
        explain,
        searchOptions.view,
        searchOptions.limit,
      ),
    );
    return mapNativeSearchResult(r);
  }

export async function searchTextOnly(nativeEngine: NativeEngine, query: string, view?: SearchOptions): Promise<SearchResult> {
    validateFfiString(query);
    const searchOptions = splitSearchOptions(view);
    const r = await intercept(() =>
      nativeEngine.searchTextOnly(query, searchOptions.view, searchOptions.limit),
    );
    const branch = r.softFallback?.branch;
    return {
      projectionCursor: r.projectionCursor,
      softFallback:
        branch === "vector" || branch === "text" || branch === "text_edge" || branch === "graph_arm"
          ? { branch: branch as SoftFallbackBranch }
          : null,
      results: r.results.map((h) => ({
        id: { space: h.id.space, value: h.id.value },
        kind: h.kind,
        body: h.body,
        score: h.score,
        branch:
          h.branch === "vector" || h.branch === "text_edge" || h.branch === "graph_arm"
            ? (h.branch as SoftFallbackBranch)
            : "text",
        sourceId: h.sourceId ?? null,
        ceScore: h.ceScore ?? null,
      })),
      explanation: null,
    };
  }

export async function searchProjectedText(nativeEngine: NativeEngine, query: string, name: string, filter?: SearchFilter, view?: SearchOptions): Promise<SearchResult> {
    validateFfiString(query);
    validateFfiString(name);
    if (filter?.sourceType !== undefined) validateFfiString(filter.sourceType);
    if (filter?.kind !== undefined) validateFfiString(filter.kind);
    if (filter?.status !== undefined) validateFfiString(filter.status);
    if (filter?.attributes !== undefined) {
      for (const pair of filter.attributes) {
        if (!Array.isArray(pair) || pair.length !== 2) {
          throw new InvalidFilterError("attribute predicates must be [name, canonicalText] pairs");
        }
        validateFfiString(pair[0]);
        validateFfiString(pair[1]);
      }
    }
    const searchOptions = splitSearchOptions(view);
    const r = await intercept(() =>
      nativeEngine.searchProjectedText(
        query,
        name,
        filter,
        searchOptions.view,
        searchOptions.limit,
      ),
    );
    return {
      projectionCursor: r.projectionCursor,
      softFallback: null,
      results: r.results.map((h) => ({
        id: { space: h.id.space, value: h.id.value },
        kind: h.kind,
        body: h.body,
        score: h.score,
        branch: "text" as SoftFallbackBranch,
        sourceId: h.sourceId ?? null,
        ceScore: h.ceScore ?? null,
      })),
      explanation: null,
    };
  }
