import type { NativeEvidenceSearchResultV1, NativeResolvedEvidenceV1 } from "./binding.js";
import { EvidenceError } from "./errors.js";

function fail(reason: string, fieldPath: string): never {
  throw new EvidenceError(`${reason} at ${fieldPath}`, reason, fieldPath);
}

function schema(value: number, fieldPath: string): void {
  if (value !== 1) fail("unsupported_schema_version", fieldPath);
}

function variant(value: string, allowed: readonly string[], fieldPath: string): void {
  if (!allowed.includes(value)) fail("evidence_corrupt", fieldPath);
}

function u32(value: number | null | undefined, fieldPath: string, required = false): void {
  if (value === null || value === undefined) {
    if (required) fail("evidence_corrupt", fieldPath);
    return;
  }
  if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
    fail("evidence_corrupt", fieldPath);
  }
}

function finite(value: number | null | undefined, fieldPath: string, required = false): void {
  if (value === null || value === undefined) {
    if (required) fail("evidence_corrupt", fieldPath);
    return;
  }
  if (!Number.isFinite(value)) {
    fail("evidence_corrupt", fieldPath);
  }
}

function decimalU64(value: string | null | undefined, fieldPath: string): void {
  if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
    fail("evidence_corrupt", fieldPath);
  }
  try {
    if (BigInt(value) > 0xffffffffffffffffn) fail("evidence_corrupt", fieldPath);
  } catch {
    fail("evidence_corrupt", fieldPath);
  }
}

/** Validate the closed native response before constructing public SDK values. */
export function validateNativeEvidenceSearch(r: NativeEvidenceSearchResultV1): void {
  schema(r.schemaVersion, "/schemaVersion");
  if (r.evidence.length !== r.searchResult.results.length) {
    fail("evidence_corrupt", "/evidence");
  }
  r.evidence.forEach((item, index) => {
    schema(item.schemaVersion, `/evidence/${index}/schemaVersion`);
    u32(item.resultIndex, `/evidence/${index}/resultIndex`, true);
    if (item.resultIndex !== index) fail("evidence_corrupt", `/evidence/${index}/resultIndex`);
  });
  r.searchResult.results.forEach((hit, index) => {
    variant(
      hit.branch,
      ["vector", "text", "text_edge", "graph_arm"],
      `/searchResult/results/${index}/branch`,
    );
    finite(hit.score, `/searchResult/results/${index}/score`, true);
    finite(hit.ceScore, `/searchResult/results/${index}/ceScore`);
  });
}

/** Validate every version, closed union, and numeric invariant in resolved evidence. */
export function validateNativeResolvedEvidence(r: NativeResolvedEvidenceV1): void {
  schema(r.schemaVersion, "/schemaVersion");
  schema(r.projectionOrigin.schemaVersion, "/projectionOrigin/schemaVersion");
  schema(r.retrievalContribution.schemaVersion, "/retrievalContribution/schemaVersion");
  if (r.dependency) schema(r.dependency.schemaVersion, "/dependency/schemaVersion");

  variant(r.locator.kind, ["whole_body", "utf8_bytes"], "/locator/kind");
  if (r.locator.kind === "whole_body") {
    if (r.locator.startInclusive != null || r.locator.endExclusive != null) {
      fail("evidence_corrupt", "/locator");
    }
  } else {
    decimalU64(r.locator.startInclusive, "/locator/startInclusive");
    decimalU64(r.locator.endExclusive, "/locator/endExclusive");
    if (BigInt(r.locator.startInclusive!) > BigInt(r.locator.endExclusive!)) {
      fail("evidence_corrupt", "/locator");
    }
  }

  variant(r.artifactLifecycle.kind, ["node", "edge"], "/artifactLifecycle/kind");
  if (r.artifactLifecycle.kind === "node") {
    variant(
      r.artifactLifecycle.state ?? "",
      ["pending", "active", "deleted", "purged"],
      "/artifactLifecycle/state",
    );
    if (r.artifactLifecycle.validAtEffective != null)
      fail("evidence_corrupt", "/artifactLifecycle/validAtEffective");
  } else {
    if (r.artifactLifecycle.state != null) fail("evidence_corrupt", "/artifactLifecycle/state");
    if (typeof r.artifactLifecycle.validAtEffective !== "boolean") {
      fail("evidence_corrupt", "/artifactLifecycle/validAtEffective");
    }
  }
  variant(
    r.sourceLifecycleState,
    ["pending", "active", "deleted", "purged"],
    "/sourceLifecycleState",
  );
  variant(r.projectionOrigin.artifactClass, ["node", "edge"], "/projectionOrigin/artifactClass");
  if (r.projectionOrigin.artifactClass !== r.artifactLifecycle.kind) {
    fail("evidence_corrupt", "/projectionOrigin/artifactClass");
  }
  variant(
    r.projectionOrigin.representativeArm,
    ["vector", "text", "text_edge", "graph_arm"],
    "/projectionOrigin/representativeArm",
  );

  const graph = r.projectionOrigin.graphOrigin;
  if (r.projectionOrigin.representativeArm === "graph_arm") {
    if (!graph) fail("evidence_corrupt", "/projectionOrigin/graphOrigin");
  } else if (graph) {
    fail("evidence_corrupt", "/projectionOrigin/graphOrigin");
  }
  if (graph) {
    variant(
      graph.kind,
      ["entity_seed", "edge_seed", "traversal"],
      "/projectionOrigin/graphOrigin/kind",
    );
    if (graph.kind === "entity_seed") {
      if (graph.edgeArtifactRevisionId != null || graph.hopCount != null)
        fail("evidence_corrupt", "/projectionOrigin/graphOrigin");
    } else {
      if (!graph.edgeArtifactRevisionId)
        fail("evidence_corrupt", "/projectionOrigin/graphOrigin/edgeArtifactRevisionId");
      if (graph.kind === "edge_seed" && graph.hopCount != null)
        fail("evidence_corrupt", "/projectionOrigin/graphOrigin/hopCount");
      if (graph.kind === "traversal") {
        u32(graph.hopCount, "/projectionOrigin/graphOrigin/hopCount", true);
      }
    }
  }

  const contribution = r.retrievalContribution;
  u32(contribution.vectorRank, "/retrievalContribution/vectorRank");
  u32(contribution.textRank, "/retrievalContribution/textRank");
  u32(contribution.graphRank, "/retrievalContribution/graphRank");
  finite(contribution.fusedScore, "/retrievalContribution/fusedScore", true);
  finite(contribution.ceScore, "/retrievalContribution/ceScore");
  finite(contribution.blendedScore, "/retrievalContribution/blendedScore", true);
  finite(contribution.importance, "/retrievalContribution/importance");
  finite(contribution.confidence, "/retrievalContribution/confidence");
  if (r.dependency) {
    decimalU64(
      r.dependency.registeredDependencyGeneration,
      "/dependency/registeredDependencyGeneration",
    );
  }
}
