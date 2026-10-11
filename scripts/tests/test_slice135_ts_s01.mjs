import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import {
  assertResult,
  makeCorpus,
  nearestRank,
} from "../slice135_ts_s01.mjs";

const expectedCorpusHashes = {
  32: "e01c7b7772a925ab9c3a80ffb1d20ae1f9338c4fb0501fa16fcc844b871338dd",
  256: "ebbd94f16b40cad184526af3bc2d725b2f433e00683e508adf530d7a925860fb",
};

test("TypeScript S01 uses the same canonical corpus bytes as Python S01", () => {
  for (const size of [32, 256]) {
    const corpus = makeCorpus(size);
    assert.equal(corpus.length, size);
    assert.equal(corpus[0].logical_id, "A");
    assert.equal(corpus[1].logical_id, "B");
    assert.equal(
      createHash("sha256").update(JSON.stringify(corpus)).digest("hex"),
      expectedCorpusHashes[size],
    );
  }
  assert.throws(() => makeCorpus(31), /32 or 256/);
});

test("TypeScript S01 rejects a wrong seeded text result", () => {
  const eligible = new Set(["A", "B"]);
  const textHit = (value, branch = "text") => ({
    id: { space: "logical", value },
    branch,
  });
  assert.doesNotThrow(() => assertResult("text", { results: [textHit("A")] }, eligible));
  assert.throws(() => assertResult("text", { results: [textHit("B")] }, eligible), /seeded exact match/);
  assert.throws(() => assertResult("text", { results: [textHit("A", "vector")] }, eligible), /seeded exact match/);
  assert.throws(() => assertResult("text", { results: [textHit("unknown")] }, eligible), /unknown id/);
});

test("TypeScript S01 rejects absent vector and hybrid branches", () => {
  const eligible = new Set(["A", "B"]);
  const hit = (value, branch) => ({ id: { space: "logical", value }, branch });
  assert.doesNotThrow(() => assertResult("vector", { results: [hit("B", "vector")] }, eligible));
  assert.throws(() => assertResult("vector", { results: [hit("A", "text")] }, eligible), /vector branch absent/);
  assert.doesNotThrow(() => assertResult("hybrid", { results: [hit("A", "vector")] }, eligible));
  assert.throws(() => assertResult("hybrid", { results: [hit("B", "vector")] }, eligible), /lexical anchor/);
  assert.throws(() => assertResult("hybrid", { results: [hit("A", "graph")] }, eligible), /unexpected branch/);
});

test("TypeScript S01 nearest-rank statistics retain the observed tail", () => {
  const samples = Array.from({ length: 1000 }, (_, index) => index + 1);
  assert.equal(nearestRank(samples, 0.5), 500);
  assert.equal(nearestRank(samples, 0.95), 950);
  assert.equal(nearestRank(samples, 0.99), 990);
  assert.equal(nearestRank(samples, 1), 1000);
  assert.throws(() => nearestRank([], 0.5), /empty/);
});
