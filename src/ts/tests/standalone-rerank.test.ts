import test from "node:test";
import assert from "node:assert/strict";

import { rerank } from "../src/index.js";

const PASSAGES = [
  { id: 7, body: "alpha", score: 0.75 },
  { id: 9, body: "beta", score: 0.25 },
];

test("standalone rerank depth zero is a model-free identity path", async () => {
  assert.deepEqual(await rerank("query", PASSAGES, 0), [
    { id: 7, score: 0.75, ceScore: null },
    { id: 9, score: 0.25, ceScore: null },
  ]);
  assert.deepEqual(await rerank("query", [], 10), []);
});

test("standalone rerank validates caller input before native work", async () => {
  await assert.rejects(() => rerank("query", [{ id: -1, body: "bad", score: 1 }], 0), RangeError);
  await assert.rejects(
    () => rerank("query", [{ id: 1, body: "bad", score: Number.NaN }], 0),
    RangeError,
  );
  await assert.rejects(() => rerank("query", PASSAGES, -1), RangeError);
  await assert.rejects(() => rerank("query", PASSAGES, 1.5), RangeError);
  await assert.rejects(() => rerank("query", PASSAGES, 0, { alpha: Number.NaN }), RangeError);
  await assert.rejects(() => rerank("query", PASSAGES, 0, { poolN: -1 }), RangeError);
});
