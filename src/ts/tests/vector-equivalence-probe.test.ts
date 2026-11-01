// Actual vector verification refusal returns no Engine; report no longer has degraded state.

import test from "node:test";
import assert from "node:assert/strict";

import { Engine } from "../src/index.js";
import { FathomDbError, VectorEquivalenceMismatchError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

test("VectorEquivalenceMismatchError is a FathomDbError leaf carrying a reason", () => {
  const err = new VectorEquivalenceMismatchError("boom", "P1 flips=3");
  assert.ok(err instanceof FathomDbError);
  assert.equal(err.reason, "P1 flips=3");
});

test("openReport and engine have no obsolete refused-open state", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    const report = engine.openReport();
    assert.equal("denseDisabled" in report, false);
    assert.equal("denseDisabledReason" in report, false);
    assert.equal("denseDisabled" in engine, false);
    assert.equal("denseDisabledReason" in engine, false);
    assert.equal("vectorEquivalenceRefusalCount" in engine, false);
  } finally { await engine.close(); }
});

test("searchTextOnly serves the FTS-only path", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await engine.write([
      // 0.8.20 (R-20-E3): `sourceId` is mandatory on every canonical write.
      { kind: "note", body: "alpha bravo charlie", sourceId: "ts-test:vector-equivalence-probe" },
    ]);
    await engine.drain(30_000);
    const result = await engine.searchTextOnly("alpha");
    assert.ok(Array.isArray(result.results));
  } finally {
    await engine.close();
  }
});
