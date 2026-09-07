// FathomDB TypeScript SDK public surface.
//
// Five-verb top-level surface (Engine.open, engine.write, engine.search,
// engine.close, admin.configure), engine-attached instrumentation, and
// the FathomDbError leaf-class hierarchy per
// `dev/interfaces/typescript.md` and `dev/design/bindings.md` § 3. The
// runtime is the napi-rs binding in `fathomdb-napi`; this file is a
// thin TS wrapper that funnels every native error through
// `rethrowTyped`.

import {
  native,
  type NativeCudaDeviceInfo,
  type NativeCudaVisibleDevice,
  type NativeEffectiveEmbedDevice,
  type NativeEmbedderDeviceResolution,
  type NativeEmbedderEvent,
  type NativeEngine,
  type NativeEvidenceSearchResultV1,
  type NativeFrozenReadContextV1,
  type NativeGpuAllocationWitness,
  type NativeOpenReport,
  type NativePerHitExplain,
  type NativeResolvedEvidenceV1,
  type NativeSearchResult,
} from "./binding.js";
import {
  ActuationError,
  DependencyClosureError,
  DependencyError,
  DependencyTraceError,
  EvidenceError,
  FathomDbError,
  FrozenReadError,
  GraphExpansionError,
  InvalidArgumentError,
  InvalidFilterError,
  rethrowTyped,
} from "./errors.js";
import type { NodeRecord, Predicate, ReadView } from "./read.js";
import {
  validateNativeEvidenceSearch,
  validateNativeResolvedEvidence,
} from "./evidence-validation.js";
import {
  validateFfiString,
  validateFfiTree,
  sanitizeActuationFfiTree,
  validateWriteFfiTree,
} from "./validation.js";

export * from "./errors.js";
export { read } from "./read.js";
export type {
  BoundaryCrossing,
  NodeRecord,
  OperationalStateRecordV1,
  OpStoreRow,
  PageCursor,
  PageRequestV1,
  PageV1,
  Predicate,
  ReadCollectionOptions,
  ReadView,
} from "./read.js";

/**
 * OPP-12 Phase-1 (0.8.19 Slice 10) — the closed lifecycle existence-state
 * vocabulary accepted by {@link Engine.transition} (`toState`). `pending` and
 * `purged` are never legal `transition` targets (create-time-only and
 * purge-only respectively) — passing them surfaces an `IllegalTransitionError`.
 */
export type LifecycleState = "pending" | "active" | "deleted" | "purged";

/**
 * Embed `texts` with the pinned default BGE-small model using CLS pooling.
 *
 * This is the TypeScript peer of Python's `embed_batch_cls`. It returns one
 * L2-normalized vector per input in input order; `[]` returns `[]` without
 * loading weights. It is intentionally distinct from {@link Engine.embed},
 * which uses the engine's Mean-pooling read path. The published native package
 * includes the default embedder; a custom thin build rejects with
 * `EmbedderNotConfiguredError`.
 */
export async function embedBatchCls(texts: readonly string[]): Promise<number[][]> {
  const batch = Array.from(texts);
  for (const text of batch) validateFfiString(text);
  return intercept(() => native.embedBatchCls(batch));
}

export interface EngineConfig {
  embedderPoolSize?: number;
  schedulerRuntimeThreads?: number;
  provenanceRowCap?: number;
  embedderCallTimeoutMs?: number;
  slowThresholdMs?: number;
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

export interface WriteReceipt {
  cursor: number;
  /**
   * G0 (Slice 15) — per-row `write_cursor`s, 1:1 with the input batch order.
   * The `write_cursor`-as-row-id identity carrier; for an N-row batch this is
   * `[cursor-N+1, …, cursor]`.
   */
  rowCursors: number[];
  /**
   * G8 (Slice 20 / F10) — count of edge endpoints in this batch that point at a
   * non-existent or superseded canonical node (an active node carrying that
   * `logical_id`). `from_id`/`to_id` are probed independently, so one edge
   * contributes 0, 1, or 2. Informational only: the batch commits regardless
   * (flag-and-count). `0` when the batch committed no active edges.
   */
  danglingEdgeEndpoints: number;
}

/** Exact locator covering the entire canonical source body. */
export interface WholeBodySourceLocator {
  kind: "whole_body";
}

/** Half-open UTF-8 byte locator; offsets are canonical decimal strings. */
export interface Utf8BytesSourceLocator {
  kind: "utf8_bytes";
  startInclusive: string;
  endExclusive: string;
}

/** Closed v1 source-locator union. */
export type SourceLocator = WholeBodySourceLocator | Utf8BytesSourceLocator;

/** SHA-256 of the entire canonical source revision's stored UTF-8 bytes. */
export interface CanonicalHash {
  algorithm: "sha256";
  digestHex: string;
}

/** Complete provenance for a canonical source node. */
export interface CanonicalWriteProvenanceV1 {
  schemaVersion: 1;
  role: "canonical";
  artifactRevisionId: string;
  sourceVersionId: string;
}

/** Complete provenance for an artifact derived from a canonical source. */
export interface DerivedWriteProvenanceV1 {
  schemaVersion: 1;
  role: "derived";
  artifactRevisionId: string;
  sourceVersionId: string;
  sourceRevisionId: string;
  sourceLocator: SourceLocator;
  canonicalSourceHash: CanonicalHash;
}

/** Closed schema-version-1 provenance accepted by versioned writes. */
export type WriteProvenanceV1 = CanonicalWriteProvenanceV1 | DerivedWriteProvenanceV1;

/** Closed request registering one immutable source dependency. */
export interface SourceDependencyRegistrationV1 {
  schemaVersion: 1;
  dependencyId: string;
  sourceRevisionId: string;
  derivedRevisionId: string;
}

/** Closed source-side dependency lookup request. */
export interface DependencySourceLookupV1 {
  schemaVersion: 1;
  sourceRevisionId: string;
}

/** Closed derived-side dependency lookup request. */
export interface DependencyDerivedLookupV1 {
  schemaVersion: 1;
  derivedRevisionId: string;
}

/** One immutable dependency with its independent registration generation. */
export interface SourceDependencyV1 {
  schemaVersion: 1;
  dependencyId: string;
  sourceRevisionId: string;
  derivedRevisionId: string;
  registeredDependencyGeneration: string;
}

/** Bounded, deterministically ordered dependency result. */
export interface DependencyListV1 {
  schemaVersion: 1;
  items: SourceDependencyV1[];
}

/** Closed lookup for one opaque Engine-minted closure operation. */
export interface ClosureLookupV1 {
  schemaVersion: 1;
  closureOperationId: string;
}

/** Scalar zero-proof for a completed dependency closure. */
export interface ClosureProofV1 {
  schemaVersion: 1;
  proofWriteBoundary: string;
  currentActiveDependentNodes: string;
  currentDerivedEdges: string;
  viewEligibleDependents: string;
  ownerlessProjectionRows: string;
  postAdmissionRegistrations: string;
  remainingDependencyRows: string | null;
  remainingCanonicalRows: string | null;
  remainingProjectionRows: string | null;
  remainingReceiptReferenceRows: string | null;
}

/** Current durable status of one dependency closure. */
export interface ClosureStatusV1 {
  schemaVersion: 1;
  closureOperationId: string;
  root:
    | { type: "source_revision"; sourceRevisionId: string }
    | { type: "source_bucket"; sourceId: string };
  cause: "superseded" | "soft_deleted" | "purged" | "source_erased";
  phase: "proving" | "at_rest_pending" | "complete" | "incomplete";
  effectiveAtEpochS: string;
  admittedWriteBoundary: string;
  admittedDependencyGeneration: string;
  affectedCount: string;
  blockerCode: string | null;
  proof: ClosureProofV1 | null;
}

export type ActuationOperationV1 =
  | { type: "put_canonical_node"; record: Record<string, unknown> }
  | { type: "put_derived_node"; record: Record<string, unknown> }
  | {
      type: "register_source_dependency";
      dependency: SourceDependencyRegistrationV1;
    }
  | {
      type: "transition_lifecycle";
      logicalId: string;
      expectedCurrentRevisionId: string;
      toState: "active" | "deleted";
      reason?: string | null;
    };

/** Bounded, model-free, caller-decided atomic actuation request. */
export interface ActuationBatchV1 {
  schemaVersion: 1;
  operationId: string;
  decisionPolicyId?: string | null;
  expectedWriteBoundary?: string | null;
  operations: ActuationOperationV1[];
}

/** Compact terminal receipt for one actuation operation ID. */
export interface ActuationReceiptV1 {
  schemaVersion: 1;
  operationId: string;
  requestSha256: string;
  outcome: "committed" | "committed_closure_pending" | "refused";
  refusedOperationIndex: number | null;
  refusedFieldPath: string | null;
  reasonCodes: string[];
  affectedRevisionIds: string[];
  resultingWriteBoundary: string | null;
  resultingDependencyGeneration: string | null;
  pendingProjectionWriteCursors: string[];
  projectionGenerationId: string | null;
  closureOperationIds: string[];
}

export type ProjectionReadinessV1 = "ready" | "processing" | "blocked" | "deferred" | "degraded";
export type ProjectionRuntimeStateV1 = "absent" | "usable" | "refused";
export type ProjectionGenerationOriginV1 =
  "fresh" | "legacy_unverified" | "configuration" | "rebuild";

export interface ProjectionGenerationStatusV1 {
  readonly schemaVersion: 1;
  readonly generationId: string;
  readonly declarationSha256: string;
  readonly origin: ProjectionGenerationOriginV1;
  readonly transitionBoundary: string;
  readonly effectiveAtEpochS: number;
  readonly observedBoundary: string;
  readonly readyThrough: string;
  readonly readiness: ProjectionReadinessV1;
  readonly runtimeState: ProjectionRuntimeStateV1;
  readonly pendingCount: string;
  readonly failedCount: string;
}

export interface MutationProjectionStatusRequestV1 {
  readonly schemaVersion: 1;
  readonly operationId: string;
  readonly writeCursor: string;
  readonly expectedGenerationId: string;
}

export interface MutationProjectionStatusV1 {
  readonly schemaVersion: 1;
  readonly operationId: string;
  readonly writeCursor: string;
  readonly generationId: string;
  readonly effectiveAtEpochS: number;
  readonly observedBoundary: string;
  readonly readyThrough: string;
  readonly readiness: ProjectionReadinessV1;
  readonly runtimeState: ProjectionRuntimeStateV1;
  readonly pendingCount: string;
  readonly failedCount: string;
}

function closureResponseError(fieldPath: string): never {
  throw new DependencyClosureError(
    `dependency closure response invalid at ${fieldPath}`,
    "unknown_field",
    fieldPath,
  );
}

function closureRecord(value: unknown, fieldPath: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    closureResponseError(fieldPath);
  }
  return value as Record<string, unknown>;
}

function closureExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  fieldPath: string,
): void {
  const allowedSet = new Set(allowed);
  const unknown = Object.keys(value).find((key) => !allowedSet.has(key));
  if (unknown !== undefined) closureResponseError(`${fieldPath}/${unknown}`);
}

function closureString(value: unknown, fieldPath: string): string {
  if (typeof value !== "string" || value.length === 0) closureResponseError(fieldPath);
  return value;
}

function closureDecimal(value: unknown, fieldPath: string, signed = false): string {
  const parsed = closureString(value, fieldPath);
  const pattern = signed ? /^(0|-[1-9][0-9]*|[1-9][0-9]*)$/ : /^(0|[1-9][0-9]*)$/;
  if (!pattern.test(parsed)) closureResponseError(fieldPath);
  return parsed;
}

function closureProofResponse(value: unknown): ClosureProofV1 {
  const proof = closureRecord(value, "/proof");
  closureExactKeys(
    proof,
    [
      "schemaVersion",
      "proofWriteBoundary",
      "currentActiveDependentNodes",
      "currentDerivedEdges",
      "viewEligibleDependents",
      "ownerlessProjectionRows",
      "postAdmissionRegistrations",
      "remainingDependencyRows",
      "remainingCanonicalRows",
      "remainingProjectionRows",
      "remainingReceiptReferenceRows",
    ],
    "/proof",
  );
  if (proof.schemaVersion !== 1) closureResponseError("/proof/schemaVersion");
  const nullableDecimal = (field: string): string | null =>
    proof[field] === null ? null : closureDecimal(proof[field], `/proof/${field}`);
  return {
    schemaVersion: 1,
    proofWriteBoundary: closureDecimal(proof.proofWriteBoundary, "/proof/proofWriteBoundary"),
    currentActiveDependentNodes: closureDecimal(
      proof.currentActiveDependentNodes,
      "/proof/currentActiveDependentNodes",
    ),
    currentDerivedEdges: closureDecimal(proof.currentDerivedEdges, "/proof/currentDerivedEdges"),
    viewEligibleDependents: closureDecimal(
      proof.viewEligibleDependents,
      "/proof/viewEligibleDependents",
    ),
    ownerlessProjectionRows: closureDecimal(
      proof.ownerlessProjectionRows,
      "/proof/ownerlessProjectionRows",
    ),
    postAdmissionRegistrations: closureDecimal(
      proof.postAdmissionRegistrations,
      "/proof/postAdmissionRegistrations",
    ),
    remainingDependencyRows: nullableDecimal("remainingDependencyRows"),
    remainingCanonicalRows: nullableDecimal("remainingCanonicalRows"),
    remainingProjectionRows: nullableDecimal("remainingProjectionRows"),
    remainingReceiptReferenceRows: nullableDecimal("remainingReceiptReferenceRows"),
  };
}

