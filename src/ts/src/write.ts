import type { NativeEngine } from "./binding.js";
// Private write owner for the TypeScript SDK package root.

import { ActuationError, DependencyClosureError, DependencyError, rethrowTyped } from "./errors.js";
import { intercept } from "./native-call.js";
import { sanitizeActuationFfiTree, validateFfiString, validateSourceIdFfiString, validateWriteFfiTree } from "./validation.js";

/**
 * OPP-12 Phase-1 (0.8.19 Slice 10) — the closed lifecycle existence-state
 * vocabulary accepted by {@link Engine.transition} (`toState`). `pending` and
 * `purged` are never legal `transition` targets (create-time-only and
 * purge-only respectively) — passing them surfaces an `IllegalTransitionError`.
 */
export type LifecycleState = "pending" | "active" | "deleted" | "purged";

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
  | { type: "put_derived_edge"; record: Record<string, unknown> }
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

export function closureResponseError(fieldPath: string): never {
  throw new DependencyClosureError(
    `dependency closure response invalid at ${fieldPath}`,
    "unknown_field",
    fieldPath,
  );
}

export function closureRecord(value: unknown, fieldPath: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    closureResponseError(fieldPath);
  }
  return value as Record<string, unknown>;
}

export function closureExactKeys(
  value: Record<string, unknown>,
  allowed: readonly string[],
  fieldPath: string,
): void {
  const allowedSet = new Set(allowed);
  const unknown = Object.keys(value).find((key) => !allowedSet.has(key));
  if (unknown !== undefined) closureResponseError(`${fieldPath}/${unknown}`);
}

export function closureString(value: unknown, fieldPath: string): string {
  if (typeof value !== "string" || value.length === 0) closureResponseError(fieldPath);
  return value;
}

export function closureDecimal(value: unknown, fieldPath: string, signed = false): string {
  const parsed = closureString(value, fieldPath);
  const pattern = signed ? /^(0|-[1-9][0-9]*|[1-9][0-9]*)$/ : /^(0|[1-9][0-9]*)$/;
  if (!pattern.test(parsed)) closureResponseError(fieldPath);
  return parsed;
}

export function closureProofResponse(value: unknown): ClosureProofV1 {
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

export function closureResponse(value: unknown): ClosureStatusV1 {
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

export function dependencyResponse(value: {
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

export function actuationResponse(value: {
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

export async function write(nativeEngine: NativeEngine, batch: unknown[] = []): Promise<WriteReceipt> {
    validateWriteFfiTree(batch);
    return intercept(() => nativeEngine.write(batch));
  }

export async function registerSourceDependency(nativeEngine: NativeEngine, request: SourceDependencyRegistrationV1): Promise<SourceDependencyV1> {
    return dependencyResponse(
      await intercept(() => nativeEngine.registerSourceDependency(request)),
    );
  }

export async function actuate(nativeEngine: NativeEngine, request: ActuationBatchV1): Promise<ActuationReceiptV1> {
    try {
      return actuationResponse(await nativeEngine.actuate(sanitizeActuationFfiTree(request)));
    } catch (err) {
      rethrowTyped(err);
    }
  }

export async function dependenciesForSource(nativeEngine: NativeEngine, request: DependencySourceLookupV1): Promise<DependencyListV1> {
    const value = await intercept(() => nativeEngine.dependenciesForSource(request));
    if (value.schemaVersion !== 1) {
      throw new DependencyError(
        "dependency unsupported_schema_version at /schemaVersion",
        "unsupported_schema_version",
        "/schemaVersion",
      );
    }
    return { schemaVersion: 1, items: value.items.map(dependencyResponse) };
  }

export async function dependencyForDerived(nativeEngine: NativeEngine, request: DependencyDerivedLookupV1): Promise<SourceDependencyV1 | null> {
    const value = await intercept(() => nativeEngine.dependencyForDerived(request));
    return value === null ? null : dependencyResponse(value);
  }

export async function readDependencyClosure(nativeEngine: NativeEngine, request: ClosureLookupV1): Promise<ClosureStatusV1 | null> {
    try {
      const value = await nativeEngine.readDependencyClosure(request);
      return value === null ? null : closureResponse(value);
    } catch (err) {
      rethrowTyped(err);
    }
  }

export async function transition(nativeEngine: NativeEngine, logicalId: string, toState: LifecycleState, reason?: string | null): Promise<void> {
    validateFfiString(logicalId);
    if (reason !== undefined && reason !== null) {
      validateFfiString(reason);
    }
    return intercept(() => nativeEngine.transition(logicalId, toState, reason ?? null));
  }

export async function purge(nativeEngine: NativeEngine, logicalId: string): Promise<void> {
    validateFfiString(logicalId);
    return intercept(() => nativeEngine.purge(logicalId));
  }

export async function eraseSource(nativeEngine: NativeEngine, sourceId: string): Promise<EraseReport> {
    validateSourceIdFfiString(sourceId);
    return intercept(() => nativeEngine.eraseSource(sourceId));
  }

export async function ingestWithExtractor(nativeEngine: NativeEngine, cmd: string[], documents: ExtractDocument[]): Promise<IngestWithExtractorReceipt> {
    // fix-28 [P2]: validate all user-controlled strings at the FFI boundary.
    for (const arg of cmd) validateFfiString(arg);
    for (const doc of documents) {
      validateFfiString(doc.sourceDocId);
      validateFfiString(doc.body);
    }
    const nativeDocs = documents.map((d) => ({ sourceDocId: d.sourceDocId, body: d.body }));
    const r = await intercept(() => nativeEngine.ingestWithExtractor(cmd, nativeDocs));
    return {
      nodesWritten: r.nodesWritten,
      edgesWritten: r.edgesWritten,
      docsProcessed: r.docsProcessed,
    };
  }

export async function consolidateWithProvider(nativeEngine: NativeEngine, cmd: string[], axes: ConsolidateAxis[]): Promise<ConsolidateReceipt> {
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
    const r = await intercept(() => nativeEngine.consolidateWithProvider(cmd, nativeAxes));
    return {
      clustersProcessed: r.clustersProcessed,
      edgesExamined: r.edgesExamined,
      edgesKept: r.edgesKept,
      edgesInvalidated: r.edgesInvalidated,
      edgesSuperseded: r.edgesSuperseded,
    };
  }
