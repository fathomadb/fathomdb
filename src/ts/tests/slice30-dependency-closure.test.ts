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
      ["/proof", { ...valid, proof: null }],
      ["/proof", { ...valid, phase: "proving" }],
      ["/blockerCode", { ...valid, phase: "incomplete", blockerCode: null }],
      [
        "/blockerCode",
        { ...valid, phase: "proving", proof: null, blockerCode: "proof_unavailable" },
      ],
      ["/phase", { ...valid, cause: "purged", phase: "proving", proof: null }],
      ["/phase", { ...valid, phase: "at_rest_pending" }],
      [
        "/proof",
        {
          ...valid,
          cause: "source_erased",
          phase: "incomplete",
          blockerCode: "wal_checkpoint",
          proof: null,
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

test("closure response decoder accepts a non-null installed status", async () => {
  const engine = await Engine.open(freshDbPath());
  try {
    await engine.write([
      {
        kind: "doc",
        body: "source body",
        sourceId: "source-a",
        logicalId: "source",
        provenance: {
          schemaVersion: 1,
          role: "canonical",
          artifactRevisionId: "source-r1",
          sourceVersionId: "source-v1",
        },
      },
      {
        kind: "fact",
        body: "derived body",
        sourceId: "source-a",
        logicalId: "derived",
        provenance: {
          schemaVersion: 1,
          role: "derived",
          artifactRevisionId: "derived-r1",
          sourceVersionId: "source-v1",
          sourceRevisionId: "source-r1",
          sourceLocator: { kind: "whole_body" },
          canonicalSourceHash: {
            algorithm: "sha256",
            digestHex: "8e0217a3ecb3eea361aa1807153c7ad853ff9e4d3e107a2d8be40ad66ceb2dc6",
          },
        },
      },
    ]);
    await engine.registerSourceDependency({
      schemaVersion: 1,
      dependencyId: "source-derived",
      sourceRevisionId: "source-r1",
      derivedRevisionId: "derived-r1",
    });
    const receipt = await engine.actuate({
      schemaVersion: 1,
      operationId: "typescript-closure-status",
      operations: [
        {
          type: "transition_lifecycle",
          logicalId: "source",
          expectedCurrentRevisionId: "source-r1",
          toState: "deleted",
        },
      ],
    });
    assert.equal(receipt.closureOperationIds.length, 1);
    const status = await engine.readDependencyClosure({
      schemaVersion: 1,
      closureOperationId: receipt.closureOperationIds[0]!,
    });
    assert.equal(status?.phase, "complete");
    assert.equal(status?.cause, "soft_deleted");
    assert.deepEqual(status?.root, {
      type: "source_revision",
      sourceRevisionId: "source-r1",
    });
    assert.equal(status?.proof?.currentActiveDependentNodes, "0");
    assert.equal(status?.proof?.ownerlessProjectionRows, "0");
  } finally {
    await engine.close();
  }
});