function closureResponse(value: unknown): ClosureStatusV1 {
  const status = closureRecord(value, "");
  closureExactKeys(
    status,
    [
      "schemaVersion",
      "closureOperationId",
      "root",
      "cause",
      "phase",
      "effectiveAtEpochS",
      "admittedWriteBoundary",
      "admittedDependencyGeneration",
      "affectedCount",
      "blockerCode",
      "proof",
    ],
    "",
  );
  if (status.schemaVersion !== 1) {
    throw new DependencyClosureError(
      "dependency closure unsupported_schema_version at /schemaVersion",
      "unsupported_schema_version",
      "/schemaVersion",
    );
  }
  const closureOperationId = closureString(status.closureOperationId, "/closureOperationId");
  if (!/^_fdb:c:[0-9a-f]{64}$/.test(closureOperationId)) {
    throw new DependencyClosureError(
      "dependency closure closure_operation_id_invalid at /closureOperationId",
      "closure_operation_id_invalid",
      "/closureOperationId",
    );
  }
  const rootValue = closureRecord(status.root, "/root");
  const rootType = rootValue.type;
  let root: ClosureStatusV1["root"];
  if (rootType === "source_revision") {
    closureExactKeys(rootValue, ["type", "sourceRevisionId"], "/root");
    root = {
      type: "source_revision",
      sourceRevisionId: closureString(rootValue.sourceRevisionId, "/root/sourceRevisionId"),
    };
  } else if (rootType === "source_bucket") {
    closureExactKeys(rootValue, ["type", "sourceId"], "/root");
    root = {
      type: "source_bucket",
      sourceId: closureString(rootValue.sourceId, "/root/sourceId"),
    };
  } else {
    closureResponseError("/root/type");
  }
  const cause = closureString(status.cause, "/cause");
  if (!["superseded", "soft_deleted", "purged", "source_erased"].includes(cause)) {
    closureResponseError("/cause");
  }
  const phase = closureString(status.phase, "/phase");
  if (!["proving", "at_rest_pending", "complete", "incomplete"].includes(phase)) {
    closureResponseError("/phase");
  }
  const blockerCode = status.blockerCode;
  if (
    blockerCode !== null &&
    ![
      "projection_state_unavailable",
      "proof_unavailable",
      "telemetry_redaction",
      "wal_checkpoint",
    ].includes(String(blockerCode))
  ) {
    closureResponseError("/blockerCode");
  }
  if (blockerCode !== null && typeof blockerCode !== "string") {
    closureResponseError("/blockerCode");
  }
  const proof = status.proof === null ? null : closureProofResponse(status.proof);
  const physicalCause = cause === "purged" || cause === "source_erased";
  if ((phase === "proving" && physicalCause) || (phase === "at_rest_pending" && !physicalCause)) {
    closureResponseError("/phase");
  }
  if ((phase === "incomplete") !== (blockerCode !== null)) {
    closureResponseError("/blockerCode");
  }
  const proofShapeValid =
    (phase === "complete" && proof !== null) ||
    (phase === "proving" && proof === null) ||
    (phase === "at_rest_pending" && proof !== null) ||
    (phase === "incomplete" && (proof !== null) === physicalCause);
  if (!proofShapeValid) closureResponseError("/proof");
  return {
    schemaVersion: 1,
    closureOperationId,
    root,
    cause: cause as ClosureStatusV1["cause"],
    phase: phase as ClosureStatusV1["phase"],
    effectiveAtEpochS: closureDecimal(status.effectiveAtEpochS, "/effectiveAtEpochS", true),
    admittedWriteBoundary: closureDecimal(status.admittedWriteBoundary, "/admittedWriteBoundary"),
    admittedDependencyGeneration: closureDecimal(
      status.admittedDependencyGeneration,
      "/admittedDependencyGeneration",
    ),
    affectedCount: closureDecimal(status.affectedCount, "/affectedCount"),
    blockerCode,
    proof,
  };
}

function dependencyResponse(value: {
  schemaVersion: number;
  dependencyId: string;
  sourceRevisionId: string;
  derivedRevisionId: string;
  registeredDependencyGeneration: string;
}): SourceDependencyV1 {
  if (value.schemaVersion !== 1) {
    throw new DependencyError(
      "dependency unsupported_schema_version at /schemaVersion",
      "unsupported_schema_version",
      "/schemaVersion",
    );
  }
  return { ...value, schemaVersion: 1 };
}

function actuationResponse(value: {
  schemaVersion: number;
  operationId: string;
  requestSha256: string;
  outcome: string;
  refusedOperationIndex?: number | null;
  refusedFieldPath?: string | null;
  reasonCodes: string[];
  affectedRevisionIds: string[];
  resultingWriteBoundary?: string | null;
  resultingDependencyGeneration?: string | null;
  pendingProjectionWriteCursors: string[];
  projectionGenerationId?: string | null;
  closureOperationIds: string[];
}): ActuationReceiptV1 {
  if (
    value.schemaVersion !== 1 ||
    !["committed", "committed_closure_pending", "refused"].includes(value.outcome)
  ) {
    throw new ActuationError(
      "actuation response has an unsupported schema or outcome",
      "unsupported_schema_version",
      "/schemaVersion",
    );
  }
  return {
    ...value,
    schemaVersion: 1,
    outcome: value.outcome as ActuationReceiptV1["outcome"],
    refusedOperationIndex: value.refusedOperationIndex ?? null,
    refusedFieldPath: value.refusedFieldPath ?? null,
    resultingWriteBoundary: value.resultingWriteBoundary ?? null,
    resultingDependencyGeneration: value.resultingDependencyGeneration ?? null,
    projectionGenerationId: value.projectionGenerationId ?? null,
  };
}

/**
 * 0.8.20 Slice 5d (R-20-E4) — outcome of {@link Engine.eraseSource}.
 */
export interface EraseReport {
  /** The `sourceId` that was erased (echoed back). */
  sourceRef: string;
  /** Canonical node rows deleted. */
  nodesExcised: number;
  /** Canonical edge rows deleted. */
  edgesExcised: number;
  /**
   * Row-owned projection rows (FTS5 + vec0 + `search_index_v2`) dropped
   * alongside the canonical rows.
   */
  projectionsInvalidated: number;
}

/**
 * 0.8.20 Slice 15d (R-20-PR) — the three projection roles (set members).
 * `searchable→FTS` and `searchable→vector` are NOT roles — they are tier labels
 * carried by the `fts`/`vector` sub-objects of a {@link ProjectionSpec}.
 */
export type ProjectionRole = "filterable" | "rankable" | "searchable";

/**
 * 0.8.20 Slice 15d (R-20-PR / C-1) — a declarative projection declaration.
 * HITL-ratified shape `{ name, roles, fts?, vector? }`; `roles` carries SET
 * semantics. `fts` selects the `searchable→FTS` sub-target (optional custom
 * `ftsTokenizer`); `vector` selects `searchable→vector` (optional
 * `vectorEmbedder`). The `vector` sub-object is stored by Slice 15d; Slice 20
 * (R-20-DR) hangs the engine-set `vectorDenseReadiness` off it. Mirrors the
 * Python `ProjectionSpec` (cross-binding parity).
 */
export interface ProjectionSpec {
  name: string;
  roles: ProjectionRole[];
  /** `true` when the `searchable→FTS` sub-target is declared. */
  fts: boolean;
  /** Optional custom tokenizer; omitted = engine default (only with `fts`). */
  ftsTokenizer?: string | null;
  /** `true` when the `searchable→vector` sub-target is declared. */
  vector: boolean;
  /** Optional embedder override; omitted = engine default (only with `vector`). */
  vectorEmbedder?: string | null;
  /** Literal canonical-body member path; omitted retains top-level `name`. */
  source?: string[] | null;
  /**
   * 0.8.22 Slice 21 F5 — **READ METADATA, engine-set.** The signed closed
   * vocabulary is `"unavailable"` / `"embedding"` / `"ready"`; `null` is on
   * every caller-authored spec. Caller input is accept-inert; on reads the
   * engine selects `"unavailable"` when no usable dense runtime exists, or
   * `"embedding"` / `"ready"` from the shared outstanding-work predicate.
   *
   * `filterable` / `searchable→FTS` are same-transaction (non-stale on commit)
   * so they carry no readiness; `searchable→vector` is async and
   * rebuild-durable, so it does. The value is DERIVED from outstanding
   * projection work, never stored — which is what makes
   * `{vector-insert ∧ readiness := ready}` atomic by construction: a `"ready"`
   * reading can never be observed with the vector row absent.
   *
   * `"pending"` is NOT a value here: that token is reserved for the orthogonal
   * ADMISSION axis (quarantine/trust, an app judgment).
   *
   * Supplying it to `configureProjections` is INERT — it is not part of the
   * declaration and the engine always reports the derived truth — so
   * `read.projections` output still re-applies as a no-op. Supplying it with
   * `vector: false`, or any spelling outside
   * `{"unavailable", "embedding", "ready"}`, throws a
   * typed `FDB_INVALID_ARGUMENT` (it could not round-trip).
   */
  vectorDenseReadiness?: DenseReadiness | null;
}

/**
 * 0.8.22 Slice 21 F5 — the signed, closed readiness vocabulary of the
 * `searchable→vector` projection. It is engine-selected and accept-inert on
 * caller input: `"unavailable"` means no usable dense runtime, while
 * `"embedding"` / `"ready"` describe work under one. Mirrors Rust and Python;
 * `"pending"` is deliberately absent because it is admission-only.
 */
export type DenseReadiness = "unavailable" | "embedding" | "ready";

/**
 * 0.8.20 Slice 15d (R-20-PR) — the diff `configureProjections` applied.
 * Idempotent re-registration yields `unchanged: true` with all arrays empty; a
 * destructive change without an explicit drop throws instead of returning.
 */
export interface ProjectionDelta {
  built: string[];
  dropped: string[];
  deferred: string[];
  unchanged: boolean;
  /**
   * 0.8.20 Slice 22 (R-20-VC / TC-67) — node **kinds**, not attribute names: the
   * vector-eligible kinds present in the corpus that the vector writer can NEVER
   * commit, so no `searchable→vector` declaration will ever produce an embedding
   * for them. Such rows remain fully FTS/lexically searchable.
   *
   * This is what distinguishes "`deferred` because the embedder is still working
   * / absent this session" (transient) from "this kind will never be embedded"
   * (permanent). It is a STATE report, not a diff: it is populated on an
   * idempotent re-apply too (`unchanged: true`), which is also how you refresh it
   * after writing new kinds. Empty (never absent) when there is nothing to
   * report. Output-only — `configureProjections` accepts specs, not deltas.
   */
  vectorUnsupportedKinds: string[];
}

/** Reason an open engine session has no usable dense runtime. */
export type ProjectionRuntimeUnavailabilityReason =
  "none" | "no_runtime" | "vector_equivalence_disabled";

/** Dense status for one declared projection in {@link ProjectionRuntimeStatus}. */
export type ProjectionStatusDenseReadiness = "not_declared" | "unavailable" | "embedding" | "ready";

/** One declared projection's current dense status. */
export interface ProjectionRuntimeStatusEntry {
  name: string;
  denseReadiness: ProjectionStatusDenseReadiness;
}

/**
 * Pure current projection-runtime facts for an open {@link Engine}.
 *
 * This is not a configuration echo. `projections` contains every declaration
 * in ascending name order. `vectorUnsupportedKinds` is empty unless a
 * declaration has an effective `searchable→vector` arm.
 */
export interface ProjectionRuntimeStatus {
  runtimeEmbedderAvailable: boolean;
  runtimeUnavailabilityReason: ProjectionRuntimeUnavailabilityReason;
  projections: ProjectionRuntimeStatusEntry[];
  vectorUnsupportedKinds: string[];
}

export type EmbeddingReadinessState = "ready" | "processing" | "deferred" | "blocked";
export type EmbeddingOperation = "graph_edge_body_projection" | "vector_projection";
export interface EmbeddingReadiness {
  state: EmbeddingReadinessState;
  usableEmbedder: boolean;
  pendingCount: number;
  affectedKinds: string[];
  code: "FDB_EMBEDDER_REQUIRED" | null;
  operation: EmbeddingOperation | null;
  remediations: string[];
  documentationUrl: string | null;
}

/** G11 (Slice 15) — BYO-LLM ingest receipt. */
export interface IngestWithExtractorReceipt {
  /** Number of `canonical_nodes` rows written (new insertions only). */
  nodesWritten: number;
  /** Number of `canonical_edges` rows written (new fact-edge insertions). */
  edgesWritten: number;
  /** Number of documents processed (including no-facts documents). */
  docsProcessed: number;
}

/** G11 (Slice 15) — a document sent to a BYO-LLM extraction harness. */
export interface ExtractDocument {
  /** Stable opaque identifier for this document. */
  sourceDocId: string;
  /** Full text body to extract entities and relationships from. */
  body: string;
}

/** 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation receipt. */
export interface ConsolidateReceipt {
  /** Number of (subject, relation) axes with a non-empty cluster dispatched. */
  clustersProcessed: number;
  /** Number of candidate edges presented across all clusters. */
  edgesExamined: number;
  /** Number of edges the harness ruled `keep`. */
  edgesKept: number;
  /** Number of edges the harness ruled `invalidate` (t_invalid set). */
  edgesInvalidated: number;
  /** Number of edges the harness ruled `supersede`/`merge` (marked superseded). */
  edgesSuperseded: number;
}

/** 0.8.12 Slice 15 (OPP-2) — one (subject, relation) axis to consolidate. */
export interface ConsolidateAxis {
  /** Stable `logicalId` of the subject entity (edge `fromId`). */
  subjectLogicalId: string;
  /** The relation/edge kind whose competing fact-edges form the cluster. */
  relation: string;
}

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

export type DependencyTraceDirectionV1 = "to_source" | "to_dependents";

export interface DependencyTraceRequestV1 {
  schemaVersion: 1;
  rootRevisionId: string;
  direction: DependencyTraceDirectionV1;
  context: FrozenReadContextV1;
  maxRelations?: number;
  maxWorkUnits?: number;
}

export interface TraceNodeLifecycleV1 {
  schemaVersion: 1;
  artifactClass: "node" | "edge";
  state?: "pending" | "active" | "deleted";
  superseded: boolean;
  validAtEffective: boolean;
}

export interface DependencyTraceNodeV1 {
  schemaVersion: 1;
  artifactRevisionId: string;
  artifactClass: "node" | "edge";
  role: "canonical_source" | "derived";
  depth: number;
  lifecycle: TraceNodeLifecycleV1;
}

