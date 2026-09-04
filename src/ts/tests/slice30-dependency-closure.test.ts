import test from "node:test";
import assert from "node:assert/strict";

import { Engine } from "../src/index.js";
import { DependencyClosureError } from "../src/errors.js";
import { freshDbPath } from "./helpers.js";

test("closure lookup is closed and absent ids disclose nothing", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    assert.equal(
      await engine.readDependencyClosure({
        schemaVersion: 1,
        closureOperationId: `_fdb:c:${"a".repeat(64)}`,
      }),
      null,
    );
    await assert.rejects(
      () =>
        engine.readDependencyClosure({
          schemaVersion: 2 as 1,
          closureOperationId: `_fdb:c:${"a".repeat(64)}`,
        }),
      (error: unknown) => {
        assert.ok(error instanceof DependencyClosureError);
        assert.equal(error.reason, "unsupported_schema_version");
        assert.equal(error.fieldPath, "/schemaVersion");
        return true;
      },
    );
  } finally {
    await engine.close();
  }
});

test("closure response decoder rejects unknown variants and malformed decimals", async () => {
  const engine = await Engine.open(freshDbPath());
  const native = engine._native as unknown as {
    readDependencyClosure(request: unknown): Promise<unknown>;
  };
  const original = native.readDependencyClosure;
  const valid = {
    schemaVersion: 1,
    closureOperationId: `_fdb:c:${"a".repeat(64)}`,
    root: { type: "source_revision", sourceRevisionId: "source-r1" },
    cause: "soft_deleted",
    phase: "complete",
    effectiveAtEpochS: "0",
    admittedWriteBoundary: "1",
    admittedDependencyGeneration: "1",
    affectedCount: "1",
    blockerCode: null,
    proof: {
      schemaVersion: 1,
      proofWriteBoundary: "1",
      currentActiveDependentNodes: "0",
      currentDerivedEdges: "0",
      viewEligibleDependents: "0",
      ownerlessProjectionRows: "0",
      postAdmissionRegistrations: "0",
      remainingDependencyRows: null,
      remainingCanonicalRows: null,
      remainingProjectionRows: null,
      remainingReceiptReferenceRows: null,
    },
  };
  try {
    for (const [fieldPath, response] of [
      ["/cause", { ...valid, cause: "future_cause" }],
      [
        "/proof/currentActiveDependentNodes",
        {
          ...valid,
          proof: { ...valid.proof, currentActiveDependentNodes: "00" },
        },
      ],
    ] as const) {
      native.readDependencyClosure = async () => response;
      await assert.rejects(
        () =>
          engine.readDependencyClosure({
            schemaVersion: 1,
            closureOperationId: `_fdb:c:${"a".repeat(64)}`,
          }),
        (error: unknown) => {
          assert.ok(error instanceof DependencyClosureError);
          assert.equal(error.reason, "unknown_field");
          assert.equal(error.fieldPath, fieldPath);
          return true;
        },
      );
    }
  } finally {
    native.readDependencyClosure = original;
    await engine.close();
  }
});
