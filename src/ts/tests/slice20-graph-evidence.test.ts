import assert from "node:assert/strict";
import test from "node:test";

import {
  graph,
  type Engine,
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
      snapshotWriteCursor: "3",
      projectionGeneration: "1",
      validityPolicyId: "fathomdb.validity.explicit-v1",
      effectiveValidAt: 1_700_000_000,
      eligibilityCommitment: "a".repeat(64),
      integrityTag: "b".repeat(64),
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
          seeds: [],
          targets: [{
            logicalId: "target",
            kind: "claim",
            body: "target body",
            writeCursor: "2",
            origin: {
              schemaVersion: 1,
              seedLogicalId: "root",
              targetLogicalId: "target",
              hopDepth: 1,
              terminalEdgeKind: "supports",
              terminalDirection: "outgoing",
            },
          }],
          workUnits: "1",
          complete: true,
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
