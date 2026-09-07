import assert from "node:assert/strict";
import test from "node:test";

import { GraphExpansionError, graph, type Engine, type GraphExpandRequestV1 } from "../src/index.js";

const request: GraphExpandRequestV1 = {
  schemaVersion: 1,
  seed: { schemaVersion: 1, type: "explicit", logicalIds: [{ space: "logical", value: "root" }] },
  direction: "outgoing",
  edgeKinds: [],
  targetKinds: [],
  context: {
    schemaVersion: 1,
    type: "current",
    context: { schemaVersion: 1, view: { validAsOf: 1_700_000_000 }, eligibility: {} },
  },
  maxDepth: 0,
  resultLimit: 1,
  maxWorkUnits: "1",
  includeExplanation: false,
};

test("graph.expand preserves Rust RFC 6901 tilde-only unknown-field paths", async () => {
  const engine = { _native: { graphExpand: async () => "{}" } } as unknown as Engine;
  for (const [value, path] of [
    [{ ...request, "top~only": true }, "/top~0only"],
    [{ ...request, context: { ...request.context, "nested~only": true } }, "/context/nested~0only"],
  ] as const) {
    await assert.rejects(
      graph.expand(engine, value as unknown as GraphExpandRequestV1),
      (error: unknown) =>
        error instanceof GraphExpansionError &&
        error.reason === "unknown_field" &&
        error.fieldPath === path,
    );
  }
});