export interface DependencyTraceEdgeV1 {
  schemaVersion: 1;
  dependencyId: string;
  sourceRevisionId: string;
  derivedRevisionId: string;
  registeredDependencyGeneration: string;
}

export interface TraceReadBoundaryV1 {
  schemaVersion: 1;
  effectiveAtEpochS: number;
  observedWriteBoundary: string;
  dependencyGeneration: string;
  projectionGenerationId: string;
}

export interface DependencyTraceResultV1 {
  schemaVersion: 1;
  rootRevisionId: string;
  direction: DependencyTraceDirectionV1;
  nodes: DependencyTraceNodeV1[];
  dependencyEdges: DependencyTraceEdgeV1[];
  checkedWorkUnits: number;
  complete: true;
  readBoundary: TraceReadBoundaryV1;
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

export interface EvidenceSearchRequestV1 {
  schemaVersion: 1;
  query: string;
  context: FrozenReadContextV1;
  rerankDepth?: number;
  useGraphArm?: boolean;
  alpha?: number;
  poolN?: number;
  includeExplanation?: boolean;
  limit?: number;
}

export interface EvidenceResolveRequestV1 {
  schemaVersion: 1;
  evidenceRef: string;
  context: FrozenReadContextV1;
}

export interface EvidenceSidecarEntryV1 {
  schemaVersion: 1;
  resultIndex: number;
  artifactRevisionId: string;
  evidenceRef: string;
}

export interface EvidenceContributionV1 {
  schemaVersion: 1;
  vectorRank: number | null;
  textRank: number | null;
  graphRank: number | null;
  fusedScore: number;
  ceScore: number | null;
  blendedScore: number;
  importance: number | null;
  confidence: number | null;
}

export interface EvidenceGraphOriginV1 {
  kind: "edge_seed" | "traversal";
  edgeArtifactRevisionId: string | null;
  hopCount: number | null;
}

export interface EvidenceProjectionOriginV1 {
  schemaVersion: 1;
  artifactClass: "node" | "edge";
  representativeArm: SoftFallbackBranch;
  projectionGenerationId: string;
  graphOrigin: EvidenceGraphOriginV1 | null;
}

export interface EvidenceArtifactLifecycleV1 {
  kind: "node" | "edge";
  state: LifecycleState | null;
  superseded: boolean;
  validAtEffective: boolean | null;
}

export interface EvidenceSearchResultV1 {
  schemaVersion: 1;
  searchResult: SearchResult;
  evidence: EvidenceSidecarEntryV1[];
}

export interface ResolvedEvidenceV1 {
  schemaVersion: 1;
  logicalId: string | null;
  artifactRevisionId: string;
  sourceId: string;
  sourceVersionId: string;
  sourceRevisionId: string;
  locator:
    { kind: "whole_body" } | { kind: "utf8_bytes"; startInclusive: string; endExclusive: string };
  canonicalSourceBody: string;
  evidenceText: string;
  canonicalSourceHash: string;
  effectiveValidAt: number;
  artifactLifecycle: EvidenceArtifactLifecycleV1;
  sourceLifecycleState: LifecycleState;
  projectionOrigin: EvidenceProjectionOriginV1;
  retrievalContribution: EvidenceContributionV1;
  dependency: SourceDependencyV1 | null;
}

/** Options shared by ranked search methods. `limit` defaults to 10 and is 1–100. */
export type SearchOptions = ReadView & {
  limit?: number;
};

/** Options for the initial ranked hits returned by {@link graph.searchExpand}. */
export interface SearchExpandOptions {
  searchLimit?: number;
}

function validateRankedResultLimit(name: string, limit: number | undefined): number {
  const resolved = limit ?? 10;
  if (!Number.isInteger(resolved) || resolved < 1 || resolved > 100) {
    throw new InvalidArgumentError(`${name} must be an integer in 1..=100; got ${resolved}`);
  }
  return resolved;
}

function splitSearchOptions(options: SearchOptions | undefined): {
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

function mapNativeEvidenceSearch(r: NativeEvidenceSearchResultV1): EvidenceSearchResultV1 {
  validateNativeEvidenceSearch(r);
  return {
    schemaVersion: r.schemaVersion as 1,
    searchResult: mapNativeSearchResult(r.searchResult),
    evidence: r.evidence.map((item) => ({
      schemaVersion: item.schemaVersion as 1,
      resultIndex: item.resultIndex,
      artifactRevisionId: item.artifactRevisionId,
      evidenceRef: item.evidenceRef,
    })),
  };
}

function mapNativeResolvedEvidence(r: NativeResolvedEvidenceV1): ResolvedEvidenceV1 {
  validateNativeResolvedEvidence(r);
  const graph = r.projectionOrigin.graphOrigin;
  const locator: ResolvedEvidenceV1["locator"] =
    r.locator.kind === "utf8_bytes"
      ? {
          kind: "utf8_bytes",
          startInclusive: r.locator.startInclusive!,
          endExclusive: r.locator.endExclusive!,
        }
      : { kind: "whole_body" };
  return {
    schemaVersion: r.schemaVersion as 1,
    logicalId: r.logicalId ?? null,
    artifactRevisionId: r.artifactRevisionId,
    sourceId: r.sourceId,
    sourceVersionId: r.sourceVersionId,
    sourceRevisionId: r.sourceRevisionId,
    locator,
    canonicalSourceBody: r.canonicalSourceBody,
    evidenceText: r.evidenceText,
    canonicalSourceHash: r.canonicalSourceHash,
    effectiveValidAt: r.effectiveValidAt,
    artifactLifecycle: {
      kind: r.artifactLifecycle.kind as "node" | "edge",
      state: (r.artifactLifecycle.state ?? null) as LifecycleState | null,
      superseded: r.artifactLifecycle.superseded,
      validAtEffective: r.artifactLifecycle.validAtEffective ?? null,
    },
    sourceLifecycleState: r.sourceLifecycleState as LifecycleState,
    projectionOrigin: {
      schemaVersion: r.projectionOrigin.schemaVersion as 1,
      artifactClass: r.projectionOrigin.artifactClass as "node" | "edge",
      representativeArm: r.projectionOrigin.representativeArm as SoftFallbackBranch,
      projectionGenerationId: r.projectionOrigin.projectionGenerationId,
      graphOrigin: graph
        ? {
            kind: graph.kind as EvidenceGraphOriginV1["kind"],
            edgeArtifactRevisionId: graph.edgeArtifactRevisionId ?? null,
            hopCount: graph.hopCount ?? null,
          }
        : null,
    },
    retrievalContribution: {
      schemaVersion: r.retrievalContribution.schemaVersion as 1,
      vectorRank: r.retrievalContribution.vectorRank ?? null,
      textRank: r.retrievalContribution.textRank ?? null,
      graphRank: r.retrievalContribution.graphRank ?? null,
      fusedScore: r.retrievalContribution.fusedScore,
      ceScore: r.retrievalContribution.ceScore ?? null,
      blendedScore: r.retrievalContribution.blendedScore,
      importance: r.retrievalContribution.importance ?? null,
      confidence: r.retrievalContribution.confidence ?? null,
    },
    dependency: r.dependency
      ? {
          schemaVersion: r.dependency.schemaVersion as 1,
          dependencyId: r.dependency.dependencyId,
          sourceRevisionId: r.dependency.sourceRevisionId,
          derivedRevisionId: r.dependency.derivedRevisionId,
          registeredDependencyGeneration: r.dependency.registeredDependencyGeneration,
        }
      : null,
  };
}

function assertKnownKeys(value: object, allowed: readonly string[], name: string): void {
  const unknown = Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .sort();
  if (unknown.length > 0) {
    throw new InvalidArgumentError(`${name} has unknown field ${unknown[0]}`);
  }
}

function evidenceRequestError(reason: string, fieldPath: string): never {
  throw new EvidenceError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

function evidencePointerSegment(value: string): string {
  return value.replaceAll("~", "~0").replaceAll("/", "~1");
}

function assertKnownEvidenceKeys(value: object, allowed: readonly string[], basePath = ""): void {
  const unknown = Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .sort();
  if (unknown.length > 0) {
    evidenceRequestError("unknown_field", `${basePath}/${evidencePointerSegment(unknown[0]!)}`);
  }
}

function validateEvidenceFrozenContext(context: FrozenReadContextV1): void {
  assertKnownEvidenceKeys(
    context,
    ["schemaVersion", "effectiveValidAt", "context", "token"],
    "/context",
  );
  assertKnownEvidenceKeys(
    context.context,
    ["schemaVersion", "view", "eligibility"],
    "/context/context",
  );
  assertKnownEvidenceKeys(
    context.context.view,
    ["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
    "/context/context/view",
  );
  assertKnownEvidenceKeys(
    context.context.eligibility,
    ["sourceType", "kind", "createdAfter", "status", "attributes"],
    "/context/context/eligibility",
  );
}

function validateReadContext(context: ReadContextV1): void {
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

function nativeFrozenContext(context: FrozenReadContextV1): NativeFrozenReadContextV1 {
  return context;
}

function dependencyTraceRequestError(reason: string, fieldPath: string): never {
  throw new DependencyTraceError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

function frozenTraceRequestError(reason: string, fieldPath: string): never {
  throw new FrozenReadError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

function frozenTraceRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    frozenTraceRequestError("context_invalid", path);
  }
  return value as Record<string, unknown>;
}

function validateFrozenTraceKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
): void {
  const unknown = Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .sort()[0];
  if (unknown !== undefined) {
    frozenTraceRequestError("context_invalid", `${path}/${escapeTracePointerToken(unknown)}`);
  }
}

function validateFrozenTraceContext(value: unknown): FrozenReadContextV1 {
  const frozen = frozenTraceRecord(value, "/context");
  validateFrozenTraceKeys(
    frozen,
    ["schemaVersion", "effectiveValidAt", "context", "token"],
    "/context",
  );
  if (frozen.schemaVersion !== 1) {
    frozenTraceRequestError("unsupported_schema_version", "/context/schemaVersion");
  }
  if (!Number.isSafeInteger(frozen.effectiveValidAt)) {
    frozenTraceRequestError("context_invalid", "/context/effectiveValidAt");
  }
  if (typeof frozen.token !== "string") {
    frozenTraceRequestError("token_malformed", "/context/token");
  }
  if (new TextEncoder().encode(frozen.token).length > 1024) {
    frozenTraceRequestError("token_too_large", "/context/token");
  }
  const context = frozenTraceRecord(frozen.context, "/context/context");
  validateFrozenTraceKeys(context, ["schemaVersion", "view", "eligibility"], "/context/context");
  if (context.schemaVersion !== 1) {
    frozenTraceRequestError("unsupported_schema_version", "/context/context/schemaVersion");
  }
  const view = frozenTraceRecord(context.view, "/context/context/view");
  validateFrozenTraceKeys(
    view,
    ["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
    "/context/context/view",
  );
  for (const field of ["includeSuperseded", "includeInactive", "includeOutOfWindow"] as const) {
    if (view[field] !== undefined && typeof view[field] !== "boolean") {
      frozenTraceRequestError("context_invalid", `/context/context/view/${field}`);
    }
  }
  if (
    view.validAsOf !== undefined &&
    view.validAsOf !== null &&
    !Number.isSafeInteger(view.validAsOf)
  ) {
    frozenTraceRequestError("context_invalid", "/context/context/view/validAsOf");
  }
  const eligibility = frozenTraceRecord(context.eligibility, "/context/context/eligibility");
  validateFrozenTraceKeys(
    eligibility,
    ["sourceType", "kind", "createdAfter", "status", "attributes"],
    "/context/context/eligibility",
  );
  for (const field of ["sourceType", "kind", "status"] as const) {
    if (eligibility[field] !== undefined && typeof eligibility[field] !== "string") {
      frozenTraceRequestError("context_invalid", `/context/context/eligibility/${field}`);
    }
  }
  if (
    eligibility.createdAfter !== undefined &&
    eligibility.createdAfter !== null &&
    !Number.isSafeInteger(eligibility.createdAfter)
  ) {
    frozenTraceRequestError("context_invalid", "/context/context/eligibility/createdAfter");
  }
  if (eligibility.attributes !== undefined) {
    if (!Array.isArray(eligibility.attributes) || eligibility.attributes.length > 64) {
      frozenTraceRequestError("context_invalid", "/context/context/eligibility/attributes");
    }
    eligibility.attributes.forEach((pair, index) => {
      if (
        !Array.isArray(pair) ||
        pair.length !== 2 ||
        typeof pair[0] !== "string" ||
        typeof pair[1] !== "string"
      ) {
        frozenTraceRequestError(
          "context_invalid",
          `/context/context/eligibility/attributes/${index}`,
        );
      }
    });
  }
  return value as FrozenReadContextV1;
}

function escapeTracePointerToken(value: string): string {
  return value.replaceAll("~", "~0").replaceAll("/", "~1");
}

function validateDependencyTraceRequest(request: DependencyTraceRequestV1): void {
  if (request.schemaVersion !== 1) {
    dependencyTraceRequestError("unsupported_schema_version", "/schemaVersion");
  }
  const unknown = Object.keys(request)
    .filter(
      (key) =>
        ![
          "schemaVersion",
          "rootRevisionId",
          "direction",
          "context",
          "maxRelations",
          "maxWorkUnits",
        ].includes(key),
    )
    .sort()[0];
  if (unknown !== undefined) {
    dependencyTraceRequestError("unknown_field", `/${escapeTracePointerToken(unknown)}`);
  }
  if (
    typeof request.rootRevisionId !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(request.rootRevisionId) ||
    request.rootRevisionId.startsWith("_fdb:")
  ) {
    dependencyTraceRequestError("trace_root_invalid", "/rootRevisionId");
  }
  if (request.direction !== "to_source" && request.direction !== "to_dependents") {
    dependencyTraceRequestError("trace_direction_invalid", "/direction");
  }
  if (
    typeof request.context !== "object" ||
    request.context === null ||
    Array.isArray(request.context)
  ) {
    dependencyTraceRequestError("trace_corrupt", "/context");
  }
  validateFrozenTraceContext(request.context);
  if (
    request.maxRelations !== undefined &&
    (!Number.isInteger(request.maxRelations) ||
      request.maxRelations < 1 ||
      request.maxRelations > 100)
  ) {
    dependencyTraceRequestError("trace_limit_invalid", "/maxRelations");
  }
  if (
    request.maxWorkUnits !== undefined &&
    (!Number.isInteger(request.maxWorkUnits) ||
      request.maxWorkUnits < 1 ||
      request.maxWorkUnits > 101)
  ) {
    dependencyTraceRequestError("trace_limit_invalid", "/maxWorkUnits");
  }
}

function traceRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value as Record<string, unknown>;
}

function traceArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) dependencyTraceRequestError("trace_corrupt", path);
  return value;
}

function traceField(value: Record<string, unknown>, name: string, path: string): unknown {
  if (!(name in value)) dependencyTraceRequestError("trace_corrupt", path);
  return value[name];
}

function traceSchema(value: Record<string, unknown>, path: string): void {
  if (traceField(value, "schemaVersion", path) !== 1) {
    dependencyTraceRequestError("unsupported_schema_version", path);
  }
}

function traceId(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(value) ||
    value.startsWith("_fdb:")
  ) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value;
}

function traceU32(value: unknown, path: string): number {
  if (!Number.isInteger(value) || (value as number) < 0 || (value as number) > 0xffff_ffff) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value as number;
}

function traceI64(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value)) dependencyTraceRequestError("trace_corrupt", path);
  return value as number;
}

