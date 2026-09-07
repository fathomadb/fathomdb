import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { graph, type Engine, type GraphExpandRequestV1 } from "../src/index.js";

const fixture = JSON.parse(
  readFileSync(resolve(process.cwd(), "../../dev/fixtures/slice60-fix2-unicode-v1.json"), "utf8"),
) as { request: string; result: string };

const request: GraphExpandRequestV1 = {
  schemaVersion: 1,
  seed: { schemaVersion: 1, type: "query", text: "café", rankedLimit: 1 },
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

test("graph.expand transports Unicode request and native result bytes without escaping", async () => {
  let requestBytes = "";
  const engine = {
    _native: {
      graphExpand: async (wire: string) => {
        requestBytes = wire;
        return fixture.result;
      },
    },
  } as unknown as Engine;

  const result = await graph.expand(engine, request);
  assert.deepEqual(Buffer.from(requestBytes), Buffer.from(fixture.request));
  assert.equal(requestBytes.includes("\\u00e9"), false);
  assert.equal(result.seeds[0]?.logicalId, "café");
});
