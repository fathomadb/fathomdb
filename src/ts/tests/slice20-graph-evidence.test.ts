import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  Engine,
  GraphExpansionError,
  graph,
  type GraphExpandRequestV1,
  type GraphEvidenceSidecarV1,
} from "../src/index.js";

const request: GraphExpandRequestV1 = {
  schemaVersion: 1,
  seed: {
    schemaVersion: 1,
    type: "explicit",
    logicalIds: [{ space: "logical", value: "root" }],
  },
  direction: "outgoing",
  edgeKinds: ["supports"],
  targetKinds: ["claim"],
  context: {
    schemaVersion: 1,
    type: "frozen",
    context: {
      schemaVersion: 1,
      effectiveValidAt: 1_700_000_000,
      context: {
        schemaVersion: 1,
        view: { validAsOf: 1_700_000_000 },
        eligibility: {},
      },
      token: "opaque-frozen-token",
    },
  },
  maxDepth: 1,
  resultLimit: 1,
  maxWorkUnits: "10",
  includeExplanation: false,
  includeEvidence: true,
};

test("graph.expand returns the positional evidence sidecar", async () => {
  let requestWire = "";
  const evidence: GraphEvidenceSidecarV1 = {
    schemaVersion: 1,
    entries: [{
      schemaVersion: 1,
      targetIndex: 0,
      targetArtifactRevisionId: "target-r1",
      targetEvidenceRef: "fdbgev1.target",
      terminalEdgeArtifactRevisionId: "edge-r1",
      terminalEdgeEvidenceRef: "fdbgev1.edge",
    }],
  };
  const engine = {
    _native: {
      graphExpand: async (wire: string) => {
        requestWire = wire;
        return JSON.stringify({
          schemaVersion: 1,
          seeds: [{ schemaVersion: 1, logicalId: "root", seedOrdinal: 0, queryScore: null }],
          targets: [{
            schemaVersion: 1,
            logicalId: "target",
            kind: "claim",
            body: "target body",
            writeCursor: "2",
            origin: {
              schemaVersion: 1,
              seedLogicalId: "root",
              seedOrdinal: 0,
              predecessorLogicalId: "root",
              targetLogicalId: "target",
              hopCount: 1,
              terminalEdgeKind: "supports",
              terminalDirection: "outgoing",
            },
          }],
          workUnits: "1",
          complete: true,
          degradationCodes: [],
          explanation: null,
          evidence,
        });
      },
    },
  } as unknown as Engine;

  const result = await graph.expand(engine, request);
  assert.equal(JSON.parse(requestWire).includeEvidence, true);
  assert.deepEqual(result.evidence, evidence);
});

function resolvedPayload(): Record<string, unknown> {
  return {
    schemaVersion: 1,
    artifactRevisionId: "target-r1",
    artifact: { artifactClass: "node", logicalId: "target", kind: "claim", body: "target" },
    sourceId: "owner",
    sourceVersionId: "source-v1",
    sourceRevisionId: "source-r1",
    locator: { kind: "whole_body", startInclusive: null, endExclusive: null },
    canonicalSourceBody: "source",
    evidenceText: "source",
    canonicalSourceHash: { algorithm: "sha256", digestHex: "0".repeat(64) },
    effectiveValidAt: 1_700_000_000,
    artifactLifecycle: { kind: "node", state: "active", superseded: false, validAtEffective: null },
    sourceLifecycleState: "active",
    dependency: null,
  };
}

