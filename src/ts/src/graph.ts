// Private graph owner for the TypeScript SDK package root.
import { native } from "./binding.js";
import { GraphExpansionError, InvalidArgumentError } from "./errors.js";
import type { NodeRecord, ReadView } from "./read.js";

import { validateFfiString } from "./validation.js";

import { LifecycleState, SourceDependencyV1 } from "./write.js";

import type { Engine } from "./core.js";

import {
  SoftFallbackBranch,
  IdSpace,
  SearchHit,
  SearchFilter,
  ReadContextV1,
  FrozenReadContextV1,
  SearchExpandOptions,
  validateRankedResultLimit,
  StructuralDependencyStateV1,
  StructuralLifecycleStateV1,
} from "./search.js";
import type { EvidenceArtifactLifecycleV1 } from "./evidence.js";
import { evidencePointerSegment } from "./validation.js";

import { intercept } from "./native-call.js";

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
  includeEvidence?: boolean;
}

export interface GraphEvidenceSidecarEntryV1 {
  schemaVersion: 1;
  targetIndex: number;
  targetArtifactRevisionId: string;
  targetEvidenceRef: string;
  terminalEdgeArtifactRevisionId: string;
  terminalEdgeEvidenceRef: string;
}

export interface GraphEvidenceSidecarV1 {
  schemaVersion: 1;
  entries: GraphEvidenceSidecarEntryV1[];
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
  evidence?: GraphEvidenceSidecarV1;
}

export interface GraphEvidenceResolveRequestV1 {
  schemaVersion: 1;
  evidenceRef: string;
  context: FrozenReadContextV1;
}

export type GraphEvidenceArtifactV1 =
  | { artifactClass: "node"; logicalId: string; kind: string; body: string }
  | {
      artifactClass: "edge";
      logicalId: string | null;
      kind: string;
      body: string | null;
      from: string;
      to: string;
    };

export interface ResolvedGraphEvidenceV1 {
  schemaVersion: 1;
  artifactRevisionId: string;
  artifact: GraphEvidenceArtifactV1;
  sourceId: string;
  sourceVersionId: string;
  sourceRevisionId: string;
  locator: { kind: "whole_body"; startInclusive: null; endExclusive: null } |
    { kind: "utf8_bytes"; startInclusive: string; endExclusive: string };
  canonicalSourceBody: string;
  evidenceText: string;
  canonicalSourceHash: { algorithm: "sha256"; digestHex: string };
  effectiveValidAt: number;
  artifactLifecycle: EvidenceArtifactLifecycleV1;
  sourceLifecycleState: LifecycleState;
  dependency: SourceDependencyV1 | null;
}

export type GraphObject = Record<string, unknown>;

export function graphRefuse(reason: string, fieldPath: string): never {
  throw new GraphExpansionError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

export function graphObject(value: unknown, path: string): GraphObject {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    graphRefuse("graph_corrupt", path);
  }
  return value as GraphObject;
}

export function graphField(value: GraphObject, name: string, path: string): unknown {
  if (!Object.hasOwn(value, name)) graphRefuse("graph_corrupt", path);
  return value[name];
}

export function graphExactKeys(value: GraphObject, allowed: readonly string[], path: string): void {
  const unknown = Object.keys(value).filter((key) => !allowed.includes(key)).sort()[0];
  if (unknown !== undefined) graphRefuse("graph_corrupt", `${path}/${evidencePointerSegment(unknown)}`);
}

export function graphArtifactRevision(value: unknown, path: string): string {
  const revision = graphString(value, path);
  if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(revision) || revision.startsWith("_fdb:")) {
    graphRefuse("graph_corrupt", path);
  }
  return revision;
}

export function graphSchema(value: GraphObject, path: string): void {
  if (graphField(value, "schemaVersion", path) !== 1) {
    graphRefuse("unsupported_schema_version", path);
  }
}

export function graphString(value: unknown, path: string): string {
  if (typeof value !== "string") graphRefuse("graph_corrupt", path);
  return value;
}

export function graphU32(value: unknown, path: string): number {
  if (!Number.isInteger(value) || typeof value !== "number" || value < 0 || value > 0xffff_ffff) {
    graphRefuse("graph_corrupt", path);
  }
  return value;
}

