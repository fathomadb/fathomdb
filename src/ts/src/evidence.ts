import type { NativeEngine } from "./binding.js";
// Private evidence owner for the TypeScript SDK package root.
import { type NativeEvidenceSearchResultV1, type NativeResolvedEvidenceV1 } from "./binding.js";
import { DependencyTraceError, EvidenceError, FrozenReadError, InvalidArgumentError } from "./errors.js";
import { intercept } from "./native-call.js";
import { evidencePointerSegment, validateFfiString } from "./validation.js";
import {
  GraphEvidenceResolveRequestV1,
  ResolvedGraphEvidenceV1,
  validateResolvedGraphEvidence,
} from "./graph.js";
import { nativeFrozenContext, validateRankedResultLimit } from "./search.js";

import { validateNativeEvidenceSearch, validateNativeResolvedEvidence } from "./evidence-validation.js";

import { LifecycleState, SourceDependencyV1 } from "./write.js";

import { SoftFallbackBranch, FrozenReadContextV1, mapNativeSearchResult, SearchResult } from "./search.js";


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

export function mapNativeEvidenceSearch(r: NativeEvidenceSearchResultV1): EvidenceSearchResultV1 {
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

export function mapNativeResolvedEvidence(r: NativeResolvedEvidenceV1): ResolvedEvidenceV1 {
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

export function evidenceRequestError(reason: string, fieldPath: string): never {
  throw new EvidenceError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

export function assertKnownEvidenceKeys(value: object, allowed: readonly string[], basePath = ""): void {
  const unknown = Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .sort();
  if (unknown.length > 0) {
    evidenceRequestError("unknown_field", `${basePath}/${evidencePointerSegment(unknown[0]!)}`);
  }
}

export function validateEvidenceFrozenContext(context: FrozenReadContextV1): void {
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

export function dependencyTraceRequestError(reason: string, fieldPath: string): never {
  throw new DependencyTraceError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

export function frozenTraceRequestError(reason: string, fieldPath: string): never {
  throw new FrozenReadError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

export function frozenTraceRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    frozenTraceRequestError("context_invalid", path);
  }
  return value as Record<string, unknown>;
}

export function validateFrozenTraceKeys(
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

export function validateFrozenTraceContext(value: unknown): FrozenReadContextV1 {
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

export function escapeTracePointerToken(value: string): string {
  return value.replaceAll("~", "~0").replaceAll("/", "~1");
}

export function validateDependencyTraceRequest(request: DependencyTraceRequestV1): void {
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

export function traceRecord(value: unknown, path: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value as Record<string, unknown>;
}

export function traceArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) dependencyTraceRequestError("trace_corrupt", path);
  return value;
}

export function traceField(value: Record<string, unknown>, name: string, path: string): unknown {
  if (!(name in value)) dependencyTraceRequestError("trace_corrupt", path);
  return value[name];
}

export function traceSchema(value: Record<string, unknown>, path: string): void {
  if (traceField(value, "schemaVersion", path) !== 1) {
    dependencyTraceRequestError("unsupported_schema_version", path);
  }
}

export function traceId(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(value) ||
    value.startsWith("_fdb:")
  ) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value;
}

export function traceU32(value: unknown, path: string): number {
  if (!Number.isInteger(value) || (value as number) < 0 || (value as number) > 0xffff_ffff) {
    dependencyTraceRequestError("trace_corrupt", path);
  }
  return value as number;
}

export function traceI64(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value)) dependencyTraceRequestError("trace_corrupt", path);
  return value as number;
}

export function traceU64(value: unknown, path: string): string {
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

export function traceBoolean(value: unknown, path: string): boolean {
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

export async function traceDependency(nativeEngine: NativeEngine, request: DependencyTraceRequestV1): Promise<DependencyTraceResultV1> {
    validateDependencyTraceRequest(request);
    const encoded = await intercept(() =>
      nativeEngine.traceDependency(
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

export async function searchWithEvidence(nativeEngine: NativeEngine, request: EvidenceSearchRequestV1): Promise<EvidenceSearchResultV1> {
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
      nativeEngine.searchWithEvidence(
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

export async function resolveEvidence(nativeEngine: NativeEngine, request: EvidenceResolveRequestV1): Promise<ResolvedEvidenceV1> {
    assertKnownEvidenceKeys(request, ["schemaVersion", "evidenceRef", "context"]);
    if (request.schemaVersion !== 1) {
      evidenceRequestError("unsupported_schema_version", "/schemaVersion");
    }
    validateFfiString(request.evidenceRef);
    validateEvidenceFrozenContext(request.context);
    return mapNativeResolvedEvidence(
      await intercept(() =>
        nativeEngine.resolveEvidence(request.evidenceRef, nativeFrozenContext(request.context)),
      ),
    );
  }

export async function resolveGraphEvidence(nativeEngine: NativeEngine, request: GraphEvidenceResolveRequestV1): Promise<ResolvedGraphEvidenceV1> {
    assertKnownEvidenceKeys(request, ["schemaVersion", "evidenceRef", "context"]);
    if (request.schemaVersion !== 1) {
      evidenceRequestError("unsupported_schema_version", "/schemaVersion");
    }
    validateFfiString(request.evidenceRef);
    validateEvidenceFrozenContext(request.context);
    const encoded = await intercept(() =>
      nativeEngine.resolveGraphEvidence(
        request.evidenceRef,
        nativeFrozenContext(request.context),
      ),
    );
    let decoded: unknown;
    try {
      decoded = JSON.parse(encoded);
    } catch {
      evidenceRequestError("evidence_corrupt", "");
    }
    return validateResolvedGraphEvidence(decoded);
  }
