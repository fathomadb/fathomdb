import assert from "node:assert/strict";
import test from "node:test";

import type { Explanation, PerHitExplain } from "../src/index.js";

test("slice55 old object literals remain source compatible", () => {
  const hit: PerHitExplain = {
    id: 1,
    arm: "text",
    vectorRank: null,
    textRank: 0,
    graphRank: null,
    fusedScore: 1,
    ceScore: null,
    blended: 1,
    importance: null,
    confidence: null,
  };
  const explanation: Explanation = { trace: {} as never, perHit: [hit] };
  assert.equal(explanation.correlationId, undefined);
  assert.equal(hit.structural, undefined);
});