test("resolved graph evidence response is recursively closed", async () => {
  const directory = await mkdtemp(join(tmpdir(), "fathomdb-slice20-codec-"));
  const engine = await Engine.open(join(directory, "codec.fathom"), { useDefaultEmbedder: false });
  try {
    const context = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const payload = resolvedPayload();
    payload["z/future~field"] = true;
    engine._native.resolveGraphEvidence = async () => JSON.stringify(payload);
    await assert.rejects(
      engine.resolveGraphEvidence({ schemaVersion: 1, evidenceRef: "fdbgev1.ref", context }),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.reason === "graph_corrupt" &&
        error.fieldPath === "/z~1future~0field",
    );

    for (const [field, value, path] of [
      ["sourceRevisionId", "other-source-r1", "/dependency/sourceRevisionId"],
      ["derivedRevisionId", "other-target-r1", "/dependency/derivedRevisionId"],
    ] as const) {
      const incoherent = resolvedPayload();
      incoherent.dependency = {
        schemaVersion: 1,
        dependencyId: "dep-1",
        sourceRevisionId: "source-r1",
        derivedRevisionId: "target-r1",
        registeredDependencyGeneration: "1",
        [field]: value,
      };
      engine._native.resolveGraphEvidence = async () => JSON.stringify(incoherent);
      await assert.rejects(
        engine.resolveGraphEvidence({ schemaVersion: 1, evidenceRef: "fdbgev1.ref", context }),
        (error: unknown) =>
          error instanceof GraphExpansionError &&
          error.reason === "graph_corrupt" &&
          error.fieldPath === path,
      );
    }
  } finally {
    await engine.close();
    await rm(directory, { recursive: true, force: true });
  }
});

test("real engine resolves exact target and terminal edge graph evidence", async () => {
  const directory = await mkdtemp(join(tmpdir(), "fathomdb-slice20-real-"));
  const engine = await Engine.open(join(directory, "graph-evidence.fathom"), {
    useDefaultEmbedder: false,
  });
  try {
    const source = "typescript canonical graph evidence bytes";
    const derived = (artifactRevisionId: string) => ({
      schemaVersion: 1 as const,
      role: "derived" as const,
      artifactRevisionId,
      sourceVersionId: "ts-source-v1",
      sourceRevisionId: "ts-source-r1",
      sourceLocator: { kind: "whole_body" as const },
      canonicalSourceHash: {
        algorithm: "sha256" as const,
        digestHex: createHash("sha256").update(source).digest("hex"),
      },
    });
    await engine.write([
      {
        kind: "document",
        body: source,
        sourceId: "ts-owner",
        logicalId: "ts-source",
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "ts-source-r1",
          sourceVersionId: "ts-source-v1",
        },
      },
      { kind: "claim", body: "root", sourceId: "ts-owner", logicalId: "ts-root", provenance: derived("ts-root-r1") },
      { kind: "claim", body: "target", sourceId: "ts-owner", logicalId: "ts-target", provenance: derived("ts-target-r1") },
      {
        edge: {
          kind: "supports",
          from: "ts-root",
          to: "ts-target",
          sourceId: "ts-owner",
          logicalId: "ts-winner",
          provenance: derived("ts-edge-r1"),
        },
      },
    ]);
    await engine.drain(30_000);
    const context = await engine.freezeReadContext({ schemaVersion: 1, view: {}, eligibility: {} });
    const expanded = await graph.expand(engine, {
      schemaVersion: 1,
      seed: { schemaVersion: 1, type: "explicit", logicalIds: [{ space: "logical", value: "ts-root" }] },
      direction: "outgoing",
      edgeKinds: ["supports"],
      targetKinds: ["claim"],
      context: { schemaVersion: 1, type: "frozen", context },
      maxDepth: 1,
      resultLimit: 1,
      maxWorkUnits: "10",
      includeExplanation: false,
      includeEvidence: true,
    });
    const entry = expanded.evidence!.entries[0]!;
    const target = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.targetEvidenceRef,
      context,
    });
    const edge = await engine.resolveGraphEvidence({
      schemaVersion: 1,
      evidenceRef: entry.terminalEdgeEvidenceRef,
      context,
    });
    assert.deepEqual(
      [target.artifact.artifactClass, target.artifact.logicalId, target.canonicalSourceBody],
      ["node", "ts-target", source],
    );
    assert.equal(edge.artifact.artifactClass, "edge");
    if (edge.artifact.artifactClass === "edge") {
      assert.deepEqual([edge.artifact.from, edge.artifact.to], ["ts-root", "ts-target"]);
    }
  } finally {
    await engine.close();
    await rm(directory, { recursive: true, force: true });
  }
});
