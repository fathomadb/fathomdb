import test from "node:test";
import assert from "node:assert/strict";

import { rerank } from "../src/index.js";
import { native } from "../src/binding.js";

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

test("standalone rerank is identity in the feature-off build", async () => {
  assert.deepEqual(await rerank("query", PASSAGES, 2), [
    { id: 7, score: 0.75, ceScore: null },
    { id: 9, score: 0.25, ceScore: null },
  ]);
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

test("standalone rerank rejects non-string query and body before native work", async () => {
  const original = native.rerank;
  let nativeCalls = 0;
  native.rerank = async () => {
    nativeCalls += 1;
    return [];
  };
  try {
    await assert.rejects(
      () => rerank(42 as unknown as string, [], 0),
      (error: unknown) => {
        assert.ok(error instanceof TypeError);
        assert.match(error.message, /query must be a string/);
        return true;
      },
    );
    await assert.rejects(
      () =>
        rerank(
          "query",
          [{ id: 1, body: 42 as unknown as string, score: 1 }],
          0,
        ),
      (error: unknown) => {
        assert.ok(error instanceof TypeError);
        assert.match(error.message, /passage body must be a string/);
        return true;
      },
    );
    assert.equal(nativeCalls, 0, "malformed runtime types must not reach native.rerank");
  } finally {
    native.rerank = original;
  }
});
