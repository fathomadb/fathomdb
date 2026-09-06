import assert from "node:assert/strict";
import test from "node:test";

import { FathomDbError, mapPerHitExplain } from "../src/index.js";

type NativePerHitExplain = Parameters<typeof mapPerHitExplain>[0];

test("slice55 absent native fields use legacy defaults", () => {
  const native: NativePerHitExplain = {
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
  assert.equal(mapPerHitExplain(native).structural, undefined);
});

test("slice55 direct candidate native structural value is validated", () => {
  const native = {
    id: 1,
    arm: "text",
    fusedScore: 1,
    blended: 1,
    structural: {
      schemaVersion: 1,
      inclusionState: "included",
      projectionOrigin: "synchronous_body_fts",
      dependencyState: "not_applicable",
      lifecycleState: "node_active",
      degradationCodes: [],
    },
  } as NativePerHitExplain;
  assert.equal(mapPerHitExplain(native).structural?.schemaVersion, 1);
});

test("slice55 recursively rejects incoherent structural explanation", () => {
  const native = {
    id: 1,
    arm: "text",
    fusedScore: 1,
    blended: 1,
    structural: {
      schemaVersion: 1,
      inclusionState: "included",
      projectionOrigin: "synchronous_body_fts",
      dependencyState: "not_applicable",
      lifecycleState: "node_active",
      degradationCodes: ["projection_blocked"],
    },
  } as NativePerHitExplain;
  assert.throws(
    () => mapPerHitExplain(native),
    (error: unknown) =>
      error instanceof FathomDbError &&
      error.message ===
        "invalid explanation response at /structural/inclusionState",
  );
});

test("slice55 recursively rejects duplicate structural degradation codes", () => {
  const native = {
    id: 1,
    arm: "text",
    fusedScore: 1,
    blended: 1,
    structural: {
      schemaVersion: 1,
      inclusionState: "degraded",
      projectionOrigin: "synchronous_body_fts",
      dependencyState: "not_applicable",
      lifecycleState: "node_active",
      degradationCodes: ["projection_blocked", "projection_blocked"],
    },
  } as NativePerHitExplain;
  assert.throws(
    () => mapPerHitExplain(native),
    (error: unknown) =>
      error instanceof FathomDbError &&
      error.message ===
        "invalid explanation response at /structural/degradationCodes/1",
  );
});