function traceU64(value: unknown, path: string): string {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  try {
    if (BigInt(value) > 0xffff_ffff_ffff_ffffn) {
      dependencyTraceRequestError("trace_corrupt", path);
    }
  } catch {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value;
}

function traceBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") dependencyTraceRequestError("trace_corrupt", path);
  return value;
}

/** @internal Recursively validate one decoded native dependency-trace response. */
export function validateDependencyTraceResponse(value: unknown): DependencyTraceResultV1 {
  const rootValue = traceRecord(value, "");
  traceSchema(rootValue, "/schemaVersion");
  const rootRevisionId = traceId(
    traceField(rootValue, "rootRevisionId", "/rootRevisionId"),
    "/rootRevisionId",
  );
  const direction = traceField(rootValue, "direction", "/direction");
  if (direction !== "to_source" && direction !== "to_dependents") {
    dependencyTraceRequestError("trace_corrupt", "/direction");
  }
  const rawNodes = traceArray(traceField(rootValue, "nodes", "/nodes"), "/nodes");
  if (rawNodes.length < 1 || rawNodes.length > 101) {
    dependencyTraceRequestError("trace_corrupt", "/nodes");
  }
  const revisionIds = new Set<string>();
  const nodes = rawNodes.map((rawNode, index): DependencyTraceNodeV1 => {
    const base = `/nodes/${index}`;
    const node = traceRecord(rawNode, base);
    traceSchema(node, `${base}/schemaVersion`);
    const artifactRevisionId = traceId(
      traceField(node, "artifactRevisionId", `${base}/artifactRevisionId`),
      `${base}/artifactRevisionId`,
    );
    if (revisionIds.has(artifactRevisionId)) {
      dependencyTraceRequestError("trace_corrupt", `${base}/artifactRevisionId`);
    }
    revisionIds.add(artifactRevisionId);
    const artifactClass = traceField(node, "artifactClass", `${base}/artifactClass`);
    if (artifactClass !== "node" && artifactClass !== "edge") {
      dependencyTraceRequestError("trace_corrupt", `${base}/artifactClass`);
    }
    const role = traceField(node, "role", `${base}/role`);
    if (role !== "canonical_source" && role !== "derived") {
      dependencyTraceRequestError("trace_corrupt", `${base}/role`);
    }
    const lifecyclePath = `${base}/lifecycle`;
    const lifecycle = traceRecord(traceField(node, "lifecycle", lifecyclePath), lifecyclePath);
    traceSchema(lifecycle, `${lifecyclePath}/schemaVersion`);
    if (
      traceField(lifecycle, "artifactClass", `${lifecyclePath}/artifactClass`) !== artifactClass
    ) {
      dependencyTraceRequestError("trace_corrupt", `${lifecyclePath}/artifactClass`);
    }
    const state = lifecycle.state;
    if (artifactClass === "node") {
      if (state !== "pending" && state !== "active" && state !== "deleted") {
        dependencyTraceRequestError("trace_corrupt", `${lifecyclePath}/state`);
      }
    } else if ("state" in lifecycle) {
      dependencyTraceRequestError("trace_corrupt", `${lifecyclePath}/state`);
    }
    return {
      schemaVersion: 1,
      artifactRevisionId,
      artifactClass,
      role,
      depth: traceU32(traceField(node, "depth", `${base}/depth`), `${base}/depth`),
      lifecycle: {
        schemaVersion: 1,
        artifactClass,
        ...(artifactClass === "node" ? { state: state as "pending" | "active" | "deleted" } : {}),
        superseded: traceBoolean(
          traceField(lifecycle, "superseded", `${lifecyclePath}/superseded`),
          `${lifecyclePath}/superseded`,
        ),
        validAtEffective: traceBoolean(
          traceField(lifecycle, "validAtEffective", `${lifecyclePath}/validAtEffective`),
          `${lifecyclePath}/validAtEffective`,
        ),
      },
    };
  });
  const rawEdges = traceArray(
    traceField(rootValue, "dependencyEdges", "/dependencyEdges"),
    "/dependencyEdges",
  );
  if (rawEdges.length > 100) dependencyTraceRequestError("trace_corrupt", "/dependencyEdges");
  const dependencyIds = new Set<string>();
  const dependencyEdges = rawEdges.map((rawEdge, index): DependencyTraceEdgeV1 => {
    const base = `/dependencyEdges/${index}`;
    const edge = traceRecord(rawEdge, base);
    traceSchema(edge, `${base}/schemaVersion`);
    const dependencyId = traceId(
      traceField(edge, "dependencyId", `${base}/dependencyId`),
      `${base}/dependencyId`,
    );
    if (dependencyIds.has(dependencyId)) {
      dependencyTraceRequestError("trace_corrupt", `${base}/dependencyId`);
    }
    dependencyIds.add(dependencyId);
    return {
      schemaVersion: 1,
      dependencyId,
      sourceRevisionId: traceId(
        traceField(edge, "sourceRevisionId", `${base}/sourceRevisionId`),
        `${base}/sourceRevisionId`,
      ),
      derivedRevisionId: traceId(
        traceField(edge, "derivedRevisionId", `${base}/derivedRevisionId`),
        `${base}/derivedRevisionId`,
      ),
      registeredDependencyGeneration: traceU64(
        traceField(
          edge,
          "registeredDependencyGeneration",
          `${base}/registeredDependencyGeneration`,
        ),
        `${base}/registeredDependencyGeneration`,
      ),
    };
  });
  const checkedWorkUnits = traceU32(
    traceField(rootValue, "checkedWorkUnits", "/checkedWorkUnits"),
    "/checkedWorkUnits",
  );
  if (
    checkedWorkUnits !== dependencyEdges.length + 1 ||
    nodes.length !== dependencyEdges.length + 1
  ) {
    dependencyTraceRequestError("trace_corrupt", "/checkedWorkUnits");
  }
  if (traceField(rootValue, "complete", "/complete") !== true) {
    dependencyTraceRequestError("trace_corrupt", "/complete");
  }
  const expectedRootRole = direction === "to_source" ? "derived" : "canonical_source";
  if (nodes[0]!.artifactRevisionId !== rootRevisionId || nodes[0]!.depth !== 0) {
    dependencyTraceRequestError("trace_corrupt", "/nodes/0");
  }
  if (nodes[0]!.role !== expectedRootRole) {
    dependencyTraceRequestError("trace_corrupt", "/nodes/0/role");
  }
  for (let index = 0; index < dependencyEdges.length; index += 1) {
    const node = nodes[index + 1]!;
    const edge = dependencyEdges[index]!;
    if (node.depth !== 1) dependencyTraceRequestError("trace_corrupt", `/nodes/${index + 1}/depth`);
    if (direction === "to_dependents") {
      if (node.role !== "derived")
        dependencyTraceRequestError("trace_corrupt", `/nodes/${index + 1}/role`);
      if (edge.sourceRevisionId !== rootRevisionId)
        dependencyTraceRequestError("trace_corrupt", `/dependencyEdges/${index}/sourceRevisionId`);
      if (edge.derivedRevisionId !== node.artifactRevisionId)
        dependencyTraceRequestError("trace_corrupt", `/dependencyEdges/${index}/derivedRevisionId`);
    } else {
      if (node.role !== "canonical_source")
        dependencyTraceRequestError("trace_corrupt", `/nodes/${index + 1}/role`);
      if (edge.derivedRevisionId !== rootRevisionId)
        dependencyTraceRequestError("trace_corrupt", `/dependencyEdges/${index}/derivedRevisionId`);
      if (edge.sourceRevisionId !== node.artifactRevisionId)
        dependencyTraceRequestError("trace_corrupt", `/dependencyEdges/${index}/sourceRevisionId`);
    }
  }
  for (let index = 2; index < nodes.length; index += 1) {
    if (nodes[index - 1]!.artifactRevisionId > nodes[index]!.artifactRevisionId) {
      dependencyTraceRequestError("trace_corrupt", "/nodes");
    }
  }
  for (let index = 1; index < dependencyEdges.length; index += 1) {
    const previous = dependencyEdges[index - 1]!;
    const current = dependencyEdges[index]!;
    if (
      `${previous.derivedRevisionId}\0${previous.dependencyId}` >
      `${current.derivedRevisionId}\0${current.dependencyId}`
    ) {
      dependencyTraceRequestError("trace_corrupt", "/dependencyEdges");
    }
  }
  const boundaryValue = traceRecord(
    traceField(rootValue, "readBoundary", "/readBoundary"),
    "/readBoundary",
  );
  traceSchema(boundaryValue, "/readBoundary/schemaVersion");
  const projectionGenerationId = traceField(
    boundaryValue,
    "projectionGenerationId",
    "/readBoundary/projectionGenerationId",
  );
  if (
    typeof projectionGenerationId !== "string" ||
    !/^pgen1:[0-9a-f]{32}$/.test(projectionGenerationId)
  ) {
    dependencyTraceRequestError("trace_corrupt", "/readBoundary/projectionGenerationId");
  }
  const effectiveAtEpochS = traceI64(
    traceField(boundaryValue, "effectiveAtEpochS", "/readBoundary/effectiveAtEpochS"),
    "/readBoundary/effectiveAtEpochS",
  );
  const observedWriteBoundary = traceU64(
    traceField(boundaryValue, "observedWriteBoundary", "/readBoundary/observedWriteBoundary"),
    "/readBoundary/observedWriteBoundary",
  );
  const dependencyGeneration = traceU64(
    traceField(boundaryValue, "dependencyGeneration", "/readBoundary/dependencyGeneration"),
    "/readBoundary/dependencyGeneration",
  );
  for (let index = 0; index < dependencyEdges.length; index += 1) {
    const edgeGeneration = BigInt(dependencyEdges[index]!.registeredDependencyGeneration);
    if (edgeGeneration === 0n || edgeGeneration > BigInt(dependencyGeneration)) {
      dependencyTraceRequestError(
        "trace_corrupt",
        `/dependencyEdges/${index}/registeredDependencyGeneration`,
      );
    }
  }
  return {
    schemaVersion: 1,
    rootRevisionId,
    direction,
    nodes,
    dependencyEdges,
    checkedWorkUnits,
    complete: true,
    readBoundary: {
      schemaVersion: 1,
      effectiveAtEpochS,
      observedWriteBoundary,
      dependencyGeneration,
      projectionGenerationId,
    },
  };
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

const VEC0_JSON_REJECT =
  "arbitrary json-path predicate not supported on search_filtered; it would " +
  "require a post-KNN json_extract that defeats the indexed pre-KNN filter " +
  "(ADR-0.8.11 D3 no-demotion guarantee)";

function isUnifiedFilter(f: SearchFilter | Filter | undefined): f is Filter {
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

function mapCandidateNativeExplanation(
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
 * Rust `Explanation`): a query-level `trace` + a per-hit breakdown. Returned on
 * `SearchResult.explanation` only when `search(..., { explain: true })`; `null`
 * (default) keeps the result byte-identical to the pre-0.8.8 shape.
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
   * 0.8.8 EXP-OBS (Slice 10) — opt-in explanation sidecar; `null` unless
   * `search(..., { explain: true })`.
   */
  explanation: Explanation | null;
}

export interface MigrationStepReport {
  readonly stepId: number;
  readonly durationMs: number | null;
  readonly failed: boolean;
}

export interface EmbedderIdentity {
  readonly name: string;
  readonly revision: string;
  readonly dimension: number;
}

/**
 * EU-6 FIX-2 — discriminated-union shape for `OpenReport.embedderEvents`.
 *
 * Each variant interface carries a closed `kind` literal + the
 * variant-specific payload fields (non-optional). Callers pattern-match
 * with `if (event.kind === "...")` and tsc narrows the payload access
 * accordingly. See `dev/design/0.7.1-EU-6-FIX-2-design.md` §6.3.
 */
export interface DefaultEmbedderDownloadEvent {
  readonly kind: "DefaultEmbedderDownload";
  readonly file: string;
  readonly url: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly cachePath: string;
  readonly durationMs: number;
}

export interface DefaultEmbedderCacheHitEvent {
  readonly kind: "DefaultEmbedderCacheHit";
  readonly file: string;
  readonly sha256: string;
  readonly cachePath: string;
}

export interface MeanVecPinnedEvent {
  readonly kind: "MeanVecPinned";
  readonly dim: number;
  readonly docCount: number;
}

/**
 * Forward-compat fallback for `kind` values not known to this build.
 * Part of the public `EmbedderEvent` union for soundness: a future or
 * replaced native extension may emit kinds this build does not know
 * about, and exposing them under a typed fallback member is more honest
 * than pretending the runtime is exhaustive at compile time. Because
 * `kind` here is the open type `string`, tsc cannot exclude this member
 * purely from a literal `event.kind === "..."` check on the bare union
 * — wrap such checks in {@link isKnownEmbedderEvent} first to recover
 * precise narrowing on the three known variants.
 */
export interface UnknownEmbedderEvent {
  readonly kind: string;
  readonly [field: string]: unknown;
}

export type EmbedderEvent =
  | DefaultEmbedderDownloadEvent
  | DefaultEmbedderCacheHitEvent
  | MeanVecPinnedEvent
  | UnknownEmbedderEvent;

/**
 * Type guard that narrows an {@link EmbedderEvent} to the three known
 * variants, excluding {@link UnknownEmbedderEvent}. Use as a gate before
 * discriminating on `event.kind`:
 *
 * ```ts
 * if (isKnownEmbedderEvent(event)) {
 *   if (event.kind === "DefaultEmbedderDownload") {
 *     const bytes: number = event.bytes; // narrowed precisely
 *   }
 * }
 * ```
 *
 * Without this guard, the open `kind: string` on `UnknownEmbedderEvent`
 * prevents tsc from removing it from the union on a literal-equality
 * check, so payload field access widens to `unknown`.
 */
export function isKnownEmbedderEvent(
  event: EmbedderEvent,
): event is DefaultEmbedderDownloadEvent | DefaultEmbedderCacheHitEvent | MeanVecPinnedEvent {
  return (
    event.kind === "DefaultEmbedderDownload" ||
    event.kind === "DefaultEmbedderCacheHit" ||
    event.kind === "MeanVecPinned"
  );
}

/**
 * @internal — maps the wide napi-rs `NativeEmbedderEvent` into the
 * narrow discriminated `EmbedderEvent` union at the binding → SDK
 * seam. The non-null assertions are sound under the Rust emitter
 * invariant codified by AC-FIX2-6's runtime shape consistency test:
 * for each known `kind`, the emitter populates exactly the variant-
 * appropriate fields. Unknown `kind` values pass through as
 * `UnknownEmbedderEvent` so a forward-compatible variant addition
 * remains a strict refinement, not a breaking change.
 */
export function mapEmbedderEvent(n: NativeEmbedderEvent): EmbedderEvent {
  switch (n.kind) {
    case "DefaultEmbedderDownload":
      return {
        kind: "DefaultEmbedderDownload",
        file: n.file!,
        url: n.url!,
        bytes: n.bytes!,
        sha256: n.sha256!,
        cachePath: n.cachePath!,
        durationMs: n.durationMs!,
      };
    case "DefaultEmbedderCacheHit":
      return {
        kind: "DefaultEmbedderCacheHit",
        file: n.file!,
        sha256: n.sha256!,
        cachePath: n.cachePath!,
      };
    case "MeanVecPinned":
      return {
        kind: "MeanVecPinned",
        dim: n.dim!,
        docCount: n.docCount!,
      };
    default: {
      // Forward-compat: surface unknown kinds verbatim, dropping any
      // nullish wide-shape fields so the resulting object has only the
      // keys the emitter actually populated. `UnknownEmbedderEvent` is
      // part of the declared `EmbedderEvent` union, so no cast through
      // `unknown` is required — callers recover precise narrowing on
      // the known variants via `isKnownEmbedderEvent`.
      const out: Record<string, unknown> = { kind: n.kind };
      for (const [k, v] of Object.entries(n)) {
        if (k !== "kind" && v !== null && v !== undefined) out[k] = v;
      }
      return out as UnknownEmbedderEvent;
    }
  }
}

export interface OpenReport {
  readonly schemaVersionBefore: number;
  readonly schemaVersionAfter: number;
  readonly migrationSteps: ReadonlyArray<MigrationStepReport>;
  readonly embedderWarmupMs: number;
  readonly queryBackend: string;
  readonly defaultEmbedder: EmbedderIdentity;
  /** EU-5b — wall-time ms the loader spent fetching default-embedder
   *  weights, or `null` on full cache hit / caller-supplied embedder. */
  readonly embedderDownloadMs: number | null;
  /** EU-5b — structured loader events (downloads, cache hits,
   *  mean-vec pin). */
  readonly embedderEvents: ReadonlyArray<EmbedderEvent>;
  /** EU-5b — static identity capability (mean-centering required for
   *  bge-small). */
  readonly embedderMeanCenteringRequired: boolean;
  /** EU-5a2 — dynamic workspace state (`mean_vec IS NOT NULL` after the
   *  256-doc threshold crossing). */
  readonly embedderMeanVecPinned: boolean;
  /** 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — `true` iff the
   *  open-time self-check found a vector-equivalence divergence and every
   *  vector-dependent arm now refuses at query time with
   *  `VectorEquivalenceMismatchError`. The `searchTextOnly` path stays
   *  serviceable. */
  readonly denseDisabled: boolean;
  /** R-VEQ-6 — reason for `denseDisabled`, or `null` when dense is healthy. */
  readonly denseDisabledReason: string | null;
  /** Strict CPU/CUDA selection used to construct the embedder, or `null` when
   *  no embedder was configured. */
  readonly embedderDeviceResolution: DeviceResolution | null;
  /** Independent cross-encoder CPU/CUDA selection. It never attests SQLite or
   * embedding work and is `null` when this artifact lacks the reranker. */
  readonly rerankerDeviceResolution: DeviceResolution | null;
  /** 0.8.23 Slice 80.6 (D-80.6-6, AC80-6) — the in-process GPU allocation
   *  witness measured during this open, or `null` when none was measured.
   *
   *  `null` means **no witness was taken**, never "a witness measured
   *  nothing": a zero, negative, or below-floor allocation delta is a typed
   *  failure inside the witness and fails the open, so a zero-valued record is
   *  not reachable here. */
  readonly embedderGpuAllocationWitness: GpuAllocationWitness | null;
}

/**
 * 0.8.23 Slice 80.6 (D-80.6-6, R80-13) — the retained
 * `fathomdb.tegra-gpu-allocation-witness/v1` record, measured in the
 * artifact's own process.
 *
 * Every number the verdict used is present, so a reader re-derives the verdict
 * instead of trusting it: `freeBeforeBytes - freeAfterBytes` is `deltaBytes`,
 * which must be at least `deltaFloorBytes`, and the deliberate control
 * allocation shows the shared iGPU memory counter was live and attributable at
 * the time. Byte counts are JavaScript numbers, which are exact for every
 * physically reachable device-memory value.
 */
export interface GpuAllocationWitness {
  /** Schema string of the retained record. */
  readonly schema: string;
  /** The precondition the witness run states rather than assumes. */
  readonly soleGpuConsumerPrecondition: string;
  readonly deviceOrdinalRequested: number;
  readonly deviceOrdinalActual: number;
  readonly deviceUuid: string;
  readonly deviceName: string;
  readonly computeCapability: string;
  readonly freeBeforeBytes: number;
  readonly freeAfterBytes: number;
  readonly totalBytes: number;
  readonly deltaBytes: number;
  readonly deltaFloorBytes: number;
  readonly controlAllocationRequestBytes: number;
  readonly controlBlockCount: number;
  readonly controlFreeBeforeBytes: number;
  readonly controlFreeAfterBytes: number;
  readonly controlDeltaBytes: number;
  readonly embeddedVectorDim: number;
}

/** Safe CUDA provider facts associated with an effective CUDA selection. */
export interface CudaDeviceInfo {
  readonly ordinal: number;
  readonly uuid: string | null;
  readonly name: string | null;
  readonly driverVersion: string | null;
  readonly computeCapability: string | null;
  readonly cudaToolkitVersion: string | null;
}

/** One CUDA device visible to the process after `CUDA_VISIBLE_DEVICES`. */
export interface CudaVisibleDevice {
  readonly visibleOrdinal: number;
  readonly uuid: string;
  readonly name: string;
  readonly computeCapability: string | null;
}

/** The CPU or CUDA backend selected for one embedder device policy. */
export type EffectiveEmbedDevice =
  | { readonly kind: "cpu"; readonly cudaDevice: null }
  | { readonly kind: "cuda"; readonly cudaDevice: CudaDeviceInfo };

/**
 * Strict CPU/CUDA policy outcome captured when an embedder was constructed.
 * `requestedPolicy` is exactly `auto`, `cpu`, or `cuda:N`; `reason` explains
 * an automatic CPU fallback and is `null` for an explicitly selected device.
 */
export interface DeviceResolution {
  readonly requestedPolicy: string;
  readonly cudaCompiled: boolean;
  readonly effectiveDevice: EffectiveEmbedDevice;
  readonly visibleCudaDevices: readonly CudaVisibleDevice[];
  readonly selectedCudaUuid: string | null;
  readonly reason: string | null;
}

function mapCudaDeviceInfo(info: NativeCudaDeviceInfo): CudaDeviceInfo {
  return {
    ordinal: info.ordinal,
    uuid: info.uuid ?? null,
    name: info.name ?? null,
    driverVersion: info.driverVersion ?? null,
    computeCapability: info.computeCapability ?? null,
    cudaToolkitVersion: info.cudaToolkitVersion ?? null,
  };
}

function mapCudaVisibleDevice(device: NativeCudaVisibleDevice): CudaVisibleDevice {
  return {
    visibleOrdinal: device.visibleOrdinal,
    uuid: device.uuid,
    name: device.name,
    computeCapability: device.computeCapability ?? null,
  };
}

function mapEffectiveEmbedDevice(device: NativeEffectiveEmbedDevice): EffectiveEmbedDevice {
  if (device.kind === "cpu") return { kind: "cpu", cudaDevice: null };
  if (device.kind === "cuda" && device.cudaDevice) {
    return { kind: "cuda", cudaDevice: mapCudaDeviceInfo(device.cudaDevice) };
  }
  throw new Error(`invalid native embedder effective device: ${device.kind}`);
}

function mapDeviceResolution(resolution: NativeEmbedderDeviceResolution): DeviceResolution {
  return {
    requestedPolicy: resolution.requestedPolicy,
    cudaCompiled: resolution.cudaCompiled,
    effectiveDevice: mapEffectiveEmbedDevice(resolution.effectiveDevice),
    visibleCudaDevices: resolution.visibleCudaDevices.map(mapCudaVisibleDevice),
    selectedCudaUuid: resolution.selectedCudaUuid ?? null,
    reason: resolution.reason ?? null,
  };
}

function mapGpuAllocationWitness(witness: NativeGpuAllocationWitness): GpuAllocationWitness {
  // Field-for-field, deliberately: R80-13 requires the record stay
  // re-derivable, so nothing here summarizes or drops a number.
  return {
    schema: witness.schema,
    soleGpuConsumerPrecondition: witness.soleGpuConsumerPrecondition,
    deviceOrdinalRequested: witness.deviceOrdinalRequested,
    deviceOrdinalActual: witness.deviceOrdinalActual,
    deviceUuid: witness.deviceUuid,
    deviceName: witness.deviceName,
    computeCapability: witness.computeCapability,
    freeBeforeBytes: witness.freeBeforeBytes,
    freeAfterBytes: witness.freeAfterBytes,
    totalBytes: witness.totalBytes,
    deltaBytes: witness.deltaBytes,
    deltaFloorBytes: witness.deltaFloorBytes,
    controlAllocationRequestBytes: witness.controlAllocationRequestBytes,
    controlBlockCount: witness.controlBlockCount,
    controlFreeBeforeBytes: witness.controlFreeBeforeBytes,
    controlFreeAfterBytes: witness.controlFreeAfterBytes,
    controlDeltaBytes: witness.controlDeltaBytes,
    embeddedVectorDim: witness.embeddedVectorDim,
  };
}

/**
 * @internal Map the native open-time snapshot into the public SDK shape.
 * Kept separate so the binding contract is testable without a CUDA host.
 */
export function mapOpenReport(r: NativeOpenReport): OpenReport {
  return {
    schemaVersionBefore: r.schemaVersionBefore,
    schemaVersionAfter: r.schemaVersionAfter,
    migrationSteps: r.migrationSteps,
    embedderWarmupMs: r.embedderWarmupMs,
    queryBackend: r.queryBackend,
    defaultEmbedder: r.defaultEmbedder,
    embedderDownloadMs: r.embedderDownloadMs,
    embedderEvents: r.embedderEvents.map(mapEmbedderEvent),
    embedderMeanCenteringRequired: r.embedderMeanCenteringRequired,
    embedderMeanVecPinned: r.embedderMeanVecPinned,
    denseDisabled: r.denseDisabled,
    denseDisabledReason: r.denseDisabledReason ?? null,
    embedderDeviceResolution: r.embedderDeviceResolution
      ? mapDeviceResolution(r.embedderDeviceResolution)
      : null,
    rerankerDeviceResolution: r.rerankerDeviceResolution
      ? mapDeviceResolution(r.rerankerDeviceResolution)
      : null,
    embedderGpuAllocationWitness: r.embedderGpuAllocationWitness
      ? mapGpuAllocationWitness(r.embedderGpuAllocationWitness)
      : null,
  };
}

export interface CounterSnapshot {
  queries: number;
  writes: number;
  writeRows: number;
  adminOps: number;
  cacheHit: number;
  cacheMiss: number;
}

export interface SubscriberEvent {
  [key: string]: unknown;
}

export type SubscriberCallback = (event: SubscriberEvent) => void;

export interface AttachSubscriberOptions {
  heartbeatIntervalMs?: number;
}

export interface AdminConfigureOptions {
  name: string;
  body: string;
}

async function intercept<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    rethrowTyped(err);
  }
}