export function graphU64(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^(?:0|[1-9][0-9]*)$/.test(value) ||
    BigInt(value) > 0xffff_ffff_ffff_ffffn
  ) {
    graphRefuse("graph_corrupt", path);
  }
  return value;
}

export function graphArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) graphRefuse("graph_corrupt", path);
  return value;
}

export function graphEnum<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    graphRefuse("graph_corrupt", path);
  }
  return value as T;
}

export function validateGraphOrigin(value: unknown, base: string): GraphOriginV1 {
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

export const graphDegradationCodes = [
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
  let evidence: GraphEvidenceSidecarV1 | undefined;
  if (Object.hasOwn(root, "evidence")) {
    const object = graphObject(root.evidence, "/evidence");
    graphExactKeys(object, ["schemaVersion", "entries"], "/evidence");
    graphSchema(object, "/evidence/schemaVersion");
    const entries = graphArray(graphField(object, "entries", "/evidence/entries"), "/evidence/entries")
      .map((raw, index) => {
        const base = `/evidence/entries/${index}`;
        const entry = graphObject(raw, base);
        graphExactKeys(entry, [
          "schemaVersion", "targetIndex", "targetArtifactRevisionId", "targetEvidenceRef",
          "terminalEdgeArtifactRevisionId", "terminalEdgeEvidenceRef",
        ], base);
        graphSchema(entry, `${base}/schemaVersion`);
        const targetIndex = graphU32(
          graphField(entry, "targetIndex", `${base}/targetIndex`),
          `${base}/targetIndex`,
        );
        if (targetIndex !== index) graphRefuse("graph_corrupt", `${base}/targetIndex`);
        const reference = (name: string): string => {
          const value = graphString(graphField(entry, name, `${base}/${name}`), `${base}/${name}`);
          if (value.length === 0 || value.length > 2048) graphRefuse("graph_corrupt", `${base}/${name}`);
          return value;
        };
        return {
          schemaVersion: 1 as const,
          targetIndex,
          targetArtifactRevisionId: graphArtifactRevision(
            graphField(entry, "targetArtifactRevisionId", `${base}/targetArtifactRevisionId`),
            `${base}/targetArtifactRevisionId`,
          ),
          targetEvidenceRef: reference("targetEvidenceRef"),
          terminalEdgeArtifactRevisionId: graphArtifactRevision(
            graphField(entry, "terminalEdgeArtifactRevisionId", `${base}/terminalEdgeArtifactRevisionId`),
            `${base}/terminalEdgeArtifactRevisionId`,
          ),
          terminalEdgeEvidenceRef: reference("terminalEdgeEvidenceRef"),
        };
      });
    if (entries.length !== targets.length) graphRefuse("graph_corrupt", "/evidence");
    evidence = { schemaVersion: 1, entries };
  }
  return {
    schemaVersion: 1,
    seeds,
    targets,
    complete: true,
    workUnits,
    degradationCodes,
    explanation,
    ...(evidence === undefined ? {} : { evidence }),
  };
}

export function validateResolvedGraphEvidence(value: unknown): ResolvedGraphEvidenceV1 {
  const root = graphObject(value, "");
  graphSchema(root, "/schemaVersion");
  graphExactKeys(root, [
    "schemaVersion",
    "artifactRevisionId",
    "artifact",
    "sourceId",
    "sourceVersionId",
    "sourceRevisionId",
    "locator",
    "canonicalSourceBody",
    "evidenceText",
    "canonicalSourceHash",
    "effectiveValidAt",
    "artifactLifecycle",
    "sourceLifecycleState",
    "dependency",
  ], "");
  const artifact = graphObject(graphField(root, "artifact", "/artifact"), "/artifact");
  const artifactClass = graphField(artifact, "artifactClass", "/artifact/artifactClass");
  if (artifactClass !== "node" && artifactClass !== "edge") {
    graphRefuse("graph_corrupt", "/artifact/artifactClass");
  }
  graphExactKeys(
    artifact,
    artifactClass === "node"
      ? ["artifactClass", "logicalId", "kind", "body"]
      : ["artifactClass", "logicalId", "kind", "body", "from", "to"],
    "/artifact",
  );
  const parsedArtifact: GraphEvidenceArtifactV1 = artifactClass === "node"
    ? {
        artifactClass,
        logicalId: graphString(graphField(artifact, "logicalId", "/artifact/logicalId"), "/artifact/logicalId"),
        kind: graphString(graphField(artifact, "kind", "/artifact/kind"), "/artifact/kind"),
        body: graphString(graphField(artifact, "body", "/artifact/body"), "/artifact/body"),
      }
    : {
        artifactClass,
        logicalId: artifact.logicalId === null ? null : graphString(artifact.logicalId, "/artifact/logicalId"),
        kind: graphString(graphField(artifact, "kind", "/artifact/kind"), "/artifact/kind"),
        body: artifact.body === null ? null : graphString(artifact.body, "/artifact/body"),
        from: graphString(graphField(artifact, "from", "/artifact/from"), "/artifact/from"),
        to: graphString(graphField(artifact, "to", "/artifact/to"), "/artifact/to"),
      };
  const locatorRaw = graphObject(graphField(root, "locator", "/locator"), "/locator");
  const locatorKind = graphField(locatorRaw, "kind", "/locator/kind");
  let locator: ResolvedGraphEvidenceV1["locator"];
  if (locatorKind === "whole_body") {
    graphExactKeys(locatorRaw, ["kind", "startInclusive", "endExclusive"], "/locator");
    if (locatorRaw.startInclusive !== null) graphRefuse("graph_corrupt", "/locator/startInclusive");
    if (locatorRaw.endExclusive !== null) graphRefuse("graph_corrupt", "/locator/endExclusive");
    locator = { kind: "whole_body", startInclusive: null, endExclusive: null };
  } else if (locatorKind === "utf8_bytes") {
    graphExactKeys(locatorRaw, ["kind", "startInclusive", "endExclusive"], "/locator");
    const startInclusive = graphU64(graphField(locatorRaw, "startInclusive", "/locator/startInclusive"), "/locator/startInclusive");
    const endExclusive = graphU64(graphField(locatorRaw, "endExclusive", "/locator/endExclusive"), "/locator/endExclusive");
    if (BigInt(startInclusive) > BigInt(endExclusive)) graphRefuse("graph_corrupt", "/locator");
    locator = { kind: "utf8_bytes", startInclusive, endExclusive };
  } else {
    graphRefuse("graph_corrupt", "/locator/kind");
  }
  const hashRaw = graphObject(graphField(root, "canonicalSourceHash", "/canonicalSourceHash"), "/canonicalSourceHash");
  graphExactKeys(hashRaw, ["algorithm", "digestHex"], "/canonicalSourceHash");
  if (hashRaw.algorithm !== "sha256") graphRefuse("graph_corrupt", "/canonicalSourceHash/algorithm");
  const digestHex = graphString(hashRaw.digestHex, "/canonicalSourceHash/digestHex");
  if (!/^[0-9a-f]{64}$/.test(digestHex)) graphRefuse("graph_corrupt", "/canonicalSourceHash/digestHex");
  const lifecycleRaw = graphObject(graphField(root, "artifactLifecycle", "/artifactLifecycle"), "/artifactLifecycle");
  graphExactKeys(lifecycleRaw, ["kind", "state", "superseded", "validAtEffective"], "/artifactLifecycle");
  const lifecycleKind = graphEnum(lifecycleRaw.kind, ["node", "edge"] as const, "/artifactLifecycle/kind");
  if (lifecycleKind !== artifactClass) graphRefuse("graph_corrupt", "/artifactLifecycle/kind");
  if (typeof lifecycleRaw.superseded !== "boolean") graphRefuse("graph_corrupt", "/artifactLifecycle/superseded");
  const state = lifecycleRaw.state;
  const validAtEffective = lifecycleRaw.validAtEffective;
  if (lifecycleKind === "node") {
    graphEnum(state, ["pending", "active", "deleted"] as const, "/artifactLifecycle/state");
    if (validAtEffective !== null) graphRefuse("graph_corrupt", "/artifactLifecycle/validAtEffective");
  } else {
    if (state !== null) graphRefuse("graph_corrupt", "/artifactLifecycle/state");
    if (typeof validAtEffective !== "boolean") graphRefuse("graph_corrupt", "/artifactLifecycle/validAtEffective");
  }
  const sourceLifecycleState = graphEnum(
    graphField(root, "sourceLifecycleState", "/sourceLifecycleState"),
    ["pending", "active", "deleted"] as const,
    "/sourceLifecycleState",
  );
  const artifactRevisionId = graphArtifactRevision(
    graphField(root, "artifactRevisionId", "/artifactRevisionId"),
    "/artifactRevisionId",
  );
  const sourceRevisionId = graphArtifactRevision(
    graphField(root, "sourceRevisionId", "/sourceRevisionId"),
    "/sourceRevisionId",
  );
  const dependencyRaw = graphField(root, "dependency", "/dependency");
  let dependency: SourceDependencyV1 | null = null;
  if (dependencyRaw !== null) {
    const item = graphObject(dependencyRaw, "/dependency");
    graphExactKeys(item, ["schemaVersion", "dependencyId", "sourceRevisionId", "derivedRevisionId", "registeredDependencyGeneration"], "/dependency");
    graphSchema(item, "/dependency/schemaVersion");
    const dependencySourceRevisionId = graphArtifactRevision(
      item.sourceRevisionId,
      "/dependency/sourceRevisionId",
    );
    if (dependencySourceRevisionId !== sourceRevisionId)
      graphRefuse("graph_corrupt", "/dependency/sourceRevisionId");
    const dependencyDerivedRevisionId = graphArtifactRevision(
      item.derivedRevisionId,
      "/dependency/derivedRevisionId",
    );
    if (dependencyDerivedRevisionId !== artifactRevisionId)
      graphRefuse("graph_corrupt", "/dependency/derivedRevisionId");
    dependency = {
      schemaVersion: 1,
      dependencyId: graphString(item.dependencyId, "/dependency/dependencyId"),
      sourceRevisionId: dependencySourceRevisionId,
      derivedRevisionId: dependencyDerivedRevisionId,
      registeredDependencyGeneration: graphU64(item.registeredDependencyGeneration, "/dependency/registeredDependencyGeneration"),
    };
  }
  const effectiveValidAt = graphField(root, "effectiveValidAt", "/effectiveValidAt");
  if (typeof effectiveValidAt !== "number" || !Number.isSafeInteger(effectiveValidAt)) graphRefuse("graph_corrupt", "/effectiveValidAt");
  return {
    schemaVersion: 1,
    artifactRevisionId,
    artifact: parsedArtifact,
    sourceId: graphString(graphField(root, "sourceId", "/sourceId"), "/sourceId"),
    sourceVersionId: graphString(graphField(root, "sourceVersionId", "/sourceVersionId"), "/sourceVersionId"),
    sourceRevisionId,
    locator,
    canonicalSourceBody: graphString(graphField(root, "canonicalSourceBody", "/canonicalSourceBody"), "/canonicalSourceBody"),
    evidenceText: graphString(graphField(root, "evidenceText", "/evidenceText"), "/evidenceText"),
    canonicalSourceHash: { algorithm: "sha256", digestHex },
    effectiveValidAt,
    artifactLifecycle: {
      kind: lifecycleKind,
      state: state as LifecycleState | null,
      superseded: lifecycleRaw.superseded as boolean,
      validAtEffective: validAtEffective as boolean | null,
    },
    sourceLifecycleState,
    dependency,
  };
}

export function graphRequestString(value: unknown, reason: string, path: string): asserts value is string {
  if (typeof value !== "string" || value.includes("\0")) graphRefuse(reason, path);
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) graphRefuse(reason, path);
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      graphRefuse(reason, path);
    }
  }
}

