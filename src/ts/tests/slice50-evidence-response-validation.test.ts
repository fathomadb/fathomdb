import assert from "node:assert/strict";
import test from "node:test";

import {
  validateNativeEvidenceSearch,
  validateNativeResolvedEvidence,
} from "../src/evidence-validation.js";
import { EvidenceError } from "../src/index.js";

type NativeResolved = Parameters<typeof validateNativeResolvedEvidence>[0];

function resolvedFixture(): NativeResolved {
  return {
    schemaVersion: 1,
    logicalId: "claim",
    artifactRevisionId: "claim-r1",
    sourceId: "source",
    sourceVersionId: "source-v1",
    sourceRevisionId: "source-r1",
    locator: { kind: "whole_body" },
    canonicalSourceBody: "source bytes",
    evidenceText: "source bytes",
    canonicalSourceHash: "00",
    effectiveValidAt: 1,
    artifactLifecycle: {
      kind: "node",
      state: "active",
      superseded: false,
    },
    sourceLifecycleState: "active",
    projectionOrigin: {
      schemaVersion: 1,
      artifactClass: "node",
      representativeArm: "text",
      projectionGenerationId: "generation",
    },
    retrievalContribution: {
      schemaVersion: 1,
      textRank: 0,
      fusedScore: 1,
      blendedScore: 1,
    },
    dependency: {
      schemaVersion: 1,
      dependencyId: "dependency",
      sourceRevisionId: "source-r1",
      derivedRevisionId: "claim-r1",
      registeredDependencyGeneration: "1",
    },
  };
}

function expectCorrupt(call: () => void, fieldPath: string): void {
  assert.throws(
    call,
    (error: unknown) =>
      error instanceof EvidenceError &&
      error.reason === "evidence_corrupt" &&
      error.fieldPath === fieldPath,
  );
}

test("unknown native evidence response versions fail closed", () => {
  for (const [validate, value] of [
    [validateNativeEvidenceSearch, { schemaVersion: 2 }],
    [validateNativeResolvedEvidence, { schemaVersion: 2 }],
  ] as const) {
    assert.throws(
      () => validate(value as never),
      (error: unknown) =>
        error instanceof EvidenceError &&
        error.reason === "unsupported_schema_version" &&
        error.fieldPath === "/schemaVersion",
    );
  }
});

test("unknown unions and invalid numerics in native evidence fail closed", () => {
  const badLocator = resolvedFixture();
  badLocator.locator.kind = "words";
  expectCorrupt(() => validateNativeResolvedEvidence(badLocator), "/locator/kind");

  const missingHop = resolvedFixture();
  missingHop.projectionOrigin.representativeArm = "graph_arm";
  missingHop.projectionOrigin.graphOrigin = {
    kind: "traversal",
    edgeArtifactRevisionId: "edge-r1",
  };
  expectCorrupt(
    () => validateNativeResolvedEvidence(missingHop),
    "/projectionOrigin/graphOrigin/hopCount",
  );

  const badScore = resolvedFixture();
  badScore.retrievalContribution.fusedScore = Number.NaN;
  expectCorrupt(
    () => validateNativeResolvedEvidence(badScore),
    "/retrievalContribution/fusedScore",
  );

  const missingFusedScore = resolvedFixture();
  missingFusedScore.retrievalContribution.fusedScore = null as unknown as number;
  expectCorrupt(
    () => validateNativeResolvedEvidence(missingFusedScore),
    "/retrievalContribution/fusedScore",
  );

  const missingBlendedScore = resolvedFixture();
  missingBlendedScore.retrievalContribution.blendedScore = undefined as unknown as number;
  expectCorrupt(
    () => validateNativeResolvedEvidence(missingBlendedScore),
    "/retrievalContribution/blendedScore",
  );

  const mismatchedArtifactClass = resolvedFixture();
  mismatchedArtifactClass.projectionOrigin.artifactClass = "edge";
  expectCorrupt(
    () => validateNativeResolvedEvidence(mismatchedArtifactClass),
    "/projectionOrigin/artifactClass",
  );

  const badGeneration = resolvedFixture();
  badGeneration.dependency!.registeredDependencyGeneration = "01";
  expectCorrupt(
    () => validateNativeResolvedEvidence(badGeneration),
    "/dependency/registeredDependencyGeneration",
  );
});