function interceptSync<T>(fn: () => T): T {
  try {
    return fn();
  } catch (err) {
    rethrowTyped(err);
  }
}

/**
 * 0.8.8 Slice 15 — validate a relevance-label id array before the native call
 * (mirrors the Python `_validate_id_list` guard for cross-SDK parity). Ids are
 * non-negative integers (the stable `SearchHit.id` identity carrier).
 */
function validateIdArray(name: string, value: number[]): void {
  if (!Array.isArray(value)) {
    throw new TypeError(`${name} must be an array of non-negative integers`);
  }
  for (const item of value) {
    if (!Number.isInteger(item)) {
      throw new RangeError(`${name} must contain only integers, got ${typeof item}`);
    }
    if (item < 0) {
      throw new RangeError(`${name} must contain only non-negative integers, got ${item}`);
    }
  }
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
    const inner = await intercept(() => native.Engine.open(path, options));
    return new Engine(inner, options.engineConfig ?? {});
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
  async write(batch: unknown[] = []): Promise<WriteReceipt> {
    validateWriteFfiTree(batch);
    return intercept(() => this.#native.write(batch));
  }

  /** Register one pinned dependency; exact replay is a no-op success. */
  async registerSourceDependency(
    request: SourceDependencyRegistrationV1,
  ): Promise<SourceDependencyV1> {
    return dependencyResponse(
      await intercept(() => this.#native.registerSourceDependency(request)),
    );
  }

  /** Atomically apply a bounded set of caller-decided memory operations. */
  async actuate(request: ActuationBatchV1): Promise<ActuationReceiptV1> {
    try {
      return actuationResponse(await this.#native.actuate(sanitizeActuationFfiTree(request)));
    } catch (err) {
      rethrowTyped(err);
    }
  }

  /** Return at most 100 dependencies in stable derived-revision order. */
  async dependenciesForSource(request: DependencySourceLookupV1): Promise<DependencyListV1> {
    const value = await intercept(() => this.#native.dependenciesForSource(request));
    if (value.schemaVersion !== 1) {
      throw new DependencyError(
        "dependency unsupported_schema_version at /schemaVersion",
        "unsupported_schema_version",
        "/schemaVersion",
      );
    }
    return { schemaVersion: 1, items: value.items.map(dependencyResponse) };
  }

  /** Return the dependency for one derived revision, or `null`. */
  async dependencyForDerived(
    request: DependencyDerivedLookupV1,
  ): Promise<SourceDependencyV1 | null> {
    const value = await intercept(() => this.#native.dependencyForDerived(request));
    return value === null ? null : dependencyResponse(value);
  }

  /** Return current closure status, or `null` for an absent opaque ID. */
  async readDependencyClosure(request: ClosureLookupV1): Promise<ClosureStatusV1 | null> {
    try {
      const value = await this.#native.readDependencyClosure(request);
      return value === null ? null : closureResponse(value);
    } catch (err) {
      rethrowTyped(err);
    }
  }

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
  ): Promise<void> {
    validateFfiString(logicalId);
    if (reason !== undefined && reason !== null) {
      validateFfiString(reason);
    }
    return intercept(() => this.#native.transition(logicalId, toState, reason ?? null));
  }

  /**
   * OPP-12 Phase-1 (0.8.19 Slice 10) — `purge` lifecycle verb. Irreversible,
   * deleted-first, idempotent hard-erase of a governed node across every
   * row-owned target (all versions + FTS/vector shadows + touching edges,
   * cascade-removed). A SEPARATE verb from `transition` (NOT a recovery-denylist
   * name). Keys on the bare `logicalId` (`l:` only) — a non-`l:` id throws
   * `NotLifecycleAddressableError`; a non-`deleted` node throws
   * `IllegalTransitionError`.
   */
  async purge(logicalId: string): Promise<void> {
    validateFfiString(logicalId);
    return intercept(() => this.#native.purge(logicalId));
  }

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
  async eraseSource(sourceId: string): Promise<EraseReport> {
    validateFfiString(sourceId);
    return intercept(() => this.#native.eraseSource(sourceId));
  }

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
  async configureProjections(specs: ProjectionSpec[], drop?: string[]): Promise<ProjectionDelta> {
    // TC-47 (keystone terminal codex P2) — every string in the spec/drop tree
    // (projection name, each role, ftsTokenizer, vectorEmbedder, each drop entry)
    // crosses to native. napi-rs silently replaces a lone UTF-16 surrogate with
    // U+FFFD BEFORE the Rust-side guard runs, so — exactly like `write` — the
    // surrogate check must happen JS-side or the mangled U+FFFD is persisted
    // instead of raising WriteValidationError. (A NUL survives the napi UTF-8
    // path as a real byte and is already caught Rust-side; the surrogate is not.)
    validateFfiTree(specs);
    if (drop !== undefined) validateFfiTree(drop);
    // 0.8.20 keystone closeout fix-4 — normalize an explicit `null` sub-field to
    // `undefined` (⇒ napi `None`). `read.projections` EMITS `ftsTokenizer: null`
    // / `vectorEmbedder: null` for a spec with no custom sub-field, but napi-rs
    // rejects an explicit `null` for an `Option<String>` field with an opaque
    // `StringExpected` — so feeding read output straight back into
    // `configureProjections` threw, diverging from pyo3 (which accepts `None`)
    // and breaking the read→configure round-trip. Mapping `null → undefined`
    // here makes the two bindings behave identically and keeps the caller's
    // objects untouched (a shallow copy per spec).
    // 0.8.20 Slice 20 (R-20-DR) — `vectorDenseReadiness` gets the SAME
    // null→undefined normalization: `read.projections` emits an explicit `null`
    // for a spec with no vector sub-object, and napi-rs would reject that for an
    // `Option<String>` field. Its non-null value is carried through unchanged
    // (it is inert engine-side, so the read→configure round-trip stays a no-op).
    const nativeSpecs = specs.map((s) => ({
      ...s,
      ftsTokenizer: s.ftsTokenizer ?? undefined,
      vectorEmbedder: s.vectorEmbedder ?? undefined,
      vectorDenseReadiness: s.vectorDenseReadiness ?? undefined,
      source: s.source ?? undefined,
    }));
    return intercept(() => this.#native.configureProjections(nativeSpecs, drop ?? null));
  }

  /** Mint a restart-stable read context bound to this database state. */
  async freezeReadContext(context: ReadContextV1): Promise<FrozenReadContextV1> {
    validateReadContext(context);
    const frozen = await intercept(() => this.#native.freezeReadContext(context));
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

  /** Trace one reciprocal registered dependency under a frozen context. */
  async traceDependency(request: DependencyTraceRequestV1): Promise<DependencyTraceResultV1> {
    validateDependencyTraceRequest(request);
    const encoded = await intercept(() =>
      this.#native.traceDependency(
        request.rootRevisionId,
        request.direction,
        nativeFrozenContext(request.context),
        request.maxRelations,
        request.maxWorkUnits,
      ),
    );
    let value: unknown;
    try {
      value = JSON.parse(encoded);
    } catch {
      dependencyTraceRequestError("trace_corrupt", "");
    }
    return validateDependencyTraceResponse(value);
  }

  /** Search under an Engine-authenticated frozen validity/eligibility context. */
  async searchFrozen(
    query: string,
    context: FrozenReadContextV1,
    options: FrozenSearchOptions = {},
  ): Promise<SearchResult> {
    const nativeContext = nativeFrozenContext(context);
    await intercept(() => this.#native.validateFrozenReadContext(nativeContext));
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
      this.#native.searchFrozen(
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

  /** Search under a frozen context and attach one evidence reference per hit. */
  async searchWithEvidence(request: EvidenceSearchRequestV1): Promise<EvidenceSearchResultV1> {
    assertKnownEvidenceKeys(request, [
      "schemaVersion",
      "query",
      "context",
      "rerankDepth",
      "useGraphArm",
      "alpha",
      "poolN",
      "includeExplanation",
      "limit",
    ]);
    if (request.schemaVersion !== 1) {
      evidenceRequestError("unsupported_schema_version", "/schemaVersion");
    }
    validateFfiString(request.query);
    validateEvidenceFrozenContext(request.context);
    const limit = validateRankedResultLimit("limit", request.limit);
    if (request.rerankDepth !== undefined) {
      if (request.rerankDepth < 0) {
        throw new InvalidArgumentError(`rerankDepth must be >= 0; got ${request.rerankDepth}`);
      }
      if (!Number.isInteger(request.rerankDepth) || request.rerankDepth > 0xffffffff) {
        throw new RangeError(
          `rerankDepth must be an integer in 0..=4294967295; got ${request.rerankDepth}`,
        );
      }
    }
    if (request.useGraphArm !== undefined && typeof request.useGraphArm !== "boolean") {
      throw new TypeError(`useGraphArm must be a boolean, got ${typeof request.useGraphArm}`);
    }
    if (
      request.alpha !== undefined &&
      (typeof request.alpha !== "number" || !Number.isFinite(request.alpha))
    ) {
      throw new RangeError(`alpha must be a finite number, got ${request.alpha}`);
    }
    if (request.poolN !== undefined) {
      if (request.poolN < 0) {
        throw new InvalidArgumentError(`poolN must be >= 0; got ${request.poolN}`);
      }
      if (!Number.isInteger(request.poolN) || request.poolN > 0xffffffff) {
        throw new RangeError(`poolN must be an integer in 0..=4294967295; got ${request.poolN}`);
      }
    }
    if (
      request.includeExplanation !== undefined &&
      typeof request.includeExplanation !== "boolean"
    ) {
      throw new TypeError(
        `includeExplanation must be a boolean, got ${typeof request.includeExplanation}`,
      );
    }
    const result = await intercept(() =>
      this.#native.searchWithEvidence(
        request.query,
        nativeFrozenContext(request.context),
        request.rerankDepth,
        request.useGraphArm,
        request.alpha,
        request.poolN,
        request.includeExplanation,
        limit,
      ),
    );
    return mapNativeEvidenceSearch(result);
  }

  /** Resolve exact source bytes under an equivalent frozen context. */
  async resolveEvidence(request: EvidenceResolveRequestV1): Promise<ResolvedEvidenceV1> {
    assertKnownEvidenceKeys(request, ["schemaVersion", "evidenceRef", "context"]);
    if (request.schemaVersion !== 1) {
      evidenceRequestError("unsupported_schema_version", "/schemaVersion");
    }
    validateFfiString(request.evidenceRef);
    validateEvidenceFrozenContext(request.context);
    return mapNativeResolvedEvidence(
      await intercept(() =>
        this.#native.resolveEvidence(request.evidenceRef, nativeFrozenContext(request.context)),
      ),
    );
  }

  /** Search and graph-expand on one frozen reader transaction. */
  async searchExpandFrozen(
    query: string,
    context: FrozenReadContextV1,
    depth: number,
    options: SearchExpandOptions = {},
  ): Promise<SearchExpandResult> {
    const nativeContext = nativeFrozenContext(context);
    await intercept(() => this.#native.validateFrozenReadContext(nativeContext));
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
      this.#native.searchExpandFrozen(query, nativeContext, depth, options.searchLimit),
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
  ): Promise<SearchResult> {
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
      this.#native.search(
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
  async searchTextOnly(query: string, view?: SearchOptions): Promise<SearchResult> {
    validateFfiString(query);
    const searchOptions = splitSearchOptions(view);
    const r = await intercept(() =>
      this.#native.searchTextOnly(query, searchOptions.view, searchOptions.limit),
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

  /** Search exactly one declared `searchable` property-FTS projection. */
  async searchProjectedText(
    query: string,
    name: string,
    filter?: SearchFilter,
    view?: SearchOptions,
  ): Promise<SearchResult> {
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
      this.#native.searchProjectedText(
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

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — `true` iff the engine opened degraded (the #5
   * self-check found a vector-equivalence divergence and every dense arm is
   * refusing). Mirrors `OpenReport.denseDisabled`.
   */
  denseDisabled(): boolean {
    return this.#native.denseDisabled();
  }

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — the human-readable reason for the degraded state,
   * or `null` when dense is healthy.
   */
  denseDisabledReason(): string | null {
    return this.#native.denseDisabledReason() ?? null;
  }

  /**
   * 0.8.18 Slice 5 (R-VEQ-6) — telemetry counter: query-time dense-arm refusals
   * raised because the engine opened degraded.
   */
  vectorEquivalenceRefusalCount(): number {
    return this.#native.vectorEquivalenceRefusalCount();
  }

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
  ): Promise<IngestWithExtractorReceipt> {
    // fix-28 [P2]: validate all user-controlled strings at the FFI boundary.
    for (const arg of cmd) validateFfiString(arg);
    for (const doc of documents) {
      validateFfiString(doc.sourceDocId);
      validateFfiString(doc.body);
    }
    const nativeDocs = documents.map((d) => ({ sourceDocId: d.sourceDocId, body: d.body }));
    const r = await intercept(() => this.#native.ingestWithExtractor(cmd, nativeDocs));
    return {
      nodesWritten: r.nodesWritten,
      edgesWritten: r.edgesWritten,
      docsProcessed: r.docsProcessed,
    };
  }

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
  ): Promise<ConsolidateReceipt> {
    // Validate all user-controlled strings at the FFI boundary.
    for (const arg of cmd) validateFfiString(arg);
    for (const axis of axes) {
      validateFfiString(axis.subjectLogicalId);
      validateFfiString(axis.relation);
    }
    const nativeAxes = axes.map((a) => ({
      subjectLogicalId: a.subjectLogicalId,
      relation: a.relation,
    }));
    const r = await intercept(() => this.#native.consolidateWithProvider(cmd, nativeAxes));
    return {
      clustersProcessed: r.clustersProcessed,
      edgesExamined: r.edgesExamined,
      edgesKept: r.edgesKept,
      edgesInvalidated: r.edgesInvalidated,
      edgesSuperseded: r.edgesSuperseded,
    };
  }

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
  async embed(text: string): Promise<number[]> {
    validateFfiString(text);
    return intercept(() => this.#native.embed(text));
  }

  /**
   * 0.8.8 Slice 15 (OPP-9) — enable opt-in local telemetry capture to a JSONL
   * `sinkPath`. Off by default; local file only (no egress). Once enabled, each
   * `search` records a query→result event keyed on the stable id, and
   * `recordFeedback` appends correlated agent labels. The query text and
   * `sourceId` are NEVER written (privacy, ADR §C).
   */
  async enableTelemetry(sinkPath: string): Promise<void> {
    validateFfiString(sinkPath);
    await intercept(() => this.#native.enableTelemetry(sinkPath));
  }

  /**
   * 0.8.8 Slice 15 — the most-recent captured `queryId` (for `recordFeedback`),
   * or `null` when telemetry is off / no query has been captured yet.
   */
  lastTelemetryQueryId(): string | null {
    return interceptSync(() => this.#native.lastTelemetryQueryId());
  }

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
  ): Promise<void> {
    validateFfiString(queryId);
    validateFfiString(labelSource);
    validateIdArray("relevantIds", relevantIds);
    validateIdArray("irrelevantIds", irrelevantIds);
    await intercept(() =>
      this.#native.recordFeedback(queryId, relevantIds, irrelevantIds, labelSource),
    );
  }

  counters(): CounterSnapshot {
    return interceptSync(() => this.#native.counters());
  }

  openReport(): OpenReport {
    return interceptSync(() => mapOpenReport(this.#native.openReport()));
  }

  setProfiling(enabled: boolean): void {
    interceptSync(() => this.#native.setProfiling(enabled));
  }

  setSlowThresholdMs(value: number): void {
    interceptSync(() => this.#native.setSlowThresholdMs(value));
  }

  attachSubscriber(callback: SubscriberCallback, options: AttachSubscriberOptions = {}): void {
    interceptSync(() => this.#native.attachSubscriber(callback, options));
  }

  /** @internal — handle to the napi-rs binding, used by `admin.configure`. */
  get _native(): NativeEngine {
    return this.#native;
  }
}

export const admin = {
  async configure(engine: Engine, options: AdminConfigureOptions): Promise<WriteReceipt> {
    validateFfiString(options.name);
    validateFfiString(options.body);
    return intercept(() => native.adminConfigure(engine._native, options));
  },
};

// ===== Slice 20 (G5/G6) — graph traversal ================================

/**
 * Slice 20 (G6) — one node reached by BFS traversal in `graph.searchExpand`.
 *
 * `hopCount` is the BFS distance from the nearest search-hit root. Only nodes
 * NOT already in the search-hit set appear in `SearchExpandResult.expanded`
 * (deduplication: search score takes priority).
 */
export interface ExpandedNode {
  node: NodeRecord;
  hopCount: number;
}

/**
 * Slice 20 (G6) — result of `graph.searchExpand`.
 *
 * `searchHits` — original RRF-scored results from the search step.
 * `expanded`   — nodes reachable from any search hit within `depth` hops
 *                that are NOT in `searchHits`.
 * `allLogicalIds` — deduplicated union of both sets.
 */
export interface SearchExpandResult {
  searchHits: SearchHit[];
  expanded: ExpandedNode[];
  allLogicalIds: string[];
}

/** Direction to follow when traversing `canonical_edges`. */
export type TraversalDirection = "outgoing" | "incoming" | "both";

export interface GraphQuerySeedV1 {
  schemaVersion: 1;
  type: "query";
  text: string;
  rankedLimit: number;
}

export interface GraphExplicitSeedV1 {
  schemaVersion: 1;
  type: "explicit";
  logicalIds: IdSpace[];
}

export type GraphSeedV1 = GraphQuerySeedV1 | GraphExplicitSeedV1;

export interface CurrentGraphReadContextV1 {
  schemaVersion: 1;
  type: "current";
  context: ReadContextV1;
}

export interface FrozenGraphReadContextV1 {
  schemaVersion: 1;
  type: "frozen";
  context: FrozenReadContextV1;
}

export type GraphReadContextV1 = CurrentGraphReadContextV1 | FrozenGraphReadContextV1;

export interface GraphExpandRequestV1 {
  schemaVersion: 1;
  seed: GraphSeedV1;
  direction: TraversalDirection;
  edgeKinds: string[];
  targetKinds: string[];
  context: GraphReadContextV1;
  maxDepth: number;
  resultLimit: number;
  maxWorkUnits: string;
  includeExplanation: boolean;
}

export interface ResolvedGraphSeedV1 {
  schemaVersion: 1;
  logicalId: string;
  seedOrdinal: number;
  queryScore: number | null;
}

export interface GraphOriginV1 {
  schemaVersion: 1;
  seedLogicalId: string;
  seedOrdinal: number;
  predecessorLogicalId: string;
  targetLogicalId: string;
  hopCount: number;
  terminalEdgeKind: string;
  terminalDirection: TraversalDirection;
}

export interface GraphTargetV1 {
  schemaVersion: 1;
  logicalId: string;
  kind: string;
  body: string;
  writeCursor: string;
  origin: GraphOriginV1;
}

export type GraphSeedSourceV1 = "query" | "explicit";
export type GraphReadModeV1 = "current" | "frozen";
export type GraphProjectionOriginV1 =
  "not_applicable" | "fresh" | "legacy_unverified" | "configuration" | "rebuild";
export type GraphProjectionReadinessV1 =
  "not_applicable" | "ready" | "processing" | "blocked" | "deferred" | "degraded";
export type GraphExpansionDegradationCodeV1 =
  | "query_seed_text_fallback"
  | "projection_legacy_unverified"
  | "projection_processing"
  | "projection_blocked"
  | "projection_deferred"
  | "projection_degraded";

export interface GraphTargetExplanationV1 {
  schemaVersion: 1;
  targetIndex: number;
  origin: GraphOriginV1;
  lifecycleState: StructuralLifecycleStateV1;
  dependencyState: StructuralDependencyStateV1;
}

export interface GraphExpansionExplanationV1 {
  schemaVersion: 1;
  correlationId: string;
  seedSource: GraphSeedSourceV1;
  readMode: GraphReadModeV1;
  projectionGenerationId: string | null;
  projectionOrigin: GraphProjectionOriginV1;
  projectionReadiness: GraphProjectionReadinessV1;
  degradationCodes: GraphExpansionDegradationCodeV1[];
  perTarget: GraphTargetExplanationV1[];
}

export interface GraphExpandResultV1 {
  schemaVersion: 1;
  seeds: ResolvedGraphSeedV1[];
  targets: GraphTargetV1[];
  complete: true;
  workUnits: string;
  degradationCodes: GraphExpansionDegradationCodeV1[];
  explanation: GraphExpansionExplanationV1 | null;
}

type GraphObject = Record<string, unknown>;

function graphRefuse(reason: string, fieldPath: string): never {
  throw new GraphExpansionError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

function graphObject(value: unknown, path: string): GraphObject {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    graphRefuse("graph_corrupt", path);
  }
  return value as GraphObject;
}

function graphField(value: GraphObject, name: string, path: string): unknown {
  if (!Object.hasOwn(value, name)) graphRefuse("graph_corrupt", path);
  return value[name];
}

function graphSchema(value: GraphObject, path: string): void {
  if (graphField(value, "schemaVersion", path) !== 1) {
    graphRefuse("unsupported_schema_version", path);
  }
}

function graphString(value: unknown, path: string): string {
  if (typeof value !== "string") graphRefuse("graph_corrupt", path);
  return value;
}

function graphU32(value: unknown, path: string): number {
  if (!Number.isInteger(value) || typeof value !== "number" || value < 0 || value > 0xffff_ffff) {
    graphRefuse("graph_corrupt", path);
  }
  return value;
}

function graphU64(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^(?:0|[1-9][0-9]*)$/.test(value) ||
    BigInt(value) > 0xffff_ffff_ffff_ffffn
  ) {
    graphRefuse("graph_corrupt", path);
  }
  return value;
}

function graphArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) graphRefuse("graph_corrupt", path);
  return value;
}

function graphEnum<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    graphRefuse("graph_corrupt", path);
  }
  return value as T;
}

function validateGraphOrigin(value: unknown, base: string): GraphOriginV1 {
  const object = graphObject(value, base);
  graphSchema(object, `${base}/schemaVersion`);
  return {
    schemaVersion: 1,
    seedLogicalId: graphString(
      graphField(object, "seedLogicalId", `${base}/seedLogicalId`),
      `${base}/seedLogicalId`,
    ),
    seedOrdinal: graphU32(
      graphField(object, "seedOrdinal", `${base}/seedOrdinal`),
      `${base}/seedOrdinal`,
    ),
    predecessorLogicalId: graphString(
      graphField(object, "predecessorLogicalId", `${base}/predecessorLogicalId`),
      `${base}/predecessorLogicalId`,
    ),
    targetLogicalId: graphString(
      graphField(object, "targetLogicalId", `${base}/targetLogicalId`),
      `${base}/targetLogicalId`,
    ),
    hopCount: graphU32(graphField(object, "hopCount", `${base}/hopCount`), `${base}/hopCount`),
    terminalEdgeKind: graphString(
      graphField(object, "terminalEdgeKind", `${base}/terminalEdgeKind`),
      `${base}/terminalEdgeKind`,
    ),
    terminalDirection: graphEnum(
      graphField(object, "terminalDirection", `${base}/terminalDirection`),
      ["incoming", "outgoing", "both"] as const,
      `${base}/terminalDirection`,
    ),
  };
}

const graphDegradationCodes = [
  "query_seed_text_fallback",
  "projection_legacy_unverified",
  "projection_processing",
  "projection_blocked",
  "projection_deferred",
  "projection_degraded",
] as const;

/** Validate an additive native graph-expansion response and its coherence. */
export function validateGraphExpandResult(value: unknown): GraphExpandResultV1 {
  const root = graphObject(value, "");
  graphSchema(root, "/schemaVersion");
  const seeds = graphArray(graphField(root, "seeds", "/seeds"), "/seeds").map((seed, index) => {
    const base = `/seeds/${index}`;
    const object = graphObject(seed, base);
    graphSchema(object, `${base}/schemaVersion`);
    const seedOrdinal = graphU32(
      graphField(object, "seedOrdinal", `${base}/seedOrdinal`),
      `${base}/seedOrdinal`,
    );
    if (seedOrdinal !== index) graphRefuse("graph_corrupt", `${base}/seedOrdinal`);
    const queryScore = graphField(object, "queryScore", `${base}/queryScore`);
    if (queryScore !== null && (typeof queryScore !== "number" || !Number.isFinite(queryScore))) {
      graphRefuse("graph_corrupt", `${base}/queryScore`);
    }
    return {
      schemaVersion: 1 as const,
      logicalId: graphString(
        graphField(object, "logicalId", `${base}/logicalId`),
        `${base}/logicalId`,
      ),
      seedOrdinal,
      queryScore: queryScore as number | null,
    };
  });
  const targets = graphArray(graphField(root, "targets", "/targets"), "/targets").map(
    (target, index) => {
      const base = `/targets/${index}`;
      const object = graphObject(target, base);
      graphSchema(object, `${base}/schemaVersion`);
      return {
        schemaVersion: 1 as const,
        logicalId: graphString(
          graphField(object, "logicalId", `${base}/logicalId`),
          `${base}/logicalId`,
        ),
        kind: graphString(graphField(object, "kind", `${base}/kind`), `${base}/kind`),
        body: graphString(graphField(object, "body", `${base}/body`), `${base}/body`),
        writeCursor: graphU64(
          graphField(object, "writeCursor", `${base}/writeCursor`),
          `${base}/writeCursor`,
        ),
        origin: validateGraphOrigin(
          graphField(object, "origin", `${base}/origin`),
          `${base}/origin`,
        ),
      };
    },
  );
  if (graphField(root, "complete", "/complete") !== true) graphRefuse("graph_corrupt", "/complete");
  const workUnits = graphU64(graphField(root, "workUnits", "/workUnits"), "/workUnits");
  const degradationCodes = graphArray(
    graphField(root, "degradationCodes", "/degradationCodes"),
    "/degradationCodes",
  ).map((code, index) => graphEnum(code, graphDegradationCodes, `/degradationCodes/${index}`));
  targets.forEach((target, index) => {
    const seed = seeds[target.origin.seedOrdinal];
    if (seed === undefined) graphRefuse("graph_corrupt", `/targets/${index}/origin/seedOrdinal`);
    if (target.origin.seedLogicalId !== seed.logicalId)
      graphRefuse("graph_corrupt", `/targets/${index}/origin/seedLogicalId`);
    if (target.origin.targetLogicalId !== target.logicalId)
      graphRefuse("graph_corrupt", `/targets/${index}/origin/targetLogicalId`);
  });

  const rawExplanation = graphField(root, "explanation", "/explanation");
  let explanation: GraphExpansionExplanationV1 | null = null;
  if (rawExplanation !== null) {
    const base = "/explanation";
    const object = graphObject(rawExplanation, base);
    graphSchema(object, `${base}/schemaVersion`);
    const rawPerTarget = graphArray(
      graphField(object, "perTarget", `${base}/perTarget`),
      `${base}/perTarget`,
    );
    if (rawPerTarget.length !== targets.length) graphRefuse("graph_corrupt", `${base}/perTarget`);
    const perTarget = rawPerTarget.map((item, index) => {
      const itemBase = `${base}/perTarget/${index}`;
      const itemObject = graphObject(item, itemBase);
      graphSchema(itemObject, `${itemBase}/schemaVersion`);
      const targetIndex = graphU32(
        graphField(itemObject, "targetIndex", `${itemBase}/targetIndex`),
        `${itemBase}/targetIndex`,
      );
      if (targetIndex !== index) graphRefuse("graph_corrupt", `${itemBase}/targetIndex`);
      const origin = validateGraphOrigin(
        graphField(itemObject, "origin", `${itemBase}/origin`),
        `${itemBase}/origin`,
      );
      if (JSON.stringify(origin) !== JSON.stringify(targets[index]?.origin))
        graphRefuse("graph_corrupt", `${itemBase}/origin`);
      return {
        schemaVersion: 1 as const,
        targetIndex,
        origin,
        lifecycleState: graphEnum(
          graphField(itemObject, "lifecycleState", `${itemBase}/lifecycleState`),
          ["node_pending", "node_active", "node_deleted", "edge_valid"] as const,
          `${itemBase}/lifecycleState`,
        ),
        dependencyState: graphEnum(
          graphField(itemObject, "dependencyState", `${itemBase}/dependencyState`),
          ["not_applicable", "not_registered", "registered"] as const,
          `${itemBase}/dependencyState`,
        ),
      };
    });
    const explanationDegradations = graphArray(
      graphField(object, "degradationCodes", `${base}/degradationCodes`),
      `${base}/degradationCodes`,
    );
    if (JSON.stringify(explanationDegradations) !== JSON.stringify(degradationCodes))
      graphRefuse("graph_corrupt", `${base}/degradationCodes`);
    const projectionGenerationId = graphField(
      object,
      "projectionGenerationId",
      `${base}/projectionGenerationId`,
    );
    if (projectionGenerationId !== null && typeof projectionGenerationId !== "string")
      graphRefuse("graph_corrupt", `${base}/projectionGenerationId`);
    explanation = {
      schemaVersion: 1,
      correlationId: graphString(
        graphField(object, "correlationId", `${base}/correlationId`),
        `${base}/correlationId`,
      ),
      seedSource: graphEnum(
        graphField(object, "seedSource", `${base}/seedSource`),
        ["query", "explicit"] as const,
        `${base}/seedSource`,
      ),
      readMode: graphEnum(
        graphField(object, "readMode", `${base}/readMode`),
        ["current", "frozen"] as const,
        `${base}/readMode`,
      ),
      projectionGenerationId,
      projectionOrigin: graphEnum(
        graphField(object, "projectionOrigin", `${base}/projectionOrigin`),
        ["not_applicable", "fresh", "legacy_unverified", "configuration", "rebuild"] as const,
        `${base}/projectionOrigin`,
      ),
      projectionReadiness: graphEnum(
        graphField(object, "projectionReadiness", `${base}/projectionReadiness`),
        ["not_applicable", "ready", "processing", "blocked", "deferred", "degraded"] as const,
        `${base}/projectionReadiness`,
      ),
      degradationCodes,
      perTarget,
    };
  }
  return {
    schemaVersion: 1,
    seeds,
    targets,
    complete: true,
    workUnits,
    degradationCodes,
    explanation,
  };
}

function validateGraphExpandRequest(value: unknown): asserts value is GraphExpandRequestV1 {
  const root = graphObject(value, "");
  if (root.schemaVersion !== 1) graphRefuse("unsupported_schema_version", "/schemaVersion");
  const allowed = new Set([
    "schemaVersion",
    "seed",
    "direction",
    "edgeKinds",
    "targetKinds",
    "context",
    "maxDepth",
    "resultLimit",
    "maxWorkUnits",
    "includeExplanation",
  ]);
  const unknown = Object.keys(root)
    .filter((key) => !allowed.has(key))
    .sort()[0];
  if (unknown !== undefined)
    graphRefuse("unknown_field", `/${unknown.replaceAll("~", "~0").replaceAll("/", "~1")}`);
  const close = (object: GraphObject, fields: readonly string[], base: string): void => {
    const extra = Object.keys(object)
      .filter((key) => !fields.includes(key))
      .sort()[0];
    if (extra !== undefined) {
      const escaped = extra.replaceAll("~", "~0").replaceAll("/", "~1");
      graphRefuse("unknown_field", `${base}/${escaped}`);
    }
  };
  const requestObject = (member: unknown, reason: string, path: string): GraphObject => {
    if (member === null || typeof member !== "object" || Array.isArray(member)) {
      graphRefuse(reason, path);
    }
    return member as GraphObject;
  };
  const schema = (object: GraphObject, path: string): void => {
    if (object.schemaVersion !== 1) graphRefuse("unsupported_schema_version", path);
  };

  const seed = requestObject(root.seed, "graph_seed_invalid", "/seed");
  schema(seed, "/seed/schemaVersion");
  if (seed.type === "query") {
    close(seed, ["schemaVersion", "type", "text", "rankedLimit"], "/seed");
    if (
      typeof seed.text !== "string" ||
      !Number.isInteger(seed.rankedLimit) ||
      typeof seed.rankedLimit !== "number"
    )
      graphRefuse("graph_seed_invalid", "/seed");
  } else if (seed.type === "explicit") {
    close(seed, ["schemaVersion", "type", "logicalIds"], "/seed");
    if (!Array.isArray(seed.logicalIds)) graphRefuse("graph_seed_invalid", "/seed/logicalIds");
    seed.logicalIds.forEach((id, index) => {
      const path = `/seed/logicalIds/${index}`;
      const object = requestObject(id, "graph_seed_invalid", path);
      close(object, ["space", "value"], path);
      if (typeof object.space !== "string" || typeof object.value !== "string")
        graphRefuse("graph_seed_invalid", path);
    });
  } else {
    graphRefuse("graph_seed_invalid", "/seed");
  }

  const context = requestObject(root.context, "graph_context_invalid", "/context");
  schema(context, "/context/schemaVersion");
  close(context, ["schemaVersion", "type", "context"], "/context");
  const readContext = requestObject(context.context, "graph_context_invalid", "/context/context");
  if (context.type === "frozen") {
    schema(readContext, "/context/context/schemaVersion");
    close(
      readContext,
      ["schemaVersion", "effectiveValidAt", "context", "token"],
      "/context/context",
    );
    if (
      typeof readContext.token !== "string" ||
      !Number.isInteger(readContext.effectiveValidAt) ||
      typeof readContext.effectiveValidAt !== "number"
    )
      graphRefuse("graph_context_invalid", "/context/context");
  } else if (context.type !== "current") {
    graphRefuse("graph_context_invalid", "/context/type");
  }
  const current =
    context.type === "frozen"
      ? requestObject(readContext.context, "graph_context_invalid", "/context/context/context")
      : readContext;
  const currentBase = context.type === "frozen" ? "/context/context/context" : "/context/context";
  schema(current, `${currentBase}/schemaVersion`);
  close(current, ["schemaVersion", "view", "eligibility"], currentBase);
  const view = requestObject(current.view, "graph_context_invalid", `${currentBase}/view`);
  close(
    view,
    ["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
    `${currentBase}/view`,
  );
  for (const name of ["includeSuperseded", "includeInactive", "includeOutOfWindow"] as const) {
    if (view[name] !== undefined && typeof view[name] !== "boolean")
      graphRefuse("graph_context_invalid", `${currentBase}/view/${name}`);
  }
  if (
    view.validAsOf !== undefined &&
    view.validAsOf !== null &&
    (!Number.isInteger(view.validAsOf) || typeof view.validAsOf !== "number")
  )
    graphRefuse("graph_context_invalid", `${currentBase}/view/validAsOf`);
  const eligibility = requestObject(
    current.eligibility,
    "graph_context_invalid",
    `${currentBase}/eligibility`,
  );
  close(
    eligibility,
    ["sourceType", "kind", "createdAfter", "status", "attributes"],
    `${currentBase}/eligibility`,
  );
  if (!["incoming", "outgoing", "both"].includes(String(root.direction)))
    graphRefuse("graph_direction_invalid", "/direction");
  if (!Array.isArray(root.edgeKinds) || root.edgeKinds.some((item) => typeof item !== "string"))
    graphRefuse("graph_edge_kinds_invalid", "/edgeKinds");
  if (!Array.isArray(root.targetKinds) || root.targetKinds.some((item) => typeof item !== "string"))
    graphRefuse("graph_target_kinds_invalid", "/targetKinds");
  if (!Number.isInteger(root.maxDepth) || typeof root.maxDepth !== "number")
    graphRefuse("graph_depth_invalid", "/maxDepth");
  if (!Number.isInteger(root.resultLimit) || typeof root.resultLimit !== "number")
    graphRefuse("graph_result_limit_invalid", "/resultLimit");
  if (
    typeof root.maxWorkUnits !== "string" ||
    !/^(?:0|[1-9][0-9]*)$/.test(root.maxWorkUnits) ||
    BigInt(root.maxWorkUnits) > 0xffff_ffff_ffff_ffffn
  )
    graphRefuse("graph_work_limit_invalid", "/maxWorkUnits");
  if (typeof root.includeExplanation !== "boolean")
    graphRefuse("graph_context_invalid", "/includeExplanation");
  // The canonical Rust decoder remains the semantic authority for bounds,
  // duplicate seeds, eligibility values, and frozen authentication precedence.
}

function canonicalGraphReadContext(context: ReadContextV1): GraphObject {
  return {
    schemaVersion: context.schemaVersion,
    view: {
      includeSuperseded: context.view.includeSuperseded ?? false,
      includeInactive: context.view.includeInactive ?? false,
      includeOutOfWindow: context.view.includeOutOfWindow ?? false,
      validAsOf: context.view.validAsOf ?? null,
    },
    eligibility: {
      sourceType: context.eligibility.sourceType ?? null,
      kind: context.eligibility.kind ?? null,
      createdAfter: context.eligibility.createdAfter ?? null,
      status: context.eligibility.status ?? null,
      attributes: context.eligibility.attributes ?? [],
    },
  };
}

function canonicalGraphRequest(request: GraphExpandRequestV1): GraphObject {
  const context =
    request.context.type === "current"
      ? {
          schemaVersion: request.context.schemaVersion,
          type: request.context.type,
          context: canonicalGraphReadContext(request.context.context),
        }
      : {
          schemaVersion: request.context.schemaVersion,
          type: request.context.type,
          context: {
            schemaVersion: request.context.context.schemaVersion,
            effectiveValidAt: request.context.context.effectiveValidAt,
            context: canonicalGraphReadContext(request.context.context.context),
            token: request.context.context.token,
          },
        };
  return {
    schemaVersion: request.schemaVersion,
    seed:
      request.seed.type === "query"
        ? {
            schemaVersion: request.seed.schemaVersion,
            type: request.seed.type,
            text: request.seed.text,
            rankedLimit: request.seed.rankedLimit,
          }
        : {
            schemaVersion: request.seed.schemaVersion,
            type: request.seed.type,
            logicalIds: request.seed.logicalIds.map((id) => ({ space: id.space, value: id.value })),
          },
    direction: request.direction,
    edgeKinds: request.edgeKinds,
    targetKinds: request.targetKinds,
    context,
    maxDepth: request.maxDepth,
    resultLimit: request.resultLimit,
    maxWorkUnits: request.maxWorkUnits,
    includeExplanation: request.includeExplanation,
  };
}

export const graph = {
  /** Run one bounded, deterministic, all-or-nothing graph expansion. */
  async expand(engine: Engine, request: GraphExpandRequestV1): Promise<GraphExpandResultV1> {
    validateGraphExpandRequest(request);
    const encoded = await intercept(() =>
      engine._native.graphExpand(JSON.stringify(canonicalGraphRequest(request))),
    );
    let decoded: unknown;
    try {
      decoded = JSON.parse(encoded);
    } catch {
      graphRefuse("graph_corrupt", "");
    }
    return validateGraphExpandResult(decoded);
  },
  /**
   * G5 — bounded BFS from `logicalId` over `canonical_edges`.
   *
   * `depth` must be 1–3; rejects depth > 3 with `InvalidArgumentError`.
   * `direction` is `"outgoing"`, `"incoming"`, or `"both"`.
   * Returns up to 50 `NodeRecord`s reachable within `depth` hops (root excluded).
   * Edges with `t_invalid` in the past are not traversed (valid-time filter).
   */
  async neighbors(
    engine: Engine,
    logicalId: string,
    depth: number,
    direction: TraversalDirection = "both",
    view?: ReadView,
  ): Promise<NodeRecord[]> {
    validateFfiString(logicalId);
    if (!Number.isInteger(depth) || depth < 1 || depth > 3) {
      throw new InvalidArgumentError(
        `graph.neighbors depth must be an integer between 1 and 3; got ${depth}`,
      );
    }
    return intercept(() =>
      native.graphNeighbors(engine._native, logicalId, depth, direction, view),
    );
  },

  /**
   * G6 — FTS/vector search followed by bounded BFS expansion.
   *
   * Runs `engine.search(query, filter)` (G1), then expands each hit via
   * `graph.neighbors(depth, "both")`. Nodes appearing in both the search hit
   * set and the traversal reach appear only in `searchHits` (deduplication).
   *
   * `depth` must be 0–3; 0 skips expansion. Raises `InvalidArgumentError` for depth > 3.
   */
  async searchExpand(
    engine: Engine,
    query: string,
    depth: number,
    filter?: SearchFilter,
    options?: SearchExpandOptions,
  ): Promise<SearchExpandResult> {
    validateFfiString(query);
    if (!Number.isInteger(depth) || depth < 0 || depth > 3) {
      throw new InvalidArgumentError(
        `graph.searchExpand depth must be an integer between 0 and 3; got ${depth}`,
      );
    }
    if (filter?.sourceType !== undefined) validateFfiString(filter.sourceType);
    if (filter?.kind !== undefined) validateFfiString(filter.kind);
    if (filter?.status !== undefined) validateFfiString(filter.status);
    const searchLimit = validateRankedResultLimit("searchLimit", options?.searchLimit);
    const r = await intercept(() =>
      native.searchExpand(
        engine._native,
        query,
        depth,
        filter?.sourceType,
        filter?.kind,
        filter?.createdAfter,
        filter?.status,
        searchLimit,
      ),
    );
    return {
      searchHits: r.searchHits.map((h) => ({
        id: { space: h.id.space, value: h.id.value },
        kind: h.kind,
        body: h.body,
        score: h.score,
        branch:
          h.branch === "vector" || h.branch === "text_edge"
            ? (h.branch as SoftFallbackBranch)
            : "text",
        sourceId: h.sourceId ?? null,
        // 0.8.5 — searchExpand never reranks (depth=0) → ceScore is always null.
        ceScore: h.ceScore ?? null,
      })),
      expanded: r.expanded.map((e) => ({
        node: e.node,
        hopCount: e.hopCount,
      })),
      allLogicalIds: r.allLogicalIds,
    };
  },
};
