import assert from "node:assert/strict";
import test from "node:test";

import { FathomDbError, mapNativeSearchResult } from "../src/index.js";

type Candidate = ReturnType<typeof candidateNativeSearchResult>;

function candidateNativeSearchResult() {
  return {
    projectionCursor: 1,
    softFallback: null,
    results: [
      {
        id: { space: "logical", value: "hit-1" },
        kind: "note",
        body: "needle",
        score: 1,
        branch: "text",
        sourceId: "source-1",
        ceScore: null,
      },
    ],
    explanation: {
      trace: {
        queryChars: 6,
        k: 1,
        rerankDepth: 0,
        poolN: 0,
        alpha: 0.3,
        useGraphArm: false,
        recency: false,
        embedderId: "",
        ceActive: false,
        vectorHits: 0,
        textHits: 1,
        graphHits: 0,
        droppedEdgeHits: 0,
      },
      perHit: [
        {
          id: 7,
          arm: "text",
          vectorRank: null,
          textRank: 0,
          graphRank: null,
          fusedScore: 0.5,
          ceScore: null,
          blended: 1,
          importance: null,
          confidence: null,
          structural: {
            schemaVersion: 1,
            inclusionState: "included",
            projectionOrigin: "synchronous_body_fts",
            dependencyState: "not_applicable",
            lifecycleState: "node_active",
            degradationCodes: [],
          },
        },
      ],
      correlationId: "x0123456789abcdef0123456789abcdef-0",
    },
  };
}

const cases: Array<[string, (value: Candidate) => void, string]> = [
  ["query u32", (value) => { value.explanation.trace.queryChars = true as never; }, "/trace/queryChars"],
  ["query limit", (value) => { value.explanation.trace.k = 0; }, "/trace/k"],
  ["query finite", (value) => { value.explanation.trace.alpha = Number.NaN; }, "/trace/alpha"],
  ["correlation", (value) => { value.explanation.correlationId = ""; }, "/correlationId"],
  [
    "structural presence",
    (value) => { delete (value.explanation.perHit[0] as { structural?: object }).structural; },
    "/perHit/0/structural",
  ],
  ["count", (value) => { value.explanation.perHit = []; }, "/perHit"],
  ["position arm", (value) => { value.explanation.perHit[0]!.arm = "vector"; }, "/perHit/0/arm"],
  ["position score", (value) => { value.explanation.perHit[0]!.blended = 0.5; }, "/perHit/0/blended"],
  ["unknown result arm", (value) => { value.results[0]!.branch = "future_arm"; }, "/results/0/branch"],
  [
    "duplicate positional id",
    (value) => {
      value.results.push(structuredClone(value.results[0]!));
      value.results[1]!.id.value = "hit-2";
      value.explanation.perHit.push(structuredClone(value.explanation.perHit[0]!));
    },
    "/perHit/1/id",
  ],
];

for (const [name, mutate, path] of cases) {
  test(`slice55 candidate native explanation rejects ${name}`, () => {
    const native = candidateNativeSearchResult();
    mutate(native);
    assert.throws(
      () => mapNativeSearchResult(native as never),
      (error: unknown) =>
        error instanceof FathomDbError &&
        error.message === `invalid explanation response at ${path}`,
    );
  });
}
