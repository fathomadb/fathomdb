import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { Engine, type ActuationBatchV1 } from "../src/index.js";

const sharedFixture = JSON.parse(
  readFileSync(
    join(process.cwd(), "../../dev/fixtures/slice35-actuation-conformance-v1.json"),
    "utf8",
  ),
) as {
  request: ActuationBatchV1;
  expected: {
    requestSha256: string;
    outcome: string;
    affectedRevisionIds: string[];
    resultingWriteBoundary: string;
    resultingDependencyGeneration: string;
    pendingProjectionWriteCursors: string[];
    closureOperationIds: string[];
  };
};

test("Slice 35 shared edge fixture has exact receipt and digest", async () => {
  const dir = mkdtempSync(join(tmpdir(), "fathomdb-slice35-actuation-shared-"));
  const engine = await Engine.open(join(dir, "actuation.fathom"));
  const receipt = await engine.actuate(sharedFixture.request);
  assert.equal(receipt.requestSha256, sharedFixture.expected.requestSha256);
  assert.equal(receipt.outcome, sharedFixture.expected.outcome);
  assert.deepEqual(receipt.affectedRevisionIds, sharedFixture.expected.affectedRevisionIds);
  assert.equal(
    receipt.resultingWriteBoundary,
    sharedFixture.expected.resultingWriteBoundary,
  );
  assert.equal(
    receipt.resultingDependencyGeneration,
    sharedFixture.expected.resultingDependencyGeneration,
  );
  assert.deepEqual(
    receipt.pendingProjectionWriteCursors,
    sharedFixture.expected.pendingProjectionWriteCursors,
  );
  assert.deepEqual(receipt.closureOperationIds, sharedFixture.expected.closureOperationIds);
  assert.deepEqual(await engine.actuate(sharedFixture.request), receipt);
  await engine.close();
});
