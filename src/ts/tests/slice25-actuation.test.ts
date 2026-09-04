import assert from "node:assert/strict";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { ActuationError, Engine, type ActuationBatchV1 } from "../src/index.js";

function request(operationId = "typescript-actuation"): ActuationBatchV1 {
  return {
    schemaVersion: 1,
    operationId,
    operations: [
      {
        type: "put_canonical_node",
        record: {
          kind: "doc",
          body: "source body",
          sourceId: "source-a",
          logicalId: "source",
          provenance: {
            schemaVersion: 1,
            role: "canonical",
            artifactRevisionId: "source-r1",
            sourceVersionId: "version-r1",
          },
        },
      },
    ],
  };
}

test("actuation commits and exact replay returns the terminal receipt", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-actuation-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const input = request();
  const receipt = await engine.actuate(input);
  assert.equal(receipt.outcome, "committed");
  assert.equal(receipt.resultingWriteBoundary, "1");
  assert.deepEqual(receipt.affectedRevisionIds, ["source-r1"]);
  assert.deepEqual(await engine.actuate(input), receipt);
  await engine.close();
});

test("actuation errors preserve reason and canonical field path", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-actuation-error-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const input = { ...request("unknown-field"), zUnknown: true };
  await assert.rejects(
    engine.actuate(input as ActuationBatchV1),
    (error: unknown) =>
      error instanceof ActuationError &&
      error.reason === "unknown_field" &&
      error.fieldPath === "/zUnknown",
  );
  await engine.close();
});

test("actuation rejects unknown nested record fields", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-actuation-record-error-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const input = request("unknown-record-field");
  const record = (
    input.operations[0] as Extract<
      (typeof input.operations)[number],
      { type: "put_canonical_node" }
    >
  ).record as Record<string, unknown>;
  record.zUnknown = true;
  await assert.rejects(
    engine.actuate(input),
    (error: unknown) =>
      error instanceof ActuationError &&
      error.reason === "unknown_field" &&
      error.fieldPath === "/operations/0/record/zUnknown",
  );
  await engine.close();
});
