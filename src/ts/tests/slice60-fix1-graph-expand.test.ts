import assert from "node:assert/strict";
import test from "node:test";

import { GraphExpansionError, graph, type Engine, type GraphExpandRequestV1 } from "../src/index.js";

function request(): GraphExpandRequestV1 {
  return {
    schemaVersion: 1,
    seed: { schemaVersion: 1, type: "explicit", logicalIds: [{ space: "logical", value: "seed" }] },
    direction: "outgoing",
    edgeKinds: [],
    targetKinds: [],
    context: { schemaVersion: 1, type: "current", context: { schemaVersion: 1, view: {}, eligibility: {} } },
    maxDepth: 1,
    resultLimit: 1,
    maxWorkUnits: "1",
    includeExplanation: false,
  };
}

test("graph.expand rejects nested NUL, lone surrogates, and invalid eligibility locally", async () => {
  let calls = 0;
  const engine = { _native: { graphExpand: async () => { calls += 1; return "{}"; } } } as unknown as Engine;
  for (const mutate of [
    (value: any) => { value.edgeKinds = ["contains\0nul"]; },
    (value: any) => { value.seed.logicalIds[0].value = "lone\ud800surrogate"; },
    (value: any) => { value.context.context.eligibility.createdAfter = "1"; },
    (value: any) => { value.context.context.eligibility.attributes = [["a", 1]]; },
  ]) {
    const value = request();
    mutate(value);
    await assert.rejects(graph.expand(engine, value), GraphExpansionError);
  }
  assert.equal(calls, 0);
});