export function validateGraphExpandRequest(value: unknown): asserts value is GraphExpandRequestV1 {
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
    "includeEvidence",
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
  close(seed, ["schemaVersion", "type", "text", "rankedLimit", "logicalIds"], "/seed");
  if (seed.type === "query") {
    if ("logicalIds" in seed) graphRefuse("graph_seed_invalid", "/seed");
    if (
      typeof seed.text !== "string" ||
      !Number.isInteger(seed.rankedLimit) ||
      typeof seed.rankedLimit !== "number"
    )
      graphRefuse("graph_seed_invalid", "/seed");
    graphRequestString(seed.text, "graph_seed_invalid", "/seed/text");
  } else if (seed.type === "explicit") {
    if ("text" in seed || "rankedLimit" in seed) graphRefuse("graph_seed_invalid", "/seed");
    if (!Array.isArray(seed.logicalIds)) graphRefuse("graph_seed_invalid", "/seed/logicalIds");
    seed.logicalIds.forEach((id, index) => {
      const path = `/seed/logicalIds/${index}`;
      const object = requestObject(id, "graph_seed_invalid", path);
      close(object, ["space", "value"], path);
      if (typeof object.space !== "string" || typeof object.value !== "string")
        graphRefuse("graph_seed_invalid", path);
      graphRequestString(object.space, "graph_seed_invalid", `${path}/space`);
      graphRequestString(object.value, "graph_seed_invalid", `${path}/value`);
    });
  } else {
    graphRefuse("graph_seed_invalid", "/seed");
  }

  const context = requestObject(root.context, "graph_context_invalid", "/context");
  schema(context, "/context/schemaVersion");
  close(context, ["schemaVersion", "type", "context"], "/context");
  if (context.type !== "current" && context.type !== "frozen")
    graphRefuse("graph_context_invalid", "/context/type");
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
    graphRequestString(readContext.token, "graph_context_invalid", "/context/context/token");
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
  for (const name of ["sourceType", "kind", "status"] as const) {
    if (eligibility[name] !== undefined && eligibility[name] !== null) {
      graphRequestString(eligibility[name], "graph_context_invalid", `${currentBase}/eligibility/${name}`);
    }
  }
  if (
    eligibility.createdAfter !== undefined &&
    eligibility.createdAfter !== null &&
    (!Number.isInteger(eligibility.createdAfter) || typeof eligibility.createdAfter !== "number")
  ) graphRefuse("graph_context_invalid", `${currentBase}/eligibility/createdAfter`);
  if (eligibility.attributes !== undefined) {
    if (!Array.isArray(eligibility.attributes)) graphRefuse("graph_context_invalid", `${currentBase}/eligibility/attributes`);
    eligibility.attributes.forEach((pair, index) => {
      if (!Array.isArray(pair) || pair.length !== 2) graphRefuse("graph_context_invalid", `${currentBase}/eligibility/attributes/${index}`);
      graphRequestString(pair[0], "graph_context_invalid", `${currentBase}/eligibility/attributes/${index}/0`);
      graphRequestString(pair[1], "graph_context_invalid", `${currentBase}/eligibility/attributes/${index}/1`);
    });
  }
  if (!["incoming", "outgoing", "both"].includes(String(root.direction)))
    graphRefuse("graph_direction_invalid", "/direction");
  if (!Array.isArray(root.edgeKinds) || root.edgeKinds.some((item) => typeof item !== "string"))
    graphRefuse("graph_edge_kinds_invalid", "/edgeKinds");
  root.edgeKinds.forEach((item, index) => graphRequestString(item, "graph_edge_kinds_invalid", `/edgeKinds/${index}`));
  if (!Array.isArray(root.targetKinds) || root.targetKinds.some((item) => typeof item !== "string"))
    graphRefuse("graph_target_kinds_invalid", "/targetKinds");
  root.targetKinds.forEach((item, index) => graphRequestString(item, "graph_target_kinds_invalid", `/targetKinds/${index}`));
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
  if (root.includeEvidence !== undefined && typeof root.includeEvidence !== "boolean")
    graphRefuse("graph_context_invalid", "/includeEvidence");
  // The canonical Rust decoder remains the semantic authority for bounds,
  // duplicate seeds, eligibility values, and frozen authentication precedence.
}

export function canonicalGraphReadContext(context: ReadContextV1): GraphObject {
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

export function canonicalGraphRequest(request: GraphExpandRequestV1): GraphObject {
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
    ...(request.includeEvidence === true ? { includeEvidence: true } : {}),
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
